use std::{collections::HashMap, fmt};

use rand::Rng;

use crate::{
    board::{BoardSize, board_11x11, board_13x13},
    game::Game,
    play::{Plae, Plays},
    status::Status,
};

#[derive(Clone, Debug)]
pub struct Tree {
    here: u128,
    pub game: Game,
    arena: HashMap<u128, Node>,
    already_played: HashMap<u64, u128>,
    next_index: u128,
}

impl Tree {
    fn insert_child(&mut self, index_parent: u128, play: Plae, game: &Game) -> u128 {
        let index = self.next_index;
        self.next_index += 1;

        let node = self
            .arena
            .get_mut(&index_parent)
            .unwrap_or_else(|| panic!("The hashmap should have the node {index_parent}."));

        node.children.push(index);
        self.already_played.insert(game.calculate_hash(), index);

        self.arena.insert(
            index,
            Node {
                index,
                play: Some(play),
                score: 0.0,
                count: 1.0,
                parent: Some(index_parent),
                children: Vec::new(),
            },
        );

        index
    }

    #[allow(clippy::too_many_lines)]
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

                here = if let Some(index) = self.already_played.get(&game.calculate_hash()) {
                    let node = self
                        .arena
                        .get_mut(index)
                        .expect("The hashmap should have the node.");
                    node.count += 1.0;
                    *index
                } else {
                    self.insert_child(here, play, &game)
                };

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

        let mut arena = HashMap::new();
        arena.insert(
            0,
            Node {
                index: 0,
                play: None,
                score: 0.0,
                count: 0.0,
                parent: None,
                children: Vec::new(),
            },
        );

        Self {
            here: 0,
            game,
            arena,
            already_played: HashMap::new(),
            next_index: 1,
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

        arena.insert(
            0,
            Node {
                index: 0,
                play: play.clone(),
                score: 0.0,
                count: 0.0,
                parent: None,
                children: Vec::new(),
            },
        );

        Self {
            here: 0,
            game,
            arena,
            already_played: HashMap::new(),
            next_index: 1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub index: u128,
    pub play: Option<Plae>,
    pub score: f64,
    pub count: f64,
    parent: Option<u128>,
    children: Vec<u128>,
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
