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

#![deny(clippy::expect_used)]
#![deny(clippy::indexing_slicing)]
#![deny(clippy::panic)]
#![deny(clippy::unwrap_used)]
#![cfg(feature = "json")]

use std::fs;

use serde_json::Value;

fn main() -> anyhow::Result<()> {
    let json_content = fs::read_to_string("locales/app.json")?;
    let json_value: serde_json::Value = serde_json::from_str(&json_content)?;

    if let Value::Object(map) = json_value {
        let mut count = 0;

        for (words, _) in map {
            // println!("{words:?}");
            count += words.split_whitespace().count();
        }

        println!("{count}");
    }

    Ok(())
}
