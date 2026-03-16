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

#[derive(Clone, Debug)]
pub struct Characters {
    pub attacker: String,
    pub arrow_down: String,
    pub arrow_left: String,
    pub arrow_right: String,
    pub arrow_up: String,
    pub captured: String,
    pub dagger: String,
    pub defender: String,
    pub double_arrow_left: String,
    pub double_arrow_left_full: String,
    pub double_arrow_right: String,
    pub double_arrow_right_full: String,
    pub king: String,
    pub people: String,
    pub restricted_square: String,
    pub shield: String,
    pub warning: String,
}

impl Default for Characters {
    fn default() -> Self {
        Self {
            attacker: "♟".to_string(),
            arrow_down: "↓".to_string(),
            arrow_left: "←".to_string(),
            arrow_right: "→".to_string(),
            arrow_up: "↑".to_string(),
            captured: "🗙".to_string(),
            dagger: "🗡".to_string(),
            defender: "♙".to_string(),
            double_arrow_left: "⏪".to_string(),
            double_arrow_left_full: "⏮".to_string(),
            double_arrow_right: "⏩".to_string(),
            double_arrow_right_full: "⏭".to_string(),
            king: "♔".to_string(),
            people: "👥".to_string(),
            restricted_square: "⌘".to_string(),
            shield: "⛨".to_string(),
            warning: "⚠".to_string(),
        }
    }
}

impl Characters {
    pub fn ascii(&mut self) {
        self.attacker = "A".to_string();
        self.arrow_down = "v".to_string();
        self.arrow_left = "<".to_string();
        self.arrow_right = ">".to_string();
        self.arrow_up = "^".to_string();
        self.captured = "X".to_string();
        self.dagger = "A".to_string();
        self.defender = "D".to_string();
        self.double_arrow_left = "<".to_string();
        self.double_arrow_left_full = "<<".to_string();
        self.double_arrow_right = ">".to_string();
        self.double_arrow_right_full = ">>".to_string();
        self.king = "K".to_string();
        self.people = "OO".to_string();
        self.restricted_square = "#".to_string();
        self.shield = "D".to_string();
        self.warning = "X".to_string();
    }
}
