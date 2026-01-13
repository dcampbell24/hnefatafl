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
#![cfg(feature = "urls")]

use std::{fs, path::PathBuf};

use reqwest::blocking::Client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "http://api.indexnow.org/IndexNow";
    let client = Client::new();
    let mut dir = PathBuf::new();
    dir.push("website");
    dir.push("index-now");

    for entry in fs::read_dir(dir)? {
        let key: String = fs::read_to_string(entry?.path())?;
        let json_data = format!(
            r#"{{
    "host": "hnefatafl.org",
    "key": "{key}",
    "urlList": [
        "https://hnefatafl.org",
        "https://hnefatafl.org/install.html",
        "https://hnefatafl.org/rules.html",
        "https://hnefatafl.org/sitemap.xml"
    ]
}}
"#
        );

        let response = client
            .post(url)
            .header("Content-Type", "application/json; charset=utf-8")
            .body(json_data.to_string())
            .send()?;

        println!("Status: {:?}", response);
        println!("Response Body:\n{}", response.text()?);
    }

    Ok(())
}
