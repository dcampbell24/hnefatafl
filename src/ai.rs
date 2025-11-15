use std::{fmt, sync::mpsc::channel, time::Duration};

use chrono::Utc;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use rustc_hash::FxHashMap;

use crate::{
    board::{BoardSize, InvalidMove},
    game::Game,
    game_tree::{Node, Tree},
    heat_map::HeatMap,
    play::Plae,
    role::Role,
    status::Status,
};

pub trait AI {
    /// # Errors
    ///
    /// When the game is already over.
    fn generate_move(&mut self, game: &mut Game) -> anyhow::Result<GenerateMove>;
    #[allow(clippy::missing_errors_doc)]
    fn play(&mut self, game: &mut Game, play: &Plae) -> anyhow::Result<()>;
}

#[derive(Clone, Debug)]
pub struct GenerateMove {
    pub play: Plae,
    pub score: f64,
    pub delay_milliseconds: i64,
    pub loops: u64,
    pub heat_map: HeatMap,
}

impl fmt::Display for GenerateMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{}, score: {}, delay milliseconds: {}, loops: {}",
            self.play, self.score, self.delay_milliseconds, self.loops
        )
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
        })
    }

    fn play(&mut self, game: &mut Game, play: &Plae) -> anyhow::Result<()> {
        game.play(play)?;
        Ok(())
    }
}

pub struct AiBasic {
    _duration: Duration,
    depth: u8,
}

impl AiBasic {
    #[must_use]
    pub fn new(duration: Duration, depth: u8) -> Self {
        Self {
            _duration: duration,
            depth,
        }
    }
}

impl AI for AiBasic {
    fn generate_move(&mut self, game: &mut Game) -> anyhow::Result<GenerateMove> {
        let t0 = Utc::now().timestamp_millis();

        if game.status != Status::Ongoing {
            return Err(InvalidMove::GameOver.into());
        }

        if let Some(play) = game.obvious_play() {
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
            });
        }

        let (play, score) = game.alpha_beta(
            self.depth as usize,
            self.depth,
            -f64::INFINITY,
            f64::INFINITY,
        );
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
        })
    }

    fn play(&mut self, game: &mut Game, play: &Plae) -> anyhow::Result<()> {
        game.play(play)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct AiMonteCarlo {
    pub size: BoardSize,
    pub trees: Vec<Tree>,
    duration: Duration,
    depth: u8,
}

impl Default for AiMonteCarlo {
    fn default() -> Self {
        let game = Game::default();

        Self {
            size: game.board.size(),
            trees: Self::make_trees(&game).unwrap(),
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

        for tree in &mut self.trees {
            *tree = Tree::from(game.clone());
        }

        let (tx, rx) = channel();
        self.trees.par_iter_mut().for_each_with(tx, |tx, tree| {
            let nodes = tree.monte_carlo_tree_search(self.duration, self.depth);
            tx.send(nodes).unwrap();
        });

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
        let node = match turn {
            Role::Attacker => nodes.last().unwrap(),
            Role::Defender => nodes.first().unwrap(),
            Role::Roleless => unreachable!(),
        };

        let play = node.play.as_ref().unwrap();
        game.play(play)?;

        let here_tree = Tree::from(game.clone());
        for tree in &mut self.trees {
            *tree = here_tree.clone();
        }

        let t1 = Utc::now().timestamp_millis();
        let delay_milliseconds = t1 - t0;
        let heat_map = HeatMap::from(&nodes);

        Ok(GenerateMove {
            play: node.play.clone().expect("there should be a play"),
            score: node.score,
            delay_milliseconds,
            loops: loops_total,
            heat_map,
        })
    }

    fn play(&mut self, game: &mut Game, play: &Plae) -> anyhow::Result<()> {
        game.play(play)?;
        let tree_game = Tree::from(game.clone());
        for tree in &mut self.trees {
            *tree = tree_game.clone();
        }

        Ok(())
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

    #[allow(clippy::missing_errors_doc)]
    pub fn new(game: &Game, duration: Duration, depth: u8) -> anyhow::Result<Self> {
        Ok(Self {
            size: game.board.size(),
            trees: Self::make_trees(game)?,
            duration,
            depth,
        })
    }
}
