use rand::{TryRngCore, rngs::OsRng};

use crate::{
    board::{Board, BoardSize},
    play::Vertex,
    role::Role,
    space::Space,
};

pub struct ZobristTable {
    /// Bits representing piece placement.
    table: Vec<[u64; 3]>,
    /// Bits to use used when it's the defender's move.
    defender_to_move: u64,
}

impl ZobristTable {
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

        Ok(Self {
            table,
            defender_to_move: rng.try_next_u64()?,
        })
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn hash(&self, board: &Board, turn: Role) -> u64 {
        let mut hash = 0u64;

        if turn == Role::Defender {
            hash ^= self.defender_to_move;
        }

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

                    hash ^= self.table[i][j];
                }
            }
        }

        hash
    }
}
