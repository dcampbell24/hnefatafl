//! An AI, client, engine, and server for the game of Copenhagen Hnefatafl.
//!
//! ## Feature Flags
//!
//! By default the `client` and `server` feature flags are enabled.
//!
//! * client - enable the `hnefatafl-client` binary
//! * console - on Windows print output to the console
//! * debug - enable iced debug mode, also log on the debug level
//! * js - enable options for generating javascript code
//! * runic - enable the `icelandic-runic` binary for translating Icelandic to Icelandic Runic
//! * server - enable the `hnefatafl-server-full` binary
//!
//! ## Message Protocol
//!
//! Get more information about the [message protocol] used by the engine.
//!
//! [message protocol]: https://docs.rs/hnefatafl-copenhagen/latest/hnefatafl_copenhagen/message/enum.Message.html

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

#![deny(clippy::panic)]

use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
};

rust_i18n::i18n!();

pub mod accounts;
pub mod ai;
pub mod board;
pub mod characters;
pub mod draw;
pub mod game;
pub mod game_tree;
pub mod glicko;
pub mod heat_map;
pub mod locale;
pub mod message;
pub mod play;
pub mod rating;
pub mod role;
pub mod server_game;
pub mod smtp;
pub mod space;
pub mod status;
pub mod time;
pub mod tournament;
pub mod tree;
pub mod utils;

pub type Id = u128;
pub const HOME: &str = "hnefatafl-copenhagen";
pub const SERVER_PORT: &str = ":49152";
pub const SOCKET_PATH: &str = "/tmp/hnefatafl.sock";
pub const VERSION_ID: &str = "ad746a65";

pub const COPYRIGHT: &str = r".SH COPYRIGHT
Copyright (C) 2025-2026 David Lawrence Campbell

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
";

pub const LONG_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "
Copyright (c) 2025 David Lawrence Campbell
Licensed under the AGPLv3"
);

/// # Errors
///
/// If read fails.
pub fn read_response(reader: &mut BufReader<TcpStream>) -> anyhow::Result<String> {
    let mut reply = String::new();
    reader.read_line(&mut reply)?;
    print!("<- {reply}");
    Ok(reply)
}

/// # Errors
///
/// If write fails.
pub fn write_command(command: &str, stream: &mut TcpStream) -> anyhow::Result<()> {
    print!("-> {command}");
    stream.write_all(command.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fmt, str::FromStr, time::Duration};

    use crate::{
        ai::{AI, AiBanal},
        board::BoardSize,
        game_tree::Tree,
    };

    use super::*;
    use board::{Board, STARTING_POSITION_11X11};
    use game::Game;
    use play::Vertex;
    use role::Role;
    use status::Status;

    fn assert_error_str<T: fmt::Debug>(result: anyhow::Result<T>, string: &str) {
        if let Err(error) = result {
            assert_eq!(error.to_string(), string);
        }
    }

    #[test]
    fn monte_carlo() {
        let mut tree = Tree::new(Game::new_game(BoardSize::_11, None));
        let depth = 80;
        let (loops, _plays) = tree.monte_carlo_tree_search(Duration::from_secs(1), depth);
        println!("{loops}");
    }

    #[ignore = "takes too long"]
    #[test]
    fn monte_carlo_long() {
        let mut tree = Tree::new(Game::new_game(BoardSize::_11, None));
        let depth = 40;
        let (loops, _plays) = tree.monte_carlo_tree_search(Duration::from_secs(10), depth);
        println!("{loops}");
    }

    #[test]
    fn flood_fill_1() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            ".........X.",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".....O.....",
            "....O.O....",
            "....O.O....",
            "....OKO....",
        ];

        let game = game::Game {
            board: board_1.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        let vertex = Vertex::from_str("f1")?;
        assert!(game.board.flood_fill_defender_wins(&vertex));

        Ok(())
    }

    #[test]
    fn flood_fill_2() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            ".........X.",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "....OO.....",
            "....O......",
            "....O.O....",
            "....OKO....",
        ];

        let game = game::Game {
            board: board_1.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        let vertex = Vertex::from_str("f1")?;
        assert!(!game.board.flood_fill_defender_wins(&vertex));

        Ok(())
    }

    // One

    #[test]
    fn starting_position() -> anyhow::Result<()> {
        let game = Game::default();
        assert_eq!(game.board, STARTING_POSITION_11X11.try_into()?);

        Ok(())
    }

    // Two

    #[test]
    fn first_turn() {
        let game = Game::default();
        assert_eq!(game.turn, Role::Attacker);
    }

    // Three

    #[test]
    fn move_orthogonally_1() -> anyhow::Result<()> {
        let board = [
            "...X.......",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "X..O......X",
            "...........",
            "...........",
            "...X.......",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        let mut result = game.read_line("play defender d4 d1");
        assert!(result.is_err());
        assert_error_str(result, "play: you have to play through empty locations");

        result = game.read_line("play defender d4 d11");
        assert!(result.is_err());
        assert_error_str(result, "play: you have to play through empty locations");

        result = game.read_line("play defender d4 a4");
        assert!(result.is_err());
        assert_error_str(result, "play: you have to play through empty locations");

        result = game.read_line("play defender d4 k4");
        assert!(result.is_err());
        assert_error_str(result, "play: you have to play through empty locations");

        Ok(())
    }

    #[test]
    fn move_orthogonally_2() -> anyhow::Result<()> {
        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...O.......",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        // Play a junk move:
        let mut result = game.read_line("play defender junk d1");
        assert!(result.is_err());
        assert_error_str(result, "invalid digit found in string");

        result = game.read_line("play defender d4 junk");
        assert!(result.is_err());
        assert_error_str(result, "invalid digit found in string");

        // Diagonal play:
        result = game.read_line("play defender d4 a3");
        assert!(result.is_err());
        assert_error_str(result, "play: you can only play in a straight line");

        // Play out of bounds:
        result = game.read_line("play defender d4 m4");
        assert!(result.is_err());
        assert_error_str(result, "play: the first letter is not a legal char");

        result = game.read_line("play defender d4 d12");
        assert!(result.is_err());
        assert_error_str(result, "play: invalid coordinate");

        result = game.read_line("play defender d4 d0");
        assert!(result.is_err());
        assert_error_str(result, "play: invalid coordinate");

        // Don't move:
        result = game.read_line("play defender d4 d4");
        assert!(result.is_err());
        assert_error_str(result, "play: you have to change location");

        // Move all the way to the right:
        let mut game_1 = game.clone();
        game_1.read_line("play defender d4 a4")?;
        // Move all the way to the left:
        let mut game_2 = game.clone();
        game_2.read_line("play defender d4 k4")?;
        // Move all the way up:
        let mut game_3 = game.clone();
        game_3.read_line("play defender d4 d11")?;
        // Move all the way down:
        let mut game_4 = game.clone();
        game_4.read_line("play defender d4 d1")?;

        Ok(())
    }

    // Four

    #[test]
    fn sandwich_capture_1() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...X.......",
            "...O.......",
            ".XO.OX.....",
            "...........",
            "...X.......",
            "...........",
        ];

        let board_2 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...X.......",
            "...........",
            ".X.X.X.....",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board_1.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker d2 d4")?;
        assert_eq!(game.board, board_2.try_into()?);

        Ok(())
    }

    #[test]
    fn sandwich_capture_2() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...X.......",
            "...K.......",
            ".XO.OX.....",
            "...........",
            "...X.......",
            "...........",
        ];

        let board_2 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...X.......",
            "...K.......",
            ".X.X.X.....",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board_1.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker d2 d4")?;
        assert_eq!(game.board, board_2.try_into()?);

        Ok(())
    }

    #[test]
    fn sandwich_capture_3() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".....O.....",
            ".X.........",
            "...........",
            "...........",
            "...........",
        ];

        let board_2 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".....X.....",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board_1.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker b4 f4")?;
        assert_eq!(game.board, board_2.try_into()?);

        Ok(())
    }

    #[test]
    fn sandwich_capture_4() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "..K........",
            "...........",
            "...........",
            "..X........",
            "..O........",
            "...........",
        ];

        let board_2 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "..K........",
            "...........",
            "..O........",
            "...........",
        ];

        let mut game = game::Game {
            board: board_1.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        game.read_line("play defender c6 c4")?;
        assert_eq!(game.board, board_2.try_into()?);

        Ok(())
    }

    #[test]
    fn sandwich_capture_5() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".....K.....",
            ".....X.....",
            ".O.........",
            "...........",
            "...........",
            "...........",
        ];

        let board_2 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".....K.....",
            "...........",
            ".....O.....",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board_1.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        game.read_line("play defender b4 f4")?;
        assert_eq!(game.board, board_2.try_into()?);

        Ok(())
    }

    #[test]
    fn sandwich_capture_6() -> anyhow::Result<()> {
        let board_1 = [
            ".O.........",
            "...........",
            "..X........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let board_2 = [
            "..X........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board_1.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker c9 c11")?;
        assert_eq!(game.board, board_2.try_into()?);

        Ok(())
    }

    #[test]
    fn sandwich_capture_7() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".....K.....",
            ".....O.....",
            ".X.........",
            "...........",
            "...........",
            "...........",
        ];

        let board_2 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".....K.....",
            ".....O.....",
            ".....X.....",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board_1.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker b4 f4")?;
        assert_eq!(game.board, board_2.try_into()?);

        Ok(())
    }

    #[test]
    fn sandwich_capture_8() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".O.O.......",
            "...........",
            "..X........",
            "...........",
            "...........",
        ];

        let board_2 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".OXO.......",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board_1.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker c3 c5")?;
        assert_eq!(game.board, board_2.try_into()?);

        Ok(())
    }

    #[test]
    fn shield_wall_1() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "..O........",
            "...OOO.....",
            "...XXXO....",
        ];

        let board_2 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...OOO.....",
            "..O...O....",
        ];

        let mut game_1 = game::Game {
            board: board_1.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        game_1.read_line("play defender c3 c1")?;
        assert_eq!(game_1.board, board_2.try_into()?);

        let board_3 = [
            "...XXXO....",
            "...OOO.....",
            "..O........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let board_4 = [
            "..O...O....",
            "...OOO.....",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game_2 = game::Game {
            board: board_3.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        game_2.read_line("play defender c9 c11")?;
        assert_eq!(game_2.board, board_4.try_into()?);

        Ok(())
    }

    #[test]
    fn shield_wall_1_13() -> anyhow::Result<()> {
        let board_1 = [
            "...XXXO......",
            "...OOO.......",
            "..O..........",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
        ];

        let board_2 = [
            "..O...O......",
            "...OOO.......",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
        ];

        let mut game_1 = game::Game {
            board: board_1.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        game_1.read_line("play defender C11 C13")?;
        assert_eq!(game_1.board, board_2.try_into()?);

        Ok(())
    }

    #[test]
    fn shield_wall_2() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "..O........",
            "XO.........",
            "XO.........",
            "XO.........",
            "O..........",
            "...........",
            "...........",
        ];

        let board_2 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "O..........",
            ".O.........",
            ".O.........",
            ".O.........",
            "O..........",
            "...........",
            "...........",
        ];

        let mut game_1 = game::Game {
            board: board_1.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        game_1.read_line("play defender c7 a7")?;
        assert_eq!(game_1.board, board_2.try_into()?);

        let board_3 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "........O..",
            ".........OX",
            ".........OX",
            ".........OX",
            "..........O",
            "...........",
            "...........",
        ];

        let board_4 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "..........O",
            ".........O.",
            ".........O.",
            ".........O.",
            "..........O",
            "...........",
            "...........",
        ];

        let mut game_2 = game::Game {
            board: board_3.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        game_2.read_line("play defender i7 k7")?;
        assert_eq!(game_2.board, board_4.try_into()?);

        Ok(())
    }

    #[test]
    fn shield_wall_3() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "........XX.",
            ".....X..OK.",
        ];

        let board_2 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "........XX.",
            ".......X.K.",
        ];

        let mut game_1 = game::Game {
            board: board_1.try_into()?,
            ..Default::default()
        };

        game_1.read_line("play attacker f1 h1")?;
        assert_eq!(game_1.board, board_2.try_into()?);

        let board_3 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".XX........",
            ".KO..X.....",
        ];

        let board_4 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".XX........",
            ".K.X.......",
        ];

        let mut game_2 = game::Game {
            board: board_3.try_into()?,
            ..Default::default()
        };

        game_2.read_line("play attacker f1 d1")?;
        assert_eq!(game_2.board, board_4.try_into()?);

        Ok(())
    }

    #[test]
    fn shield_wall_4() -> anyhow::Result<()> {
        let board_1 = [
            ".....X..OK.",
            "........XX.",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let board_2 = [
            ".......X.K.",
            "........XX.",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game_1 = game::Game {
            board: board_1.try_into()?,
            ..Default::default()
        };

        game_1.read_line("play attacker f11 h11")?;
        assert_eq!(game_1.board, board_2.try_into()?);

        let board_3 = [
            ".KO..X.....",
            ".XX........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let board_4 = [
            ".K.X.......",
            ".XX........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game_2 = game::Game {
            board: board_3.try_into()?,
            ..Default::default()
        };

        game_2.read_line("play attacker f11 d11")?;
        assert_eq!(game_2.board, board_4.try_into()?);

        Ok(())
    }

    #[test]
    fn shield_wall_5() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "X..........",
            "...........",
            "...........",
            "OX.........",
            "KX.........",
            "...........",
        ];

        let board_2 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "X..........",
            ".X.........",
            "KX.........",
            "...........",
        ];

        let mut game_1 = game::Game {
            board: board_1.try_into()?,
            ..Default::default()
        };

        game_1.read_line("play attacker a6 a4")?;
        assert_eq!(game_1.board, board_2.try_into()?);

        let board_3 = [
            "...........",
            "KX.........",
            "OX.........",
            "...........",
            "...........",
            "X..........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let board_4 = [
            "...........",
            "KX.........",
            ".X.........",
            "X..........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game_2 = game::Game {
            board: board_3.try_into()?,
            ..Default::default()
        };

        game_2.read_line("play attacker a6 a8")?;
        assert_eq!(game_2.board, board_4.try_into()?);

        Ok(())
    }

    #[test]
    fn shield_wall_6() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "..........X",
            "...........",
            "...........",
            ".........XO",
            ".........XK",
            "...........",
        ];

        let board_2 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "..........X",
            ".........X.",
            ".........XK",
            "...........",
        ];

        let mut game_1 = game::Game {
            board: board_1.try_into()?,
            ..Default::default()
        };

        game_1.read_line("play attacker k6 k4")?;
        assert_eq!(game_1.board, board_2.try_into()?);

        let board_3 = [
            "...........",
            ".........XK",
            ".........XO",
            "...........",
            "...........",
            "..........X",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let board_4 = [
            "...........",
            ".........XK",
            ".........X.",
            "..........X",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game_2 = game::Game {
            board: board_3.try_into()?,
            ..Default::default()
        };

        game_2.read_line("play attacker k6 k8")?;
        assert_eq!(game_2.board, board_4.try_into()?);

        Ok(())
    }

    #[test]
    fn shield_wall_7() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "........X.O",
            "..........X",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let board_2 = [
            "...........",
            ".........XO",
            "..........X",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board_1.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker i10 j10")?;
        assert_eq!(game.board, board_2.try_into()?);

        let board_3 = [
            "...........",
            "..........X",
            "........X.O",
            "..........X",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let board_4 = [
            "...........",
            "..........X",
            ".........XO",
            "..........X",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board_3.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker i9 j9")?;
        assert_eq!(game.board, board_4.try_into()?);

        Ok(())
    }

    #[test]
    fn shield_wall_nope() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "........X.O",
            ".........XO",
            "..........X",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let board_2 = [
            "...........",
            ".........XO",
            ".........XO",
            "..........X",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board_1.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker i10 j10")?;
        assert_eq!(game.board, board_2.try_into()?);

        Ok(())
    }

    // Five

    #[test]
    fn kings_1() {
        let board = [
            "KK.........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let result: anyhow::Result<Board> = board.try_into();
        assert!(result.is_err());
        assert_error_str(result, "You can only have one king!");
    }

    #[test]
    fn kings_2() -> anyhow::Result<()> {
        let board = [
            ".X.........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            ..Default::default()
        };

        let result = game.read_line("play attacker b11 a11");
        assert!(result.is_err());
        assert_error_str(
            result,
            "play: only the king may move to a restricted square",
        );

        Ok(())
    }

    #[test]
    fn kings_3() -> anyhow::Result<()> {
        let board_1 = [
            "K..........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];
        let _board: Board = board_1.try_into()?;

        let board_2 = [
            "X..........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let result: anyhow::Result<Board> = board_2.try_into();
        assert!(result.is_err());
        assert_error_str(result, "Only the king is allowed on restricted squares!");

        Ok(())
    }

    #[test]
    fn kings_4() -> anyhow::Result<()> {
        let board_1 = [
            "..........K",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];
        let _board: Board = board_1.try_into()?;

        let board_2 = [
            "..........X",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let result: anyhow::Result<Board> = board_2.try_into();
        assert!(result.is_err());
        assert_error_str(result, "Only the king is allowed on restricted squares!");

        Ok(())
    }

    #[test]
    fn kings_5() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".....K.....",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];
        let _board: Board = board_1.try_into()?;

        let board_2 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".....X.....",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let result: anyhow::Result<Board> = board_2.try_into();
        assert!(result.is_err());
        assert_error_str(result, "Only the king is allowed on restricted squares!");

        Ok(())
    }

    #[test]
    fn kings_6() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "K..........",
        ];
        let _board: Board = board_1.try_into()?;

        let board_2 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "X..........",
        ];

        let result: anyhow::Result<Board> = board_2.try_into();
        assert!(result.is_err());
        assert_error_str(result, "Only the king is allowed on restricted squares!");

        Ok(())
    }

    #[test]
    fn kings_7() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "..........K",
        ];
        let _board: Board = board_1.try_into()?;

        let board_2 = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "..........X",
        ];

        let result: anyhow::Result<Board> = board_2.try_into();
        assert!(result.is_err());
        assert_error_str(result, "Only the king is allowed on restricted squares!");

        Ok(())
    }

    // Six

    #[test]
    fn defender_wins_exit() -> anyhow::Result<()> {
        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".....K.....",
        ];

        let mut game_1 = game::Game {
            board: board.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };
        let mut game_2 = game_1.clone();

        game_1.read_line("play defender f1 k1")?;
        assert_eq!(game_1.status, Status::DefenderWins);
        game_2.read_line("play defender f1 a1")?;
        assert_eq!(game_2.status, Status::DefenderWins);

        let board = [
            ".....K.....",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game_1 = game::Game {
            board: board.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };
        let mut game_2 = game_1.clone();

        game_1.read_line("play defender f11 k11")?;
        assert_eq!(game_1.status, Status::DefenderWins);
        game_2.read_line("play defender f11 a11")?;
        assert_eq!(game_2.status, Status::DefenderWins);

        Ok(())
    }

    #[test]
    fn defender_wins_escape_fort_1() -> anyhow::Result<()> {
        let board = [
            "....O.O....",
            "....OKO....",
            "....OO.....",
            "....XX.....",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        game.read_line("play defender f10 f11")?;
        assert_eq!(game.status, Status::DefenderWins);

        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "....XX.....",
            "....OO.....",
            "....OKO....",
            "....O.O....",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };
        game.read_line("play defender f2 f1")?;
        assert_eq!(game.status, Status::DefenderWins);

        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "OOOX.......",
            ".KOX.......",
            "OO.........",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };
        game.read_line("play defender b6 a6")?;
        assert_eq!(game.status, Status::DefenderWins);

        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            ".......XOOO",
            ".......XOK.",
            ".........OO",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        game.read_line("play defender j6 k6")?;
        assert_eq!(game.status, Status::DefenderWins);

        Ok(())
    }

    #[test]
    fn defender_wins_escape_fort_1_13() -> anyhow::Result<()> {
        let board = [
            "....OKO......",
            "....O........",
            "....OOO......",
            "....XX.......",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
            ".............",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        game.read_line("play defender G11 G12")?;
        assert_eq!(game.status, Status::DefenderWins);

        Ok(())
    }

    #[test]
    fn defender_wins_escape_fort_2() -> anyhow::Result<()> {
        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "........XOO",
            "........OK.",
            ".......X.OO",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        game.read_line("play defender j6 k6")?;
        assert_eq!(game.status, Status::Ongoing);

        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "........XOO",
            "........OK.",
            ".........OO",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        game.read_line("play defender j6 k6")?;
        assert_eq!(game.status, Status::DefenderWins);

        Ok(())
    }

    #[test]
    fn defender_wins_escape_fort_3() -> anyhow::Result<()> {
        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "........X.O",
            ".........O.",
            ".......X.OK",
            "..........O",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        game.read_line("play defender k5 k6")?;
        assert_eq!(game.status, Status::DefenderWins);

        Ok(())
    }

    // Seven

    #[test]
    fn kings_not_captured() -> anyhow::Result<()> {
        let board_1 = [
            "...........",
            "...........",
            "...........",
            ".....X.....",
            "...........",
            ".....KX....",
            ".....X.....",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let board_2 = [
            "...........",
            "...........",
            "...........",
            "...........",
            ".....X.....",
            ".....KX....",
            ".....X.....",
            "...........",
            "...........",
            "...........",
            "...........",
        ]
        .try_into()?;

        let mut game = game::Game {
            board: board_1.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker f8 f7")?;
        assert_eq!(game.board, board_2);

        Ok(())
    }

    #[test]
    fn kings_captured_1() -> anyhow::Result<()> {
        let board = [
            "...........",
            "...........",
            "...........",
            ".....X.....",
            "...........",
            "....XKX....",
            ".....X.....",
            "...........",
            "...........",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker f8 f7")?;
        assert_eq!(game.status, Status::AttackerWins);

        Ok(())
    }

    #[test]
    fn kings_captured_2() -> anyhow::Result<()> {
        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "....XKX....",
            "...........",
            ".....X.....",
            "...........",
            "...........",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker f3 f4")?;
        assert_eq!(game.status, Status::AttackerWins);

        Ok(())
    }

    #[test]
    fn kings_captured_3() -> anyhow::Result<()> {
        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "....X......",
            "...XKX.....",
            "...........",
            "....X......",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker e1 e2")?;
        assert_eq!(game.status, Status::AttackerWins);

        Ok(())
    }

    #[test]
    fn kings_captured_4() -> anyhow::Result<()> {
        let board = [
            "...........",
            ".O.........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "X..........",
            "K.X........",
            "X..........",
            "...........",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker c3 b3")?;
        assert_eq!(game.status, Status::Ongoing);

        Ok(())
    }

    #[test]
    fn kings_captured_5() -> anyhow::Result<()> {
        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "X..........",
            "K.X........",
            "...........",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker c2 b2")?;
        assert_eq!(game.status, Status::Ongoing);

        Ok(())
    }

    #[test]
    fn kings_captured_surround_1() -> anyhow::Result<()> {
        let board = [
            "...........",
            "...........",
            ".....XXX...",
            "....X...XX.",
            ".X...O....X",
            "..X.......X",
            ".X.O...O..X",
            ".X..OK...X.",
            "..X...O.X..",
            "...XXX.X...",
            "......X....",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker b7 c7")?;
        assert_eq!(game.status, Status::Ongoing);
        game.read_line("play defender f7 g7")?;
        assert_eq!(game.status, Status::Ongoing);
        game.read_line("play attacker c7 d7")?;
        assert_eq!(game.status, Status::AttackerWins);

        Ok(())
    }

    #[test]
    fn kings_captured_surround_2() -> anyhow::Result<()> {
        let board = [
            "...........",
            "...O.......",
            ".....XXX...",
            "....X...XX.",
            "..X..O....X",
            "..X.......X",
            ".X.O...O..X",
            ".X..OK...X.",
            "..X...O.X..",
            "...XXX.X...",
            "......X....",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker c7 d7")?;
        assert_eq!(game.status, Status::Ongoing);

        Ok(())
    }

    // Eight

    #[test]
    fn can_not_repeat_moves() -> anyhow::Result<()> {
        let mut game = Game::default();

        game.read_line("play attacker f2 f3")?;
        game.read_line("play defender f4 g4")?;
        game.read_line("play attacker f3 f2")?;

        let result = game.read_line("play defender g4 f4");
        assert!(result.is_err());
        assert_error_str(result, "play: you already reached that position");

        Ok(())
    }

    // Nine

    #[test]
    fn attacker_automatically_loses() -> anyhow::Result<()> {
        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".....K.....",
            ".....X.....",
            "...........",
            ".....O.....",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        game.read_line("play defender f1 f2")?;
        assert_eq!(game.status, Status::DefenderWins);

        Ok(())
    }

    #[test]
    fn defender_automatically_loses_1() -> anyhow::Result<()> {
        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".....X.....",
            ".X..XKX....",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker b1 b2")?;
        assert_eq!(game.status, Status::AttackerWins);

        Ok(())
    }

    #[test]
    fn defender_automatically_loses_2() -> anyhow::Result<()> {
        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".....X.....",
            ".X..X.X....",
            "....XKX....",
        ];

        let mut game = game::Game {
            board: board.try_into()?,
            ..Default::default()
        };

        game.read_line("play attacker b2 b1")?;
        game.read_line("play defender f1 f2")?;
        game.read_line("play attacker b1 b2")?;
        game.read_line("play defender f2 f1")?;
        game.read_line("play attacker b2 b1")?;

        assert_eq!(game.status, Status::AttackerWins);

        Ok(())
    }

    #[test]
    fn exit_one_1() -> anyhow::Result<()> {
        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".X...K...X.",
        ];

        let game = game::Game {
            board: board.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        assert!(!game.exit_one());

        Ok(())
    }

    #[test]
    fn exit_one_2() -> anyhow::Result<()> {
        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".....K.....",
        ];

        let game = game::Game {
            board: board.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        assert!(game.exit_one());

        Ok(())
    }

    #[test]
    fn exit_one_3() -> anyhow::Result<()> {
        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".....K.....",
            "...........",
        ];

        let game = game::Game {
            board: board.try_into()?,
            turn: Role::Defender,
            ..Default::default()
        };

        assert!(!game.exit_one());

        Ok(())
    }

    #[test]
    fn exit_one_4() -> anyhow::Result<()> {
        let board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            ".....K.....",
        ];

        let game = game::Game {
            board: board.try_into()?,
            ..Default::default()
        };

        assert!(!game.exit_one());

        Ok(())
    }

    #[test]
    fn closed_off_exits() -> anyhow::Result<()> {
        let board: Board = [
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "X..........",
            ".X.........",
            "..X..K.....",
        ]
        .try_into()?;

        assert_eq!(board.closed_off_exits(), 1);

        let board: Board = [
            "..X....X...",
            ".X......X..",
            "X........X.",
            "X.........X",
            "X..........",
            "X..........",
            "X..........",
            "X..........",
            "X.........X",
            ".X.......X.",
            "..X..K..X..",
        ]
        .try_into()?;

        assert_eq!(board.closed_off_exits(), 4);

        let board: Board = [
            ".XX.....XX.",
            "X.........X",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "X.........X",
            ".XX..K..XX.",
        ]
        .try_into()?;

        assert_eq!(board.closed_off_exits(), 0);

        let board: Board = [
            ".X.......X.",
            "X.........X",
            "X.........X",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "X.........X",
            "X.........X",
            ".X...K...X.",
        ]
        .try_into()?;

        assert_eq!(board.closed_off_exits(), 0);

        let board: Board = [
            ".XX.....XX.",
            "X.........X",
            "X.........X",
            "...........",
            "...........",
            "...........",
            "...........",
            "...........",
            "X.........X",
            "X.........X",
            ".XX..K..XX.",
        ]
        .try_into()?;

        assert_eq!(board.closed_off_exits(), 4);

        Ok(())
    }
    #[test]
    fn someone_wins() {
        let mut game = Game::default();
        let mut ai: Box<dyn AI> = Box::new(AiBanal);

        loop {
            // Do nothing but play.
            if ai.generate_move(&mut game).is_err() {
                break;
            }
        }

        assert!(game.status == Status::AttackerWins || game.status == Status::DefenderWins);
    }
}
