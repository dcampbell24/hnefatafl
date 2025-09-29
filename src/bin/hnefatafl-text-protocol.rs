use std::{
    io::{self, BufReader, Stdin},
    net::TcpStream,
    process::{Command, ExitStatus},
    sync::mpsc::channel,
};

use clap::command;
use clap::{self, Parser};

use hnefatafl_copenhagen::{
    ai::{AI, AiBanal},
    game::Game,
    game_tree::Tree,
    read_response,
    role::Role,
    status::Status,
    write_command,
};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};

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
        return tcp_connect(&address);
    }

    let mut buffer = String::new();
    let stdin = io::stdin();
    let mut game = Game::default();

    let count = std::thread::available_parallelism()?.get();
    let mut trees = Vec::with_capacity(count);
    for _ in 0..count {
        trees.push(Tree::new(game.board.size()));
    }

    if args.display_game {
        clear_screen()?;
        println!("{game}\n");
        println!("Enter 'list_commands' for a list of commands.");
    }

    loop {
        if args.ai {
            play_ai(&args, &mut game, &mut trees)?;
        } else {
            play(&mut buffer, &stdin, &args, &mut game)?;
        }

        buffer.clear();
    }
}

fn play(buffer: &mut String, stdin: &Stdin, args: &Args, game: &mut Game) -> anyhow::Result<()> {
    if let Err(error) = stdin.read_line(buffer) {
        println!("? {error}\n");
        buffer.clear();
        return Ok(());
    }

    let result = game.read_line(buffer);

    if args.display_game {
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

    Ok(())
}

fn play_ai(args: &Args, game: &mut Game, trees: &mut Vec<Tree>) -> anyhow::Result<()> {
    let (tx, rx) = channel();
    trees.par_iter_mut().for_each_with(tx, |tx, tree| {
        let nodes = tree.monte_carlo_tree_search(args.loops);
        tx.send(nodes).unwrap();
    });
    let mut nodes: Vec<_> = rx.iter().flatten().collect();
    nodes.sort_by(|a, b| a.score.total_cmp(&b.score));

    let turn = game.turn;
    let node = match turn {
        Role::Attacker => nodes.last().unwrap(),
        Role::Defender => nodes.first().unwrap(),
        Role::Roleless => unreachable!(),
    };

    let play = node.play.as_ref().unwrap();
    match game.play(play) {
        Ok(_captures) => {}
        Err(err) => {
            println!("invalid play: {play}");
            return Err(err);
        }
    }

    let hash = game.calculate_hash();
    let mut here_tree = Tree::new(game.board.size());
    for tree in trees.iter() {
        if hash == tree.here_game().calculate_hash() {
            here_tree = tree.clone();
        }
    }
    for tree in trees {
        if hash != tree.here_game().calculate_hash() {
            *tree = here_tree.clone();
        }
    }

    if args.display_game {
        clear_screen()?;
        println!("{game}\n");
    }

    println!("= {play}, score: {}", node.score);

    if game.status != Status::Ongoing {
        return Ok(());
    }

    match turn {
        Role::Attacker => {
            for node in nodes.iter().rev().take(10) {
                println!("{node}");
            }
        }
        Role::Defender => {
            for node in &nodes[..10] {
                println!("{node}");
            }
        }
        Role::Roleless => unreachable!(),
    }

    Ok(())
}

fn tcp_connect(address: &str) -> anyhow::Result<()> {
    let mut ai: Box<dyn AI + 'static> = Box::new(AiBanal);
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
