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

use std::{
    io::{BufReader, Write},
    net::{Shutdown, TcpListener, TcpStream},
    thread,
};

use clap::{self, CommandFactory, Parser};

use hnefatafl_copenhagen::{
    COPYRIGHT, SERVER_PORT, game::Game, read_response, status::Status, utils::clear_screen,
    write_command,
};

/// A Hnefatafl Copenhagen Server
///
/// This is a TCP server that listens for HTP engines
/// to connect and then plays them against each other.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Listen for HTP drivers on host and port
    #[arg(default_value = "0.0.0.0", index = 1, value_name = "host")]
    host: String,

    /// Build the manpage
    #[arg(long)]
    man: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.man {
        let mut buffer: Vec<u8> = Vec::default();
        let cmd = Args::command().name("hnefatafl-server").long_version(None);
        let man = clap_mangen::Man::new(cmd).date("2025-11-21");

        man.render(&mut buffer)?;
        write!(buffer, "{COPYRIGHT}")?;

        std::fs::write("hnefatafl-server.1", buffer)?;
        return Ok(());
    }

    let mut address = args.host;
    address.push_str(SERVER_PORT);
    start(&address)
}

struct Htp {
    attacker_connection: TcpStream,
    defender_connection: TcpStream,
}

impl Htp {
    fn start(&mut self) -> anyhow::Result<()> {
        let mut attacker_reader = BufReader::new(self.attacker_connection.try_clone()?);
        let mut defender_reader = BufReader::new(self.defender_connection.try_clone()?);

        let mut game = Game::default();

        loop {
            write_command("generate_move attacker\n", &mut self.attacker_connection)?;
            let attacker_move = read_response(&mut attacker_reader)?;
            game.read_line(&attacker_move)?;
            write_command(&attacker_move, &mut self.defender_connection)?;

            clear_screen()?;
            println!("{game}");

            if game.status != Status::Ongoing {
                break;
            }

            write_command("generate_move defender\n", &mut self.defender_connection)?;
            let defender_move = read_response(&mut defender_reader)?;
            game.read_line(&defender_move)?;
            write_command(&defender_move, &mut self.attacker_connection)?;

            clear_screen()?;
            println!("{game}");

            if game.status != Status::Ongoing {
                break;
            }
        }

        self.attacker_connection.shutdown(Shutdown::Both)?;
        self.defender_connection.shutdown(Shutdown::Both)?;

        Ok(())
    }
}

fn start(address: &str) -> anyhow::Result<()> {
    let listener = TcpListener::bind(address)?;
    println!("listening on {address} ...");

    let mut players = Vec::new();

    for stream in listener.incoming() {
        let stream = stream?;

        if players.is_empty() {
            players.push(stream);
        } else {
            let mut game = Htp {
                attacker_connection: players.pop().unwrap(),
                defender_connection: stream,
            };

            thread::spawn(move || game.start());
        }
    }

    Ok(())
}
