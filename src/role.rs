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

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
pub enum Role {
    #[default]
    Attacker,
    Defender,
    Roleless,
}

impl Role {
    #[must_use]
    pub fn opposite(&self) -> Self {
        match self {
            Self::Attacker => Self::Defender,
            Self::Defender => Self::Attacker,
            Self::Roleless => Self::Roleless,
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::Attacker => write!(f, "attacker"),
            Role::Defender => write!(f, "defender"),
            Role::Roleless => write!(f, "roleless"),
        }
    }
}

impl FromStr for Role {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> anyhow::Result<Self> {
        let string = string.to_lowercase();

        match string.as_str() {
            "a" | "attacker" => Ok(Self::Attacker),
            "d" | "defender" => Ok(Self::Defender),
            _ => Err(anyhow::Error::msg(format!(
                "Error trying to convert '{string}' to a Role!"
            ))),
        }
    }
}
