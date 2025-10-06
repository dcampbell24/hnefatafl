use std::{collections::HashMap, fmt};

use crate::{
    game_tree::Node,
    play::{Plae, Vertex},
    role::Role,
};

#[derive(Clone, Debug, Default)]
pub struct HeatMap(HashMap<(Role, Vertex), Vec<f64>>);

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
                                let score = board
                                    .get_mut(board_index)
                                    .expect("The board should contain this space.");

                                println!("score: {score}, score_add {}", node.score);
                                *score = node.score;
                            })
                            .or_insert({
                                let size: usize = play.from.size.into();
                                let mut board = vec![0.0; size * size];

                                let score = board
                                    .get_mut(board_index)
                                    .expect("The board should contain this space.");

                                println!("score: {}", node.score);
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
                    let space = board[y * board_size + x];
                    if space == 0.0 {
                        write!(f, "------- ")?;
                    } else {
                        write!(f, "{space:+.4} ")?;
                    }
                }
                writeln!(f)?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}
