use std::{collections::HashMap, fmt, sync::mpsc::channel, time::Duration};

use chrono::Utc;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};

use crate::{
    board::BoardSize,
    game::Game,
    game_tree::{Node, Tree},
    heat_map::HeatMap,
    play::Plae,
    role::Role,
    status::Status,
};

pub trait AI {
    fn generate_move(&mut self, game: &mut Game) -> GenerateMove;
    #[allow(clippy::missing_errors_doc)]
    fn play(&mut self, game: &mut Game, play: &Plae) -> anyhow::Result<()>;
}

#[derive(Clone, Debug)]
pub struct GenerateMove {
    pub play: Option<Plae>,
    pub score: f64,
    pub delay_milliseconds: i64,
    pub loops: u64,
    pub heat_map: HeatMap,
}

impl fmt::Display for GenerateMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(play) = &self.play {
            writeln!(
                f,
                "{play}, score: {}, delay milliseconds: {}, loops: {}",
                self.score, self.delay_milliseconds, self.loops
            )
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct AiBanal;

impl AI for AiBanal {
    fn generate_move(&mut self, game: &mut Game) -> GenerateMove {
        let mut generate_move = GenerateMove {
            play: None,
            score: 0.0,
            delay_milliseconds: 0,
            loops: 0,
            heat_map: HeatMap::default(),
        };

        if game.status != Status::Ongoing {
            return generate_move;
        }

        let play = game.all_legal_plays()[0].clone();
        match game.play(&play) {
            Ok(_captures) => {}
            Err(_) => {
                return generate_move;
            }
        }

        generate_move.play = Some(play);
        generate_move
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
    seconds: Duration,
}

impl Default for AiMonteCarlo {
    fn default() -> Self {
        let size = BoardSize::default();

        Self {
            size,
            trees: Self::make_trees(size).unwrap(),
            seconds: Duration::from_secs(1),
        }
    }
}

impl AI for AiMonteCarlo {
    fn generate_move(&mut self, game: &mut Game) -> GenerateMove {
        let generate_move = GenerateMove {
            play: None,
            score: 0.0,
            delay_milliseconds: 0,
            loops: 0,
            heat_map: HeatMap::default(),
        };

        if game.status != Status::Ongoing {
            return generate_move;
        }

        let t0 = Utc::now().timestamp_millis();

        for tree in &mut self.trees {
            *tree = Tree::from(game.clone());
        }

        let (tx, rx) = channel();
        self.trees.par_iter_mut().for_each_with(tx, |tx, tree| {
            let nodes = tree.monte_carlo_tree_search(self.seconds);
            tx.send(nodes).unwrap();
        });

        let mut loops_total = 0;
        let mut nodes_master = HashMap::new();
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
        match game.play(play) {
            Ok(_captures) => {}
            Err(_) => {
                return generate_move;
            }
        }

        let here_tree = Tree::from(game.clone());
        for tree in &mut self.trees {
            *tree = here_tree.clone();
        }

        let t1 = Utc::now().timestamp_millis();
        let delay_milliseconds = t1 - t0;
        let heat_map = HeatMap::from(&nodes);

        GenerateMove {
            play: node.play.clone(),
            score: node.score,
            delay_milliseconds,
            loops: loops_total,
            heat_map,
        }
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
    fn make_trees(size: BoardSize) -> anyhow::Result<Vec<Tree>> {
        let count = std::thread::available_parallelism()?.get();
        let mut trees = Vec::with_capacity(count);

        for _ in 0..count {
            trees.push(Tree::new(size));
        }

        Ok(trees)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn new(size: BoardSize, seconds: Duration) -> anyhow::Result<Self> {
        Ok(Self {
            size,
            trees: Self::make_trees(size)?,
            seconds,
        })
    }
}
