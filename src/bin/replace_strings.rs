use std::fs;

const INDEX: &str = "Join a friendly community and play Copenhagen Hnefatafl. Figure out how to \
install the software, play by the rules, chat on Discord, find Help, and Donate.";

const CANONICAL: &str = r#"<!-- Custom HTML head -->
        <link rel="canonical" href="https://hnefatafl.org" />"#;

const INSTALL: &str = "Determine how to install Copenhagen Hnefatafl. Install using the Arch User \
Repository, Chocolatey, a Debian package (.deb), a flathub package, or Rust's cargo";

const RULES: &str = "Learn the rules to the game of Copenhagen Hnefatafl. Move your pieces until \
you achieve victory or lose. Try not to get surrounded as the defenders and escape.";

fn main() -> Result<(), anyhow::Error> {
    // let index_path = "book/index.html";
    let index_path = "/var/www/html/index.html";
    let file = fs::read_to_string(index_path)?;
    let content = file.replace("{{description}}", INDEX);
    fs::write(index_path, content)?;

    let file = fs::read_to_string(index_path)?;
    let content = file.replace("<!-- Custom HTML head -->", CANONICAL);
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

    Ok(())
}
