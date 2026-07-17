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

use std::collections::{HashMap, HashSet};

use crate::{email::Email, glicko::Rating};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::Id;

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
            && self.software_id == other.software_id
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DateTimeUtc(pub Timestamp);

impl Default for DateTimeUtc {
    fn default() -> Self {
        Self(Timestamp::now())
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct User {
    pub username: String,
    pub wins: u64,
    pub losses: u64,
    pub draws: u64,
    pub rating: Rating,
    pub logged_in: bool,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Users(pub HashMap<String, User>);

impl From<&Accounts> for Users {
    fn from(accounts_1: &Accounts) -> Self {
        let mut accounts_2 = HashMap::with_capacity(accounts_1.0.len());

        for (username, account) in &accounts_1.0 {
            let logged_in = account.logged_in.is_some();
            let username = username.clone();

            accounts_2.insert(
                username.clone(),
                User {
                    username,
                    wins: account.wins,
                    losses: account.losses,
                    draws: account.draws,
                    rating: account.rating.clone(),
                    logged_in,
                },
            );
        }

        Self(accounts_2)
    }
}

impl Users {
    #[must_use]
    pub fn rating(&self, attacker: Option<&str>, defender: Option<&str>) -> (f64, f64) {
        let mut rating_1 = if let Some(attacker) = attacker
            && let Some(user) = self.0.get(attacker)
        {
            user.rating.rating
        } else {
            0.0
        };

        let mut rating_2 = if let Some(defender) = defender
            && let Some(user) = self.0.get(defender)
        {
            user.rating.rating
        } else {
            0.0
        };

        if rating_2 > rating_1 {
            std::mem::swap(&mut rating_1, &mut rating_2);
        }

        (rating_1, rating_2)
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Accounts(pub HashMap<String, Account>);

impl Accounts {
    #[must_use]
    pub fn rating(&self, attacker: Option<&str>, defender: Option<&str>) -> (f64, f64) {
        let mut rating_1 = if let Some(attacker) = attacker
            && let Some(account) = self.0.get(attacker)
        {
            account.rating.rating
        } else {
            0.0
        };

        let mut rating_2 = if let Some(defender) = defender
            && let Some(account) = self.0.get(defender)
        {
            account.rating.rating
        } else {
            0.0
        };

        if rating_2 > rating_1 {
            std::mem::swap(&mut rating_1, &mut rating_2);
        }

        (rating_1, rating_2)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum AccountsOrUsers {
    Accounts(Accounts),
    Users(Users),
}
