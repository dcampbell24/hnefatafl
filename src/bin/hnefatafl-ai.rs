use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
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

// Move 26, defender wins, corner escape, time per move 15s 2025-03-06 (hnefatafl-equi).

const PORT: &str = ":49152";

/// Copenhagen Hnefatafl AI
///
/// This is an AI client that connects to a server.
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

    /// Build the manpage
    #[arg(long)]
    man: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    utils::init_logger(false);

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

    let mut address = args.host.clone();
    address.push_str(PORT);

    let mut buf = String::new();
    let mut tcp = TcpStream::connect(address.clone())?;
    let mut reader = BufReader::new(tcp.try_clone()?);

    tcp.write_all(format!("{VERSION_ID} login {username} {}\n", args.password).as_bytes())?;
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

            let message: Vec<_> = buf.split_ascii_whitespace().collect();
            info!("{message:?}");
            let game_id = message[3].parse::<u128>()?;
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

        let message: Vec<_> = buf.split_ascii_whitespace().collect();

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
            let words = &message[2..];
            let play = Plae::try_from(words.to_vec())?;
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
