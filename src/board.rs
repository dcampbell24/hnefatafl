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
// SPDX-FileCopyrightText: 2025 David Campbell <david@hnefatafl.org>

use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Display},
    hash::{Hash, Hasher},
    num::ParseIntError,
    str::FromStr,
};

use colored::Colorize;
use rustc_hash::FxHashSet;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, Visitor},
};
use thiserror::Error;

use crate::{
    game::PreviousBoards,
    play::{EXIT_SQUARES_11X11, EXIT_SQUARES_13X13, Plae, Play, Vertex},
    role::Role,
    space::Space,
    status::Status,
};

pub const BOARD_LETTERS: &str = " A B C D E F G H I J K L M ";

pub const STARTING_POSITION_11X11: [&str; 11] = [
    "...XXXXX...",
    ".....X.....",
    "...........",
    "X....O....X",
    "X...OOO...X",
    "XX.OOKOO.XX",
    "X...OOO...X",
    "X....O....X",
    "...........",
    ".....X.....",
    "...XXXXX...",
];

pub const STARTING_POSITION_13X13: [&str; 13] = [
    "...XXXXXXX...",
    "......X......",
    ".............",
    "X.....O.....X",
    "X.....O.....X",
    "X....OOO....X",
    "XX.OOOKOOO.XX",
    "X....OOO....X",
    "X.....O.....X",
    "X.....O.....X",
    ".............",
    "......X......",
    "...XXXXXXX...",
];

#[derive(Clone, Deserialize, Eq, Serialize)]
pub struct Board {
    pub spaces: Vec<Space>,
    #[serde(skip)]
    pub king: Option<Vertex>,
    #[serde(skip)]
    pub attackers_captured: usize,
    #[serde(skip)]
    pub defenders_captured: usize,
    #[serde(skip)]
    pub display_ascii: bool,
}

impl PartialEq for Board {
    fn eq(&self, other: &Self) -> bool {
        self.spaces == other.spaces
    }
}

impl Hash for Board {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.spaces.hash(state);
    }
}

impl Default for Board {
    fn default() -> Self {
        board_11x11()
    }
}

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let board_size: usize = self.size().into();

        writeln!(f)?;
        for y in 0..board_size {
            write!(f, r#"""#)?;

            for x in 0..board_size {
                match self.spaces[(y * board_size) + x] {
                    Space::Attacker => write!(f, "X")?,
                    Space::Empty => write!(f, ".")?,
                    Space::King => write!(f, "K")?,
                    Space::Defender => write!(f, "O")?,
                }
            }
            writeln!(f, r#"""#)?;
        }

        Ok(())
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let board_size: usize = self.size().into();
        let mut letters = " ".repeat(3).clone();
        letters.push_str(&BOARD_LETTERS[..board_size * 2]);
        let bar = "─".repeat(board_size * 2 + 1);

        writeln!(f, "\n{letters}\n  ┌{bar}┐")?;
        for y in 0..board_size {
            let y_label = board_size - y;
            write!(f, "{y_label:2}│ ")?;

            for x in 0..board_size {
                if (((y, x) == (0, 0)
                    || (y, x) == (10, 0)
                    || (y, x) == (0, 10)
                    || (y, x) == (10, 10)
                    || (y, x) == (5, 5))
                    && self.spaces[y * board_size + x] == Space::Empty
                    && board_size == 11)
                    || (((y, x) == (0, 0)
                        || (y, x) == (12, 0)
                        || (y, x) == (0, 12)
                        || (y, x) == (12, 12)
                        || (y, x) == (6, 6))
                        && self.spaces[y * board_size + x] == Space::Empty
                        && board_size == 13)
                {
                    if self.display_ascii {
                        write!(f, "{} ", "#".green())?;
                    } else {
                        write!(f, "{} ", "⌘".green())?;
                    }
                } else if self.display_ascii {
                    write!(f, "{} ", self.spaces[y * board_size + x].display_ascii())?;
                } else {
                    write!(f, "{} ", self.spaces[y * board_size + x])?;
                }
            }
            writeln!(f, "│{y_label:2}")?;
        }
        write!(f, "  └{bar}┘\n{letters}")
    }
}

impl TryFrom<[&str; 11]> for Board {
    type Error = anyhow::Error;

    fn try_from(value: [&str; 11]) -> anyhow::Result<Self> {
        let mut spaces = Vec::with_capacity(11 * 11);
        let mut kings = 0;
        let mut king = None;

        for (y, row) in value.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                let space = ch.try_into()?;

                match space {
                    Space::Attacker | Space::Defender => {
                        let vertex = Vertex {
                            size: BoardSize::_11,
                            x,
                            y,
                        };
                        if vertex.on_restricted_square() {
                            return Err(anyhow::Error::msg(
                                "Only the king is allowed on restricted squares!",
                            ));
                        }
                    }
                    Space::Empty => {}
                    Space::King => {
                        kings += 1;
                        king = Some(Vertex {
                            size: BoardSize::_11,
                            x,
                            y,
                        });

                        if kings > 1 {
                            return Err(anyhow::Error::msg("You can only have one king!"));
                        }
                    }
                }

                spaces.push(space);
            }
        }

        let mut board = Self {
            spaces,
            attackers_captured: 0,
            defenders_captured: 0,
            king,
            display_ascii: false,
        };

        let captured = board.captured();
        board.attackers_captured = captured.attacker;
        board.defenders_captured = captured.defender;

        Ok(board)
    }
}

impl TryFrom<[&str; 13]> for Board {
    type Error = anyhow::Error;

    fn try_from(value: [&str; 13]) -> anyhow::Result<Self> {
        let mut spaces = Vec::with_capacity(13 * 13);
        let mut kings = 0;
        let mut king = None;

        for (y, row) in value.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                let space = ch.try_into()?;

                match space {
                    Space::Attacker | Space::Defender => {
                        let vertex = Vertex {
                            size: BoardSize::_13,
                            x,
                            y,
                        };

                        if vertex.on_restricted_square() {
                            return Err(anyhow::Error::msg(
                                "Only the king is allowed on restricted squares!",
                            ));
                        }
                    }
                    Space::Empty => {}
                    Space::King => {
                        kings += 1;
                        king = Some(Vertex {
                            size: BoardSize::_13,
                            x,
                            y,
                        });

                        if kings > 1 {
                            return Err(anyhow::Error::msg("You can only have one king!"));
                        }
                    }
                }

                spaces.push(space);
            }
        }

        let mut board = Self {
            spaces,
            attackers_captured: 0,
            defenders_captured: 0,
            king,
            display_ascii: false,
        };

        let captured = board.captured();
        board.attackers_captured = captured.attacker;
        board.defenders_captured = captured.defender;

        Ok(board)
    }
}

impl Board {
    #[must_use]
    pub fn new(board_size: BoardSize) -> Self {
        match board_size {
            BoardSize::_11 => board_11x11(),
            BoardSize::_13 => board_13x13(),
        }
    }

    fn able_to_move(&self, play_from: &Vertex) -> bool {
        if let Some(vertex) = play_from.up()
            && self.get(&vertex) == Space::Empty
        {
            return true;
        }

        if let Some(vertex) = play_from.left()
            && self.get(&vertex) == Space::Empty
        {
            return true;
        }

        if let Some(vertex) = play_from.down()
            && self.get(&vertex) == Space::Empty
        {
            return true;
        }

        if let Some(vertex) = play_from.right()
            && self.get(&vertex) == Space::Empty
        {
            return true;
        }

        false
    }

    #[must_use]
    pub fn a_legal_move_exists(
        &self,
        status: &Status,
        turn: &Role,
        previous_boards: &PreviousBoards,
    ) -> bool {
        let size = self.size();
        let board_size_usize: usize = size.into();

        for y in 0..board_size_usize {
            for x in 0..board_size_usize {
                let vertex_from = Vertex { size, x, y };
                if Role::from(self.get(&vertex_from)) != *turn {
                    continue;
                }

                for y in 0..board_size_usize {
                    for x in 0..board_size_usize {
                        let vertex_to = Vertex { size, x, y };

                        if vertex_to.x != vertex_from.x && vertex_to.y != vertex_from.y {
                            continue;
                        }

                        let play = Play {
                            role: *turn,
                            from: vertex_from,
                            to: vertex_to,
                        };

                        if self
                            .legal_move(&play, status, turn, previous_boards)
                            .is_ok()
                        {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    #[must_use]
    pub fn captured(&self) -> Captured {
        let mut attacker = 0;
        let mut defender = 0;
        let mut king = true;

        for space in &self.spaces {
            match space {
                Space::Attacker => attacker += 1,
                Space::Empty => {}
                Space::King => king = false,
                Space::Defender => defender += 1,
            }
        }

        match self.size() {
            BoardSize::_11 => {
                attacker = 24 - attacker;
                defender = 12 - defender;
            }
            BoardSize::_13 => {
                attacker = 32 - attacker;
                defender = 16 - defender;
            }
        }

        Captured {
            attacker,
            defender,
            king,
        }
    }

    fn captures(&mut self, play_to: &Vertex, role_from: Role, captures: &mut Vec<Vertex>) {
        self.captures_(play_to, role_from, captures, super::play::Vertex::up);
        self.captures_(play_to, role_from, captures, super::play::Vertex::left);
        self.captures_(play_to, role_from, captures, super::play::Vertex::down);
        self.captures_(play_to, role_from, captures, super::play::Vertex::right);
    }

    fn captures_<T: Fn(&Vertex) -> Option<Vertex>>(
        &mut self,
        play_to: &Vertex,
        role_from: Role,
        captures: &mut Vec<Vertex>,
        over: T,
    ) {
        if let Some(right_1) = over(play_to) {
            let space = self.get(&right_1);
            if space != Space::King
                && Role::from(space) == role_from.opposite()
                && let Some(right_2) = over(&right_1)
                && ((right_2.on_restricted_square() && self.get(&right_2) != Space::King)
                    || Role::from(self.get(&right_2)) == role_from)
                && self.set_if_not_king(&right_1, Space::Empty)
            {
                captures.push(right_1);
            }
        }
    }

    // y counts up going down.
    #[allow(clippy::too_many_lines)]
    fn captures_shield_wall(
        &mut self,
        role_from: Role,
        vertex_to: &Vertex,
        captures: &mut Vec<Vertex>,
    ) {
        let size = self.size();
        let board_size_usize: usize = size.into();

        // bottom row
        for x_1 in 0..board_size_usize {
            let vertex_1 = Vertex {
                size,
                x: x_1,
                y: board_size_usize - 1,
            };
            if Role::from(self.get(&vertex_1)) == role_from || vertex_1.on_restricted_square() {
                let mut count = 0;

                if x_1 == board_size_usize - 1 {
                    break;
                }
                let start = x_1 + 1;

                for x_2 in start..board_size_usize {
                    let vertex_2 = Vertex {
                        size,
                        x: x_2,
                        y: board_size_usize - 1,
                    };
                    let vertex_3 = Vertex {
                        size,
                        x: x_2,
                        y: board_size_usize - 2,
                    };
                    let role_2 = Role::from(self.get(&vertex_2));
                    let role_3 = Role::from(self.get(&vertex_3));
                    if role_2 == role_from.opposite() && role_3 == role_from {
                        count += 1;
                    } else {
                        break;
                    }
                }

                let finish = start + count;
                let vertex = Vertex {
                    size,
                    x: finish,
                    y: board_size_usize - 1,
                };
                let role = Role::from(self.get(&vertex));
                if count > 1
                    && (role == role_from || vertex.on_restricted_square())
                    && (vertex_to
                        == &(Vertex {
                            size,
                            x: start - 1,
                            y: board_size_usize - 1,
                        })
                        || vertex_to
                            == &(Vertex {
                                size,
                                x: finish,
                                y: board_size_usize - 1,
                            }))
                {
                    for x_2 in start..finish {
                        let vertex = Vertex {
                            size,
                            x: x_2,
                            y: board_size_usize - 1,
                        };
                        if self.set_if_not_king(&vertex, Space::Empty) {
                            captures.push(vertex);
                        }
                    }
                }
            }
        }

        // top row
        for x_1 in 0..board_size_usize {
            let vertex_1 = Vertex { size, x: x_1, y: 0 };
            if Role::from(self.get(&vertex_1)) == role_from || vertex_1.on_restricted_square() {
                let mut count = 0;

                if x_1 == board_size_usize - 1 {
                    break;
                }
                let start = x_1 + 1;

                for x_2 in start..board_size_usize {
                    let vertex_2 = Vertex { size, x: x_2, y: 0 };
                    let vertex_3 = Vertex { size, x: x_2, y: 1 };
                    let role_2 = Role::from(self.get(&vertex_2));
                    let role_3 = Role::from(self.get(&vertex_3));
                    if role_2 == role_from.opposite() && role_3 == role_from {
                        count += 1;
                    } else {
                        break;
                    }
                }

                let finish = start + count;
                let vertex = Vertex {
                    size,
                    x: finish,
                    y: 0,
                };
                let role = Role::from(self.get(&vertex));
                if count > 1
                    && (role == role_from || vertex.on_restricted_square())
                    && (vertex_to
                        == &(Vertex {
                            size,
                            x: start - 1,
                            y: 0,
                        })
                        || vertex_to
                            == &(Vertex {
                                size,
                                x: finish,
                                y: 0,
                            }))
                {
                    for x_2 in start..finish {
                        let vertex = Vertex { size, x: x_2, y: 0 };
                        if self.set_if_not_king(&vertex, Space::Empty) {
                            captures.push(vertex);
                        }
                    }
                }
            }
        }

        // left row
        for y_1 in 0..board_size_usize {
            let vertex_1 = Vertex { size, x: 0, y: y_1 };
            if Role::from(self.get(&vertex_1)) == role_from || vertex_1.on_restricted_square() {
                let mut count = 0;

                if y_1 == board_size_usize - 1 {
                    break;
                }
                let start = y_1 + 1;

                for y_2 in start..board_size_usize {
                    let vertex_2 = Vertex { size, x: 0, y: y_2 };
                    let vertex_3 = Vertex { size, x: 1, y: y_2 };
                    let role_2 = Role::from(self.get(&vertex_2));
                    let role_3 = Role::from(self.get(&vertex_3));
                    if role_2 == role_from.opposite() && role_3 == role_from {
                        count += 1;
                    } else {
                        break;
                    }
                }

                let finish = start + count;
                let vertex = Vertex {
                    size,
                    x: 0,
                    y: finish,
                };
                let role = Role::from(self.get(&vertex));
                if count > 1
                    && (role == role_from || vertex.on_restricted_square())
                    && (vertex_to
                        == &(Vertex {
                            size,
                            x: 0,
                            y: start - 1,
                        })
                        || vertex_to
                            == &(Vertex {
                                size,
                                x: 0,
                                y: finish,
                            }))
                {
                    for y_2 in start..finish {
                        let vertex = Vertex { size, x: 0, y: y_2 };
                        if self.set_if_not_king(&vertex, Space::Empty) {
                            captures.push(vertex);
                        }
                    }
                }
            }
        }

        // right row
        for y_1 in 0..board_size_usize {
            let vertex_1 = Vertex {
                size,
                x: board_size_usize - 1,
                y: y_1,
            };
            if Role::from(self.get(&vertex_1)) == role_from || vertex_1.on_restricted_square() {
                let mut count = 0;

                if y_1 == board_size_usize - 1 {
                    break;
                }
                let start = y_1 + 1;

                for y_2 in start..board_size_usize {
                    let vertex_2 = Vertex {
                        size,
                        x: board_size_usize - 1,
                        y: y_2,
                    };
                    let vertex_3 = Vertex {
                        size,
                        x: board_size_usize - 2,
                        y: y_2,
                    };
                    let role_2 = Role::from(self.get(&vertex_2));
                    let role_3 = Role::from(self.get(&vertex_3));
                    if role_2 == role_from.opposite() && role_3 == role_from {
                        count += 1;
                    } else {
                        break;
                    }
                }

                let finish = start + count;
                let vertex = Vertex {
                    size,
                    x: board_size_usize - 1,
                    y: finish,
                };
                let role = Role::from(self.get(&vertex));
                if count > 1
                    && (role == role_from || vertex.on_restricted_square())
                    && (vertex_to
                        == &(Vertex {
                            size,
                            x: board_size_usize - 1,
                            y: start - 1,
                        })
                        || vertex_to
                            == &(Vertex {
                                size,
                                x: board_size_usize - 1,
                                y: finish,
                            }))
                {
                    for y_2 in start..finish {
                        let vertex = Vertex {
                            size,
                            x: board_size_usize - 1,
                            y: y_2,
                        };
                        if self.set_if_not_king(&vertex, Space::Empty) {
                            captures.push(vertex);
                        }
                    }
                }
            }
        }
    }

    // Fixme: slow!
    #[allow(clippy::unwrap_used)]
    #[must_use]
    fn closed_off_exit(&self, exit: Vertex) -> Option<Vec<Vertex>> {
        let size = self.size();
        let board_size_usize: usize = size.into();
        let mut already_checked = vec![0; board_size_usize * board_size_usize];
        already_checked[usize::from(&exit)] += 1;

        let mut pre_stack = Vec::with_capacity(board_size_usize * board_size_usize);
        let up = expand_flood_fill(exit.up(), &mut already_checked, &mut pre_stack);
        let left = expand_flood_fill(exit.left(), &mut already_checked, &mut pre_stack);
        let down = expand_flood_fill(exit.down(), &mut already_checked, &mut pre_stack);
        let right = expand_flood_fill(exit.right(), &mut already_checked, &mut pre_stack);

        if up && left {
            let up_v1 = &pre_stack[0];
            let up_v2 = &pre_stack[0].up().unwrap();
            let left_v1 = &pre_stack[1];
            let left_v2 = &pre_stack[1].left().unwrap();

            if let Some(defended) = self.closed_off_exit_2(up_v1, up_v2, left_v1, left_v2, &exit) {
                return Some(defended);
            }
        }

        if up && right {
            let up_v1 = &pre_stack[0];
            let up_v2 = &pre_stack[0].up().unwrap();
            let right_v1 = &pre_stack[1];
            let right_v2 = &pre_stack[1].right().unwrap();

            if let Some(defended) = self.closed_off_exit_2(up_v1, up_v2, right_v1, right_v2, &exit)
            {
                return Some(defended);
            }
        }

        if left && down {
            let left_v1 = &pre_stack[0];
            let left_v2 = &pre_stack[0].left().unwrap();
            let down_v1 = &pre_stack[1];
            let down_v2 = &pre_stack[1].down().unwrap();

            if let Some(defended) =
                self.closed_off_exit_2(left_v1, left_v2, down_v1, down_v2, &exit)
            {
                return Some(defended);
            }
        }

        if down && right {
            let down_v1 = &pre_stack[0];
            let down_v2 = &pre_stack[0].down().unwrap();
            let right_v1 = &pre_stack[1];
            let right_v2 = &pre_stack[1].right().unwrap();

            if let Some(defended) =
                self.closed_off_exit_2(down_v1, down_v2, right_v1, right_v2, &exit)
            {
                return Some(defended);
            }
        }

        let mut defended = Vec::with_capacity(32);
        let mut stack = Vec::with_capacity((board_size_usize * board_size_usize) / 2);

        for vertex in pre_stack {
            let space = self.get(&vertex);
            if space == Space::Empty || space == Space::Attacker {
                if vertex.touches_wall() {
                    defended.push(vertex);
                }

                let _ = expand_flood_fill(vertex.up(), &mut already_checked, &mut stack);
                let _ = expand_flood_fill(vertex.left(), &mut already_checked, &mut stack);
                let _ = expand_flood_fill(vertex.down(), &mut already_checked, &mut stack);
                let _ = expand_flood_fill(vertex.right(), &mut already_checked, &mut stack);
            }
        }

        while !stack.is_empty() {
            if let Some(vertex) = stack.pop() {
                let space = self.get(&vertex);

                if space == Space::Empty {
                    if vertex.touches_wall() {
                        defended.push(vertex);
                    }

                    let _ = expand_flood_fill(vertex.right(), &mut already_checked, &mut stack);
                    let _ = expand_flood_fill(vertex.left(), &mut already_checked, &mut stack);

                    let _ = expand_flood_fill(vertex.down(), &mut already_checked, &mut stack);
                    let _ = expand_flood_fill(vertex.up(), &mut already_checked, &mut stack);
                } else if Into::<Role>::into(space) == Role::Defender {
                    return None;
                }
            }
        }

        // _print_u32(&already_checked);

        defended.push(exit);

        Some(defended)
    }

    #[must_use]
    #[allow(clippy::similar_names)]
    fn closed_off_exit_2(
        &self,
        d1_v1: &Vertex,
        d1_v2: &Vertex,
        d2_v1: &Vertex,
        d2_v2: &Vertex,
        exit: &Vertex,
    ) -> Option<Vec<Vertex>> {
        let mut defended = Vec::new();

        let d1_s1 = self.get(d1_v1);
        let d1_s2 = self.get(d1_v2);
        let d2_s1 = self.get(d2_v1);
        let d2_s2 = self.get(d2_v2);

        if d1_s1 == Space::Attacker
            && d1_s2 == Space::Attacker
            && d2_s1 == Space::Attacker
            && d2_s2 == Space::Attacker
        {
            for vertex in [exit, d1_v1, d1_v2, d2_v1, d2_v2] {
                defended.push(*vertex);
            }

            Some(defended)
        } else {
            None
        }
    }

    #[must_use]
    pub fn closed_off_exits(&self) -> Option<HashSet<Vertex>> {
        let mut defended_spaces = HashSet::new();

        for exit in self.exit_squares() {
            if let Some(defended) = self.closed_off_exit(exit) {
                // DEBUG!
                /*
                if defended_spaces.contains(&exit) == true && defended.contains(&exit) == false {
                    println!("{exit}\n{self}");
                }
                */

                for vertex in defended {
                    defended_spaces.insert(vertex);
                }
            } else {
                return None;
            }
        }

        Some(defended_spaces)
    }

    #[must_use]
    pub fn can_not_escape(&self) -> bool {
        let defenders_left = match self.size() {
            BoardSize::_11 => 12 - self.defenders_captured,
            BoardSize::_13 => 16 - self.defenders_captured,
        };

        let attackers_left = match self.size() {
            BoardSize::_11 => 24 - self.attackers_captured,
            BoardSize::_13 => 32 - self.attackers_captured,
        };

        if self.king_trapped(defenders_left, attackers_left) {
            return true;
        }

        if let Some(defended_spaces) = self.closed_off_exits() {
            (defenders_left < 4 && attackers_left >= 13)
                || (defenders_left < 6 && self.n_or_less_side_spaces(&defended_spaces, 3))
                || self.n_or_less_side_spaces(&defended_spaces, 2)
        } else {
            false
        }
    }

    #[must_use]
    pub fn difference(&self, other: &Board) -> Option<Plae> {
        let size = self.size();
        let size_usize = size.into();
        let mut role = None;
        let mut from = None;
        let mut to = None;

        for y in 0..size_usize {
            for x in 0..size_usize {
                let vertex = Vertex { size, x, y };
                let a = self.get(&vertex);
                let b = other.get(&vertex);
                if a != b {
                    if a == Space::Empty {
                        to = Some(vertex);
                        role = Some(b.into());
                    }
                    if b == Space::Empty {
                        from = Some(vertex);
                    }
                }
            }
        }

        if let (Some(role), Some(from), Some(to)) = (role, from, to) {
            Some(Plae::Play(Play { role, from, to }))
        } else {
            None
        }
    }

    #[must_use]
    pub fn exit_squares(&self) -> Vec<Vertex> {
        match self.size() {
            BoardSize::_11 => EXIT_SQUARES_11X11.into(),
            BoardSize::_13 => EXIT_SQUARES_13X13.into(),
        }
    }

    fn capture_the_king(
        &mut self,
        role_from: Role,
        play_to: &Vertex,
        captures: &mut Vec<Vertex>,
    ) -> bool {
        if let Some(kings_vertex) = self.king
            && role_from == Role::Attacker
            && let Some(right) = kings_vertex.right()
            && let Some(left) = kings_vertex.left()
            && let Some(down) = kings_vertex.down()
            && let Some(up) = kings_vertex.up()
            && (*play_to == up || *play_to == left || *play_to == down || *play_to == right)
            && (self.get(&up) == Space::Attacker || up.on_throne())
            && (self.get(&left) == Space::Attacker || left.on_throne())
            && (self.get(&down) == Space::Attacker || down.on_throne())
            && (self.get(&right) == Space::Attacker || right.on_throne())
        {
            self.set(&kings_vertex, Space::Empty);
            self.king = None;
            captures.push(kings_vertex);

            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn capture_the_king_one_move(&self) -> Option<Vertex> {
        let mut spaces_left = 4;
        let mut capture = None;

        if let Some(kings_vertex) = self.king {
            if let Some(vertex) = kings_vertex.up() {
                if vertex.on_throne() || self.get(&vertex) == Space::Attacker {
                    spaces_left -= 1;
                } else {
                    capture = Some(vertex);
                }
            }

            if let Some(vertex) = kings_vertex.left() {
                if vertex.on_throne() || self.get(&vertex) == Space::Attacker {
                    spaces_left -= 1;
                } else {
                    capture = Some(vertex);
                }
            }

            if let Some(vertex) = kings_vertex.down() {
                if vertex.on_throne() || self.get(&vertex) == Space::Attacker {
                    spaces_left -= 1;
                } else {
                    capture = Some(vertex);
                }
            }

            if let Some(vertex) = kings_vertex.right() {
                if vertex.on_throne() || self.get(&vertex) == Space::Attacker {
                    spaces_left -= 1;
                } else {
                    capture = Some(vertex);
                }
            }
        }

        if spaces_left == 1 { capture } else { None }
    }

    fn exit_forts(&self) -> bool {
        match self.king {
            Some(kings_vertex) => {
                kings_vertex.touches_wall()
                    && self.able_to_move(&kings_vertex)
                    && self.flood_fill_defender_wins(&kings_vertex)
            }
            None => false,
        }
    }

    #[inline]
    fn flood_fill_attacker_wins(&self) -> bool {
        let size = self.size();
        let board_size_usize: usize = size.into();

        match self.king {
            Some(kings_vertex) => {
                let mut already_checked = vec![0; board_size_usize * board_size_usize];
                already_checked[usize::from(&kings_vertex)] += 1;

                let mut stack = Vec::with_capacity(32);
                stack.push(kings_vertex);

                while !stack.is_empty() {
                    if let Some(vertex) = stack.pop() {
                        let space = self.get(&vertex);
                        if space == Space::Empty || Role::from(space) == Role::Defender {
                            if !expand_flood_fill(vertex.up(), &mut already_checked, &mut stack) {
                                return false;
                            }
                            if !expand_flood_fill(vertex.left(), &mut already_checked, &mut stack) {
                                return false;
                            }
                            if !expand_flood_fill(vertex.down(), &mut already_checked, &mut stack) {
                                return false;
                            }
                            if !expand_flood_fill(vertex.right(), &mut already_checked, &mut stack)
                            {
                                return false;
                            }
                        }
                    }
                }

                for y in 0..board_size_usize {
                    for x in 0..board_size_usize {
                        let vertex = Vertex { size, x, y };
                        if Role::from(self.get(&vertex)) == Role::Defender
                            && already_checked[usize::from(&vertex)] == 0
                        {
                            return false;
                        }
                    }
                }

                true
            }
            None => false,
        }
    }

    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn flood_fill_defender_wins(&self, vertex: &Vertex) -> bool {
        let size = self.size();
        let board_size_usize = size.into();

        let mut attacker_has_enough_pieces = false;
        let mut count = 0;
        'outer: for y in 0..board_size_usize {
            for x in 0..board_size_usize {
                let vertex = Vertex { size, x, y };
                if Role::from(self.get(&vertex)) == Role::Attacker {
                    count += 1;
                }

                if count > 1 {
                    attacker_has_enough_pieces = true;
                    break 'outer;
                }
            }
        }

        let mut already_checked = FxHashSet::default();
        let mut stack = vec![];

        if let Some(vertex) = vertex.up() {
            stack.push((vertex, Direction::LeftRight));
        }
        if let Some(vertex) = vertex.left() {
            stack.push((vertex, Direction::UpDown));
        }
        if let Some(vertex) = vertex.down() {
            stack.push((vertex, Direction::LeftRight));
        }
        if let Some(vertex) = vertex.right() {
            stack.push((vertex, Direction::UpDown));
        }

        while !stack.is_empty() {
            if let Some((vertex, direction)) = stack.pop() {
                let space = self.get(&vertex);
                if space == Space::Empty {
                    if let Some(vertex) = vertex.up()
                        && !already_checked.contains(&vertex)
                    {
                        stack.push((vertex, Direction::LeftRight));
                        already_checked.insert(vertex);
                    }
                    if let Some(vertex) = vertex.left()
                        && !already_checked.contains(&vertex)
                    {
                        stack.push((vertex, Direction::UpDown));
                        already_checked.insert(vertex);
                    }
                    if let Some(vertex) = vertex.down()
                        && !already_checked.contains(&vertex)
                    {
                        stack.push((vertex, Direction::LeftRight));
                        already_checked.insert(vertex);
                    }
                    if let Some(vertex) = vertex.right()
                        && !already_checked.contains(&vertex)
                    {
                        stack.push((vertex, Direction::UpDown));
                        already_checked.insert(vertex);
                    }
                } else if Role::from(space) == Role::Attacker {
                    return false;
                } else if direction == Direction::UpDown {
                    let mut vertex_1 = false;
                    let mut vertex_2 = false;

                    if let Some(vertex) = vertex.up() {
                        if Role::from(self.get(&vertex)) == Role::Defender {
                            vertex_1 = true;
                        }
                    } else {
                        vertex_1 = true;
                    }
                    if let Some(vertex) = vertex.down() {
                        if Role::from(self.get(&vertex)) == Role::Defender {
                            vertex_2 = true;
                        }
                    } else {
                        vertex_2 = true;
                    }

                    if !vertex_1 && !vertex_2 && attacker_has_enough_pieces {
                        return false;
                    }
                } else {
                    let mut vertex_1 = false;
                    let mut vertex_2 = false;

                    if let Some(vertex) = vertex.right() {
                        if Role::from(self.get(&vertex)) == Role::Defender {
                            vertex_1 = true;
                        }
                    } else {
                        vertex_1 = true;
                    }
                    if let Some(vertex) = vertex.left() {
                        if Role::from(self.get(&vertex)) == Role::Defender {
                            vertex_2 = true;
                        }
                    } else {
                        vertex_2 = true;
                    }

                    if !vertex_1 && !vertex_2 && attacker_has_enough_pieces {
                        return false;
                    }
                }
            }
        }

        true
    }

    #[must_use]
    pub fn get(&self, vertex: &Vertex) -> Space {
        let board_size: usize = self.size().into();
        self.spaces[vertex.y * board_size + vertex.x]
    }

    #[must_use]
    pub fn get_neighbors(
        &self,
        starts: &Vec<Vertex>,
        visited: &HashMap<Vertex, (u8, Option<Vertex>)>,
    ) -> Vec<Vertex> {
        let mut neighbors = Vec::new();
        let size = self.size();
        let board_usize = size.into();

        for start in starts {
            for x in 1..=start.x {
                let index = start.x - x;
                let vertex = Vertex {
                    size,
                    x: index,
                    y: start.y,
                };

                if self.get(&vertex) != Space::Empty {
                    break;
                }

                if !visited.contains_key(&vertex) {
                    neighbors.push(vertex);
                }
            }

            for x in (start.x + 1)..board_usize {
                let vertex = Vertex {
                    size,
                    x,
                    y: start.y,
                };

                if self.get(&vertex) != Space::Empty {
                    break;
                }

                if !visited.contains_key(&vertex) {
                    neighbors.push(vertex);
                }
            }

            for y in 1..=start.y {
                let index = start.y - y;
                let vertex = Vertex {
                    size,
                    x: start.x,
                    y: index,
                };

                if self.get(&vertex) != Space::Empty {
                    break;
                }

                if !visited.contains_key(&vertex) {
                    neighbors.push(vertex);
                }
            }

            for y in (start.y + 1)..board_usize {
                let vertex = Vertex {
                    size,
                    x: start.x,
                    y,
                };

                if self.get(&vertex) != Space::Empty {
                    break;
                }

                if !visited.contains_key(&vertex) {
                    neighbors.push(vertex);
                }
            }
        }

        neighbors
    }

    #[must_use]
    pub fn size(&self) -> BoardSize {
        let len = self.spaces.len();

        if len == 11 * 11 {
            BoardSize::_11
        } else if len == 13 * 13 {
            BoardSize::_13
        } else {
            eprintln!("len is {len} not 11^2 or 13^2");
            unreachable!();
        }
    }

    /// # Errors
    ///
    /// If the move is illegal.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss
    )]
    pub fn legal_move(
        &self,
        play: &Play,
        status: &Status,
        turn: &Role,
        previous_boards: &PreviousBoards,
    ) -> Result<Board, InvalidMove> {
        let size = self.size();

        if *status != Status::Ongoing {
            return Err(InvalidMove::GameOver);
        }

        let space_from = self.get(&play.from);
        let role_from = Role::from(space_from);

        if role_from == Role::Roleless {
            return Err(InvalidMove::Role);
        } else if *turn != role_from {
            return Err(InvalidMove::Turn);
        }

        let x_diff = play.from.x as i32 - play.to.x as i32;
        let y_diff = play.from.y as i32 - play.to.y as i32;

        if x_diff != 0 && y_diff != 0 {
            return Err(InvalidMove::StraightLine);
        }

        if x_diff == 0 && y_diff == 0 {
            return Err(InvalidMove::Location);
        }

        if x_diff != 0 {
            let x_diff_sign = x_diff.signum();
            for x_diff in 1..=x_diff.abs() {
                let vertex = Vertex {
                    size,
                    x: (play.from.x as i32 - (x_diff * x_diff_sign)) as usize,
                    y: play.from.y,
                };

                let space = self.get(&vertex);
                if space != Space::Empty {
                    return Err(InvalidMove::Empty);
                }
            }
        } else {
            let y_diff_sign = y_diff.signum();
            for y_diff in 1..=y_diff.abs() {
                let vertex = Vertex {
                    size,
                    x: play.from.x,
                    y: (play.from.y as i32 - (y_diff * y_diff_sign)) as usize,
                };

                let space = self.get(&vertex);
                if space != Space::Empty {
                    return Err(InvalidMove::Empty);
                }
            }
        }

        if space_from != Space::King && play.to.on_restricted_square() {
            return Err(InvalidMove::Restricted);
        }

        let mut board = self.clone();
        board.set(&play.from, Space::Empty);
        board.set(&play.to, space_from);

        if space_from == Space::King {
            board.king = Some(play.to);
        }

        if turn == &Role::Defender && previous_boards.0.contains(&board) {
            return Err(InvalidMove::RepeatMove);
        }

        Ok(board)
    }

    #[must_use]
    fn no_attacker_pieces_left(&self) -> bool {
        let size = self.size();
        let board_size_usize: usize = size.into();

        for y in 0..board_size_usize {
            for x in 0..board_size_usize {
                let v = Vertex { size, x, y };
                if Role::from(self.get(&v)) == Role::Attacker {
                    return false;
                }
            }
        }

        true
    }

    /// # Errors
    ///
    /// If the move is illegal.
    pub fn play(
        &mut self,
        play: &Plae,
        status: &Status,
        turn: &Role,
        previous_boards: &mut PreviousBoards,
    ) -> Result<(Vec<Vertex>, Status), InvalidMove> {
        let (board, captures, status) = self.play_internal(play, status, turn, previous_boards)?;
        previous_boards.0.push(board.clone());
        *self = board;

        Ok((captures, status))
    }

    #[must_use]
    pub fn open_tafl_serialize(&self) -> String {
        let size = self.size();
        let size_usize: usize = self.size().into();
        let mut empty_spaces: Option<u8> = None;
        let mut open_tafl = String::new();

        for y in 0..size_usize {
            if let Some(spaces) = empty_spaces {
                open_tafl.push_str(&spaces.to_string());
                empty_spaces = None;
            }

            open_tafl.push('/');

            for x in 0..size_usize {
                let vertex = Vertex { size, x, y };
                match self.get(&vertex) {
                    Space::Empty => match empty_spaces {
                        None => {
                            empty_spaces = Some(1);
                        }
                        Some(spaces) => {
                            empty_spaces = Some(spaces + 1);
                        }
                    },
                    Space::Attacker => {
                        if let Some(spaces) = empty_spaces {
                            open_tafl.push_str(&spaces.to_string());
                            empty_spaces = None;
                        }

                        open_tafl.push('t');
                    }
                    Space::King => {
                        if let Some(spaces) = empty_spaces {
                            open_tafl.push_str(&spaces.to_string());
                            empty_spaces = None;
                        }

                        open_tafl.push('K');
                    }
                    Space::Defender => {
                        if let Some(spaces) = empty_spaces {
                            open_tafl.push_str(&spaces.to_string());
                            empty_spaces = None;
                        }

                        open_tafl.push('T');
                    }
                }
            }
        }

        if let Some(spaces) = empty_spaces {
            open_tafl.push_str(&spaces.to_string());
        }
        open_tafl.push('/');

        open_tafl
    }

    /// # Errors
    ///
    /// If it fails deserializing the `&str`.
    pub fn open_tafl_deserialize(string: &str) -> Result<Self, InvalidMove> {
        let mut board_size = 0;
        let mut number = String::new();

        'outer: for ch in string.chars().skip(1) {
            match ch {
                't' | 'T' | 'K' => {
                    if !number.is_empty() {
                        board_size += number.parse::<usize>()?;
                        number = String::new();
                    }

                    board_size += 1;
                }
                '/' => break 'outer,
                ch if ch.is_ascii_digit() => number.push(ch),
                _ => {}
            }
        }

        if !number.is_empty() {
            board_size += number.parse::<usize>()?;
            number = String::new();
        }

        let mut spaces = Vec::with_capacity(board_size * board_size);
        let mut attackers = 0;
        let mut defenders = 0;

        for ch in string.chars() {
            match ch {
                't' => {
                    if !number.is_empty() {
                        let empty_spaces = number.parse()?;
                        for _ in 0..empty_spaces {
                            spaces.push(Space::Empty);
                        }
                        number = String::new();
                    }

                    attackers += 1;
                    spaces.push(Space::Attacker);
                }
                'T' => {
                    if !number.is_empty() {
                        let empty_spaces = number.parse()?;
                        for _ in 0..empty_spaces {
                            spaces.push(Space::Empty);
                        }
                        number = String::new();
                    }

                    defenders += 1;
                    spaces.push(Space::Defender);
                }
                'K' => {
                    if !number.is_empty() {
                        let empty_spaces = number.parse()?;
                        for _ in 0..empty_spaces {
                            spaces.push(Space::Empty);
                        }
                        number = String::new();
                    }

                    spaces.push(Space::King);
                }
                '/' => {
                    if !number.is_empty() {
                        let empty_spaces = number.parse()?;
                        for _ in 0..empty_spaces {
                            spaces.push(Space::Empty);
                        }
                        number = String::new();
                    }
                }
                ch if ch.is_ascii_digit() => number.push(ch),
                _ => {}
            }
        }

        let mut board = Self {
            spaces,
            king: None,
            attackers_captured: 0,
            defenders_captured: 0,
            display_ascii: false,
        };

        let size = board.size();
        let size_usize = size.into();

        for y in 0..size_usize {
            for x in 0..size_usize {
                let vertex = Vertex { size, x, y };

                if let Space::King = board.get(&vertex) {
                    board.king = Some(vertex);
                }
            }
        }

        match size {
            BoardSize::_11 => {
                board.attackers_captured = 24 - attackers;
                board.defenders_captured = 12 - defenders;
            }
            BoardSize::_13 => {
                board.attackers_captured = 32 - attackers;
                board.defenders_captured = 16 - defenders;
            }
        }

        Ok(board)
    }

    /// # Errors
    ///
    /// If the move is illegal.
    pub fn play_internal(
        &self,
        play: &Plae,
        status: &Status,
        turn: &Role,
        previous_boards: &PreviousBoards,
    ) -> Result<(Board, Vec<Vertex>, Status), InvalidMove> {
        if *status != Status::Ongoing {
            return Err(InvalidMove::NotOngoing);
        }

        let play = match play {
            Plae::AttackerResigns => return Ok((self.clone(), Vec::new(), Status::DefenderWins)),
            Plae::DefenderResigns => return Ok((self.clone(), Vec::new(), Status::AttackerWins)),
            Plae::Play(play) => play,
        };

        let mut board = self.legal_move(play, status, turn, previous_boards)?;
        let space_from = self.get(&play.from);
        let role_from = Role::from(space_from);
        let mut captures = Vec::new();
        board.captures(&play.to, role_from, &mut captures);
        board.captures_shield_wall(role_from, &play.to, &mut captures);

        if play.to.on_exit_square() {
            return Ok((board, captures, Status::DefenderWins));
        }

        if board.capture_the_king(role_from, &play.to, &mut captures) {
            return Ok((board, captures, Status::AttackerWins));
        }

        if board.exit_forts() {
            return Ok((board, captures, Status::DefenderWins));
        }

        if board.flood_fill_attacker_wins() {
            return Ok((board, captures, Status::AttackerWins));
        }

        if board.no_attacker_pieces_left() {
            return Ok((board, captures, Status::DefenderWins));
        }

        Ok((board, captures, Status::Ongoing))
    }

    fn set(&mut self, vertex: &Vertex, space: Space) {
        let board_size: usize = self.size().into();
        self.spaces[vertex.y * board_size + vertex.x] = space;
    }

    #[must_use]
    fn set_if_not_king(&mut self, vertex: &Vertex, space: Space) -> bool {
        if self.get(vertex) == Space::King {
            false
        } else {
            self.set(vertex, space);
            true
        }
    }

    #[must_use]
    pub fn spaces_around_the_king(&self) -> Option<u8> {
        let king = self.king?;

        let Some(up) = king.up() else {
            return Some(5);
        };
        let Some(left) = king.left() else {
            return Some(5);
        };
        let Some(down) = king.down() else {
            return Some(5);
        };
        let Some(right) = king.right() else {
            return Some(5);
        };

        let mut sum = 4;
        for vertex in [up, left, down, right] {
            if self.get(&vertex) == Space::Attacker || vertex.on_throne() {
                sum -= 1;
            }
        }

        Some(sum)
    }

    #[must_use]
    pub fn king_trapped(&self, defenders_left: usize, attackers_left: usize) -> bool {
        let size_usize = usize::from(self.size());

        if let Some(king) = self.king {
            if king.x == 0 {
                self.king_trapped_x_0(king, defenders_left, attackers_left)
            } else if king.x == size_usize - 1 {
                self.king_trapped_x_size(king, defenders_left, attackers_left)
            } else if king.y == 0 {
                self.king_trapped_y_0(king, defenders_left, attackers_left)
            } else if king.y == size_usize - 1 {
                self.king_trapped_y_size(king, defenders_left, attackers_left)
            } else {
                false
            }
        } else {
            true
        }
    }

    fn king_trapped_x_0(
        &self,
        king: Vertex,
        defenders_left: usize,
        mut attackers_left: usize,
    ) -> bool {
        let size = king.size;
        let size_usize = usize::from(size);

        if king.y == 1 || king.y == size_usize - 2 {
            return false;
        }

        for y in (king.y - 2)..king.y {
            if y == 0 {
                attackers_left += 1;
                continue;
            }

            let vertex = Vertex { size, x: 0, y };

            if self.get(&vertex) != Space::Attacker {
                return false;
            }
        }

        for y in (king.y + 1)..(king.y + 3) {
            if y == size_usize - 1 {
                attackers_left += 1;
                continue;
            }

            let vertex = Vertex { size, x: 0, y };

            if self.get(&vertex) != Space::Attacker {
                return false;
            }
        }

        if attackers_left > 5
            && !(self.get(&Vertex {
                size,
                x: king.x + 1,
                y: king.y,
            }) == Space::Attacker
                && self.get(&Vertex {
                    size,
                    x: king.x + 2,
                    y: king.y,
                }) == Space::Attacker)
        {
            return false;
        }

        defenders_left < 2
            || attackers_left > 7
                && ((self.get(&Vertex {
                    size,
                    x: king.x + 1,
                    y: king.y + 1,
                }) == Space::Attacker
                    && self.get(&Vertex {
                        size,
                        x: king.x + 2,
                        y: king.y + 1,
                    }) == Space::Attacker)
                    || (self.get(&Vertex {
                        size,
                        x: king.x + 1,
                        y: king.y - 1,
                    }) == Space::Attacker
                        && self.get(&Vertex {
                            size,
                            x: king.x + 2,
                            y: king.y - 1,
                        }) == Space::Attacker))
    }

    fn king_trapped_x_size(
        &self,
        king: Vertex,
        defenders_left: usize,
        mut attackers_left: usize,
    ) -> bool {
        let size = king.size;
        let size_usize = usize::from(size);

        if king.y == 1 || king.y == size_usize - 2 {
            return false;
        }

        for y in (king.y - 2)..king.y {
            if y == 0 {
                attackers_left += 1;
                continue;
            }

            let vertex = Vertex {
                size,
                x: size_usize - 1,
                y,
            };

            if self.get(&vertex) != Space::Attacker {
                return false;
            }
        }

        for y in (king.y + 1)..(king.y + 3) {
            if y == size_usize - 1 {
                attackers_left += 1;
                continue;
            }

            let vertex = Vertex {
                size,
                x: size_usize - 1,
                y,
            };

            if self.get(&vertex) != Space::Attacker {
                return false;
            }
        }

        if attackers_left > 5
            && !(self.get(&Vertex {
                size,
                x: king.x - 1,
                y: king.y,
            }) == Space::Attacker
                && self.get(&Vertex {
                    size,
                    x: king.x - 2,
                    y: king.y,
                }) == Space::Attacker)
        {
            return false;
        }

        defenders_left < 2
            || attackers_left > 7
                && ((self.get(&Vertex {
                    size,
                    x: king.x - 1,
                    y: king.y + 1,
                }) == Space::Attacker
                    && self.get(&Vertex {
                        size,
                        x: king.x - 2,
                        y: king.y + 1,
                    }) == Space::Attacker)
                    || (self.get(&Vertex {
                        size,
                        x: king.x - 1,
                        y: king.y - 1,
                    }) == Space::Attacker
                        && self.get(&Vertex {
                            size,
                            x: king.x - 2,
                            y: king.y - 1,
                        }) == Space::Attacker))
    }

    fn king_trapped_y_0(
        &self,
        king: Vertex,
        defenders_left: usize,
        mut attackers_left: usize,
    ) -> bool {
        let size = king.size;
        let size_usize = usize::from(size);

        if king.x == 1 || king.x == size_usize - 2 {
            return false;
        }

        for x in (king.x - 2)..king.x {
            if x == 0 {
                attackers_left += 1;
                continue;
            }

            let vertex = Vertex { size, x, y: 0 };

            if self.get(&vertex) != Space::Attacker {
                return false;
            }
        }

        for x in (king.x + 1)..(king.x + 3) {
            if x == size_usize - 1 {
                attackers_left += 1;
                continue;
            }

            let vertex = Vertex { size, x, y: 0 };

            if self.get(&vertex) != Space::Attacker {
                return false;
            }
        }

        if attackers_left > 5
            && !(self.get(&Vertex {
                size,
                x: king.x,
                y: king.y + 1,
            }) == Space::Attacker
                && self.get(&Vertex {
                    size,
                    x: king.x,
                    y: king.y + 2,
                }) == Space::Attacker)
        {
            return false;
        }

        defenders_left < 2
            || attackers_left > 7
                && ((self.get(&Vertex {
                    size,
                    x: king.x + 1,
                    y: king.y + 1,
                }) == Space::Attacker
                    && self.get(&Vertex {
                        size,
                        x: king.x + 1,
                        y: king.y + 2,
                    }) == Space::Attacker)
                    || (self.get(&Vertex {
                        size,
                        x: king.x - 1,
                        y: king.y + 1,
                    }) == Space::Attacker
                        && self.get(&Vertex {
                            size,
                            x: king.x - 1,
                            y: king.y + 2,
                        }) == Space::Attacker))
    }

    fn king_trapped_y_size(
        &self,
        king: Vertex,
        defenders_left: usize,
        mut attackers_left: usize,
    ) -> bool {
        let size = king.size;
        let size_usize = usize::from(size);

        if king.x == 1 || king.x == size_usize - 2 {
            return false;
        }

        for x in (king.x - 2)..king.x {
            if x == 0 {
                attackers_left += 1;
                continue;
            }

            let vertex = Vertex {
                size,
                x,
                y: size_usize - 1,
            };

            if self.get(&vertex) != Space::Attacker {
                return false;
            }
        }

        for x in (king.x + 1)..(king.x + 3) {
            if x == size_usize - 1 {
                attackers_left += 1;
                continue;
            }

            let vertex = Vertex {
                size,
                x,
                y: size_usize - 1,
            };

            if self.get(&vertex) != Space::Attacker {
                return false;
            }
        }

        if attackers_left > 5
            && !(self.get(&Vertex {
                size,
                x: king.x,
                y: king.y - 1,
            }) == Space::Attacker
                && self.get(&Vertex {
                    size,
                    x: king.x,
                    y: king.y - 2,
                }) == Space::Attacker)
        {
            return false;
        }

        defenders_left < 2
            || attackers_left > 7
                && ((self.get(&Vertex {
                    size,
                    x: king.x + 1,
                    y: king.y - 1,
                }) == Space::Attacker
                    && self.get(&Vertex {
                        size,
                        x: king.x + 1,
                        y: king.y - 2,
                    }) == Space::Attacker)
                    || (self.get(&Vertex {
                        size,
                        x: king.x - 1,
                        y: king.y - 1,
                    }) == Space::Attacker
                        && self.get(&Vertex {
                            size,
                            x: king.x - 1,
                            y: king.y - 2,
                        }) == Space::Attacker))
    }

    #[must_use]
    fn n_or_less_side_spaces(&self, defended_spaces: &HashSet<Vertex>, n: u8) -> bool {
        let size = self.size();
        let size_usize = usize::from(size);

        let mut count = 0;
        let mut continuos = Continuos::default();
        for y in 0..size_usize {
            let vertex = Vertex { size, x: 0, y };

            if !self.connected(&vertex, defended_spaces, n, &mut count, &mut continuos) {
                return false;
            }
        }

        let mut count = 0;
        let mut continuos = Continuos::default();
        for y in 0..size_usize {
            let vertex = Vertex {
                size,
                x: size_usize - 1,
                y,
            };

            if !self.connected(&vertex, defended_spaces, n, &mut count, &mut continuos) {
                return false;
            }
        }

        let mut count = 0;
        let mut continuos = Continuos::default();
        for x in 0..size_usize {
            let vertex = Vertex { size, x, y: 0 };

            if !self.connected(&vertex, defended_spaces, n, &mut count, &mut continuos) {
                return false;
            }
        }

        let mut count = 0;
        let mut continuos = Continuos::default();
        for x in 0..size_usize {
            let vertex = Vertex {
                size,
                x,
                y: size_usize - 1,
            };

            if !self.connected(&vertex, defended_spaces, n, &mut count, &mut continuos) {
                return false;
            }
        }

        true
    }

    fn connected(
        &self,
        vertex: &Vertex,
        defended_spaces: &HashSet<Vertex>,
        spaces: u8,
        count: &mut u8,
        continuos: &mut Continuos,
    ) -> bool {
        let space = self.get(vertex);

        if space != Space::Attacker && !defended_spaces.contains(vertex) {
            if *continuos == Continuos::Next {
                return false;
            } else if *count == 0 {
                *continuos = Continuos::Some;
            }

            *count += 1;
        } else if *continuos == Continuos::Some {
            *continuos = Continuos::Next;
        }

        if *count > spaces {
            return false;
        }

        true
    }
}

#[derive(Clone, Debug)]
pub struct OpenTaflBoard {
    pub board: Board,
}

impl Serialize for OpenTaflBoard {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let board = self.board.open_tafl_serialize();
        serializer.serialize_str(&board)
    }
}

impl<'de> Deserialize<'de> for OpenTaflBoard {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MyVisitor;

        impl Visitor<'_> for MyVisitor {
            type Value = Board;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a board")
            }

            fn visit_str<E>(self, string: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Board::open_tafl_deserialize(string).expect("failed deserializing..."))
            }
        }

        Ok(OpenTaflBoard {
            board: deserializer.deserialize_str(MyVisitor)?,
        })
    }
}

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
pub enum BoardSize {
    #[default]
    _11,
    _13,
}

impl fmt::Display for BoardSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoardSize::_11 => write!(f, "11"),
            BoardSize::_13 => write!(f, "13"),
        }
    }
}

impl From<BoardSize> for usize {
    fn from(size: BoardSize) -> Self {
        match size {
            BoardSize::_11 => 11,
            BoardSize::_13 => 13,
        }
    }
}

impl FromStr for BoardSize {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "11" => Ok(BoardSize::_11),
            "13" => Ok(BoardSize::_13),
            _ => Err(anyhow::Error::msg(format!("expected 11 or 13, got {s}"))),
        }
    }
}

impl TryFrom<usize> for BoardSize {
    type Error = anyhow::Error;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            11 => Ok(BoardSize::_11),
            13 => Ok(BoardSize::_13),
            _ => Err(anyhow::Error::msg(format!(
                "an invalid board size was passed: {value}"
            ))),
        }
    }
}

#[must_use]
#[allow(clippy::missing_panics_doc)]
#[allow(clippy::unwrap_used)]
fn board_11x11() -> Board {
    let spaces: Vec<Space> = STARTING_POSITION_11X11
        .iter()
        .flat_map(|space| space.chars().map(|ch| ch.try_into().unwrap()))
        .collect();

    let mut board = Board {
        spaces,
        attackers_captured: 0,
        defenders_captured: 0,
        king: Some(Vertex {
            size: BoardSize::_11,
            x: 5,
            y: 5,
        }),
        display_ascii: false,
    };

    let captured = board.captured();
    board.attackers_captured = captured.attacker;
    board.defenders_captured = captured.defender;

    board
}

#[must_use]
#[allow(clippy::missing_panics_doc)]
#[allow(clippy::unwrap_used)]
fn board_13x13() -> Board {
    let spaces: Vec<Space> = STARTING_POSITION_13X13
        .iter()
        .flat_map(|space| space.chars().map(|ch| ch.try_into().unwrap()))
        .collect();

    let mut board = Board {
        spaces,
        attackers_captured: 0,
        defenders_captured: 0,
        king: Some(Vertex {
            size: BoardSize::_13,
            x: 6,
            y: 6,
        }),
        display_ascii: false,
    };

    let captured = board.captured();
    board.attackers_captured = captured.attacker;
    board.defenders_captured = captured.defender;

    board
}

pub struct Captured {
    pub attacker: usize,
    pub defender: usize,
    pub king: bool,
}

impl From<&Board> for Captured {
    fn from(board: &Board) -> Self {
        Captured {
            attacker: board.attackers_captured,
            defender: board.defenders_captured,
            king: board.king.is_none(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Direction {
    LeftRight,
    UpDown,
}

#[derive(Error, Debug, PartialEq)]
pub enum InvalidMove {
    #[error("to plae: {0}")]
    ToPlae(String),
    #[error("message: {0}")]
    Message(String),
    #[error("unrecognized command: {0}")]
    UnrecognizedCommand(String),
    #[error("play: the game is already over")]
    GameOver,
    #[error("play: you have to play through empty locations")]
    Empty,
    #[error("play: you have to change location")]
    Location,
    #[error("play: you already reached that position")]
    RepeatMove,
    #[error("play: only the king may move to a restricted square")]
    Restricted,
    #[error("play: you didn't select a role")]
    Role,
    #[error("play: you can only play in a straight line")]
    StraightLine,
    #[error("play: it isn't your turn")]
    Turn,
    #[error("play: the game has to be ongoing to play")]
    NotOngoing,
    #[error("play: you can't resign for the other player")]
    InvalidResign,
    #[error("play: invalid vertex")]
    InvalidVertex,
    #[error("play to: expected 'play_to role vertex'")]
    InvalidRoleVertex,
    #[error("play undo: it is the first move")]
    FirstMove,
    #[error(
        "time_settings: expected 'time_settings un-timed' or 'time_settings fischer MINUTES ADD_SECONDS'"
    )]
    InvalidArguments,
    #[error("time_settings: arg 2 is not an integer")]
    NotInteger2,
    #[error("time_settings: arg 3 is not an integer")]
    NotInteger3,
    #[error("time_settings: the time settings are un-timed")]
    UnTimed,
    #[error("error deserializing board: {0}")]
    Deserialize(String),
    #[error("error parsing: {0}")]
    Parsing(String),
    #[error("unknown error")]
    Other,
}

impl From<anyhow::Error> for InvalidMove {
    fn from(error: anyhow::Error) -> Self {
        InvalidMove::Message(error.to_string())
    }
}

impl From<ParseIntError> for InvalidMove {
    fn from(error: ParseIntError) -> Self {
        InvalidMove::Parsing(error.to_string())
    }
}

impl de::Error for InvalidMove {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        InvalidMove::Deserialize(format!("{msg}"))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
enum Continuos {
    #[default]
    None,
    Some,
    Next,
}

#[must_use]
#[inline]
fn expand_flood_fill(
    vertex: Option<Vertex>,
    already_checked: &mut [u32],
    stack: &mut Vec<Vertex>,
) -> bool {
    if let Some(vertex) = vertex {
        let i = usize::from(&vertex);
        if already_checked[i] == 0 {
            stack.push(vertex);
            already_checked[i] += 1;
        } else {
            already_checked[i] += 1;
        }

        true
    } else {
        false
    }
}

fn _print_u32(vector: &[u32]) {
    for (count, i) in vector.iter().enumerate() {
        if count % 11 == 0 {
            println!();
        }

        print!("{i} ");
    }

    println!();
}
