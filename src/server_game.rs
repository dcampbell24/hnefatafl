use std::{
    collections::{HashMap, VecDeque},
    fmt,
    str::FromStr,
    sync::mpsc::Sender,
};

use serde::{Deserialize, Serialize};

use crate::{
    board::{self, BoardSize},
    game::{Game, PreviousBoards},
    glicko::Rating,
    play::{PlayRecordTimed, Plays},
    rating::Rated,
    role::Role,
    status::Status,
    time::{Time, TimeSettings},
    tree::Tree,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchivedGame {
    pub id: usize,
    pub attacker: String,
    pub attacker_rating: Rating,
    pub defender: String,
    pub defender_rating: Rating,
    pub rated: Rated,
    pub plays: Plays,
    pub status: Status,
    pub texts: VecDeque<String>,
    pub board_size: BoardSize,
}

impl ArchivedGame {
    #[must_use]
    pub fn new(game: ServerGame, attacker_rating: Rating, defender_rating: Rating) -> Self {
        Self {
            id: game.id,
            attacker: game.attacker,
            attacker_rating,
            defender: game.defender,
            defender_rating,
            rated: game.rated,
            plays: game.game.plays,
            status: game.game.status,
            texts: game.texts,
            board_size: game.game.board.size(),
        }
    }
}

impl fmt::Display for ArchivedGame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "ID: {}, Attacker: {} {}, Defender: {} {}, Size: {}",
            self.id,
            self.attacker,
            self.attacker_rating.to_string_rounded(),
            self.defender,
            self.defender_rating.to_string_rounded(),
            self.board_size,
        )
    }
}

impl PartialEq for ArchivedGame {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ArchivedGame {}

#[derive(Clone, Debug)]
pub struct ArchivedGameHandle {
    pub boards: Tree,
    pub game: ArchivedGame,
    pub play: usize,
}

impl ArchivedGameHandle {
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn new(game: &ArchivedGame) -> ArchivedGameHandle {
        let mut board = match game.board_size {
            BoardSize::_11 => board::board_11x11(),
            BoardSize::_13 => board::board_13x13(),
        };

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

#[derive(Clone, Debug)]
pub struct Messenger(Option<Sender<String>>);

impl Messenger {
    #[must_use]
    pub fn new(sender: Sender<String>) -> Self {
        Self(Some(sender))
    }

    pub fn send(&self, string: String) {
        if let Some(sender) = &self.0 {
            let _ok = sender.send(string);
        }
    }
}

#[derive(Clone, Debug)]
pub struct ServerGame {
    pub id: usize,
    pub attacker: String,
    pub attacker_tx: Messenger,
    pub defender: String,
    pub defender_tx: Messenger,
    pub rated: Rated,
    pub game: Game,
    pub texts: VecDeque<String>,
}

impl From<ServerGameSerialized> for ServerGame {
    fn from(game: ServerGameSerialized) -> Self {
        Self {
            id: game.id,
            attacker: game.attacker,
            attacker_tx: Messenger(None),
            defender: game.defender,
            defender_tx: Messenger(None),
            rated: game.rated,
            game: game.game,
            texts: game.texts,
        }
    }
}

impl ServerGame {
    #[must_use]
    pub fn protocol(&self) -> String {
        format!(
            "game {} {} {} {}",
            self.id, self.attacker, self.defender, self.rated
        )
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn new(
        attacker_tx: Sender<String>,
        defender_tx: Sender<String>,
        game: ServerGameLight,
    ) -> Self {
        let (Some(attacker), Some(defender)) = (game.attacker, game.defender) else {
            panic!("attacker and defender should be set");
        };

        let plays = match game.timed {
            TimeSettings::Timed(time) => Plays::PlayRecordsTimed(vec![PlayRecordTimed {
                play: None,
                attacker_time: time.into(),
                defender_time: time.into(),
            }]),
            TimeSettings::UnTimed => Plays::PlayRecords(vec![None]),
        };

        let board = match game.board_size {
            BoardSize::_11 => board::board_11x11(),
            BoardSize::_13 => board::board_13x13(),
        };

        Self {
            id: game.id,
            attacker,
            attacker_tx: Messenger(Some(attacker_tx)),
            defender,
            defender_tx: Messenger(Some(defender_tx)),
            rated: game.rated,
            game: Game {
                attacker_time: game.timed.clone(),
                defender_time: game.timed,
                board,
                plays,
                ..Game::default()
            },
            texts: VecDeque::new(),
        }
    }
}

impl fmt::Display for ServerGame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}, {}, {} ",
            self.id, self.attacker, self.defender, self.rated
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerGameSerialized {
    pub id: usize,
    pub attacker: String,
    pub defender: String,
    pub rated: Rated,
    pub game: Game,
    pub texts: VecDeque<String>,
}

impl From<&ServerGame> for ServerGameSerialized {
    fn from(game: &ServerGame) -> Self {
        Self {
            id: game.id,
            attacker: game.attacker.clone(),
            defender: game.defender.clone(),
            rated: game.rated,
            game: game.game.clone(),
            texts: game.texts.clone(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ServerGames(pub HashMap<usize, ServerGame>);

#[derive(Clone, Default)]
pub struct Challenger(pub Option<String>);

impl fmt::Debug for Challenger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(challenger) = &self.0 {
            write!(f, "{challenger}")?;
        } else {
            write!(f, "_")?;
        }

        Ok(())
    }
}

impl fmt::Display for Challenger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "challenger: ")?;
        if let Some(challenger) = &self.0 {
            write!(f, "{challenger}")?;
        } else {
            write!(f, "none")?;
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct ServerGameLight {
    pub id: usize,
    pub attacker: Option<String>,
    pub defender: Option<String>,
    pub challenger: Challenger,
    pub rated: Rated,
    pub timed: TimeSettings,
    pub attacker_channel: Option<usize>,
    pub defender_channel: Option<usize>,
    pub spectators: HashMap<String, usize>,
    pub challenge_accepted: bool,
    pub game_over: bool,
    pub board_size: BoardSize,
}

impl ServerGameLight {
    #[must_use]
    pub fn new(
        game_id: usize,
        username: String,
        rated: Rated,
        timed: TimeSettings,
        board_size: BoardSize,
        index_supplied: usize,
        role: Role,
    ) -> Self {
        if role == Role::Attacker {
            Self {
                id: game_id,
                attacker: Some(username),
                defender: None,
                challenger: Challenger::default(),
                rated,
                timed,
                board_size,
                attacker_channel: Some(index_supplied),
                defender_channel: None,
                spectators: HashMap::new(),
                challenge_accepted: false,
                game_over: false,
            }
        } else {
            Self {
                id: game_id,
                attacker: None,
                defender: Some(username),
                challenger: Challenger::default(),
                rated,
                timed,
                board_size,
                attacker_channel: None,
                defender_channel: Some(index_supplied),
                spectators: HashMap::new(),
                challenge_accepted: false,
                game_over: false,
            }
        }
    }
}

impl From<&ServerGame> for ServerGameLight {
    fn from(game: &ServerGame) -> Self {
        Self {
            id: game.id,
            attacker: Some(game.attacker.clone()),
            defender: Some(game.defender.clone()),
            challenger: Challenger::default(),
            rated: game.rated,
            timed: TimeSettings::UnTimed,
            board_size: game.game.board.size(),
            attacker_channel: None,
            defender_channel: None,
            spectators: HashMap::new(),
            challenge_accepted: true,
            game_over: false,
        }
    }
}

impl fmt::Debug for ServerGameLight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let attacker = if let Some(name) = &self.attacker {
            name
        } else {
            "_"
        };

        let defender = if let Some(name) = &self.defender {
            name
        } else {
            "_"
        };

        let Ok(spectators) = ron::ser::to_string(&self.spectators) else {
            panic!("we should be able to serialize the spectators")
        };

        write!(
            f,
            "game {} {attacker} {defender} {} {:?} {} {:?} {} {spectators}",
            self.id,
            self.rated,
            self.timed,
            self.board_size,
            self.challenger,
            self.challenge_accepted,
        )
    }
}

impl TryFrom<&[&str]> for ServerGameLight {
    type Error = anyhow::Error;

    fn try_from(vector: &[&str]) -> anyhow::Result<Self> {
        let id = vector[1];
        let attacker = vector[2];
        let defender = vector[3];
        let rated = vector[4];
        let timed = vector[5];
        let minutes = vector[6];
        let add_seconds = vector[7];
        let board_size = vector[8];
        let challenger = vector[9];
        let challenge_accepted = vector[10];
        let spectators = vector[11];

        let id = id.parse::<usize>()?;

        let attacker = if attacker == "_" {
            None
        } else {
            Some(attacker.to_string())
        };

        let defender = if defender == "_" {
            None
        } else {
            Some(defender.to_string())
        };

        let timed = match timed {
            "fischer" => TimeSettings::Timed(Time {
                add_seconds: add_seconds.parse::<i64>()?,
                milliseconds_left: minutes.parse::<i64>()?,
            }),
            // "un-timed"
            _ => TimeSettings::UnTimed,
        };

        let board_size = BoardSize::from_str(board_size)?;

        let Ok(challenge_accepted) = <bool as FromStr>::from_str(challenge_accepted) else {
            panic!("the value should be a bool");
        };

        let spectators =
            ron::from_str(spectators).expect("we should be able to deserialize the spectators");

        let mut game = Self {
            id,
            attacker,
            defender,
            challenger: Challenger::default(),
            rated: Rated::from_str(rated)?,
            timed,
            board_size,
            attacker_channel: None,
            defender_channel: None,
            spectators,
            challenge_accepted,
            game_over: false,
        };

        if challenger != "_" {
            game.challenger.0 = Some(challenger.to_string());
        }

        Ok(game)
    }
}

#[derive(Clone, Default)]
pub struct ServerGamesLight(pub HashMap<usize, ServerGameLight>);

impl fmt::Debug for ServerGamesLight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for game in self.0.values().filter(|game| !game.game_over) {
            write!(f, "{game:?} ")?;
        }

        Ok(())
    }
}
