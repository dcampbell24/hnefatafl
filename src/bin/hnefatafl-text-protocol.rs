use std::{
    io::{self, BufReader},
    net::TcpStream,
    process::{Command, ExitStatus},
    time::Duration,
};

use clap::command;
use clap::{self, Parser};

use hnefatafl_copenhagen::{
    ai::{AI, AiMonteCarlo},
    game::Game,
    play::Plae,
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

    /// How many seconds to run Monte Carlo loops
    #[arg(default_value_t = 10, long)]
    seconds: u64,

    /// How deep in the game tree to go with Monte Carlo
    #[arg(default_value_t = 20, long)]
    depth: i32,

    /// Listen for HTP drivers on host and port
    #[arg(long, value_name = "host:port")]
    tcp: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let seconds = Duration::from_secs(args.seconds);

    if let Some(address) = args.tcp {
        play_tcp(&address, args.display_game, seconds, args.depth)?;
    } else if args.ai {
        play_ai(args.display_game, seconds, args.depth)?;
    } else {
        play(args.display_game)?;
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

fn play(display_game: bool) -> anyhow::Result<()> {
    let mut buffer = String::new();
    let stdin = io::stdin();
    let mut game = Game::default();

    if display_game {
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

        if display_game {
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

fn play_ai(display_game: bool, seconds: Duration, depth: i32) -> anyhow::Result<()> {
    let mut buffer = String::new();
    let mut game = Game::default();
    let mut ai = AiMonteCarlo::new(&game, seconds, depth)?;

    if display_game {
        clear_screen()?;
        println!("{game}\n");
    }

    loop {
        let generate_move = ai.generate_move(&mut game);
        if generate_move.play.is_none() {
            return Err(anyhow::Error::msg("The game is already over."));
        }

        if display_game {
            clear_screen()?;
            println!("{game}\n");
        } else {
            println!("{}", generate_move.heat_map);
        }

        println!("= {generate_move}");

        if game.status != Status::Ongoing {
            return Ok(());
        }

        buffer.clear();
    }
}

fn play_tcp(
    address: &str,
    display_game: bool,
    seconds: Duration,
    depth: i32,
) -> anyhow::Result<()> {
    let mut game = Game::default();
    let mut ai: Box<dyn AI + 'static> = Box::new(AiMonteCarlo::new(&game, seconds, depth)?);
    let mut stream = TcpStream::connect(address)?;
    println!("connected to {address} ...");

    let mut reader = BufReader::new(stream.try_clone()?);
    for i in 1..10_000 {
        println!("\n*** turn {i} ***");

        let message = read_response(&mut reader)?;

        let words: Vec<_> = message.as_str().split_ascii_whitespace().collect();

        if let Some(word) = words.first() {
            match *word {
                "play" => {
                    let play = Plae::try_from(words)?;
                    ai.play(&mut game, &play)?;

                    if display_game {
                        println!("{game}");
                    }
                }
                "generate_move" => {
                    let generate_move = ai.generate_move(&mut game);
                    if let Some(play) = &generate_move.play {
                        write_command(&format!("{play}\n"), &mut stream)?;
                    } else {
                        return Err(anyhow::Error::msg("The game is already over."));
                    }

                    if display_game {
                        println!("{game}");
                    }

                    println!("{generate_move}");
                    println!("{}", generate_move.heat_map);
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
