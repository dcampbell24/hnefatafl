// This file is part of hnefatafl-copenhagen.
//
// hnefatafl-copenhagen is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// hnefatafl-copenhagen is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{fmt, sync::mpsc::channel, time::Duration};

use chrono::Utc;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use rustc_hash::FxHashMap;

use crate::{
    board::InvalidMove,
    game::{EscapeVec, Game},
    game_tree::{Node, Tree},
    heat_map::HeatMap,
    play::Plae,
    role::Role,
    status::Status,
};

pub trait AI: Send {
    /// # Errors
    ///
    /// When the game is already over.
    fn generate_move(&mut self, game: &mut Game) -> anyhow::Result<GenerateMove>;
    #[allow(clippy::missing_errors_doc)]
    fn play(&mut self, game: &mut Game, play: &Plae) -> anyhow::Result<()> {
        game.play(play)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct GenerateMove {
    pub play: Plae,
    pub score: f64,
    pub delay_milliseconds: i64,
    pub loops: u64,
    pub heat_map: HeatMap,
    pub escape_vec: Option<EscapeVec>,
}

impl fmt::Display for GenerateMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{}, score: {}, delay milliseconds: {}, loops: {}",
            self.play, self.score, self.delay_milliseconds, self.loops
        )?;

        if let Some(escape_vec) = &self.escape_vec {
            write!(f, "escape_vec:\n\n{escape_vec}")?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct AiBanal;

impl AI for AiBanal {
    fn generate_move(&mut self, game: &mut Game) -> anyhow::Result<GenerateMove> {
        if game.status != Status::Ongoing {
            return Err(InvalidMove::GameOver.into());
        }

        let play = game.all_legal_plays()[0].clone();
        game.play(&play)?;

        Ok(GenerateMove {
            play,
            score: 0.0,
            delay_milliseconds: 0,
            loops: 0,
            heat_map: HeatMap::new(game.board.size()),
            escape_vec: None,
        })
    }
}

pub struct AiBasic {
    depth: u8,
    sequential: bool,
}

impl AiBasic {
    #[must_use]
    pub fn new(depth: u8, sequential: bool) -> Self {
        Self { depth, sequential }
    }
}

impl AI for AiBasic {
    fn generate_move(&mut self, game: &mut Game) -> anyhow::Result<GenerateMove> {
        let t0 = Utc::now().timestamp_millis();

        if game.status != Status::Ongoing {
            return Err(InvalidMove::GameOver.into());
        }

        if let Some(play) = game.obvious_play() {
            println!("1 turn: {} play: {play}", game.turn);

            game.play(&play)?;
            let score = match game.turn {
                Role::Attacker => f64::INFINITY,
                Role::Defender => -f64::INFINITY,
                Role::Roleless => unreachable!(),
            };

            let heat_map = HeatMap::from((&*game, &play));
            let t1 = Utc::now().timestamp_millis();
            let delay_milliseconds = t1 - t0;

            return Ok(GenerateMove {
                play,
                score,
                delay_milliseconds,
                loops: 0,
                heat_map,
                escape_vec: None,
            });
        }

        let (play, score, escape_vec) = if self.sequential {
            game.alpha_beta(
                self.depth as usize,
                self.depth,
                None,
                -f64::INFINITY,
                f64::INFINITY,
            )
        } else {
            game.alpha_beta_parallel(
                self.depth as usize,
                self.depth,
                None,
                -f64::INFINITY,
                f64::INFINITY,
            )
        };

        let play = match play {
            Some(play) => play,
            None => match &game.turn {
                Role::Attacker => Plae::AttackerResigns,
                Role::Defender => Plae::DefenderResigns,
                Role::Roleless => unreachable!(),
            },
        };

        println!("2 turn: {} play: {play}", game.turn);
        game.play(&play)?;

        let heat_map = HeatMap::from((&*game, &play));

        let t1 = Utc::now().timestamp_millis();
        let delay_milliseconds = t1 - t0;

        Ok(GenerateMove {
            play,
            score,
            delay_milliseconds,
            loops: 0,
            heat_map,
            escape_vec,
        })
    }
}

#[derive(Clone, Debug)]
pub struct AiMonteCarlo {
    duration: Duration,
    depth: u8,
}

impl Default for AiMonteCarlo {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(1),
            depth: 80,
        }
    }
}

impl AI for AiMonteCarlo {
    fn generate_move(&mut self, game: &mut Game) -> anyhow::Result<GenerateMove> {
        if game.status != Status::Ongoing {
            return Err(InvalidMove::GameOver.into());
        }

        let t0 = Utc::now().timestamp_millis();
        let mut trees = AiMonteCarlo::make_trees(game)?;
        let (tx, rx) = channel();

        trees.par_iter_mut().try_for_each_with(tx, |tx, tree| {
            let nodes = tree.monte_carlo_tree_search(self.duration, self.depth);
            tx.send(nodes)
        })?;

        let mut loops_total = 0;
        let mut nodes_master = FxHashMap::default();

        while let Ok((loops, nodes)) = rx.recv() {
            loops_total += loops;
            for mut node in nodes {
                if let Some(Plae::Play(play)) = node.clone().play {
                    nodes_master
                        .entry(play)
                        .and_modify(|node_master: &mut Node| {
                            if node_master.count == 0.0 {
                                node_master.count = 1.0;
                                node_master.score = node.score;
                            } else {
                                node_master.count += 1.0;
                                node_master.score += node.score;
                            }
                        })
                        .or_insert({
                            node.count = 1.0;
                            node
                        });
                }
            }
        }

        for node in nodes_master.values_mut() {
            node.score /= node.count;
            node.count = 1.0;
        }

        let mut nodes: Vec<_> = nodes_master.values().collect();
        nodes.sort_by(|a, b| a.score.total_cmp(&b.score));

        let turn = game.turn;
        let message = anyhow::Error::msg("The nodes are empty.");
        let node = match turn {
            Role::Attacker => nodes.last().ok_or(message)?,
            Role::Defender => nodes.first().ok_or(message)?,
            Role::Roleless => unreachable!(),
        };

        let play = node
            .play
            .as_ref()
            .ok_or(anyhow::Error::msg("A move has not been played yet."))?;

        game.play(play)?;

        let here_tree = Tree::from(game.clone());
        for tree in &mut trees {
            *tree = here_tree.clone();
        }

        let t1 = Utc::now().timestamp_millis();
        let delay_milliseconds = t1 - t0;
        let heat_map = HeatMap::from(&nodes);

        Ok(GenerateMove {
            play: play.clone(),
            score: node.score,
            delay_milliseconds,
            loops: loops_total,
            heat_map,
            escape_vec: None,
        })
    }
}

impl AiMonteCarlo {
    fn make_trees(game: &Game) -> anyhow::Result<Vec<Tree>> {
        let count = std::thread::available_parallelism()?.get();
        let mut trees = Vec::with_capacity(count);

        for _ in 0..count {
            trees.push(Tree::new(game.clone()));
        }

        Ok(trees)
    }

    #[must_use]
    pub fn new(duration: Duration, depth: u8) -> Self {
        Self { duration, depth }
    }
}
