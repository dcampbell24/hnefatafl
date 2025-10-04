use std::{
    io::{self, BufReader},
    net::TcpStream,
    process::{Command, ExitStatus},
};

use clap::command;
use clap::{self, Parser};

use hnefatafl_copenhagen::{
    ai::{AI, AiBanal, AiMonteCarlo},
    game::Game,
    read_response,
    status::Status,
    write_command,
};

/// Hnefatafl Copenhagen
///
/// This plays the game using the Hnefatafl Text Protocol.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Play the game with AI
    #[arg(long)]
    ai: bool,

    /// Displays the game
    #[arg(long)]
    display_game: bool,

    /// The number of Monte Carlo iterations
    #[arg(default_value_t = 1_000, long)]
    loops: u32,

    /// Listen for HTP drivers on host and port
    #[arg(long, value_name = "host:port")]
    tcp: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if let Some(address) = args.tcp {
        play_tcp(&address)?;
    } else if args.ai {
        play_ai(&args)?;
    } else {
        play(&args)?;
    }

    Ok(())
}

fn clear_screen() -> anyhow::Result<ExitStatus> {
    #[cfg(not(any(target_family = "unix", target_family = "windows")))]
    return Ok(1);

    #[cfg(target_family = "unix")]
    let exit_status = Command::new("clear").status()?;

    #[cfg(target_family = "windows")]
    let exit_status = Command::new("cls").status()?;

    Ok(exit_status)
}

fn play(args: &Args) -> anyhow::Result<()> {
    let mut buffer = String::new();
    let stdin = io::stdin();
    let mut game = Game::default();

    if args.display_game {
        clear_screen()?;
        println!("{game}\n");
        println!("Enter 'list_commands' for a list of commands.");
    }

    loop {
        if let Err(error) = stdin.read_line(&mut buffer) {
            println!("? {error}\n");
            buffer.clear();
            return Ok(());
        }

        let result = game.read_line(&buffer);

        if args.display_game {
            clear_screen()?;
            println!("{game}\n");
        }

        match result {
            Err(error) => println!("? {error}\n"),
            Ok(message) => {
                if let Some(message) = message {
                    println!("= {message}");
                }
            }
        }

        buffer.clear();
    }
}

fn play_ai(args: &Args) -> anyhow::Result<()> {
    let mut buffer = String::new();
    let mut game = Game::default();
    let mut ai = AiMonteCarlo::new(game.board.size())?;

    if args.display_game {
        clear_screen()?;
        println!("{game}\n");
    }

    loop {
        let (play, score) = ai.generate_move(&mut game, args.loops);
        let play = play.ok_or(anyhow::Error::msg("The game is already over."))?;

        if args.display_game {
            clear_screen()?;
            println!("{game}\n");
        }

        println!("= {play}, score: {score}");

        if game.status != Status::Ongoing {
            return Ok(());
        }

        buffer.clear();
    }
}

fn play_tcp(address: &str) -> anyhow::Result<()> {
    let mut game = Game::default();
    let mut ai: Box<dyn AI + 'static> = Box::new(AiBanal);
    let mut stream = TcpStream::connect(address)?;
    println!("connected to {address} ...");

    let mut reader = BufReader::new(stream.try_clone()?);
    for i in 1..10_000 {
        println!("\n*** turn {i} ***");

        let message = read_response(&mut reader)?;

        if let Some(word) = message
            .as_str()
            .split_ascii_whitespace()
            .collect::<Vec<_>>()
            .first()
        {
            match *word {
                "play" => {
                    game.read_line(&message)?;
                }
                "generate_move" => {
                    let (play, _score) = game.generate_move(&mut ai);
                    let play = play.expect("the game must be in progress");

                    write_command(&format!("{play}\n"), &mut stream)?;
                }
                _ => unreachable!("You can't get here!"),
            }
        }

        if game.status != Status::Ongoing {
            break;
        }
    }

    Ok(())
}
