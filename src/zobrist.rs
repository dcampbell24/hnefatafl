use std::hash::Hasher;

use rand::{TryRngCore, rngs::OsRng};

use crate::{
    board::{Board, BoardSize},
    play::Vertex,
    space::Space,
};

pub struct ZobristHasher {
    /// Bits representing piece placement.
    table: Vec<[u64; 3]>,
    /// The current hash.
    hash: u64,
}

impl ZobristHasher {
    /// # Errors
    ///
    /// If it fails generating a random number.
    pub fn new(board_size: BoardSize) -> anyhow::Result<Self> {
        let mut rng = OsRng;
        let size = usize::from(board_size);
        let size_2 = size * size;

        let mut table: Vec<[u64; 3]> = Vec::with_capacity(size_2);
        for _ in 0..size_2 {
            table.push([
                rng.try_next_u64()?,
                rng.try_next_u64()?,
                rng.try_next_u64()?,
            ]);
        }

        Ok(Self { table, hash: 0 })
    }
}

impl Hasher for ZobristHasher {
    fn finish(&self) -> u64 {
        self.hash
    }

    fn write(&mut self, board: &[u8]) {
        self.hash = 0u64;
        let board = Board::from(board);
        let board_size = board.size();
        let board_size_usize = usize::from(board_size);

        for y in 0..board_size_usize {
            for x in 0..board_size_usize {
                let vertex = Vertex {
                    x,
                    y,
                    size: board_size,
                };
                let space = board.get(&vertex);

                if space != Space::Empty {
                    let i = y * board_size_usize + x;
                    let j = usize::try_from(space).unwrap();

                    self.hash ^= self.table[i][j];
                }
            }
        }
    }
}
