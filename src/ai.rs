use std::{collections::HashMap, sync::mpsc::channel};

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
    fn generate_move(&mut self, game: &mut Game) -> (Option<Plae>, f64, i64, HeatMap);
    #[allow(clippy::missing_errors_doc)]
    fn play(&mut self, game: &mut Game, play: &Plae) -> anyhow::Result<()>;
}

#[derive(Clone, Debug, Default)]
pub struct AiBanal;

impl AI for AiBanal {
    fn generate_move(&mut self, game: &mut Game) -> (Option<Plae>, f64, i64, HeatMap) {
        if game.status != Status::Ongoing {
            return (None, 0.0, 0, HeatMap::default());
        }

        let play = game.all_legal_plays()[0].clone();
        match game.play(&play) {
            Ok(_captures) => {}
            Err(_) => {
                return (None, 0.0, 0, HeatMap::default());
            }
        }

        (Some(play), 0.0, 0, HeatMap::default())
    }

    fn play(&mut self, game: &mut Game, play: &Plae) -> anyhow::Result<()> {
        game.play(play)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct AiMonteCarlo {
    pub seconds_to_move: i64,
    pub size: BoardSize,
    pub trees: Vec<Tree>,
    loops: i64,
}

impl Default for AiMonteCarlo {
    fn default() -> Self {
        let size = BoardSize::default();

        Self {
            seconds_to_move: 1,
            size,
            trees: Self::make_trees(size).unwrap(),
            loops: 1_000,
        }
    }
}

impl AI for AiMonteCarlo {
    fn generate_move(&mut self, game: &mut Game) -> (Option<Plae>, f64, i64, HeatMap) {
        if game.status != Status::Ongoing {
            return (None, 0.0, 0, HeatMap::default());
        }

        let t0 = Utc::now().timestamp_millis();

        let (tx, rx) = channel();
        self.trees.par_iter_mut().for_each_with(tx, |tx, tree| {
            let nodes = tree.monte_carlo_tree_search(self.loops);
            tx.send(nodes).unwrap();
        });

        let mut nodes_master = HashMap::new();
        while let Ok(nodes) = rx.recv() {
            for node_2 in nodes {
                if let Some(Plae::Play(play)) = node_2.clone().play {
                    nodes_master
                        .entry(play)
                        .and_modify(|node_1: &mut Node| {
                            if node_1.score == 0.0 {
                                node_1.score = node_2.score;
                            } else {
                                node_1.score = f64::midpoint(node_1.score, node_2.score);
                            }
                        })
                        .or_insert(node_2);
                }
            }
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
                return (None, 0.0, 0, HeatMap::default());
            }
        }

        let here_tree = Tree::from(game.clone());
        for tree in &mut self.trees {
            *tree = here_tree.clone();
        }

        let t1 = Utc::now().timestamp_millis();
        let delay = t1 - t0;
        let heat_map = HeatMap::from(&nodes);

        (node.play.clone(), node.score, delay, heat_map)
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
    pub fn new(size: BoardSize, loops: i64) -> anyhow::Result<Self> {
        Ok(Self {
            seconds_to_move: 1,
            size,
            trees: Self::make_trees(size)?,
            loops,
        })
    }
}
