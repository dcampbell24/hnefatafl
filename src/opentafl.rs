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

use itertools::Itertools;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, fmt::Write};

use crate::{
    board::BoardSize,
    game::{Game, TimeUnix},
    play::{Plae, Plays},
    role::Role,
    server_game::{Message, ServerGame},
    time::{Time, TimeSettings},
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
pub struct OpenTaflGame {
    pub attackers: String,
    pub defenders: String,
    pub dim: usize,
    pub start: String,
    pub time_control: Option<TimeControl>,
    pub time_remaining_ms: Option<(i64, i64)>,
    // #[serde(default)]
    // pub variant: Variant,
    // #[serde(default = "default_true")]
    // pub sw: bool,
    // #[serde(default = "default_true")]
    // pub efe: bool,
    pub moves: String,
    //
    pub last_move: Option<Timestamp>,
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

                for capture in captures.captures {
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

        let time_remaining_ms =
            if let (TimeSettings::Timed(attacker_time), TimeSettings::Timed(defender_time)) = (
                &server_game.game.attacker_time,
                &server_game.game.defender_time,
            ) {
                Some((
                    attacker_time.milliseconds_left,
                    defender_time.milliseconds_left,
                ))
            } else {
                None
            };

        let last_move = if let TimeUnix::Time(time) = server_game.game.time {
            Some(Timestamp::from_millisecond(time).expect("This coversion works!"))
        } else {
            None
        };

        Self {
            attackers: server_game.attacker.clone(),
            defenders: server_game.defender.clone(),
            rated: server_game.rated.into(),
            messages: server_game.messages.clone(),
            dim,
            start,
            last_move,
            moves,
            time_control,
            time_remaining_ms,
        }
    }
}

impl From<OpenTaflGame> for Game {
    fn from(game_opentafl: OpenTaflGame) -> Self {
        let (attacker_time, defender_time) = if let (
            Some((attacker_time, defender_time)),
            Some(TimeControl {
                main_time_seconds: _,
                increment_length,
            }),
        ) =
            (game_opentafl.time_remaining_ms, game_opentafl.time_control)
        {
            (
                TimeSettings::Timed(Time {
                    milliseconds_left: attacker_time,
                    add_seconds: increment_length,
                }),
                TimeSettings::Timed(Time {
                    milliseconds_left: defender_time,
                    add_seconds: increment_length,
                }),
            )
        } else {
            (TimeSettings::UnTimed, TimeSettings::UnTimed)
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

        let time = if let Some(timestamp) = game_opentafl.last_move {
            TimeUnix::Time(timestamp.as_millisecond())
        } else {
            TimeUnix::UnTimed
        };

        game.time = time;
        game.attacker_time = attacker_time;
        game.defender_time = defender_time;

        game
    }
}
