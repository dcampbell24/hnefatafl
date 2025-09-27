use std::{collections::HashMap, fmt};

use rand::Rng;

use crate::{
    board::{BoardSize, board_11x11, board_13x13},
    game::Game,
    play::Plae,
    role::Role,
    status::Status,
};

#[derive(Clone, Debug)]
pub struct Tree {
    here: u128,
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
                game: game.clone(),
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
    pub fn monte_carlo_tree_search(&mut self, loops: u32) -> Vec<Node> {
        let game = self.here_game();

        for _ in 0..loops {
            let mut game = game.clone();
            let mut here = self.here;

            for _depth in 0..80 {
                let play = if let Some(play) = game.obvious_play() {
                    let _captures = game.play(&play);
                    play
                } else {
                    let plays = game.all_legal_plays();
                    let index = rand::thread_rng().gen_range(0..plays.len());
                    let play = plays[index].clone();
                    let _captures = game.play(&play);
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
        let mut child_nodes = children
            .iter()
            .map(|child| self.arena[child].clone())
            .collect::<Vec<_>>();

        child_nodes.sort_by(|a, b| a.score.total_cmp(&b.score));

        let here = match game.turn {
            Role::Attacker => child_nodes.last().map(|node| node.index),
            Role::Defender => child_nodes.first().map(|node| node.index),
            Role::Roleless => None,
        };

        if let Some(here) = here {
            let mut children = Vec::new();
            for child in &self
                .arena
                .get(&self.here)
                .expect("The here node should exist")
                .children
            {
                if *child != here {
                    children.push(*child);
                }
            }

            while let Some(child) = children.pop() {
                if let Some(node) = self.arena.remove(&child) {
                    for child in node.children {
                        children.push(child);
                    }
                }
            }

            self.here = here;
            child_nodes
        } else {
            Vec::new()
        }
    }

    #[must_use]
    pub fn here_game(&self) -> Game {
        self.arena[&self.here].game.clone()
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
                game,
                play: None,
                score: 0.0,
                count: 0.0,
                parent: None,
                children: Vec::new(),
            },
        );

        Self {
            here: 0,
            arena,
            already_played: HashMap::new(),
            next_index: 1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    index: u128,
    game: Game,
    pub play: Option<Plae>,
    pub score: f64,
    count: f64,
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
