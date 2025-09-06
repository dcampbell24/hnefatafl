use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

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
    here: usize,
    arena: Vec<Node>,
    already_played: HashMap<u64, usize>,
}

impl Tree {
    fn insert_child(&mut self, index_parent: usize, play: Plae, game: &Game) -> usize {
        let index = self.arena.len();
        self.arena[index_parent].children.push(index);
        self.already_played
            .insert(calculate_hash(&game.board), index);

        self.arena.push(Node {
            index,
            game: game.clone(),
            play: Some(play),
            score: 0,
            parent: Some(index_parent),
            children: Vec::new(),
        });

        index
    }

    #[must_use]
    pub fn monte_carlo_tree_search(&mut self, loops: u32) -> (Option<Plae>, i32) {
        let game = self.here_game();

        for _ in 0..loops {
            let mut game = game.clone();
            let mut here = self.here;

            for _depth in 0..100 {
                let plays = game.all_legal_plays();
                let index = rand::thread_rng().gen_range(0..plays.len());
                let play = plays[index].clone();
                let _captures = game.play(&play);

                here = if let Some(index) = self.already_played.get(&calculate_hash(&game.board)) {
                    *index
                } else {
                    self.insert_child(here, play, &game)
                };

                let status = &self.arena[self.here].game.status;

                match status {
                    Status::AttackerWins => {
                        self.arena[here].score += 1;
                        while let Some(node) = self.arena[here].parent {
                            here = node;
                        }

                        break;
                    }
                    Status::DefenderWins => {
                        self.arena[here].score -= 1;
                        while let Some(node) = self.arena[here].parent {
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

        let children = &self.arena[self.here].children;
        let here = match game.turn {
            Role::Attacker => children
                .iter()
                .map(|child| &self.arena[*child])
                .max_by(|a, b| a.score.cmp(&b.score))
                .map(|node| (node.index, node.score)),
            Role::Defender => children
                .iter()
                .map(|child| &self.arena[*child])
                .min_by(|a, b| a.score.cmp(&b.score))
                .map(|node| (node.index, node.score)),
            Role::Roleless => None,
        };

        if let Some(here) = here {
            let (here, score) = here;
            self.here = here;
            (self.arena[self.here].play.clone(), score)
        } else {
            (None, 0)
        }
    }

    #[must_use]
    fn here_game(&self) -> Game {
        self.arena[self.here].game.clone()
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

        Self {
            here: 0,
            arena: vec![Node {
                index: 0,
                game,
                play: None,
                score: 0,
                parent: None,
                children: Vec::new(),
            }],
            already_played: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    index: usize,
    game: Game,
    play: Option<Plae>,
    score: i32,
    parent: Option<usize>,
    children: Vec<usize>,
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
