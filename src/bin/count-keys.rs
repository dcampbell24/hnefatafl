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

use std::fs;

use toml::Value;

fn main() -> anyhow::Result<()> {
    let toml_content = fs::read_to_string("locales/app.toml")?;
    let toml_value: Value = toml::from_str(&toml_content)?;

    if let Value::Table(map) = toml_value {
        let mut count = 0;

        for (words, _) in map {
            count += words.split_whitespace().count();
        }

        println!("words translated: {count}");
    }

    Ok(())
}
