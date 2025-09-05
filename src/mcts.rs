use std::collections::HashMap;

use rand::Rng;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{game::Game, play::Plae, role::Role, status::Status};

#[must_use]
pub fn monte_carlo_tree_search(game: &Game) -> Option<Plae> {
    let plays = game.all_legal_plays();
    let mut scores_map = HashMap::new();

    for _ in 0..25 {
        let play = plays[rand::thread_rng().gen_range(0..plays.len())].clone();
        let games: Vec<_> = (0..100).map(|_| (game.clone(), play.clone())).collect();

        let scores: Vec<_> = games
            .into_par_iter()
            .map(|(mut game_play_out, mut next_play)| {
                let mut rng = rand::thread_rng();

                loop {
                    let _captures = game_play_out.play(&next_play);

                    match game_play_out.status {
                        Status::AttackerWins => {
                            return 1;
                        }
                        Status::DefenderWins => {
                            return -1;
                        }
                        Status::Draw => {
                            return 0;
                        }
                        Status::Ongoing => {
                            let next_plays = game_play_out.all_legal_plays();
                            next_play = next_plays[rng.gen_range(0..next_plays.len())].clone();
                        }
                    }
                }
            })
            .collect();

        for score in scores {
            let entry = scores_map.entry(play.clone()).or_insert(0);
            *entry += score;
        }
    }

    match game.turn {
        Role::Attacker => scores_map
            .iter()
            .max_by(|(_, score_a), (_, score_b)| score_a.cmp(score_b))
            .map(|(play, _score)| {
                // println!("play: {play}score: {score}");
                play.clone()
            }),
        Role::Defender => scores_map
            .iter()
            .min_by(|(_, score_a), (_, score_b)| score_a.cmp(score_b))
            .map(|(play, _score)| {
                // println!("play: {play}score: {score}");
                play.clone()
            }),
        Role::Roleless => None,
    }
}
