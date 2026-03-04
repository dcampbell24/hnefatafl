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

use clap::Parser;
use hnefatafl_copenhagen::LONG_VERSION;

/// Hnefatafl Copenhagen Client
///
/// This is a TCP client that connects to a server.
#[allow(clippy::struct_excessive_bools)]
#[derive(Parser, Debug)]
#[command(long_version = LONG_VERSION, about = "Copenhagen Hnefatafl Client")]
pub(crate) struct Args {
    /// Connect to the server at host
    #[arg(default_value = "hnefatafl.org", long)]
    pub host: String,

    /// Whether to log on the debug level
    #[arg(long)]
    pub debug: bool,

    /// What AI to use for Heat Map
    #[arg(default_value = "monte-carlo", long)]
    pub ai: String,

    /// How many seconds to run the monte-carlo AI
    #[arg(long)]
    pub seconds: Option<u64>,

    /// How deep in the game tree to go with the AI
    #[arg(long)]
    pub depth: Option<u8>,

    /// Make the window size tiny
    #[arg(long)]
    pub tiny_window: bool,

    /// Make the window size appropriate for social preview
    #[arg(long)]
    pub social_preview: bool,

    /// Render everything in ASCII
    #[arg(long)]
    pub ascii: bool,

    /// Build the manpage
    #[arg(long)]
    pub man: bool,
}
