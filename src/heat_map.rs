use std::{collections::HashMap, fmt};

use crate::{
    game_tree::Node,
    play::{Plae, Vertex},
    role::Role,
};

#[derive(Clone, Debug, Default)]
pub struct HeatMap(HashMap<(Role, Vertex), Vec<(u32, f64)>>);

impl From<&Vec<&Node>> for HeatMap {
    fn from(nodes: &Vec<&Node>) -> Self {
        let mut heat_map = HeatMap::default();

        for node in nodes {
            if let Some(play) = &node.play {
                match play {
                    Plae::AttackerResigns | Plae::DefenderResigns => {}
                    Plae::Play(play) => {
                        let board_index: usize = (&play.to).into();

                        heat_map
                            .0
                            .entry((play.role, play.from.clone()))
                            .and_modify(|board| {
                                let (count, score) = board
                                    .get_mut(board_index)
                                    .expect("The board should contain this space.");

                                *count += 1;
                                *score += node.score;
                            })
                            .or_insert({
                                let size: usize = play.from.size.into();
                                let mut board = vec![(0, 0.0); size * size];

                                let (count, score) = board
                                    .get_mut(board_index)
                                    .expect("The board should contain this space.");

                                *count = 1;
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
        for ((role, vertex), board) in &self.0 {
            let board_size = if board.len() == 11 * 11 { 11 } else { 13 };

            writeln!(f, "vertex: {vertex}, role: {role}")?;
            writeln!(
                f,
                "A       B       C       D       E       F       G       H       I       J       K       L       M"
            )?;
            for y in 0..board_size {
                for x in 0..board_size {
                    let (count, score) = board[y * board_size + x];
                    if count == 0 {
                        write!(f, "------- ")?;
                    } else {
                        let count: f64 = count.into();
                        write!(f, "{:+.4} ", score / count)?;
                    }
                }
                writeln!(f)?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}
