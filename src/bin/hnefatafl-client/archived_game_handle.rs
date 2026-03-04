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

use hnefatafl_copenhagen::{
    board::Board, game::PreviousBoards, play::Plays, role::Role, server_game::ArchivedGame,
    status::Status, tree::Tree,
};

#[derive(Clone, Debug)]
pub(crate) struct ArchivedGameHandle {
    pub boards: Tree,
    pub game: ArchivedGame,
    pub play: usize,
}

impl ArchivedGameHandle {
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    #[allow(clippy::unwrap_used)]
    pub(crate) fn new(game: &ArchivedGame) -> ArchivedGameHandle {
        let mut board = Board::new(game.board_size);
        let mut boards = Tree::new(game.board_size);
        let mut turn = Role::default();

        let plays = match &game.plays {
            Plays::PlayRecordsTimed(plays) => {
                plays.iter().map(|record| record.play.clone()).collect()
            }
            Plays::PlayRecords(plays) => plays.clone(),
        };

        for play in &plays {
            if let Some(play) = &play {
                board
                    .play(
                        play,
                        &Status::Ongoing,
                        &turn,
                        &mut PreviousBoards::default(),
                    )
                    .unwrap();

                boards.insert(&board);
                turn = match turn {
                    Role::Attacker => Role::Defender,
                    Role::Roleless => Role::Roleless,
                    Role::Defender => Role::Attacker,
                };
            }
        }

        boards.backward_all();

        ArchivedGameHandle {
            boards,
            game: game.clone(),
            play: 0,
        }
    }
}
