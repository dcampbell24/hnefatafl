// This file is part of hnefatafl-copenhagen.
//
// hnefatafl-copenhagen is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// hnefatafl-copenhagen is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Utc};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::{Id, accounts::Accounts, server_game::ServerGame, status::Status};

const GROUP_SIZE: usize = 2;

// Fixme: Arc<Mutex<T>> serializes and deserializes one object to many.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Tournament {
    pub players: HashSet<String>,
    pub date: DateTime<Utc>,
    pub groups: Option<Vec<Vec<Arc<Mutex<Group>>>>>,
    pub tournament_games: HashMap<Id, Arc<Mutex<Group>>>,
}

impl Tournament {
    #[allow(clippy::float_cmp)]
    pub fn game_over(&mut self, game: &ServerGame) -> bool {
        if let Some(group) = self.tournament_games.get_mut(&game.id) {
            if let Ok(mut group) = group.lock() {
                match game.game.status {
                    Status::AttackerWins => {
                        if let Some(record) = group.records.get_mut(game.attacker.as_str()) {
                            record.wins += 1;
                        }
                        if let Some(record) = group.records.get_mut(game.defender.as_str()) {
                            record.losses += 1;
                        }
                    }
                    Status::Draw => {
                        if let Some(record) = group.records.get_mut(game.attacker.as_str()) {
                            record.draws += 1;
                        }
                        if let Some(record) = group.records.get_mut(game.defender.as_str()) {
                            record.draws += 1;
                        }
                    }
                    Status::Ongoing => {}
                    Status::DefenderWins => {
                        if let Some(record) = group.records.get_mut(game.attacker.as_str()) {
                            record.losses += 1;
                        }
                        if let Some(record) = group.records.get_mut(game.defender.as_str()) {
                            record.wins += 1;
                        }
                    }
                }

                let mut group_finished = true;
                for record in group.records.values() {
                    if group.total_games != record.games_count() {
                        group_finished = false;
                    }
                }

                if group_finished {
                    let mut standings = Vec::new();
                    let mut players = Vec::new();
                    let mut previous_score = -1.0;

                    for (name, record) in &group.records {
                        players.push(name.clone());
                        let score = record.score();
                        // The score only ever equals a whole number or a whole number plus 0.5.
                        // This is perfectly represented as a binary floating point number.
                        if score != previous_score {
                            standings.push(Standing {
                                score: record.score(),
                                players: players.clone(),
                            });
                        } else if let Some(standing) = standings.last_mut() {
                            standing.players.push(name.clone());
                        }

                        previous_score = score;
                    }

                    group.finishing_standings = standings;

                    println!("{:#?}", group.finishing_standings);
                }
            }

            self.tournament_games.remove(&game.id);

            if let Some(round) = &self.groups
                && let Some(groups) = round.last()
            {
                let mut finished = true;
                'for_loop: for group in groups {
                    if let Ok(group) = group.lock() {
                        for record in group.records.values() {
                            if group.total_games != record.games_count() {
                                finished = false;
                                break 'for_loop;
                            }
                        }
                    }
                }

                // Fixme!
                // Generate a new round or declare victors.
                if finished && groups.len() == 1 {
                    // And there is only one person on top...
                    // Game Over
                }
            }

            true
        } else {
            false
        }
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn generate_round(&mut self, accounts: &Accounts) -> Vec<Group> {
        let mut players_vec = Vec::new();

        for player in &self.players {
            let mut rating = 1500.0;

            if let Some(account) = accounts.0.get(player.as_str()) {
                rating = account.rating.rating.round_ties_even();
            }

            players_vec.push((player.clone(), rating));
        }

        let players_len = players_vec.len();
        let mut rng = rand::rng();
        players_vec.shuffle(&mut rng);
        players_vec.sort_unstable_by(|a, b| a.1.total_cmp(&b.1));

        let groups_number = players_len / GROUP_SIZE;
        let mut group_size = 1;

        if groups_number != 0 {
            while (group_size + 1) * groups_number <= players_len {
                group_size += 1;
            }
        }

        let mut remainder = players_len % group_size;
        let mut groups = Vec::new();

        for _ in 0..groups_number {
            let mut group = Group::default();

            for _ in 0..group_size {
                group.records.insert(
                    players_vec
                        .pop()
                        .expect("There should be a player to pop.")
                        .0,
                    Record::default(),
                );
            }

            if remainder > 0 {
                group.records.insert(
                    players_vec
                        .pop()
                        .expect("There should be a player to pop.")
                        .0,
                    Record::default(),
                );
                remainder = remainder.saturating_sub(1);
            }

            groups.push(group);
        }

        groups
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Group {
    pub total_games: u8,
    pub records: HashMap<String, Record>,
    pub finishing_standings: Vec<Standing>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Standing {
    pub score: f64,
    pub players: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Record {
    pub wins: u8,
    pub losses: u8,
    pub draws: u8,
}

impl Record {
    #[must_use]
    pub fn games_count(&self) -> u8 {
        self.wins + self.losses + self.draws
    }

    pub fn reset(&mut self) {
        self.wins = 0;
        self.losses = 0;
        self.draws = 0;
    }

    #[must_use]
    pub fn score(&self) -> f64 {
        f64::from(self.wins) + f64::from(self.draws) * 0.5
    }
}
