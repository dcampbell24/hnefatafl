use std::collections::HashMap;

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
                score: 0,
                parent: Some(index_parent),
                children: Vec::new(),
            },
        );

        index
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn monte_carlo_tree_search(&mut self, loops: u32) -> (Option<Plae>, i32) {
        let game = self.here_game();

        for _ in 0..loops {
            let mut game = game.clone();
            let mut here = self.here;

            for _depth in 0..80 {
                let plays = game.all_legal_plays();
                let index = rand::thread_rng().gen_range(0..plays.len());
                let play = plays[index].clone();
                let _captures = game.play(&play);

                here = if let Some(index) = self.already_played.get(&game.calculate_hash()) {
                    *index
                } else {
                    self.insert_child(here, play, &game)
                };

                let game = &self.arena[&self.here].game;
                let mut status = game.status.clone();
                if status == Status::Ongoing {
                    status = game.obvious_play();
                }

                match status {
                    Status::AttackerWins => {
                        let node = self
                            .arena
                            .get_mut(&here)
                            .expect("The hashmap should have the node.");
                        node.score += 1;

                        while let Some(node) = self.arena[&here].parent {
                            here = node;
                        }

                        break;
                    }
                    Status::DefenderWins => {
                        let node = self
                            .arena
                            .get_mut(&here)
                            .expect("The hashmap should have the node.");
                        node.score -= 1;

                        while let Some(node) = self.arena[&here].parent {
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

        let children = &self.arena[&self.here].children;
        let here = match game.turn {
            Role::Attacker => children
                .iter()
                .map(|child| &self.arena[child])
                .max_by(|a, b| a.score.cmp(&b.score))
                .map(|node| (node.index, node.score)),
            Role::Defender => children
                .iter()
                .map(|child| &self.arena[child])
                .min_by(|a, b| a.score.cmp(&b.score))
                .map(|node| (node.index, node.score)),
            Role::Roleless => None,
        };

        if let Some(here) = here {
            let (here, score) = here;

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
            (self.arena[&self.here].play.clone(), score)
        } else {
            (None, 0)
        }
    }

    #[must_use]
    fn here_game(&self) -> Game {
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
                score: 0,
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
    play: Option<Plae>,
    score: i32,
    parent: Option<u128>,
    children: Vec<u128>,
}
