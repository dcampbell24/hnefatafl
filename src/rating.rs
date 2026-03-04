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

use std::{fmt, ops::Not, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Rated {
    No,
    #[default]
    Yes,
}

impl fmt::Display for Rated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rated::No => write!(f, "unrated"),
            Rated::Yes => write!(f, "rated"),
        }
    }
}

impl Not for Rated {
    type Output = Rated;

    fn not(self) -> Self::Output {
        match self {
            Rated::No => Rated::Yes,
            Rated::Yes => Rated::No,
        }
    }
}

impl From<bool> for Rated {
    fn from(boolean: bool) -> Self {
        if boolean { Self::Yes } else { Self::No }
    }
}

impl From<Rated> for bool {
    fn from(rated: Rated) -> Self {
        match rated {
            Rated::Yes => true,
            Rated::No => false,
        }
    }
}

impl FromStr for Rated {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> anyhow::Result<Self> {
        match string {
            "rated" => Ok(Self::Yes),
            "unrated" => Ok(Self::No),
            _ => Err(anyhow::Error::msg(format!(
                "Error trying to convert '{string}' to Rated!"
            ))),
        }
    }
}
