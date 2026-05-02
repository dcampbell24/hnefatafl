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
    fmt,
};

use crate::{email::Email, glicko::Rating};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::Id;

impl Accounts {
    /// # Errors
    ///
    /// If serialization fails.
    pub fn display_admin(&self) -> anyhow::Result<String> {
        let string = ron::ser::to_string(&self)?;

        Ok(string)
    }
}

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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Account {
    #[serde(default)]
    pub email: Option<Email>,
    /// A unix timestamp in seconds when the email was sent.
    #[serde(default)]
    pub email_sent: i64,
    #[serde(default)]
    pub password: String,
    /// If logged in, holds the index into clients.
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
    #[serde(default)]
    pub creation_date: DateTimeUtc,
    #[serde(default)]
    pub last_logged_in: DateTimeUtc,
    #[serde(default)]
    pub software_id: String,
}

impl PartialEq for Account {
    fn eq(&self, other: &Self) -> bool {
        self.email == other.email
            && self.email_sent == other.email_sent
            && self.logged_in == other.logged_in
            && self.draws == other.draws
            && self.wins == other.wins
            && self.losses == other.losses
            && self.rating == other.rating
            && self.send_emails == other.send_emails
            && self.creation_date == other.creation_date
            && self.last_logged_in == other.last_logged_in
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DateTimeUtc(pub Timestamp);

impl Default for DateTimeUtc {
    fn default() -> Self {
        Self(Timestamp::now())
    }
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
pub struct Accounts(pub HashMap<String, Account>);
