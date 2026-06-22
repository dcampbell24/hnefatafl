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

// Need to keep track of all the active games in which it is my turn, then
// starting with the game with the least time left, play.

// Go through the loop normally, but when it is my turn send on a channel to make a
// move.
use std::{
    env, fs,
    io::{BufRead, BufReader, Write},
    net::{TcpStream, ToSocketAddrs},
    process::Command,
    str::FromStr,
    sync::mpsc::{Receiver, Sender, channel},
    thread,
    time::Duration,
};

use anyhow::Error;
use clap::{CommandFactory, Parser};
use colored::Colorize;
use env_logger::Builder;
use hnefatafl_copenhagen::{
    COPYRIGHT, SOFTWARE_ID, VERSION_ID,
    ai::{AI, AiMonteCarlo},
    game::Game,
    opentafl::OpenTaflGame,
    play::{Plae, Play, Vertex},
    role::Role,
};
use log::LevelFilter;
use socket2::{Domain, SockAddr, Socket, TcpKeepalive, Type};
use taflzero::{Engine, moves::mv::create_move_from_algebraic};

const PORT: &str = ":49152";
const ONNX_PATH: &str = "/usr/share/taflzero/default_nn.onnx";
const MONTE_CARLO_SECONDS: u64 = 16;
const MONTE_CARLO_DEPTH: u8 = 20;

/// `TaflZero` AI
///
/// This is the taflzero client that connects to a hnefatafl.org server.
#[derive(Parser, Debug)]
#[command(version, about)]
#[allow(clippy::struct_excessive_bools)]
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

    /// Join a tournament
    #[arg(long)]
    join_tournament: bool,

    /// Play in a tournament
    #[arg(long)]
    play_tournament: bool,

    /// Whether the application is being run by systemd
    #[arg(long)]
    systemd: bool,

    /// Search for `u64` milliseconds
    #[arg(long, default_value_t = 4_000)]
    search_ms: u64,

    /// Whether to log at the debug level
    #[arg(long)]
    debug: bool,

    /// Build the man page
    #[arg(long)]
    man: bool,
}

struct TaflZero {
    play_tournament: bool,
    game_id: Option<u128>,
    role: Role,
    reader: BufReader<TcpStream>,
    tcp: TcpStream,
    tx: Sender<(u128, OpenTaflGame)>,
    _genmove_rx: Receiver<(u128, Plae)>,
}

#[allow(clippy::too_many_lines)]
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
    tcp.write_all(format!("software_id {SOFTWARE_ID}\n").as_bytes())?;

    if args.join_tournament {
        tcp.write_all("join_tournament\n".as_bytes())?;
        return Ok(());
    }

    let mut buf = String::new();
    reader.read_line(&mut buf)?;
    assert_eq!(buf, "= login\n");
    buf.clear();

    let role = &args.role;

    let onnx_path = if fs::exists(ONNX_PATH)? {
        ONNX_PATH
    } else {
        "default_nn.onnx"
    };

    let mut game_id = None;

    if let Some(id) = args.join_game {
        let id = id.to_string();
        tcp.write_all(format!("join_game_pending {id}\n").as_bytes())?;

        game_id = Some(0);
    }

    let (tx, rx) = channel();
    let (mut genmove_tx, genmove_rx) = channel();

    let mut taflzero = TaflZero {
        play_tournament: args.play_tournament,
        game_id,
        role: *role,
        reader,
        tcp,
        tx,
        _genmove_rx: genmove_rx,
    };

    thread::spawn(move || {
        let mut engine = Engine::new(onnx_path.to_string());
        let mut ai = AiMonteCarlo::new(Duration::from_secs(MONTE_CARLO_SECONDS), MONTE_CARLO_DEPTH);

        loop {
            let (id, opentafl_game) = rx.recv().unwrap();

            engine.set_start_position();

            // Play all the moves already played in both games.
            // If you get an error resign.

            log::info!("{opentafl_game:#?}");
            generate_move(
                &mut ai,
                &mut engine,
                args.search_ms,
                &opentafl_game,
                &mut genmove_tx,
                id,
            )
            .unwrap();
        }
    });

    loop {
        handle_messages(&mut taflzero)?;

        if args.join_game.is_some() {
            return Ok(());
        }
    }
}

#[allow(clippy::too_many_lines)]
fn handle_messages(taflzero: &mut TaflZero) -> anyhow::Result<()> {
    let mut buf = String::new();

    if !taflzero.play_tournament && taflzero.game_id.is_none() {
        taflzero.tcp.write_all(
            format!("new_game {} rated fischer 900000 10 11\n", taflzero.role).as_bytes(),
        )?;
        taflzero.game_id = Some(0);
    }

    taflzero.reader.read_line(&mut buf)?;
    if buf.trim().is_empty() {
        return Err(Error::msg("the TCP stream has closed"));
    }

    let message: Vec<_> = buf.split_ascii_whitespace().collect();

    match (message.get(1).copied(), message.get(2).copied()) {
        (Some("resume_game_json"), _) => {
            let game: Vec<_> = message.iter().skip(2).copied().collect();
            let game = game.join(" ");
            let game: OpenTaflGame = serde_json::de::from_str(&game)?;

            taflzero.tx.send((
                taflzero.game_id.expect("We should have an ID by now!"),
                game,
            ))?;
        }
        (Some(id), Some("generate_move")) => {
            taflzero
                .tcp
                .write_all(format!("resume_game_json {id}\n").as_bytes())?;
        }
        (Some("challenge_requested"), Some(game_id)) => {
            log::info!("{message:?}");

            taflzero
                .tcp
                .write_all(format!("join_game {game_id}\n").as_bytes())?;
        }
        _ => log::debug!("{buf}"),
    }

    buf.clear();

    Ok(())
}

fn systemd_delay_restart(args: &Args) -> anyhow::Result<()> {
    if args.systemd {
        let service = match args.role {
            Role::Attacker => "hnefatafl-ai-attacker.service",
            Role::Defender => "hnefatafl-ai-defender.service",
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
            thread::sleep(Duration::from_secs(delay));
        }
    }

    Ok(())
}

fn init_logger(debug: bool, systemd: bool) {
    let mut builder = Builder::new();
    let module = "taflzero";

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

fn generate_move(
    ai: &mut AiMonteCarlo,
    engine: &mut Engine,
    search_ms: u64,
    opentafl_game: &OpenTaflGame,
    genmove_tx: &mut Sender<(u128, Plae)>,
    id: u128,
) -> anyhow::Result<()> {
    let mut game = Game::from(opentafl_game);
    let turn = game.turn;

    engine.make_search(search_ms, None);

    if let Some(mv) = engine.best_move() {
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
        let play = Plae::Play(Play {
            role: game.turn,
            from,
            to,
        });

        log_play(&play, true);

        if game.play(&play).is_err() {
            let generate_move = ai.generate_move(&mut game)?;
            log::info!("changed play to: {generate_move}");

            match &generate_move.play {
                Plae::Play(play) => {
                    let mv =
                        create_move_from_algebraic(&format!("{}{}", play.from, play.to)).unwrap();

                    if let Err(invalid_play) = engine.make_move(mv) {
                        log::error!("invalid_play: {invalid_play:?}");
                        let play = player_resigns(turn);
                        genmove_tx.send((id, play))?;
                    }

                    genmove_tx.send((id, generate_move.play))?;
                }
                Plae::AttackerResigns | Plae::DefenderResigns => {
                    genmove_tx.send((id, generate_move.play))?;
                }
            }
        } else {
            if let Err(invalid_play) = engine.make_move(mv) {
                log::error!("invalid_play: {invalid_play:?}");

                let play = player_resigns(turn);
                genmove_tx.send((id, play))?;
            }

            genmove_tx.send((id, play))?;
        }
    } else {
        let play = player_resigns(turn);
        genmove_tx.send((id, play))?;
    }

    log::debug!("\n{game}");

    Ok(())
}

fn player_resigns(role: Role) -> Plae {
    match role {
        Role::Attacker => Plae::AttackerResigns,
        Role::Defender => Plae::DefenderResigns,
        Role::Roleless => unreachable!(),
    }
}

fn log_play(play_full: &Plae, sending: bool) {
    let direction = if sending { '>' } else { '<' };

    match play_full {
        Plae::Play(play) => match play.role {
            Role::Attacker => log::info!("{direction} {}", play_full.to_string().red()),
            Role::Defender => log::info!("{direction} {}", play_full.to_string().blue()),
            Role::Roleless => unreachable!(),
        },
        Plae::AttackerResigns => log::info!("{}", play_full.to_string().red()),
        Plae::DefenderResigns => log::info!("{}", play_full.to_string().blue()),
    }
}
