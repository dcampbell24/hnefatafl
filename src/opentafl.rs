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

// Todo: make the main loop read from a channel on put the TCP loop on a channel...

use itertools::Itertools;
use jiff::Timestamp;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, fmt::Write, str::FromStr};

use crate::{
    board::BoardSize,
    game::Game,
    play::{Captures, Plae, Play, Plays, Vertex},
    role::Role,
    server_game::{Message, ServerGame},
    time::{Time, TimeSettings, TimeUnix},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TimeControl {
    main_time_seconds: i64,
    increment_length: i64,
}

const fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    #[default]
    Copenhagen,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TimeRemaining {
    pub attackers: i64,
    pub defenders: i64,
    pub last_move: Timestamp,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OpenTaflGame {
    pub attackers: String,
    pub defenders: String,
    pub dim: usize,
    pub start: String,
    pub time_control: Option<TimeControl>,
    pub time_remaining_ms: Option<TimeRemaining>,
    // #[serde(default)]
    // pub variant: Variant,
    // #[serde(default = "default_true")]
    // pub sw: bool,
    // #[serde(default = "default_true")]
    // pub efe: bool,
    pub moves: String,
    //
    #[serde(default = "default_true")]
    pub rated: bool,
    pub messages: VecDeque<Message>,
}

impl From<&ServerGame> for OpenTaflGame {
    fn from(server_game: &ServerGame) -> Self {
        let dim = usize::from(server_game.game.board.size());

        let moves: Vec<Plae> = match &server_game.game.plays {
            Plays::PlayRecordsTimed(plays) => plays
                .iter()
                .filter_map(|play_record| play_record.play.clone())
                .collect(),
            Plays::PlayRecords(plays) => plays.iter().flatten().cloned().collect(),
        };

        let mut game_play = Game::make(server_game.game.board.size(), &TimeSettings::UnTimed);
        let start = game_play.board.open_tafl_serialize();

        let moves = moves
            .iter()
            .map(|play| {
                let captures = game_play.play(play).expect("This must be a valid move!");
                let mut play_string = match play {
                    Plae::Play(play) => {
                        format!("{}-{}", play.from, play.to)
                    }

                    Plae::AttackerResigns | Plae::DefenderResigns => "---".to_string(),
                };

                for capture in captures.0 {
                    let _ = write!(play_string, "x{capture}");
                }

                play_string
            })
            .join(" ");

        let time_control = if let Plays::PlayRecordsTimed(plays) = &server_game.game.plays
            && let Some(play) = plays.first()
            && let TimeSettings::Timed(time_settings) = server_game.game.defender_time
        {
            Some(TimeControl {
                main_time_seconds: play.defender_time.milliseconds_left / 1_000,
                increment_length: time_settings.add_seconds,
            })
        } else {
            None
        };

        let time_remaining_ms = if let (
            TimeSettings::Timed(attackers),
            TimeSettings::Timed(defenders),
            TimeUnix::Time(last_move),
        ) = (
            &server_game.game.attacker_time,
            &server_game.game.defender_time,
            server_game.game.time,
        ) {
            Some(TimeRemaining {
                attackers: attackers.milliseconds_left,
                defenders: defenders.milliseconds_left,
                last_move: Timestamp::from_millisecond(last_move).expect("This coversion works!"),
            })
        } else {
            None
        };

        Self {
            attackers: server_game.attacker.clone(),
            defenders: server_game.defender.clone(),
            dim,
            start,
            time_control,
            time_remaining_ms,
            moves,
            rated: server_game.rated.into(),
            messages: server_game.messages.clone(),
        }
    }
}

impl From<&OpenTaflGame> for Game {
    fn from(game_opentafl: &OpenTaflGame) -> Self {
        let (attacker_time, defender_time, last_move) = if let (
            Some(time_remaining_ms),
            Some(TimeControl {
                main_time_seconds: _,
                increment_length,
            }),
        ) = (
            game_opentafl.time_remaining_ms.clone(),
            game_opentafl.time_control.clone(),
        ) {
            (
                TimeSettings::Timed(Time {
                    milliseconds_left: time_remaining_ms.attackers,
                    add_seconds: increment_length,
                }),
                TimeSettings::Timed(Time {
                    milliseconds_left: time_remaining_ms.defenders,
                    add_seconds: increment_length,
                }),
                TimeUnix::Time(time_remaining_ms.last_move.as_millisecond()),
            )
        } else {
            (
                TimeSettings::UnTimed,
                TimeSettings::UnTimed,
                TimeUnix::UnTimed,
            )
        };

        let mut game = Game::make(
            BoardSize::try_from(game_opentafl.dim).expect("The board size must be 11 or 13!"),
            &attacker_time,
        );

        let mut plays = Vec::with_capacity(game_opentafl.moves.len());
        let mut role = Role::Attacker;

        for play in game_opentafl.moves.split_whitespace() {
            let mut play = play.to_string();
            let role_str = role.to_string();

            let play = if play == "resigns" {
                match role {
                    Role::Attacker => vec!["play", "attacker", "resigns"],
                    Role::Defender => vec!["play", "defender", "resigns"],
                    Role::Roleless => unreachable!(),
                }
            } else {
                if play.contains('x')
                    && let Some(play_capture) = play.split('x').next()
                {
                    play = play_capture.to_string();
                }

                let play_vec: Vec<_> = play.splitn(2, '-').collect();
                vec!["play", &role_str, play_vec[0], play_vec[1]]
            };

            plays.push(Plae::try_from(play).expect("This must work!"));

            role = role.opposite();
        }

        for play in plays {
            game.play(&play)
                .expect("The play was valid when it was first played.");
        }

        game.time = last_move;
        game.attacker_time = attacker_time;
        game.defender_time = defender_time;

        game
    }
}

#[derive(Clone, Debug)]
pub struct OpenTaflMoves(pub Vec<(Plae, Captures)>);

impl FromStr for OpenTaflMoves {
    type Err = anyhow::Error;

    fn from_str(moves: &str) -> Result<Self, Self::Err> {
        let mut plays = Vec::new();
        let mut role = Role::Attacker;

        for play in moves.split_ascii_whitespace() {
            if let Some((vertex, vertex_captures)) = play.split_once('-') {
                let vertex_captures: Vec<_> = vertex_captures.split('x').collect();

                if let (Ok(from), Ok(to)) = (
                    Vertex::from_str(vertex),
                    Vertex::from_str(vertex_captures[0]),
                ) {
                    let play = Play { role, from, to };
                    let mut captures = FxHashSet::default();

                    if vertex_captures.get(1).is_some() {
                        for capture in vertex_captures.into_iter().skip(1) {
                            let vertex = Vertex::from_str(capture)?;
                            if !captures.contains(&vertex) {
                                captures.insert(vertex);
                            }
                        }

                        plays.push((Plae::Play(play), Captures(captures)));
                    } else {
                        plays.push((Plae::Play(play), Captures(captures)));
                    }
                }
            }

            role = role.opposite();
        }

        Ok(OpenTaflMoves(plays))
    }
}
