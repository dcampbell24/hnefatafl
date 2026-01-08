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

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Status {
    AttackerWins,
    Draw,
    #[default]
    Ongoing,
    DefenderWins,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AttackerWins => write!(f, "attacker_wins"),
            Self::Draw => write!(f, "draw"),
            Self::Ongoing => write!(f, "ongoing"),
            Self::DefenderWins => write!(f, "defender_wins"),
        }
    }
}

impl FromStr for Status {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> anyhow::Result<Self> {
        match value {
            "Attacker" | "Black" => Ok(Self::AttackerWins),
            "Draw" => Ok(Self::Draw),
            "Ongoing" => Ok(Self::Ongoing),
            "Defender" | "White" => Ok(Self::DefenderWins),
            _ => Err(anyhow::Error::msg(format!("invalid status: {value}"))),
        }
    }
}
