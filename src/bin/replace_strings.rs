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

const INDEX: &str = "Join a friendly community and play Copenhagen Hnefatafl. Figure out how to \
install the software, play by the rules, chat on Discord, find Help, and Donate.";

const CANONICAL: &str = r#"<!-- Custom HTML head -->
        <link rel="canonical" href="https://hnefatafl.org" />"#;

const HISTORY: &str = "Get the history of Hnefatafl. It is a part of the games known Tafl games. \
Other related games are Alea evangelii, Ard Rí, Brandubh, Tablut, and Tawlbwrdd";

const INSTALL: &str = "Determine how to install Copenhagen Hnefatafl. Install using the Arch User \
Repository, Chocolatey, a Debian package, a flathub package, or Rust's cargo.";

const RULES: &str = "Learn the rules to the game of Copenhagen Hnefatafl. Move your pieces until \
you achieve victory or lose. Try not to get surrounded as the defenders and escape.";

const AI: &str = "Discover about using artificial intelligence to play the game of Copenhagen \
Hnefatafl. If you are using the Debian or Arch installs, you can run AI as a service.";

fn main() -> Result<(), anyhow::Error> {
    // let index_path = "book/index.html";
    let index_path = "/var/www/html/index.html";
    let file = fs::read_to_string(index_path)?;
    let content = file.replace("{{description}}", INDEX);
    fs::write(index_path, content)?;

    // Don't move!
    let file = fs::read_to_string(index_path)?;
    let content = file.replace("<!-- Custom HTML head -->", CANONICAL);
    fs::write(index_path, content)?;

    // let index_path = "book/history.html";
    let index_path = "/var/www/html/history.html";
    let file = fs::read_to_string(index_path)?;
    let content = file.replace("{{description}}", HISTORY);
    fs::write(index_path, content)?;

    // let install_path = "book/install.html";
    let install_path = "/var/www/html/install.html";
    let file = fs::read_to_string(install_path)?;
    let content = file.replace("{{description}}", INSTALL);
    fs::write(install_path, content)?;

    // let rules_path = "book/rules.html";
    let rules_path = "/var/www/html/rules.html";
    let file = fs::read_to_string(rules_path)?;
    let content = file.replace("{{description}}", RULES);
    fs::write(rules_path, content)?;

    // let rules_path = "book/ai.html";
    let rules_path = "/var/www/html/ai.html";
    let file = fs::read_to_string(rules_path)?;
    let content = file.replace("{{description}}", AI);
    fs::write(rules_path, content)?;

    Ok(())
}
