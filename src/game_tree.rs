use std::{collections::HashMap, fmt};

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
    board::{BoardSize, board_11x11, board_13x13},
    game::Game,
    play::{Plae, Plays},
    status::Status,
};

#[derive(Clone, Debug)]
pub struct Tree {
    here: u64,
    pub game: Game,
    arena: HashMap<u64, Node>,
}

impl Tree {
    fn insert_child(&mut self, child_index: u64, parent_index: u64, play: Plae) {
        let node = self
            .arena
            .get_mut(&parent_index)
            .unwrap_or_else(|| panic!("The hashmap should have the node {parent_index}."));

        node.children.push(child_index);

        self.arena.insert(
            child_index,
            Node {
                play: Some(play),
                score: 0.0,
                count: 1.0,
                parent: Some(parent_index),
                children: Vec::new(),
            },
        );
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn monte_carlo_tree_search(&mut self, loops: i64) -> Vec<Node> {
        for _ in 0..loops {
            let mut game = self.game.clone();
            let mut here = self.here;

            for _depth in 0..80 {
                let play = if let Some(play) = game.obvious_play() {
                    game.play(&play).expect("The play should be legal!");
                    play
                } else {
                    let plays = game.all_legal_plays();
                    let index = rand::thread_rng().gen_range(0..plays.len());
                    let play = plays[index].clone();
                    game.play(&play).expect("The play should be legal!");
                    play
                };

                let child_index = game.calculate_hash();
                if let Some(node) = self.arena.get_mut(&child_index) {
                    node.count += 1.0;
                } else {
                    self.insert_child(child_index, here, play);
                }
                here = child_index;

                let gamma = 0.95;

                match game.status {
                    Status::AttackerWins => {
                        let node = self
                            .arena
                            .get_mut(&here)
                            .expect("The hashmap should have the node.");

                        node.score += 1.0;
                        let mut g = 1.0;

                        while let Some(node) = self.arena[&here].parent {
                            let real_node =
                                self.arena.get_mut(&node).expect("The node should exist!");

                            g *= gamma;
                            real_node.score += g;
                            here = node;
                        }

                        break;
                    }
                    Status::DefenderWins => {
                        let node = self
                            .arena
                            .get_mut(&here)
                            .expect("The hashmap should have the node.");

                        node.score -= 1.0;
                        let mut g = -1.0;

                        while let Some(node) = self.arena[&here].parent {
                            let real_node =
                                self.arena.get_mut(&node).expect("The node should exist!");

                            g *= gamma;
                            real_node.score += g;
                            here = node;
                        }

                        break;
                    }
                    Status::Draw => {
                        // Add zero.
                        break;
                    }
                    Status::Ongoing => {
                        // Keep going.
                    }
                }
            }
        }

        for node in self.arena.values_mut() {
            node.score /= node.count;
            node.count = 1.0;
        }

        let children = &self.arena[&self.here].children;
        children
            .iter()
            .map(|child| self.arena[child].clone())
            .collect::<Vec<_>>()
    }

    #[must_use]
    pub fn new(board_size: BoardSize) -> Self {
        let board = match board_size {
            BoardSize::_11 => board_11x11(),
            BoardSize::_13 => board_13x13(),
        };

        let game = Game {
            board,
            ..Game::default()
        };

        let hash = game.calculate_hash();
        let mut arena = HashMap::new();
        arena.insert(
            hash,
            Node {
                play: None,
                score: 0.0,
                count: 0.0,
                parent: None,
                children: Vec::new(),
            },
        );

        Self {
            here: hash,
            game,
            arena,
        }
    }
}

impl From<Game> for Tree {
    fn from(game: Game) -> Self {
        let mut arena = HashMap::new();
        let play = match &game.plays {
            Plays::PlayRecords(plays) => plays.last().unwrap(),
            Plays::PlayRecordsTimed(plays) => &plays.last().unwrap().play,
        };

        let hash = game.calculate_hash();
        arena.insert(
            hash,
            Node {
                play: play.clone(),
                score: 0.0,
                count: 0.0,
                parent: None,
                children: Vec::new(),
            },
        );

        Self {
            here: hash,
            game,
            arena,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Node {
    pub play: Option<Plae>,
    pub score: f64,
    pub count: f64,
    parent: Option<u64>,
    children: Vec<u64>,
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(play) = &self.play {
            write!(
                f,
                "play: {play}, score: {}, count: {}",
                self.score, self.count
            )
        } else {
            write!(f, "play: None")
        }
    }
}
