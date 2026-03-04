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

use hnefatafl_copenhagen::board::BoardSize;

use crate::enums::Size;

#[derive(Clone, Debug)]
pub(crate) struct Dimensions {
    pub board_dimension: u32,
    pub letter_size: u32,
    pub piece_size: u32,
    pub spacing: u32,
}

impl Dimensions {
    pub(crate) fn new(board_size: BoardSize, screen_size: &Size) -> Self {
        let (board_dimension, letter_size, piece_size, spacing) = match board_size {
            BoardSize::_11 => match screen_size {
                Size::Large | Size::Giant => (75, 55, 60, 6),
                Size::Medium => (65, 45, 50, 8),
                Size::Small => (55, 35, 40, 11),
                Size::Tiny | Size::TinyWide => (40, 20, 25, 16),
            },
            BoardSize::_13 => match screen_size {
                Size::Large | Size::Giant => (65, 45, 50, 8),
                Size::Medium => (58, 38, 43, 10),
                Size::Small => (50, 30, 35, 12),
                Size::Tiny | Size::TinyWide => (40, 20, 25, 15),
            },
        };

        Dimensions {
            board_dimension,
            letter_size,
            piece_size,
            spacing,
        }
    }
}
