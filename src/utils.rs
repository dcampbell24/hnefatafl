use std::{env, io::Write, path::PathBuf};

use chrono::Utc;
use env_logger::Builder;
use log::LevelFilter;

#[must_use]
pub fn data_file(file: &str) -> PathBuf {
    let mut data_file = if let Some(data_file) = dirs::data_dir() {
        data_file
    } else {
        PathBuf::new()
    };
    data_file.push(file);
    data_file
}

pub fn init_logger(systemd: bool) {
    let mut builder = Builder::new();

    if systemd {
        builder.format(|formatter, record| {
            writeln!(formatter, "[{}]: {}", record.level(), record.args())
        });
    } else {
        builder.format(|formatter, record| {
            writeln!(
                formatter,
                "{} [{}] ({}): {}",
                Utc::now().format("%Y-%m-%d %H:%M:%S %z"),
                record.level(),
                record.target(),
                record.args()
            )
        });
    }

    if let Ok(var) = env::var("RUST_LOG") {
        builder.parse_filters(&var);
    } else {
        // if no RUST_LOG provided, default to logging at the Info level
        builder.filter(None, LevelFilter::Info);
    }

    builder.init();
}

#[must_use]
pub fn split_whitespace_password(string: &str) -> (String, bool) {
    let mut ends_with_whitespace = false;

    if string.ends_with(|ch: char| ch.is_whitespace()) {
        ends_with_whitespace = true;
    }

    let mut string: String = string.split_whitespace().collect();

    if string.is_empty() {
        ends_with_whitespace = false;
    }

    if ends_with_whitespace {
        string.push(' ');
    }

    (string, ends_with_whitespace)
}
