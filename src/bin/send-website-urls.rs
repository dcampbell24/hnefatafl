#![cfg(feature = "urls")]

use std::{fs, path::PathBuf};

use reqwest::blocking::Client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{fs, path::PathBuf};

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
