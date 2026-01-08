// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::VecDeque;

use crate::{
    board::{Board, BoardSize},
    game::PreviousBoards,
    play::Plays,
    role::Role,
};

#[derive(Clone, Debug)]
pub struct Tree {
    node: usize,
    pub next_child: usize,
    arena: Vec<Node>,
}

impl Tree {
    pub fn next_child(&mut self) {
        self.next_child += 1;

        if self.next_child >= self.arena[self.node].children.len() {
            self.next_child = 0;
        }
    }

    pub fn backward(&mut self) {
        if let Some(node) = self.arena[self.node].parent {
            self.node = node;
        }

        self.next_child = 0;
    }

    pub fn backward_all(&mut self) {
        let mut node = self.node;

        while let Some(node_index) = self.arena[node].parent {
            node = node_index;
        }

        self.next_child = 0;
        self.node = node;
    }

    pub fn forward(&mut self) {
        if !self.arena[self.node].children.is_empty() {
            self.node = self.arena[self.node].children[self.next_child];
        }

        self.next_child = 0;
    }

    #[must_use]
    pub fn forward_all(&mut self) -> usize {
        let mut node = self.node;
        let mut count = 0;

        while !self.arena[node].children.is_empty() {
            node = self.arena[node].children[self.next_child];
            count += 1;
        }

        self.next_child = 0;
        self.node = node;
        count
    }

    #[must_use]
    pub fn has_children(&self) -> bool {
        !self.arena[self.node].children.is_empty()
    }

    pub fn insert(&mut self, board: &Board) {
        let index = self.arena.len();
        let node = &mut self.arena[self.node];
        self.node = index;

        let old_index = node.index;
        node.children.push(index);
        let turn = node.turn.opposite();

        self.arena.push(Node {
            index,
            board: board.clone(),
            turn,
            parent: Some(old_index),
            children: Vec::new(),
        });
    }

    #[must_use]
    pub fn here(&self) -> Node {
        self.arena[self.node].clone()
    }

    #[must_use]
    pub fn here_board(&self) -> Board {
        self.arena[self.node].board.clone()
    }

    #[must_use]
    pub fn new(board_size: BoardSize) -> Self {
        Self {
            node: 0,
            next_child: 0,
            arena: vec![Node {
                index: 0,
                board: Board::new(board_size),
                turn: Role::Attacker,
                parent: None,
                children: Vec::new(),
            }],
        }
    }

    #[must_use]
    pub fn previous_boards(&self) -> (Plays, PreviousBoards) {
        let mut node = &self.here();
        let mut previous_boards = PreviousBoards::new(node.board.size());
        let mut boards = VecDeque::new();
        let mut plays = Vec::new();

        previous_boards.0.insert(node.board.clone());
        boards.push_front(node.board.clone());

        while let Some(parent) = node.parent {
            node = &self.arena[parent];
            previous_boards.0.insert(node.board.clone());
            boards.push_front(node.board.clone());
        }

        let boards: Vec<_> = boards.iter().collect();
        for windows in boards.windows(2) {
            let board_1 = windows[0];
            let board_2 = windows[1];

            let play = board_1.difference(board_2);
            plays.push(play);
        }

        let plays = Plays::PlayRecords(plays);
        (plays, previous_boards)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    index: usize,
    pub board: Board,
    pub turn: Role,
    parent: Option<usize>,
    children: Vec<usize>,
}
