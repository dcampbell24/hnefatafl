use std::{
    io::{self, BufRead, BufReader, Write},
    net::TcpStream,
    thread,
    time::Duration,
};

use anyhow::Error;
use clap::{CommandFactory, Parser, command};
use hnefatafl_copenhagen::{
    COPYRIGHT, LONG_VERSION, VERSION_ID,
    ai::{AI, AiBanal, AiBasic, AiMonteCarlo},
    game::Game,
    play::Plae,
    role::Role,
    status::Status, utils,
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
    #[arg(long)]
    username: String,

    #[arg(long)]
    password: String,

    /// attacker or defender
    #[arg(long)]
    role: Role,

    /// Connect to the HTP server at host
    #[arg(default_value = "hnefatafl.org", long)]
    host: String,

    /// Choose an AI to play as
    #[arg(default_value = "monte-carlo", long)]
    ai: String,

    /// Challenge the AI with AI CHALLENGER
    #[arg(long)]
    challenger: Option<String>,

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

    if let Some(ai_2) = args.challenger {
        new_game(&mut tcp, args.role, &mut reader, &mut buf)?;

        let message: Vec<_> = buf.split_ascii_whitespace().collect();
        let game_id = message[3].to_string();
        buf.clear();

        let game_id_2 = game_id.clone();
        let ai = args.ai;
        let tcp_clone = tcp.try_clone()?;
        thread::spawn(move || accept_challenger(&ai, &mut reader, &mut buf, &mut tcp, &game_id));

        let mut buf_2 = String::new();
        let mut tcp_2 = TcpStream::connect(address)?;
        let mut reader_2 = BufReader::new(tcp_2.try_clone()?);

        tcp_2.write_all(format!("{VERSION_ID} login ai-01 PASSWORD\n").as_bytes())?;
        reader_2.read_line(&mut buf_2)?;
        assert_eq!(buf_2, "= login\n");

        tcp_2.write_all(format!("join_game_pending {game_id_2}\n").as_bytes())?;
        let tcp_2_clone = tcp_2.try_clone()?;
        thread::spawn(move || {
            handle_messages(ai_2.as_str(), &game_id_2, &mut reader_2, &mut tcp_2)
        });

        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;
        tcp_clone.shutdown(std::net::Shutdown::Both)?;
        tcp_2_clone.shutdown(std::net::Shutdown::Both)?;
    } else {
        loop {
            new_game(&mut tcp, args.role, &mut reader, &mut buf)?;

            let message: Vec<_> = buf.split_ascii_whitespace().collect();
            info!("{message:?}");
            let game_id = message[3].to_string();
            buf.clear();

            wait_for_challenger(&mut reader, &mut buf, &mut tcp, &game_id)?;

            handle_messages(args.ai.as_str(), &game_id, &mut reader, &mut tcp)?;
        }
    }

    Ok(())
}

fn accept_challenger(
    ai: &str,
    reader: &mut BufReader<TcpStream>,
    buf: &mut String,
    tcp: &mut TcpStream,
    game_id: &str,
) -> anyhow::Result<()> {
    wait_for_challenger(reader, buf, tcp, game_id)?;

    handle_messages(ai, game_id, reader, tcp)?;
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
    game_id: &str,
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
    ai: &str,
    game_id: &str,
    reader: &mut BufReader<TcpStream>,
    tcp: &mut TcpStream,
) -> anyhow::Result<()> {
    let mut game = Game::default();
    let mut ai = choose_ai(ai, &game)?;

    debug!("{game}\n");

    let mut buf = String::new();
    loop {
        reader.read_line(&mut buf)?;

        if buf.trim().is_empty() {
            return Err(Error::msg("the TCP stream has closed"));
        }

        let message: Vec<_> = buf.split_ascii_whitespace().collect();

        if Some("generate_move") == message.get(2).copied() {
            let generate_move = ai.generate_move(&mut game);
            let play = generate_move.play.expect("the game must be in progress");
            game.play(&play)?;

            tcp.write_all(format!("game {game_id} {play}\n").as_bytes())?;

            debug!("{game}");
            info!(
                "play: {play} score: {} delay milliseconds: {}",
                generate_move.score, generate_move.delay_milliseconds
            );
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

fn choose_ai(ai: &str, game: &Game) -> anyhow::Result<Box<dyn AI>> {
    match ai {
        "banal" => Ok(Box::new(AiBanal)),
        "basic" => Ok(Box::new(AiBasic::new(
            Duration::from_secs(10),
            4
        ))),
        "monte-carlo" => Ok(Box::new(AiMonteCarlo::new(
            game,
            Duration::from_secs(10),
            20,
        )?)),
        _ => Err(anyhow::Error::msg("you didn't choose a valid AI")),
    }
}
