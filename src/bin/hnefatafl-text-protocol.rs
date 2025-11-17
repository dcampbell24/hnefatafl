use std::{
    io::{self, BufReader, Write},
    net::TcpStream,
};

use clap::{self, CommandFactory, Parser, command};

use hnefatafl_copenhagen::{
    COPYRIGHT, SERVER_PORT,
    ai::AI,
    game::Game,
    play::Plae,
    read_response,
    status::Status,
    utils::{choose_ai, clear_screen},
    write_command,
};

/// Hnefatafl Copenhagen
///
/// This plays the game using the Hnefatafl Text Protocol.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Choose an AI to play as
    #[arg(long)]
    ai: Option<String>,

    /// Displays the game
    #[arg(long)]
    display_game: bool,

    /// How many seconds to run the monte-carlo AI
    #[arg(long)]
    seconds: Option<u64>,

    /// How deep in the game tree to go with Ai
    #[arg(long)]
    depth: Option<u8>,

    /// Listen for HTP drivers on host
    #[arg(long)]
    host: Option<String>,

    /// Build the manpage
    #[arg(long)]
    man: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.man {
        let mut buffer: Vec<u8> = Vec::default();
        let cmd = Args::command()
            .name("hnefatafl-text-protocol")
            .long_version(None);
        let man = clap_mangen::Man::new(cmd).date("2025-11-17");

        man.render(&mut buffer)?;
        write!(buffer, "{COPYRIGHT}")?;

        std::fs::write("hnefatafl-text-protocol.1", buffer)?;
        return Ok(());
    }

    if let Some(mut address) = args.host {
        address.push_str(SERVER_PORT);

        let ai = match args.ai {
            Some(ai) => choose_ai(&ai, args.seconds, args.depth)?,
            None => choose_ai("monte-carlo", args.seconds, args.depth)?,
        };

        play_tcp(ai, &address, args.display_game)?;
    } else if let Some(ai) = args.ai {
        let ai = choose_ai(&ai, args.seconds, args.depth)?;

        play_ai(ai, args.display_game)?;
    } else {
        play(args.display_game)?;
    }

    Ok(())
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

fn play_ai(mut ai: Box<dyn AI>, display_game: bool) -> anyhow::Result<()> {
    let mut buffer = String::new();
    let mut game = Game::default();

    if display_game {
        clear_screen()?;
        println!("{game}\n");
    }

    loop {
        let generate_move = ai.generate_move(&mut game)?;

        if display_game {
            clear_screen()?;
            println!("{game}\n");
        }

        println!("= {generate_move}");

        if game.status != Status::Ongoing {
            return Ok(());
        }

        buffer.clear();
    }
}

fn play_tcp(mut ai: Box<dyn AI>, address: &str, display_game: bool) -> anyhow::Result<()> {
    let mut game = Game::default();
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
                    let generate_move = ai.generate_move(&mut game)?;
                    write_command(&format!("{}\n", generate_move.play), &mut stream)?;

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
