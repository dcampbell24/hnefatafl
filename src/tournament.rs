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
//
// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2025 David Campbell <david@hnefatafl.org>

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use jiff::Timestamp;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::{
    Id, accounts::Accounts, board::BoardSize, glicko::Rating, server_game::ServerGame,
    status::Status, time::TimeSettings,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupSize {
    pub size: usize,
}

impl Default for GroupSize {
    fn default() -> Self {
        Self { size: 4 }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NumberOfGames {
    pub number: usize,
}

impl Default for NumberOfGames {
    fn default() -> Self {
        Self { number: 1 }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TournamentFull {
    pub players: HashSet<String>,
    pub board_size: BoardSize,
    pub time_setting: TimeSettings,
    pub date: Option<Timestamp>,
    pub group_size: GroupSize,
    pub number_of_games: NumberOfGames,
    pub tournament: Option<Tournament>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Tournament {
    pub id: u64,
    pub players: HashSet<String>,
    pub board_size: BoardSize,
    pub time_setting: TimeSettings,
    pub date: Timestamp,
    pub group_size: usize,
    pub number_of_games: usize,
    pub groups: Vec<Vec<Arc<Mutex<Group>>>>,
    pub tournament_games: HashMap<Id, Arc<Mutex<Group>>>,
}

impl Tournament {
    #[allow(clippy::float_cmp, clippy::too_many_lines, clippy::cast_precision_loss)]
    #[must_use]
    pub fn game_over(&mut self, game: &ServerGame) -> bool {
        let mut next_round = false;

        if let Some(group) = self.tournament_games.get_mut(&game.id) {
            if let Ok(mut group) = group.lock() {
                match game.game.status {
                    Status::AttackerWins => {
                        let total_moves = (game.game.plays.len().div_ceil(2) - 1) as f64;

                        if let Some(record) = group.records.get_mut(game.attacker.as_str()) {
                            record.wins += 1;
                            record.moves += total_moves;
                        }
                        if let Some(record) = group.records.get_mut(game.defender.as_str()) {
                            record.losses += 1;
                        }
                    }
                    Status::Draw => {
                        let total_moves = (game.game.plays.len().div_ceil(2) - 1) as f64 / 2.0;

                        if let Some(record) = group.records.get_mut(game.attacker.as_str()) {
                            record.draws += 1;
                            record.moves += total_moves;
                        }
                        if let Some(record) = group.records.get_mut(game.defender.as_str()) {
                            record.draws += 1;
                            record.moves += total_moves;
                        }
                    }
                    Status::Ongoing => {}
                    Status::DefenderWins => {
                        let total_moves = (game.game.plays.len().div_ceil(2) - 1) as f64;

                        if let Some(record) = group.records.get_mut(game.attacker.as_str()) {
                            record.losses += 1;
                        }
                        if let Some(record) = group.records.get_mut(game.defender.as_str()) {
                            record.wins += 1;
                            record.moves += total_moves;
                        }
                    }
                }
            }

            self.tournament_games.remove(&game.id);

            if let Some(groups) = self.groups.last() {
                let mut finished = true;
                for group in groups {
                    if let Ok(group) = group.lock() {
                        let mut games_count = 0;
                        for record in group.records.values() {
                            games_count += record.games_count();
                        }

                        // If group not finished:
                        if group.total_games != games_count / 2 {
                            finished = false;
                            break;
                        }
                    }
                }

                if finished {
                    let mut players = HashSet::new();

                    for group in groups {
                        if let Ok(group) = group.lock()
                            && let Some(top_score) = group.records.values().map(Record::score).max()
                        {
                            let mut group_players = HashSet::new();

                            for (name, record) in &group.records {
                                if record.score() == top_score {
                                    group_players.insert(name.clone());
                                }
                            }

                            if group_players.len() > 1 {
                                let mut new_group_players = HashSet::new();

                                if let Some(min_record) = group_players
                                    .iter()
                                    .filter_map(|player: &String| group.records.get(player))
                                    .min_by(|a, b| a.moves.total_cmp(&b.moves))
                                {
                                    for player in &group_players {
                                        if let Some(record) = group.records.get(player)
                                            && record.moves == min_record.moves
                                        {
                                            new_group_players.insert(player.clone());
                                        }
                                    }

                                    group_players = new_group_players;
                                }
                            }

                            for player in group_players {
                                players.insert(player);
                            }
                        }
                    }

                    if players.len() < self.players.len() {
                        self.players = players;
                        next_round = true;
                    }
                }
            }
        }

        next_round
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn generate_round(&mut self, accounts: &Accounts) -> Vec<Group> {
        let mut players_vec = Vec::new();

        for player in &self.players {
            let mut rating = Rating::default();

            if let Some(account) = accounts.0.get(player.as_str()) {
                rating = account.rating.clone();
            }

            players_vec.push((player.clone(), rating));
        }

        let players_len = players_vec.len();
        let mut rng = rand::rng();
        players_vec.shuffle(&mut rng);
        players_vec.sort_unstable_by(|a, b| b.1.rating.total_cmp(&a.1.rating));

        let mut groups_number = players_len / self.group_size;
        let mut remainder = 0;

        if groups_number == 0 {
            groups_number = 1;
            self.group_size = players_len;
        } else {
            remainder = players_len % self.group_size;
        }

        let mut groups = Vec::new();

        for _ in 0..groups_number {
            let mut group = self.new_group();

            for _ in 0..self.group_size {
                let player = players_vec.pop().expect("There should be a player to pop.");

                group.records.insert(
                    player.0,
                    Record {
                        rating: player.1,
                        ..Record::default()
                    },
                );
            }

            groups.push(group);
        }

        if remainder != 0
            && let Some(mut group_1) = groups.pop()
        {
            for _ in 0..remainder {
                let player = players_vec.pop().expect("There should be a player to pop.");

                group_1.records.insert(
                    player.0,
                    Record {
                        rating: player.1,
                        ..Record::default()
                    },
                );
            }

            let len = group_1.records.len();
            let mut records_1: Vec<_> = group_1.records.into_iter().take(len).collect();

            records_1.shuffle(&mut rng);
            records_1.sort_unstable_by(|(_, record_1), (_, record_2)| {
                record_2.rating.rating.total_cmp(&record_1.rating.rating)
            });

            let records_2 = records_1.split_off(len / 2);

            let mut records_1a = HashMap::new();
            for (name, record) in records_1 {
                records_1a.insert(name, record);
            }
            group_1.records = records_1a;

            let mut group_2 = self.new_group();
            let mut records_2a = HashMap::new();
            for (name, record) in records_2 {
                records_2a.insert(name, record);
            }
            group_2.records = records_2a;

            groups.push(group_1);
            groups.push(group_2);
        }

        groups
    }

    #[must_use]
    pub fn is_tournament_game(&self, id: &Id) -> bool {
        self.tournament_games.contains_key(id)
    }

    pub fn remove_duplicate_ids(&mut self) {
        for round in &self.groups {
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
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Standing {
    pub score: u64,
    pub players: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Record {
    pub rating: Rating,
    pub wins: u64,
    pub losses: u64,
    pub draws: u64,
    pub moves: f64,
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
