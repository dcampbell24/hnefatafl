// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#[cfg(any(target_family = "unix", target_family = "windows"))]
use std::process::Command;
use std::{env, path::PathBuf, process::ExitStatus, time::Duration};

use env_logger::Builder;
use log::LevelFilter;

use crate::ai::{AI, AiBanal, AiBasic, AiMonteCarlo};

/// # Errors
///
/// If you don't choose banal, basic, or monte-carlo.
pub fn choose_ai(
    ai: &str,
    seconds: Option<u64>,
    depth: Option<u8>,
    sequential: bool,
) -> anyhow::Result<Box<dyn AI>> {
    match ai {
        "banal" => Ok(Box::new(AiBanal)),
        "basic" => {
            let depth = depth.unwrap_or(4);

            Ok(Box::new(AiBasic::new(depth, sequential)))
        }
        "monte-carlo" => {
            let seconds = seconds.unwrap_or(10);
            let depth = depth.unwrap_or(20);

            Ok(Box::new(AiMonteCarlo::new(
                Duration::from_secs(seconds),
                depth,
            )))
        }
        _ => Err(anyhow::Error::msg(
            "you must pass banal, basic, or monte-carlo to --ai",
        )),
    }
}

#[allow(clippy::missing_errors_doc)]
pub fn clear_screen() -> anyhow::Result<ExitStatus> {
    #[cfg(not(any(target_family = "unix", target_family = "windows")))]
    return Ok(ExitStatus::default());

    #[cfg(target_family = "unix")]
    let exit_status = Command::new("clear").status()?;

    #[cfg(target_family = "windows")]
    let exit_status = Command::new("cls").status()?;

    #[cfg(any(target_family = "unix", target_family = "windows"))]
    Ok(exit_status)
}

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

pub fn init_logger(module: &str, debug: bool, systemd: bool) {
    let mut builder = Builder::new();

    if systemd {
        builder.format_timestamp(None);
        builder.format_target(false);
    }

    if let Ok(var) = env::var("RUST_LOG") {
        builder.parse_filters(&var);
    } else if debug {
        builder.filter(Some(module), LevelFilter::Debug);
    } else {
        // If no RUST_LOG provided, default to logging at the Info level.
        #[cfg(not(feature = "debug"))]
        builder.filter(Some(module), LevelFilter::Info);
        #[cfg(feature = "debug")]
        builder.filter(Some(module), LevelFilter::Debug);
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
