use std::{cmp::Ordering, collections::HashMap, fmt};

use crate::{
    board::BoardSize,
    game_tree::Node,
    play::{Plae, Vertex},
    role::Role,
};

#[derive(Clone, Copy, Debug, Default)]
pub enum Heat {
    Ranked(u8),
    Score(f64),
    #[default]
    UnRanked,
}

// It would be Color but iced is only in the examples. This is the alpha value.
#[allow(clippy::cast_possible_truncation)]
impl From<Heat> for f32 {
    fn from(cell: Heat) -> Self {
        match cell {
            Heat::Score(score) => score as f32,
            Heat::UnRanked => 0.25,
            Heat::Ranked(rank) => match rank {
                0 => 1.0,
                1 => 0.5,
                2 => 0.25,
                3 => 0.125,
                4 => 0.0625,
                _ => 0.0,
            },
        }
    }
}

impl Ord for Heat {
    fn cmp(&self, other: &Self) -> Ordering {
        match self {
            Self::Ranked(rank) => match other {
                Self::Ranked(rank_other) => rank.cmp(rank_other),
                Self::Score(_) | Self::UnRanked => Ordering::Greater,
            },
            Self::Score(score) => match other {
                Self::Ranked(_) => Ordering::Less,
                Self::Score(score_other) => score.total_cmp(score_other),
                Self::UnRanked => Ordering::Greater,
            },
            Self::UnRanked => match other {
                Self::Ranked(_) | Self::Score(_) => Ordering::Less,
                Self::UnRanked => Ordering::Equal,
            },
        }
    }
}

impl PartialOrd for Heat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Heat {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Ranked(rank) => match other {
                Self::Ranked(rank_other) => rank == rank_other,
                Self::Score(_) | Self::UnRanked => false,
            },
            Self::Score(score) => match other {
                Self::Ranked(_) | Self::UnRanked => false,
                Self::Score(score_other) => score == score_other,
            },
            Self::UnRanked => match other {
                Self::Ranked(_) | Self::Score(_) => false,
                Self::UnRanked => true,
            },
        }
    }
}

impl Eq for Heat {}

#[derive(Clone, Debug, Default)]
pub struct HeatMap {
    pub board_size: BoardSize,
    pub spaces: HashMap<(Role, Vertex), Vec<Heat>>,
}

impl HeatMap {
    #[allow(clippy::type_complexity)]
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn draw(&self, role: Role) -> (Vec<Heat>, HashMap<(Role, Vertex), Vec<Heat>>) {
        let board_size: usize = self.board_size.into();

        let mut spaces_from = if self.board_size == BoardSize::_11 {
            vec![Heat::default(); 11 * 11]
        } else {
            vec![Heat::default(); 13 * 13]
        };

        if role == Role::Roleless {
            return (spaces_from, HashMap::new());
        }

        let mut froms = Vec::new();
        for key in self.spaces.keys() {
            let min_max = match role {
                Role::Attacker => *self.spaces[key]
                    .iter()
                    .filter(|heat| **heat != Heat::UnRanked)
                    .max_by(|a, b| Heat::cmp(a, b))
                    .expect("there is at least one value"),
                Role::Defender => *self.spaces[key]
                    .iter()
                    .filter(|heat| **heat != Heat::UnRanked)
                    .min_by(|a, b| Heat::cmp(a, b))
                    .expect("there is at least one value"),
                Role::Roleless => unreachable!(),
            };

            froms.push((key, min_max));
        }

        froms.sort_by(|a, b| Heat::cmp(&a.1, &b.1));
        if Role::Attacker == role {
            froms.reverse();
        }

        let mut froms_hash_map = HashMap::new();
        for ((play, _), rank) in froms.iter_mut().zip(0u8..) {
            froms_hash_map.insert(*play, rank);
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
                    spaces_from[y * board_size + x] = Heat::Ranked(*i);
                } else {
                    spaces_from[y * board_size + x] = Heat::UnRanked;
                }
            }
        }

        let mut spaces_to = self.spaces.clone();
        for ((role, _vertex), board) in &mut spaces_to {
            let mut played_on = Vec::new();

            for y in 0..board_size {
                for x in 0..board_size {
                    let heat = board[y * board_size + x];
                    if let Heat::Score(score) = heat {
                        let vertex = Vertex {
                            size: BoardSize::try_from(board_size)
                                .expect("we should have a valid board size"),
                            x,
                            y,
                        };
                        played_on.push((vertex, role, score));
                    }
                }
            }

            played_on.sort_by(|a, b| f64::total_cmp(&a.2, &b.2));

            if *role == Role::Attacker {
                played_on.reverse();
            }

            let mut rank = 0;
            for (vertex, _, _) in played_on {
                let heat = &mut board[vertex.y * board_size + vertex.x];
                if let Heat::Score(_) = heat {
                    *heat = Heat::Ranked(rank);
                    rank += 1;
                }
            }
        }

        (spaces_from, spaces_to)
    }

    #[must_use]
    pub fn new(board_size: BoardSize) -> Self {
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
                            .entry((play.role, play.from))
                            .and_modify(|board| {
                                let score = board
                                    .get_mut(board_index)
                                    .expect("The board should contain this space.");

                                debug_assert_eq!(*score, Heat::UnRanked);
                                *score = Heat::Score(node.score);
                            })
                            .or_insert({
                                let size: usize = play.from.size.into();
                                let mut board = vec![Heat::default(); size * size];

                                let score = board
                                    .get_mut(board_index)
                                    .expect("The board should contain this space.");

                                *score = Heat::Score(node.score);

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

            match self.board_size {
                BoardSize::_11 => writeln!(
                    f,
                    "   A       B       C       D       E       F       G       H       I       J       K"
                )?,
                BoardSize::_13 => writeln!(
                    f,
                    "   A       B       C       D       E       F       G       H       I       J       K       L       M"
                )?,
            }

            for y in 0..board_size {
                match self.board_size {
                    BoardSize::_11 => write!(f, "{:02} ", 11 - y)?,
                    BoardSize::_13 => write!(f, "{:02} ", 13 - y)?,
                }

                for x in 0..board_size {
                    let score = board[y * board_size + x];
                    if let Heat::Score(score) = score {
                        write!(f, "{score:+.4} ")?;
                    } else {
                        write!(f, "------- ")?;
                    }
                }
                writeln!(f)?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}
