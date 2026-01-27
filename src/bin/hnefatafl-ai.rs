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

use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpStream, ToSocketAddrs as _},
    time::Duration,
};

use anyhow::Error;
use clap::{CommandFactory, Parser};
use hnefatafl_copenhagen::{
    COPYRIGHT, LONG_VERSION, VERSION_ID,
    ai::AI,
    game::Game,
    play::Plae,
    role::Role,
    status::Status,
    utils::{self, choose_ai},
};
use log::{debug, info, trace};
use socket2::{Domain, SockAddr, Socket, TcpKeepalive, Type};

// Move 26, defender wins, corner escape, time per move 15s 2025-03-06 (hnefatafl-equi).

const PORT: &str = ":49152";

/// Copenhagen Hnefatafl AI
///
/// This is an AI client that connects to a server.
#[allow(clippy::struct_excessive_bools)]
#[derive(Parser, Debug)]
#[command(long_version = LONG_VERSION, about = "Copenhagen Hnefatafl AI")]
struct Args {
    /// Enter your username
    #[arg(long)]
    username: String,

    /// Enter your password
    #[arg(default_value = "", long)]
    password: String,

    /// Set the role as attacker or defender
    #[arg(default_value_t = Role::Attacker, long)]
    role: Role,

    /// Connect to the HTP server at host
    #[arg(default_value = "hnefatafl.org", long)]
    host: String,

    /// Choose an AI to play as
    #[arg(default_value = "basic", long)]
    ai: String,

    /// Whether to log on the debug level
    #[arg(long)]
    debug: bool,

    /// How many seconds to run the monte-carlo AI
    #[arg(long)]
    seconds: Option<u64>,

    /// How deep in the game tree to go with Ai
    ///
    /// [default basic: 4]
    /// [default monte-carlo: 20]
    #[arg(long)]
    depth: Option<u8>,

    /// Join game with id
    #[arg(long)]
    join_game: Option<u128>,

    /// Run the basic AI sequentially
    #[arg(long)]
    sequential: bool,

    /// Whether the application is being run by systemd
    #[arg(long)]
    systemd: bool,

    /// Build the manpage
    #[arg(long)]
    man: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    utils::init_logger("hnefatafl_ai", args.debug, args.systemd);

    if args.man {
        let mut buffer: Vec<u8> = Vec::default();
        let cmd = Args::command().name("hnefatafl-ai").long_version(None);
        let man = clap_mangen::Man::new(cmd).date("2025-06-23");

        man.render(&mut buffer)?;
        write!(buffer, "{COPYRIGHT}")?;

        std::fs::write("hnefatafl-ai.1", buffer)?;
        return Ok(());
    }

    let mut username = "ai-".to_string();
    username.push_str(&args.username);

    let mut address_string = args.host.clone();
    address_string.push_str(PORT);

    let mut is_ipv6 = false;
    let mut socket_address = None;
    let socket_addresses = address_string.to_socket_addrs()?;

    for address in socket_addresses.clone() {
        if address.is_ipv6() {
            socket_address = Some(address);
            is_ipv6 = true;
            break;
        }
    }

    if !is_ipv6 {
        for address in socket_addresses {
            if address.is_ipv4() {
                socket_address = Some(address);
                break;
            }
        }
    }

    let socket_address = socket_address.ok_or_else(|| {
        anyhow::Error::msg(format!(
            "There is no IP address for the host: {address_string}"
        ))
    })?;

    let address: SockAddr = socket_address.into();
    let keepalive = TcpKeepalive::new()
        .with_time(Duration::from_secs(30))
        .with_interval(Duration::from_secs(30))
        .with_retries(3);

    let domain_type = if is_ipv6 { Domain::IPV6 } else { Domain::IPV4 };
    let socket = Socket::new(domain_type, Type::STREAM, None)?;
    socket.set_tcp_keepalive(&keepalive)?;

    socket.connect(&address).unwrap_or_else(|error| {
        eprintln!("socket.connect {address_string}: {error}");
    });

    info!("connected to {socket_address}");

    let mut tcp: TcpStream = socket.into();
    let mut reader = BufReader::new(tcp.try_clone()?);

    tcp.write_all(format!("{VERSION_ID} login {username} {}\n", args.password).as_bytes())?;

    let mut buf = String::new();
    reader.read_line(&mut buf)?;
    assert_eq!(buf, "= login\n");
    buf.clear();

    if let Some(game_id) = args.join_game {
        tcp.write_all(format!("join_game_pending {game_id}\n").as_bytes())?;

        let ai = choose_ai(&args.ai, args.seconds, args.depth, args.sequential)?;
        handle_messages(ai, game_id, &mut reader, &mut tcp)?;
    } else {
        loop {
            new_game(&mut tcp, args.role, &mut reader, &mut buf)?;

            info!("{buf}");

            let message: Vec<_> = buf.split_ascii_whitespace().collect();
            let Some(message) = message.get(3) else {
                return Err(anyhow::Error::msg("Expecting message[3] to be a game_id"));
            };

            let game_id = message.parse()?;
            buf.clear();

            wait_for_challenger(&mut reader, &mut buf, &mut tcp, game_id)?;

            let ai = choose_ai(&args.ai, args.seconds, args.depth, args.sequential)?;
            handle_messages(ai, game_id, &mut reader, &mut tcp)?;
        }
    }

    Ok(())
}

fn new_game(
    tcp: &mut TcpStream,
    role: Role,
    reader: &mut BufReader<TcpStream>,
    buf: &mut String,
) -> anyhow::Result<()> {
    tcp.write_all(format!("new_game {role} rated fischer 900000 10 11\n").as_bytes())?;

    loop {
        reader.read_line(buf)?;

        if buf.trim().is_empty() {
            return Err(Error::msg("the TCP stream has closed"));
        }

        let message: Vec<_> = buf.split_ascii_whitespace().collect();
        if let Some(message) = message.get(1)
            && *message == "new_game"
        {
            return Ok(());
        }

        buf.clear();
    }
}

fn wait_for_challenger(
    reader: &mut BufReader<TcpStream>,
    buf: &mut String,
    tcp: &mut TcpStream,
    game_id: u128,
) -> anyhow::Result<()> {
    loop {
        reader.read_line(buf)?;

        if buf.trim().is_empty() {
            return Err(Error::msg("the TCP stream has closed"));
        }

        let message: Vec<_> = buf.split_ascii_whitespace().collect();
        if Some("challenge_requested") == message.get(1).copied() {
            info!("{message:?}");
            buf.clear();

            break;
        }

        buf.clear();
    }

    tcp.write_all(format!("join_game {game_id}\n").as_bytes())?;
    Ok(())
}

fn handle_messages(
    mut ai: Box<dyn AI>,
    game_id: u128,
    reader: &mut BufReader<TcpStream>,
    tcp: &mut TcpStream,
) -> anyhow::Result<()> {
    let mut game = Game::default();

    debug!("{game}\n");

    let mut buf = String::new();
    loop {
        reader.read_line(&mut buf)?;

        if buf.trim().is_empty() {
            return Err(Error::msg("the TCP stream has closed"));
        }

        let mut message: Vec<_> = buf.split_ascii_whitespace().collect();

        if Some("generate_move") == message.get(2).copied() {
            let generate_move = ai.generate_move(&mut game)?;

            tcp.write_all(format!("game {game_id} {}\n", generate_move.play).as_bytes())?;

            debug!("{game}");
            info!("{generate_move}");
            trace!("{}", generate_move.heat_map);

            if game.status != Status::Ongoing {
                return Ok(());
            }
        } else if Some("play") == message.get(2).copied() {
            let words = message.split_off(2);
            let play = Plae::try_from(words)?;
            ai.play(&mut game, &play)?;

            debug!("{game}\n");

            if game.status != Status::Ongoing {
                return Ok(());
            }
        } else if Some("game_over") == message.get(1).copied() {
            return Ok(());
        }

        buf.clear();
    }
}
