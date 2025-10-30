use std::{collections::HashMap, fmt, str::FromStr};

use rustc_hash::{FxBuildHasher, FxHashSet};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    game::PreviousBoards,
    play::{BOARD_LETTERS, Plae, Play, Vertex},
    role::Role,
    space::Space,
    status::Status,
};

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

const EXIT_SQUARES_11X11: [Vertex; 4] = [
    Vertex {
        size: BoardSize::_11,
        x: 0,
        y: 0,
    },
    Vertex {
        size: BoardSize::_11,
        x: 10,
        y: 0,
    },
    Vertex {
        size: BoardSize::_11,
        x: 0,
        y: 10,
    },
    Vertex {
        size: BoardSize::_11,
        x: 10,
        y: 10,
    },
];

const THRONE_11X11: Vertex = Vertex {
    size: BoardSize::_11,
    x: 5,
    y: 5,
};

const RESTRICTED_SQUARES_11X11: [Vertex; 5] = [
    Vertex {
        size: BoardSize::_11,
        x: 0,
        y: 0,
    },
    Vertex {
        size: BoardSize::_11,
        x: 10,
        y: 0,
    },
    Vertex {
        size: BoardSize::_11,
        x: 0,
        y: 10,
    },
    Vertex {
        size: BoardSize::_11,
        x: 10,
        y: 10,
    },
    THRONE_11X11,
];

const EXIT_SQUARES_13X13: [Vertex; 4] = [
    Vertex {
        size: BoardSize::_13,
        x: 0,
        y: 0,
    },
    Vertex {
        size: BoardSize::_13,
        x: 12,
        y: 0,
    },
    Vertex {
        size: BoardSize::_13,
        x: 0,
        y: 12,
    },
    Vertex {
        size: BoardSize::_13,
        x: 12,
        y: 12,
    },
];

const THRONE_13X13: Vertex = Vertex {
    size: BoardSize::_13,
    x: 6,
    y: 6,
};

const RESTRICTED_SQUARES_13X13: [Vertex; 5] = [
    Vertex {
        size: BoardSize::_13,
        x: 0,
        y: 0,
    },
    Vertex {
        size: BoardSize::_13,
        x: 12,
        y: 0,
    },
    Vertex {
        size: BoardSize::_13,
        x: 0,
        y: 12,
    },
    Vertex {
        size: BoardSize::_13,
        x: 12,
        y: 12,
    },
    THRONE_13X13,
];

#[derive(Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Board {
    pub spaces: Vec<Space>,
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
        let mut letters = " ".repeat(3).to_string();
        letters.push_str(&BOARD_LETTERS[..board_size]);
        let bar = "─".repeat(board_size);

        writeln!(f, "\n{letters}\n  ┌{bar}┐")?;
        for y in 0..board_size {
            let y_label = board_size - y;
            write!(f, "{y_label:2}│",)?;

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
                    write!(f, "⌘")?;
                } else {
                    write!(f, "{}", self.spaces[y * board_size + x])?;
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
                        if on_restricted_square(&value, &vertex) {
                            return Err(anyhow::Error::msg(
                                "Only the king is allowed on restricted squares!",
                            ));
                        }
                    }
                    Space::Empty => {}
                    Space::King => {
                        kings += 1;
                        if kings > 1 {
                            return Err(anyhow::Error::msg("You can only have one king!"));
                        }
                    }
                }

                spaces.push(space);
            }
        }

        Ok(Self { spaces })
    }
}

impl TryFrom<[&str; 13]> for Board {
    type Error = anyhow::Error;

    fn try_from(value: [&str; 13]) -> anyhow::Result<Self> {
        let mut spaces = Vec::with_capacity(13 * 13);
        let mut kings = 0;

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
                        if on_restricted_square(&value, &vertex) {
                            return Err(anyhow::Error::msg(
                                "Only the king is allowed on restricted squares!",
                            ));
                        }
                    }
                    Space::Empty => {}
                    Space::King => {
                        kings += 1;
                        if kings > 1 {
                            return Err(anyhow::Error::msg("You can only have one king!"));
                        }
                    }
                }

                spaces.push(space);
            }
        }

        Ok(Self { spaces })
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
                if self.get(&vertex_from).role() != *turn {
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

                        if let Ok(_board) = self.legal_move(&play, status, turn, previous_boards) {
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
                && space.role() == role_from.opposite()
                && let Some(right_2) = over(&right_1)
                && ((on_restricted_square(&self.spaces, &right_2)
                    && self.get(&right_2) != Space::King)
                    || self.get(&right_2).role() == role_from)
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
            if self.get(&vertex_1).role() == role_from
                || on_restricted_square(&self.spaces, &vertex_1)
            {
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
                    let role_2 = self.get(&vertex_2).role();
                    let role_3 = self.get(&vertex_3).role();
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
                let role = self.get(&vertex).role();
                if count > 1
                    && (role == role_from || on_restricted_square(&self.spaces, &vertex))
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
            if self.get(&vertex_1).role() == role_from
                || on_restricted_square(&self.spaces, &vertex_1)
            {
                let mut count = 0;

                if x_1 == board_size_usize - 1 {
                    break;
                }
                let start = x_1 + 1;

                for x_2 in start..board_size_usize {
                    let vertex_2 = Vertex { size, x: x_2, y: 0 };
                    let vertex_3 = Vertex { size, x: x_2, y: 1 };
                    let role_2 = self.get(&vertex_2).role();
                    let role_3 = self.get(&vertex_3).role();
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
                let role = self.get(&vertex).role();
                if count > 1
                    && (role == role_from || on_restricted_square(&self.spaces, &vertex))
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
            if self.get(&vertex_1).role() == role_from
                || on_restricted_square(&self.spaces, &vertex_1)
            {
                let mut count = 0;

                if y_1 == board_size_usize - 1 {
                    break;
                }
                let start = y_1 + 1;

                for y_2 in start..board_size_usize {
                    let vertex_2 = Vertex { size, x: 0, y: y_2 };
                    let vertex_3 = Vertex { size, x: 1, y: y_2 };
                    let role_2 = self.get(&vertex_2).role();
                    let role_3 = self.get(&vertex_3).role();
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
                let role = self.get(&vertex).role();
                if count > 1
                    && (role == role_from || on_restricted_square(&self.spaces, &vertex))
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
            if self.get(&vertex_1).role() == role_from
                || on_restricted_square(&self.spaces, &vertex_1)
            {
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
                    let role_2 = self.get(&vertex_2).role();
                    let role_3 = self.get(&vertex_3).role();
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
                let role = self.get(&vertex).role();
                if count > 1
                    && (role == role_from || on_restricted_square(&self.spaces, &vertex))
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

    #[must_use]
    pub fn find_the_king(&self) -> Option<Vertex> {
        let size = self.size();
        let board_size_usize: usize = size.into();

        for y in 0..board_size_usize {
            for x in 0..board_size_usize {
                let v = Vertex { size, x, y };
                if self.get(&v) == Space::King {
                    return Some(v);
                }
            }
        }

        None
    }

    fn capture_the_king(
        &self,
        role_from: Role,
        play_to: &Vertex,
        captures: &mut Vec<Vertex>,
    ) -> bool {
        if let Some(kings_vertex) = self.find_the_king()
            && role_from == Role::Attacker
        {
            let mut attacker_moved = false;

            let (move_to_capture, surrounded) =
                self.capture_the_king_space(play_to, kings_vertex.up());

            if !surrounded {
                return false;
            }
            if move_to_capture {
                attacker_moved = true;
            }

            let (move_to_capture, surrounded) =
                self.capture_the_king_space(play_to, kings_vertex.left());

            if !surrounded {
                return false;
            }
            if move_to_capture {
                attacker_moved = true;
            }

            let (move_to_capture, surrounded) =
                self.capture_the_king_space(play_to, kings_vertex.down());

            if !surrounded {
                return false;
            }
            if move_to_capture {
                attacker_moved = true;
            }

            let (move_to_capture, surrounded) =
                self.capture_the_king_space(play_to, kings_vertex.right());

            if !surrounded {
                return false;
            }
            if move_to_capture {
                attacker_moved = true;
            }

            if attacker_moved {
                captures.push(kings_vertex);
                return true;
            }
        }

        false
    }

    #[must_use]
    pub fn capture_the_king_one_move(&self) -> Option<Vertex> {
        let mut spaces_left = 4;
        let mut capture = None;

        if let Some(kings_vertex) = self.find_the_king() {
            if let Some(vertex) = kings_vertex.up() {
                if self.on_throne(&vertex) || self.get(&vertex) == Space::Attacker {
                    spaces_left -= 1;
                } else {
                    capture = Some(vertex);
                }
            }

            if let Some(vertex) = kings_vertex.left() {
                if self.on_throne(&vertex) || self.get(&vertex) == Space::Attacker {
                    spaces_left -= 1;
                } else {
                    capture = Some(vertex);
                }
            }

            if let Some(vertex) = kings_vertex.down() {
                if self.on_throne(&vertex) || self.get(&vertex) == Space::Attacker {
                    spaces_left -= 1;
                } else {
                    capture = Some(vertex);
                }
            }

            if let Some(vertex) = kings_vertex.right() {
                if self.on_throne(&vertex) || self.get(&vertex) == Space::Attacker {
                    spaces_left -= 1;
                } else {
                    capture = Some(vertex);
                }
            }
        }

        if spaces_left == 1 { capture } else { None }
    }

    #[inline]
    fn capture_the_king_space(&self, play_to: &Vertex, direction: Option<Vertex>) -> (bool, bool) {
        if let Some(surround_king) = direction {
            let move_to_capture = *play_to == surround_king;

            let surrounded = move_to_capture
                || self.on_throne(&surround_king)
                || self.get(&surround_king) == Space::Attacker;

            (move_to_capture, surrounded)
        } else {
            (false, false)
        }
    }

    fn exit_forts(&self) -> bool {
        match self.find_the_king() {
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

        match self.find_the_king() {
            Some(kings_vertex) => {
                let hasher = FxBuildHasher;
                let mut already_checked = FxHashSet::with_capacity_and_hasher(
                    board_size_usize * board_size_usize,
                    hasher,
                );

                already_checked.insert(kings_vertex);
                let mut stack = Vec::with_capacity(board_size_usize * board_size_usize);
                stack.push(kings_vertex);

                while !stack.is_empty() {
                    if let Some(vertex) = stack.pop() {
                        let space = self.get(&vertex);
                        if space == Space::Empty || space.role() == Role::Defender {
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
                        if self.get(&vertex).role() == Role::Defender
                            && !already_checked.contains(&vertex)
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
                if self.get(&vertex).role() == Role::Attacker {
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
                } else if space.role() == Role::Attacker {
                    return false;
                } else if direction == Direction::UpDown {
                    let mut vertex_1 = false;
                    let mut vertex_2 = false;

                    if let Some(vertex) = vertex.up() {
                        if self.get(&vertex).role() == Role::Defender {
                            vertex_1 = true;
                        }
                    } else {
                        vertex_1 = true;
                    }
                    if let Some(vertex) = vertex.down() {
                        if self.get(&vertex).role() == Role::Defender {
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
                        if self.get(&vertex).role() == Role::Defender {
                            vertex_1 = true;
                        }
                    } else {
                        vertex_1 = true;
                    }
                    if let Some(vertex) = vertex.left() {
                        if self.get(&vertex).role() == Role::Defender {
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
    pub fn size(&self) -> BoardSize {
        let len = self.spaces.len();

        if len == 11 * 11 {
            BoardSize::_11
        } else if len == 13 * 13 {
            BoardSize::_13
        } else {
            unreachable!()
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
            return Err(InvalidMove::Ongoing);
        }

        let space_from = self.get(&play.from);
        let role_from = space_from.role();

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

        if space_from != Space::King && on_restricted_square(&self.spaces, &play.to) {
            return Err(InvalidMove::Restricted);
        }

        let mut board = self.clone();
        board.set(&play.from, Space::Empty);
        board.set(&play.to, space_from);

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
                if self.get(&v).role() == Role::Attacker {
                    return false;
                }
            }
        }

        true
    }

    #[inline]
    #[must_use]
    pub fn on_exit_square(&self, vertex: &Vertex) -> bool {
        (self.spaces.len() == 11 * 11 && EXIT_SQUARES_11X11.contains(vertex))
            || (self.spaces.len() == 13 * 13 && EXIT_SQUARES_13X13.contains(vertex))
    }

    #[inline]
    #[must_use]
    fn on_throne(&self, vertex: &Vertex) -> bool {
        let board_size: usize = self.size().into();

        if board_size == 11 {
            THRONE_11X11 == *vertex
        } else if board_size == 13 {
            THRONE_13X13 == *vertex
        } else {
            panic!("The board size is {board_size}!");
        }
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
    ) -> anyhow::Result<(Vec<Vertex>, Status)> {
        let (board, captures, status) = self.play_internal(play, status, turn, previous_boards)?;
        previous_boards.0.insert(board.clone());
        *self = board;

        Ok((captures, status))
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
    ) -> anyhow::Result<(Board, Vec<Vertex>, Status)> {
        if *status != Status::Ongoing {
            return Err(anyhow::Error::msg(
                "play: the game has to be ongoing to play",
            ));
        }

        let play = match play {
            Plae::AttackerResigns => return Ok((self.clone(), Vec::new(), Status::DefenderWins)),
            Plae::DefenderResigns => return Ok((self.clone(), Vec::new(), Status::AttackerWins)),
            Plae::Play(play) => play,
        };

        let mut board = self.legal_move(play, status, turn, previous_boards)?;
        let space_from = self.get(&play.from);
        let role_from = space_from.role();
        let mut captures = Vec::new();
        board.captures(&play.to, role_from, &mut captures);
        board.captures_shield_wall(role_from, &play.to, &mut captures);

        if self.on_exit_square(&play.to) {
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
fn board_11x11() -> Board {
    let spaces: Vec<Space> = STARTING_POSITION_11X11
        .iter()
        .flat_map(|space| space.chars().map(|ch| ch.try_into().unwrap()))
        .collect();

    Board { spaces }
}

#[must_use]
#[allow(clippy::missing_panics_doc)]
fn board_13x13() -> Board {
    let spaces: Vec<Space> = STARTING_POSITION_13X13
        .iter()
        .flat_map(|space| space.chars().map(|ch| ch.try_into().unwrap()))
        .collect();

    Board { spaces }
}

pub struct Captured {
    pub attacker: u8,
    pub defender: u8,
    pub king: bool,
}

impl Captured {
    #[must_use]
    pub fn attacker(&self) -> String {
        format!("♟ {}", self.attacker)
    }

    #[must_use]
    pub fn defender(&self) -> String {
        let mut string = format!("♙ {}", self.defender);
        if self.king {
            string.push_str(" ♔");
        }
        string
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Direction {
    LeftRight,
    UpDown,
}

#[derive(Error, Debug)]
pub enum InvalidMove {
    #[error("play: you have to play through empty locations")]
    Empty,
    #[error("play: you have to change location")]
    Location,
    #[error("play: the game has to be ongoing to play")]
    Ongoing,
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegalMoves {
    pub role: Role,
    pub moves: HashMap<Vertex, Vec<Vertex>>,
}

#[must_use]
#[inline]
fn expand_flood_fill(
    vertex: Option<Vertex>,
    already_checked: &mut FxHashSet<Vertex>,
    stack: &mut Vec<Vertex>,
) -> bool {
    if let Some(vertex) = vertex {
        if !already_checked.contains(&vertex) {
            stack.push(vertex);
            already_checked.insert(vertex);
        }

        true
    } else {
        false
    }
}

#[must_use]
fn on_restricted_square<T>(spaces: &[T], vertex: &Vertex) -> bool {
    let mut len = spaces.len();

    len = if len == 11 * 11 {
        11
    } else if len == 13 * 13 {
        13
    } else {
        len
    };

    if len == 11 {
        RESTRICTED_SQUARES_11X11.contains(vertex)
    } else if len == 13 {
        RESTRICTED_SQUARES_13X13.contains(vertex)
    } else {
        panic!("The board size is {len}!");
    }
}
