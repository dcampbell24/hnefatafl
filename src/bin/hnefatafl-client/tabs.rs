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
// SPDX-FileCopyrightText: 2026 David Campbell <david@hnefatafl.org>

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum TabId {
    AccountSettings,
    Chat,
    #[default]
    Games,
    GameNew,
    Tournament,
    Users,
}

impl fmt::Display for TabId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AccountSettings => f.write_str("Account Settings"),
            Self::Chat => f.write_str("Chat"),
            Self::Games => f.write_str("Games"),
            Self::GameNew => f.write_str("Create Game"),
            Self::Tournament => f.write_str("Tournament"),
            Self::Users => f.write_str("Users"),
        }
    }
}
