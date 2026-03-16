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

#[cfg(feature = "server")]
use lettre::message::Mailbox;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Hash, PartialEq, Eq, Serialize)]
pub struct Email {
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub code: Option<u32>,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub verified: bool,
}

impl Email {
    #[cfg(feature = "server")]
    #[must_use]
    pub fn to_mailbox(&self) -> Option<Mailbox> {
        Some(Mailbox::new(
            Some(self.username.clone()),
            self.address.parse().ok()?,
        ))
    }

    #[must_use]
    pub fn tx(&self) -> String {
        // Note: We use a FIGURE SPACE to separate the username from the address so
        // .split_ascii_whitespace() does not treat it as a space.
        format!("{} <{}>", self.username, self.address)
    }
}
