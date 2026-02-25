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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Tournament {
    pub id: u64,
    pub players: HashSet<String>,
    pub players_left: HashSet<String>,
    pub date: DateTime<Utc>,
    pub groups: Option<Vec<Vec<Arc<Mutex<Group>>>>>,
    pub tournament_games: HashMap<Id, Arc<Mutex<Group>>>,
}

impl Tournament {
    #[allow(clippy::float_cmp, clippy::too_many_lines)]
    #[must_use]
    pub fn game_over(&mut self, game: &ServerGame) -> bool {
        let mut next_round = false;

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
                    let mut previous_score = u64::MAX;

                    for (name, record) in &group.records {
                        players.push(name.clone());
                        let score = record.score();

                        if score != previous_score {
                            standings.push(Standing {
                                score,
                                players: players.clone(),
                            });
                        } else if let Some(standing) = standings.last_mut() {
                            standing.players.push(name.clone());
                        }

                        previous_score = score;
                    }

                    group.finishing_standings = standings;
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

                if finished {
                    let mut players_left = HashSet::new();

                    for group in groups {
                        if let Ok(group) = group.lock()
                            && let Some(top_score) = group.records.values().map(Record::score).max()
                        {
                            let records: Vec<_> = group.records.iter().collect();
                            for (name, record) in &records {
                                if record.score() == top_score {
                                    players_left.insert((*name).clone());
                                } else {
                                    next_round = true;
                                }
                            }
                        }
                    }

                    self.players_left = players_left;
                }
            }
        }

        next_round
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn generate_round(&mut self, accounts: &Accounts, group_size: usize) -> Vec<Group> {
        let mut players_vec = Vec::new();

        for player in &self.players_left {
            let mut rating = 1500.0;
            let mut rating_string = String::new();

            if let Some(account) = accounts.0.get(player.as_str()) {
                rating = account.rating.rating.round_ties_even();
                rating_string = account.rating.to_string_rounded();
            }

            players_vec.push((player.clone(), rating, rating_string));
        }

        let players_len = players_vec.len();
        let mut rng = rand::rng();
        players_vec.shuffle(&mut rng);
        players_vec.sort_unstable_by(|a, b| a.1.total_cmp(&b.1));

        let mut groups_number = players_len / group_size;
        let remainder = players_len % group_size;

        if remainder != 0 {
            groups_number += 1;
        }

        let mut whole_groups = groups_number;
        if remainder != 0 {
            whole_groups = whole_groups.saturating_sub(2);
        }

        let mut groups = Vec::new();
        for _ in 0..whole_groups {
            let mut group = self.new_group();

            for _ in 0..group_size {
                let player = players_vec.pop().expect("There should be a player to pop.");

                group.records.insert(
                    player.0,
                    Record {
                        rating: player.2,
                        ..Record::default()
                    },
                );
            }

            groups.push(group);
        }

        if remainder > group_size / 2 {
            let length = players_vec.len();
            let mut length_1 = length / 2;
            let length_2 = length / 2;

            if length % 2 != 0 {
                length_1 += 1;
            }

            let mut group = self.new_group();
            for _ in 0..length_1 {
                let player = players_vec.pop().expect("There should be a player to pop.");

                group.records.insert(
                    player.0,
                    Record {
                        rating: player.2,
                        ..Record::default()
                    },
                );
            }
            groups.push(group);

            let mut group = self.new_group();
            for _ in 0..length_2 {
                let player = players_vec.pop().expect("There should be a player to pop.");

                group.records.insert(
                    player.0,
                    Record {
                        rating: player.2,
                        ..Record::default()
                    },
                );
            }
            groups.push(group);
        } else {
            if groups_number != 1 {
                let mut group = self.new_group();
                for _ in 0..group_size {
                    let player = players_vec.pop().expect("There should be a player to pop.");

                    group.records.insert(
                        player.0,
                        Record {
                            rating: player.2,
                            ..Record::default()
                        },
                    );
                }
                groups.push(group);
            }

            let mut group = self.new_group();
            for _ in 0..(remainder) {
                let player = players_vec.pop().expect("There should be a player to pop.");

                group.records.insert(
                    player.0,
                    Record {
                        rating: player.2,
                        ..Record::default()
                    },
                );
            }

            groups.push(group);
        }

        groups
    }

    pub fn remove_duplicate_ids(&mut self) {
        if let Some(groups) = &self.groups {
            for round in groups {
                for group_1 in round {
                    if let Ok(group_1a) = group_1.lock() {
                        for group_2 in self.tournament_games.values_mut() {
                            if let Ok(group_2a) = group_2.clone().lock()
                                && group_1a.id == group_2a.id
                            {
                                *group_2 = group_1.clone();
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn new_group(&mut self) -> Group {
        let group = Group {
            id: self.id,
            ..Group::default()
        };

        self.id += 1;
        group
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Group {
    pub id: u64,
    pub total_games: u64,
    pub records: HashMap<String, Record>,
    pub finishing_standings: Vec<Standing>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Standing {
    pub score: u64,
    pub players: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Record {
    pub rating: String,
    pub wins: u64,
    pub losses: u64,
    pub draws: u64,
}

impl Record {
    #[must_use]
    pub fn games_count(&self) -> u64 {
        self.wins + self.losses + self.draws
    }

    pub fn reset(&mut self) {
        self.wins = 0;
        self.losses = 0;
        self.draws = 0;
    }

    #[must_use]
    pub fn score(&self) -> u64 {
        2 * self.wins + self.draws
    }
}
