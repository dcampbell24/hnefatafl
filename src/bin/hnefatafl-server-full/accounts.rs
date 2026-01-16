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
    fmt,
};

use hnefatafl_copenhagen::{email::Email, glicko::Rating};
use serde::{Deserialize, Serialize};

use crate::Id;

impl fmt::Display for Accounts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut accounts = Vec::new();
        for (name, account) in &self.0 {
            accounts.push(format!("{name} {account}"));
        }
        accounts.sort_unstable();
        let accounts = accounts.join(" ");

        write!(f, "{accounts}")
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub(crate) struct Account {
    #[serde(default)]
    pub email: Option<Email>,
    #[serde(default)]
    pub email_sent: i64,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub logged_in: Option<usize>,
    #[serde(default)]
    pub draws: u64,
    #[serde(default)]
    pub wins: u64,
    #[serde(default)]
    pub losses: u64,
    #[serde(default)]
    pub rating: Rating,
    #[serde(default)]
    pub send_emails: bool,
    #[serde(skip)]
    pub pending_games: HashSet<Id>,
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.logged_in.is_some() {
            write!(
                f,
                "{} {} {} {} logged_in",
                self.wins, self.losses, self.draws, self.rating
            )
        } else {
            write!(
                f,
                "{} {} {} {} logged_out",
                self.wins, self.losses, self.draws, self.rating
            )
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub(crate) struct Accounts(pub HashMap<String, Account>);
