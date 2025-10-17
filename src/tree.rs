use crate::{
    board::{Board, BoardSize},
    game::PreviousBoards,
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
    pub fn previous_boards(&self) -> PreviousBoards {
        let mut node = &self.here();
        let mut previous_boards = PreviousBoards::new(node.board.size());
        previous_boards.0.insert(node.board.clone());

        while let Some(parent) = node.parent {
            node = &self.arena[parent];
            previous_boards.0.insert(node.board.clone());
        }

        previous_boards
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
