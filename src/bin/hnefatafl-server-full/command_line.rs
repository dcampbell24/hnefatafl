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

use std::io::Write as _;

use clap::{CommandFactory, Parser};
use hnefatafl_copenhagen::{COPYRIGHT, LONG_VERSION};

/// Copenhagen Hnefatafl Server
///
/// This is a TCP server that listens for client connections.
#[allow(clippy::struct_excessive_bools)]
#[derive(Parser, Debug)]
#[command(long_version = LONG_VERSION, about = "Copenhagen Hnefatafl Server")]
pub(crate) struct Args {
    /// Whether to automatically start a tournament
    #[arg(long)]
    pub autostart_tournament: bool,

    /// Whether to log on the debug level
    #[arg(long)]
    pub debug: bool,

    /// Whether to skip advertising updates
    #[arg(long)]
    pub skip_advertising_updates: bool,

    /// Whether to skip messages
    #[arg(long)]
    pub skip_message: bool,

    /// Whether to skip the data file
    #[arg(long)]
    pub skip_the_data_file: bool,

    /// Whether the application is being run by systemd
    #[arg(long)]
    pub systemd: bool,

    /// Add additional security checks
    ///
    /// - limit the number of TCP connections from a host
    #[arg(long)]
    pub secure: bool,

    /// Build the manpage
    #[arg(long)]
    pub man: bool,
}

impl Args {
    pub(crate) fn generate_man_page() -> anyhow::Result<()> {
        let mut buffer: Vec<u8> = Vec::default();
        let cmd = Self::command()
            .name("hnefatafl-server-full")
            .long_version(None);
        let man = clap_mangen::Man::new(cmd).date("2025-06-23");

        man.render(&mut buffer)?;
        write!(buffer, "{COPYRIGHT}")?;

        std::fs::write("hnefatafl-server-full.1", buffer)?;
        Ok(())
    }
}
