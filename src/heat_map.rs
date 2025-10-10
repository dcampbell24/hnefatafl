use std::{collections::HashMap, fmt};

use crate::{
    board::BoardSize,
    game_tree::Node,
    play::{Plae, Vertex},
    role::Role,
};

#[derive(Clone, Debug, Default)]
pub struct HeatMap {
    pub board_size: BoardSize,
    pub spaces: HashMap<(Role, Vertex), Vec<f64>>,
}

impl HeatMap {
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn draw(&self, role: Role) -> Vec<f64> {
        let board_size: usize = self.board_size.into();

        let mut spaces = if self.board_size == BoardSize::_11 {
            vec![0.0; 11 * 11]
        } else {
            vec![0.0; 13 * 13]
        };

        let mut froms = Vec::new();
        for key in self.spaces.keys() {
            let mut vector = self.spaces[key].clone();

            for i in &mut vector {
                if *i != 0.0 {
                    *i = f64::midpoint(*i, 1.0);
                }
            }

            let max = vector
                .iter()
                .max_by(|a, b| f64::total_cmp(a, b))
                .expect("there is at least one value");

            froms.push((key, *max));
        }

        froms.sort_by(|a, b| f64::total_cmp(&a.1, &b.1));
        let mut froms_hash_map = HashMap::new();
        let mut set_score = 1.0;
        for (play, score) in &mut froms {
            *score = set_score;
            set_score -= 0.1;
            froms_hash_map.insert(*play, *score);
        }

        // Fixme: fix the defender!
        // This is empty for the defender!
        // println!("{froms:?}");
        // Handle clicking on the board.

        for y in 0..board_size {
            for x in 0..board_size {
                if let Some(i) = froms_hash_map.get(&(
                    role,
                    Vertex {
                        x,
                        y,
                        size: self.board_size,
                    },
                )) {
                    spaces[y * board_size + x] = *i;
                } else {
                    spaces[y * board_size + x] = 0.0;
                }
            }
        }

        spaces
    }
}

impl From<&Vec<&Node>> for HeatMap {
    #[allow(clippy::float_cmp)]
    fn from(nodes: &Vec<&Node>) -> Self {
        let mut heat_map = HeatMap::default();

        for node in nodes {
            if let Some(play) = &node.play {
                match play {
                    Plae::AttackerResigns | Plae::DefenderResigns => {}
                    Plae::Play(play) => {
                        let board_index: usize = (&play.to).into();

                        // Fixme!
                        println!("{}", play.role);
                        heat_map
                            .spaces
                            .entry((play.role, play.from.clone()))
                            .and_modify(|board| {
                                let score = board
                                    .get_mut(board_index)
                                    .expect("The board should contain this space.");

                                debug_assert_eq!(*score, 0.0);
                                *score = node.score;
                            })
                            .or_insert({
                                let size: usize = play.from.size.into();
                                let mut board = vec![0.0; size * size];

                                let score = board
                                    .get_mut(board_index)
                                    .expect("The board should contain this space.");

                                *score = node.score;

                                board
                            });
                    }
                }
            }
        }

        heat_map
    }
}

impl fmt::Display for HeatMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let board_size = if self.board_size == BoardSize::_11 {
            11
        } else {
            13
        };

        for ((role, vertex), board) in &self.spaces {
            writeln!(f, "vertex: {vertex}, role: {role}")?;
            writeln!(
                f,
                "A       B       C       D       E       F       G       H       I       J       K       L       M"
            )?;
            for y in 0..board_size {
                for x in 0..board_size {
                    let score = board[y * board_size + x];
                    if score == 0.0 {
                        write!(f, "------- ")?;
                    } else {
                        write!(f, "{score:+.4} ")?;
                    }
                }
                writeln!(f)?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}
