use std::{
    io::{self, BufReader},
    net::TcpStream,
    process::{Command, ExitStatus},
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

    /// The number of Monte Carlo iterations
    #[arg(default_value_t = 1_000, long)]
    loops: i64,

    /// Listen for HTP drivers on host and port
    #[arg(long, value_name = "host:port")]
    tcp: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if let Some(address) = args.tcp {
        play_tcp(&address, args.display_game, args.loops)?;
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
    let mut ai = AiMonteCarlo::new(game.board.size(), args.loops)?;

    if args.display_game {
        clear_screen()?;
        println!("{game}\n");
    }

    loop {
        let (play, score, delay_milliseconds) = ai.generate_move(&mut game);
        let play = play.ok_or(anyhow::Error::msg("The game is already over."))?;

        if args.display_game {
            clear_screen()?;
            println!("{game}\n");
        }

        println!("= {play}, score: {score}, delay milliseconds: {delay_milliseconds}");

        if game.status != Status::Ongoing {
            return Ok(());
        }

        buffer.clear();
    }
}

fn play_tcp(address: &str, display_game: bool, loops: i64) -> anyhow::Result<()> {
    let mut game = Game::default();
    let mut ai: Box<dyn AI + 'static> = Box::new(AiMonteCarlo::new(game.board.size(), loops)?);
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
                    let (play, score, delay_milliseconds) = ai.generate_move(&mut game);
                    let play = play.ok_or(anyhow::Error::msg("The game is already over."))?;

                    write_command(&format!("{play}\n"), &mut stream)?;

                    if display_game {
                        println!("{game}\nscore: {score} delay milliseconds: {delay_milliseconds}");
                    }
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
