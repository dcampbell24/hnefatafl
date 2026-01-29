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

//! An AI, client, engine, and server for the game of Copenhagen Hnefatafl.
//!
//! ## Feature Flags
//!
//! By default the `client` and `server` feature flags are enabled.
//!
//! * client - enable the `hnefatafl-client` binary
//! * console - on Windows print output to the console
//! * debug - enable iced debug mode, also log on the debug level
//! * js - enable options for generating javascript code
//! * runic - enable the `icelandic-runic` binary for translating Icelandic to Icelandic Runic
//! * server - enable the `hnefatafl-server-full` binary
//!
//! ## Message Protocol
//!
//! Get more information about the [message protocol] used by the engine.
//!
//! [message protocol]: https://docs.rs/hnefatafl-copenhagen/latest/hnefatafl_copenhagen/message/enum.Message.html

#![deny(clippy::panic)]
#![deny(clippy::expect_used)]
#![deny(clippy::unwrap_used)]

use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
};

rust_i18n::i18n!();

pub mod ai;
pub mod board;
pub mod characters;
pub mod draw;
pub mod email;
pub mod game;
mod game_tree;
pub mod glicko;
pub mod heat_map;
pub mod locale;
mod message;
pub mod play;
pub mod rating;
pub mod role;
pub mod server_game;
pub mod space;
pub mod status;
mod tests;
pub mod time;
pub mod tournament;
pub mod tree;
pub mod utils;

pub type Id = u128;
pub const HOME: &str = "hnefatafl-copenhagen";
pub const SERVER_PORT: &str = ":49152";
pub const SOCKET_PATH: &str = "/tmp/hnefatafl.sock";
pub const VERSION_ID: &str = "ad746a65";

pub const COPYRIGHT: &str = r".SH COPYRIGHT
Copyright (C) 2025-2026 Developers of the hnefatafl-copenhagen project

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
";

pub const LONG_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "
Copyright (c) 2025-2026 Developers of the hnefatafl-copenhagen project
Licensed under the AGPLv3"
);

/// # Errors
///
/// If read fails.
pub fn read_response(reader: &mut BufReader<TcpStream>) -> anyhow::Result<String> {
    let mut reply = String::new();
    reader.read_line(&mut reply)?;
    print!("<- {reply}");
    Ok(reply)
}

/// # Errors
///
/// If write fails.
pub fn write_command(command: &str, stream: &mut TcpStream) -> anyhow::Result<()> {
    print!("-> {command}");
    stream.write_all(command.as_bytes())?;
    Ok(())
}
