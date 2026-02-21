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
use serde::{Deserialize, Serialize};

use crate::Id;

// Fixme: Arc<Mutex<T>> serializes and deserializes one object to many.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Tournament {
    pub players: HashSet<String>,
    pub date: DateTime<Utc>,
    pub groups: Option<Vec<Vec<Arc<Mutex<Group>>>>>,
    pub tournament_games: HashMap<Id, Arc<Mutex<Group>>>,
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
