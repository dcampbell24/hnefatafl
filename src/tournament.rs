// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use core::fmt;
use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Tournament {
    pub players: HashSet<String>,
    pub date: DateTime<Utc>,
    pub tree: Option<TournamentTree>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TournamentTree {
    pub rounds: Vec<Vec<Status>>,
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Status {
    pub processed: bool,
    pub status: StatusEnum,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub enum StatusEnum {
    Lost(Player),
    #[default]
    None,
    Ready(Player),
    Waiting,
    Won(Player),
}
