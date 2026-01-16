use clap::Parser;
use hnefatafl_copenhagen::LONG_VERSION;

/// Copenhagen Hnefatafl Server
///
/// This is a TCP server that listens for client connections.
#[allow(clippy::struct_excessive_bools)]
#[derive(Parser, Debug)]
#[command(long_version = LONG_VERSION, about = "Copenhagen Hnefatafl Server")]
pub(crate) struct Args {
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
