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
//
// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 David Campbell <david@hnefatafl.org>

use std::{
    collections::{HashMap, VecDeque},
    fs::File,
    io::Read,
    str::FromStr,
};

use hnefatafl_copenhagen::{
    board::BoardSize,
    game::Game,
    glicko::Rating,
    play::{Plae, Play, Vertex},
    rating::Rated,
    role::Role,
    server_game::{ArchivedGame, ServerGame, ServerGameSerialized},
    time::TimeSettings,
};
use rfd::FileDialog;

pub fn read_portable_game_notation() -> anyhow::Result<ArchivedGame> {
    let dirs =
        directories::UserDirs::new().ok_or(anyhow::Error::msg("failed to get user directories"))?;

    let dir = dirs
        .document_dir()
        .ok_or(anyhow::Error::msg("failed to get document directory"))?;

    let file = FileDialog::new()
        .add_filter("*", &["pgn"])
        .set_directory(dir)
        .pick_file()
        .ok_or(anyhow::Error::msg("failed to pick a file"))?;

    let file = File::open(file)?;
    archived_game_from_pgn(file)
}

fn archived_game_from_pgn(mut file: File) -> anyhow::Result<ArchivedGame> {
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;

    let mut metadata = HashMap::new();
    let mut parsing_moves = false;
    let mut plays = Vec::new();

    // [Event "Zevratafl game"]
    // [Site "playtafl.com"]
    // [Date "2026.04.25"]
    //
    // 1. h1h3 g5g2 2. h3h2
    for line in buf.lines() {
        if parsing_moves {
            for word in line.split_whitespace() {
                if !word.starts_with(['1', '2', '3', '4', '5', '6', '7', '8', '9']) {
                    plays.push(word);
                }
            }

            break;
        }

        if line.is_empty() {
            parsing_moves = true;
        }

        let mut key = String::new();
        let mut value = String::new();
        let mut parsing_key = true;

        for ch in line.chars().skip(1) {
            if parsing_key {
                if ch.is_whitespace() {
                    parsing_key = false;
                } else {
                    key.push(ch);
                }
            } else {
                if ch == '"' || ch == ']' {
                    continue;
                }

                value.push(ch);
            }
        }

        if !key.is_empty() {
            metadata.insert(key, value);
        }
    }

    let mut game = Game::new_game(BoardSize::_11, Some(TimeSettings::UnTimed));
    let mut role = Role::Attacker;

    for play in plays {
        let (from, to) = play.split_at(2);

        game.play(&Plae::Play(Play {
            role,
            from: Vertex::from_str(from)?,
            to: Vertex::from_str(to)?,
        }))?;

        role = role.opposite();
    }

    let game = ServerGameSerialized {
        id: 0,
        attacker: "attacker".to_string(),
        defender: "defender".to_string(),
        rated: Rated::No,
        game,
        texts: VecDeque::new(),
        timed: TimeSettings::UnTimed,
    };

    let mut game = ServerGame::from(game);

    game.game.chars.ascii();
    game.game.board.display_ascii = true;

    Ok(ArchivedGame::new(
        game,
        Rating::default(),
        Rating::default(),
    ))
}
