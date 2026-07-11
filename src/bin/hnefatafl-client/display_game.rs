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

use std::collections::VecDeque;

use hnefatafl_copenhagen::{board::Board, server_game::Message, status::Status};

#[derive(Clone, Debug)]
pub(crate) struct DisplayGame {
    pub game_id: u128,
    pub attacker: String,
    pub attacker_time: String,
    pub attacker_rating: String,
    pub defender: String,
    pub defender_time: String,
    pub defender_rating: String,
    pub board: Board,
    pub play: usize,
    pub status: Status,
    pub messages: VecDeque<Message>,
}
