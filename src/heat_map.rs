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
    // Fixme: handle clicking on the board.Map
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn draw(&self, role: Role) -> Vec<f64> {
        let board_size: usize = self.board_size.into();

        let mut spaces = if self.board_size == BoardSize::_11 {
            vec![0.0; 11 * 11]
        } else {
            vec![0.0; 13 * 13]
        };

        if role == Role::Roleless {
            return spaces;
        }

        let mut froms = Vec::new();
        for key in self.spaces.keys() {
            let min_max = match role {
                Role::Attacker => *self.spaces[key]
                    .iter()
                    .max_by(|a, b| f64::total_cmp(a, b))
                    .expect("there is at least one value"),
                Role::Defender => *self.spaces[key]
                    .iter()
                    .min_by(|a, b| f64::total_cmp(a, b))
                    .expect("there is at least one value"),
                Role::Roleless => unreachable!(),
            };

            froms.push((key, min_max));
        }

        froms.sort_by(|a, b| f64::total_cmp(&a.1, &b.1));
        if Role::Attacker == role {
            froms.reverse();
        }

        let mut froms_hash_map = HashMap::new();
        let mut score = 1.0;

        for (play, _) in &mut froms {
            froms_hash_map.insert(*play, score);
            score -= 0.3;
        }

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

    fn new(board_size: BoardSize) -> Self {
        Self {
            board_size,
            spaces: HashMap::new(),
        }
    }
}

impl From<&Vec<&Node>> for HeatMap {
    #[allow(clippy::float_cmp)]
    fn from(nodes: &Vec<&Node>) -> Self {
        let mut heat_map = if let Some(node) = nodes.first() {
            HeatMap::new(node.board_size)
        } else {
            HeatMap::default()
        };

        for node in nodes {
            if let Some(play) = &node.play {
                match play {
                    Plae::AttackerResigns | Plae::DefenderResigns => {}
                    Plae::Play(play) => {
                        let board_index: usize = (&play.to).into();

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
