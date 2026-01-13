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
    io::{self, BufReader, Write},
    net::TcpStream,
};

use clap::{self, CommandFactory, Parser};

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

    /// Render everything in ASCII
    #[arg(long)]
    ascii: bool,

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

    let args = Args::parse();

    let mut game = Game::default();
    if args.ascii {
        game.chars.ascii();
        game.board.display_ascii = true;
    }

    if let Some(mut address) = args.host {
        address.push_str(SERVER_PORT);

        let ai = match args.ai {
            Some(ai) => choose_ai(&ai, args.seconds, args.depth, true)?,
            None => choose_ai("basic", args.seconds, args.depth, true)?,
        };

        play_tcp(game, ai, &address, args.display_game)?;
    } else if let Some(ai) = args.ai {
        let ai = choose_ai(&ai, args.seconds, args.depth, true)?;

        play_ai(game, ai, args.display_game)?;
    } else {
        play(game, args.display_game)?;
    }

    Ok(())
}

fn play(mut game: Game, display_game: bool) -> anyhow::Result<()> {
    let mut buffer = String::new();
    let stdin = io::stdin();

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

fn play_ai(mut game: Game, mut ai: Box<dyn AI>, display_game: bool) -> anyhow::Result<()> {
    let mut buffer = String::new();

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

fn play_tcp(
    mut game: Game,
    mut ai: Box<dyn AI>,
    address: &str,
    display_game: bool,
) -> anyhow::Result<()> {
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
                _ => match game.read_line(&message) {
                    Err(error) => println!("? {error}\n"),
                    Ok(message) => {
                        if let Some(message) = message {
                            println!("= {message}");
                        }
                    }
                },
            }
        }

        if game.status != Status::Ongoing {
            break;
        }
    }

    Ok(())
}
