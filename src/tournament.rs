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

use core::fmt;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Id;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Tournament {
    pub players: HashSet<String>,
    pub date: DateTime<Utc>,
    pub tree: Option<TournamentTree>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TournamentTree {
    pub active_games: HashMap<Id, Arc<Mutex<Players>>>,
    pub rounds: Vec<Vec<Status>>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Players {
    pub round: usize,
    pub chunk: usize,
    pub player_1: Wins,
    pub player_2: Wins,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Wins {
    pub name: String,
    pub attacker: u8,
    pub defender: u8,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub enum Status {
    Lost(Player),
    #[default]
    None,
    Playing(Player),
    Ready(Player),
    Waiting,
    Won(Player),
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Player {
    pub name: String,
    pub rating: f64,
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:0}", self.name, self.rating)
    }
}
