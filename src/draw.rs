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

use std::{fmt, str::FromStr};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Draw {
    Accept,
    Decline,
}

impl fmt::Display for Draw {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Accept => write!(f, "accept"),
            Self::Decline => write!(f, "decline"),
        }
    }
}

impl FromStr for Draw {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> anyhow::Result<Self> {
        match value {
            "accept" => Ok(Self::Accept),
            "decline" => Ok(Self::Decline),
            s => Err(anyhow::Error::msg(format!(
                "Error trying to convert '{s}' to a Draw!"
            ))),
        }
    }
}
