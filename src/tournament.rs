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

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Id;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Tournament {
    pub players: HashSet<String>,
    pub date: DateTime<Utc>,
    pub groups: Option<Vec<Vec<Group>>>,
    pub tournament_games: HashSet<Id>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Groups(Vec<Vec<String>>);

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Group {
    pub game_count: u8,
    pub records: HashMap<String, Record>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Record {
    pub wins: u8,
    pub losses: u8,
    pub draws: u8,
}

impl Record {
    pub fn reset(&mut self) {
        self.wins = 0;
        self.losses = 0;
        self.draws = 0;
    }
}
