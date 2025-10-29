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
