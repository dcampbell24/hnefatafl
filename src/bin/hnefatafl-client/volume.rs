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

use serde::{Deserialize, Serialize};

pub const MAX_VOLUME: u32 = 8;

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Volume(pub u32);

impl Default for Volume {
    fn default() -> Self {
        Self(4)
    }
}

impl Volume {
    pub fn volume(&self) -> f32 {
        match self.0 {
            0 => 0.25,
            1 => 0.375,
            2 => 0.5,
            3 => 0.75,
            4 => 1.0,
            5 => 1.5,
            6 => 2.0,
            7 => 3.0,
            MAX_VOLUME => 4.0,
            _ => unreachable!(),
        }
    }
}
