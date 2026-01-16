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

use chrono::Local;
use serde::{Deserialize, Serialize};

/// Non-leap seconds since January 1, 1970 0:00:00 UTC.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct UnixTimestamp(pub i64);

impl Default for UnixTimestamp {
    fn default() -> Self {
        Self(Local::now().to_utc().timestamp())
    }
}
