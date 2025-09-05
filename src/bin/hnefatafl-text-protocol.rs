use std::{
    io::{self, BufReader},
    net::TcpStream,
    process::{Command, ExitStatus},
};

use clap::command;
use clap::{self, Parser};

use hnefatafl_copenhagen::{
    ai::{AI, AiBanal},
    game::Game,
    mcts::monte_carlo_tree_search,
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

    /// Listen for HTP drivers on host and port
    #[arg(long, value_name = "host:port")]
    tcp: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut ai: Box<dyn AI + 'static> = Box::new(AiBanal);

    if let Some(tcp) = args.tcp {
        let address = tcp.as_str();
        let mut stream = TcpStream::connect(address)?;
        println!("connected to {address} ...");

        let mut reader = BufReader::new(stream.try_clone()?);
        let mut game = Game::default();

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
                        let play = game
                            .generate_move(&mut ai)
                            .expect("the game must be in progress");

                        game.play(&play)?;
                        write_command(&play.to_string(), &mut stream)?;
                    }
                    _ => unreachable!("You can't get here!"),
                }
            }

            if game.status != Status::Ongoing {
                break;
            }
        }

        return Ok(());
    }

    let mut buffer = String::new();
    let stdin = io::stdin();

    let mut game = Game::default();

    if args.display_game {
        clear_screen()?;
        println!("{game}\n");
        println!("Enter 'list_commands' for a list of commands.");
    }

    loop {
        let result = if args.ai {
            let play = monte_carlo_tree_search(&game).expect("There should be a valid play.");
            let captures = game.play(&play);
            println!("{captures:?}");
            Ok(Some(String::new()))
        } else {
            if let Err(error) = stdin.read_line(&mut buffer) {
                println!("? {error}\n");
                buffer.clear();
                continue;
            }

            game.read_line(&buffer)
        };

        if args.display_game {
            #[cfg(any(target_family = "unix", target_family = "windows"))]
            clear_screen()?;
            println!("{game}\n");
        }

        match result {
            Err(error) => println!("? {error}\n"),
            Ok(message) => {
                if let Some(message) = message {
                    println!("= {message}\n");
                }
            }
        }

        buffer.clear();
    }
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
