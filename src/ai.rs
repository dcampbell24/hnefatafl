use std::sync::mpsc::channel;

use chrono::Utc;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};

use crate::{
    board::BoardSize,
    game::Game,
    game_tree::{HeatMap, Tree},
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
        let mut nodes: Vec<_> = rx.iter().flatten().collect();
        nodes.sort_by(|a, b| a.score.total_cmp(&b.score));

        let heat_map = HeatMap::from(&nodes);

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

        let hash = game.calculate_hash();
        let mut here_tree = Tree::new(game.board.size());
        for tree in &self.trees {
            if hash == tree.game.calculate_hash() {
                here_tree = tree.clone();
            }
        }
        for tree in &mut self.trees {
            if hash != tree.game.calculate_hash() {
                *tree = here_tree.clone();
            }
        }

        let t1 = Utc::now().timestamp_millis();
        let delay = t1 - t0;
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
