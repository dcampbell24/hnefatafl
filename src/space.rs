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

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::role::Role;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Space {
    Empty,
    Attacker,
    King,
    Defender,
}

impl Space {
    #[must_use]
    pub fn display_ascii(&self) -> &str {
        match self {
            Self::Attacker => "A",
            Self::Empty => ".",
            Self::King => "K",
            Self::Defender => "D",
        }
    }
}

impl TryFrom<char> for Space {
    type Error = anyhow::Error;

    fn try_from(value: char) -> anyhow::Result<Self> {
        match value {
            'X' => Ok(Self::Attacker),
            'O' => Ok(Self::Defender),
            '.' => Ok(Self::Empty),
            'K' => Ok(Self::King),
            ch => Err(anyhow::Error::msg(format!(
                "Error trying to convert '{ch}' to a Space!"
            ))),
        }
    }
}

impl fmt::Display for Space {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Attacker => write!(f, "♟"),
            Self::Empty => write!(f, "."),
            Self::King => write!(f, "♔"),
            Self::Defender => write!(f, "♙"),
        }
    }
}

impl From<Space> for Role {
    fn from(space: Space) -> Self {
        match space {
            Space::Attacker => Role::Attacker,
            Space::Defender | Space::King => Role::Defender,
            Space::Empty => Role::Roleless,
        }
    }
}

impl From<&u8> for Space {
    fn from(space: &u8) -> Self {
        match space {
            0 => Space::Attacker,
            1 => Space::Defender,
            2 => Space::Empty,
            3 => Space::King,
            _ => unreachable!(),
        }
    }
}

impl From<&Space> for u8 {
    fn from(space: &Space) -> Self {
        match space {
            Space::Attacker => 0,
            Space::Defender => 1,
            Space::Empty => 2,
            Space::King => 3,
        }
    }
}

impl TryFrom<Space> for usize {
    type Error = anyhow::Error;

    fn try_from(space: Space) -> Result<usize, anyhow::Error> {
        match space {
            Space::Attacker => Ok(0),
            Space::Defender => Ok(1),
            Space::King => Ok(2),
            Space::Empty => Err(anyhow::Error::msg(
                "we should not try to get a usize for an empty space",
            )),
        }
    }
}
