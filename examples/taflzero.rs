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
//
// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 David Campbell <david@hnefatafl.org>

use std::{
    env,
    io::{BufRead, BufReader, Write},
    net::{TcpStream, ToSocketAddrs},
    process::Command,
    str::FromStr,
    thread::sleep,
    time::Duration,
};

use anyhow::Error;
use clap::{CommandFactory, Parser};
use env_logger::Builder;
use hnefatafl_copenhagen::{
    COPYRIGHT, VERSION_ID,
    ai::{AI, AiMonteCarlo},
    game::Game,
    play::{Plae, Play, Vertex},
    role::Role,
    status::Status,
};
use log::LevelFilter;
use socket2::{Domain, SockAddr, Socket, TcpKeepalive, Type};
use taflzero::{Engine, moves::mv::create_move_from_algebraic};

const PORT: &str = ":49152";

/// `TaflZero` AI
///
/// This is the taflzero client that connects to a hnefatafl.org server.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg(long)]
    username: String,

    #[arg(default_value = "", long)]
    password: String,

    /// attacker or defender
    #[arg(long, default_value_t = Role::Defender)]
    role: Role,

    /// Connect to the server at host
    #[arg(default_value = "hnefatafl.org", long)]
    host: String,

    /// Join game with id
    #[arg(long)]
    join_game: Option<u64>,

    /// Whether the application is being run by systemd
    #[arg(long)]
    systemd: bool,

    /// Search for `u64` milliseconds
    #[arg(long, default_value_t = 4_000)]
    search: u64,

    /// Whether to log at the debug level
    #[arg(long)]
    debug: bool,

    /// Build the man page
    #[arg(long)]
    man: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    init_logger(args.debug, args.systemd);

    if args.man {
        let mut buffer: Vec<u8> = Vec::default();
        let cmd = Args::command().name("taflzero").long_version(None);
        let man = clap_mangen::Man::new(cmd).date("2026-05-13");

        man.render(&mut buffer)?;
        write!(buffer, "{COPYRIGHT}")?;

        std::fs::write("taflzero.1", buffer)?;
        return Ok(());
    }

    let mut username = "ai-taflzero-".to_string();
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
        .with_interval(Duration::from_secs(30));

    let domain_type = if is_ipv6 { Domain::IPV6 } else { Domain::IPV4 };
    let socket = Socket::new(domain_type, Type::STREAM, None)?;
    socket.set_tcp_keepalive(&keepalive)?;

    systemd_delay_restart(&args)?;

    socket.connect(&address).unwrap_or_else(|error| {
        log::error!("socket.connect {address_string}: {error}");
    });

    log::info!("connected to {socket_address}");

    let mut tcp: TcpStream = socket.into();
    let mut reader = BufReader::new(tcp.try_clone()?);

    tcp.write_all(format!("{VERSION_ID} login {username} {}\n", args.password).as_bytes())?;

    let mut buf = String::new();
    reader.read_line(&mut buf)?;
    assert_eq!(buf, "= login\n");
    buf.clear();

    let role = &args.role;
    let mut engine = Engine::new("default_nn.onnx".to_string());

    loop {
        let game_id;

        if let Some(game_id_) = args.join_game {
            game_id = game_id_.to_string();
            tcp.write_all(format!("join_game_pending {game_id}\n").as_bytes())?;
        } else {
            new_game(&mut tcp, args.role, &mut reader, &mut buf)?;

            let message: Vec<_> = buf.split_ascii_whitespace().collect();
            log::info!("{message:?}");
            game_id = message[3].to_string();
            buf.clear();

            wait_for_challenger(&mut reader, &mut buf, &mut tcp, &game_id)?;
        }

        let game = Game::default();
        let ai = AiMonteCarlo::new(Duration::from_secs(10), 20);
        engine.set_start_position();

        log::debug!("\n{}", game.board);

        handle_messages(
            ai,
            game,
            &mut engine,
            &game_id,
            *role,
            &mut reader,
            &mut tcp,
            args.search,
        )?;

        if args.join_game.is_some() {
            return Ok(());
        }
    }
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
        if message[1] == "new_game" {
            return Ok(());
        }

        buf.clear();
    }
}

fn wait_for_challenger(
    reader: &mut BufReader<TcpStream>,
    buf: &mut String,
    tcp: &mut TcpStream,
    game_id: &str,
) -> anyhow::Result<()> {
    loop {
        reader.read_line(buf)?;

        if buf.trim().is_empty() {
            return Err(Error::msg("the TCP stream has closed"));
        }

        let message: Vec<_> = buf.split_ascii_whitespace().collect();

        if Some("challenge_requested") == message.get(1).copied() {
            log::info!("{message:?}");
            buf.clear();

            break;
        }

        buf.clear();
    }

    tcp.write_all(format!("join_game {game_id}\n").as_bytes())?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_messages(
    mut ai: AiMonteCarlo,
    mut game: Game,
    engine: &mut Engine,
    game_id: &str,
    role: Role,
    reader: &mut BufReader<TcpStream>,
    tcp: &mut TcpStream,
    search: u64,
) -> anyhow::Result<()> {
    let mut buf = String::new();

    loop {
        reader.read_line(&mut buf)?;

        if buf.trim().is_empty() {
            return Err(Error::msg("the TCP stream has closed"));
        }

        let message: Vec<_> = buf.split_ascii_whitespace().collect();

        if Some("generate_move") == message.get(2).copied() {
            engine.make_search(search, None);

            if let Some(mv) = engine.best_move() {
                log::debug!("taflzero: {mv}");

                let play = mv.to_string();
                let mut play = play.chars();
                let mut from = Vec::new();

                from.push(play.next().unwrap());
                from.push(play.next().unwrap());

                let ch = play.next().unwrap();
                let to: String = if ch.is_ascii_digit() {
                    from.push(ch);

                    play.collect()
                } else {
                    let mut s = ch.to_string();
                    s.push_str(&(play.collect::<String>()));
                    s
                };

                let from: String = from.iter().collect();
                let from = Vertex::from_str(&from)?;
                let to = Vertex::from_str(&to)?;

                let play = Plae::Play(Play { role, from, to });

                log::debug!("play: {play}");

                if game.play(&play).is_err() {
                    let generate_move = ai.generate_move(&mut game)?;
                    log::debug!("changed play to: {generate_move}");
                    let play = generate_move.play;

                    let Plae::Play(play) = &play else {
                        tcp.write_all(
                            format!("game {game_id} play {role} resigns _\n").as_bytes(),
                        )?;

                        return Ok(());
                    };

                    let mv = create_move_from_algebraic(&format!("{from}{to}")).unwrap();
                    if let Err(invalid_play) = engine.make_move(mv) {
                        log::debug!("invalid_play: {invalid_play:?}");

                        tcp.write_all(
                            format!("game {game_id} play {role} resigns _\n").as_bytes(),
                        )?;
                        return Ok(());
                    }

                    tcp.write_all(format!("game {game_id} {play}\n").as_bytes())?;
                }

                if let Err(invalid_play) = engine.make_move(mv) {
                    log::debug!("invalid_play: {invalid_play:?}");

                    tcp.write_all(format!("game {game_id} play {role} resigns _\n").as_bytes())?;

                    return Ok(());
                }

                tcp.write_all(format!("game {game_id} {play}\n").as_bytes())?;
            } else {
                tcp.write_all(format!("game {game_id} play {role} resigns _\n").as_bytes())?;
                return Ok(());
            }

            log::debug!("{}", game.board);

            if game.status != Status::Ongoing {
                return Ok(());
            }
        } else if Some("play") == message.get(2).copied() {
            let play =
                Plae::try_from(message[2..].to_vec()).expect("we should be getting a valid play");

            game.play(&play)?;

            if game.status != Status::Ongoing {
                return Ok(());
            }

            let Plae::Play(play) = play else {
                unreachable!();
            };

            let mv = format!("{}{}", play.from, play.to);
            let mv = create_move_from_algebraic(&mv).unwrap();

            if let Err(invalid_play) = engine.make_move(mv) {
                log::debug!("invalid_play: {invalid_play:?}");

                tcp.write_all(format!("game {game_id} play {role} resigns _\n").as_bytes())?;
                return Ok(());
            }

            log::debug!("{}", game.board);
        } else if Some("game_over") == message.get(1).copied() {
            return Ok(());
        }

        buf.clear();
    }
}

fn systemd_delay_restart(args: &Args) -> anyhow::Result<()> {
    if args.systemd {
        let service = match args.role {
            Role::Attacker => "hnefatafl-ai-basic-attacker.service",
            Role::Defender => "hnefatafl-ai-basic-defender.service",
            Role::Roleless => unreachable!(),
        };

        let output = Command::new("systemctl")
            .args(["show", service, "-p", "NRestarts"])
            .output()?;

        let i = String::from_utf8_lossy(&output.stdout)
            .replace("NRestarts=", "")
            .trim()
            .parse()?;

        if i > 0 {
            let delay = 2u64.pow(i);
            log::info!("sleeping for {delay}s...");
            sleep(Duration::from_secs(delay));
        }
    }

    Ok(())
}

fn init_logger(debug: bool, systemd: bool) {
    let mut builder = Builder::new();
    let module = "hnefatafl_org_taflzero";

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
        builder.filter(Some(module), LevelFilter::Info);
    }

    builder.init();
}
