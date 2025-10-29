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
