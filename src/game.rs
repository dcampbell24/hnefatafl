use std::{
    borrow::Cow,
    collections::HashMap,
    fmt,
    hash::{DefaultHasher, Hash, Hasher},
    process::exit,
    str::FromStr,
};

use chrono::Utc;
use rustc_hash::{FxBuildHasher, FxHashSet};
use serde::{Deserialize, Serialize};
#[cfg(feature = "js")]
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{
    ai::{AI, AiBanal},
    board::{Board, BoardSize, PreviousBoard},
    message::{COMMANDS, Message},
    play::{Captures, Plae, Play, PlayRecordTimed, Plays, Vertex},
    role::Role,
    space::Space,
    status::Status,
    time::TimeSettings,
    tree::Node,
};

#[cfg(not(feature = "js"))]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Game {
    #[serde(skip)]
    pub ai: AiBanal,
    pub board: Board,
    pub plays: Plays,
    pub previous_board: PreviousBoard,
    pub previous_boards: PreviousBoards,
    pub status: Status,
    pub time: TimeUnix,
    pub attacker_time: TimeSettings,
    pub defender_time: TimeSettings,
    pub turn: Role,
}

#[cfg(feature = "js")]
#[wasm_bindgen]
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Game {
    #[serde(skip)]
    #[wasm_bindgen(skip)]
    pub ai: AiBanal,
    #[wasm_bindgen(skip)]
    pub board: Board,
    #[wasm_bindgen(skip)]
    pub plays: Plays,
    #[wasm_bindgen(skip)]
    pub previous_board: PreviousBoard,
    #[wasm_bindgen(skip)]
    pub previous_boards: PreviousBoards,
    #[wasm_bindgen(skip)]
    pub status: Status,
    #[wasm_bindgen(skip)]
    pub time: TimeUnix,
    #[wasm_bindgen(skip)]
    pub attacker_time: TimeSettings,
    #[wasm_bindgen(skip)]
    pub defender_time: TimeSettings,
    #[wasm_bindgen(skip)]
    pub turn: Role,
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let captured = self.board.captured();

        writeln!(f, "{}\n", self.board)?;
        writeln!(f, "move: {}", self.plays.len() + 1)?;
        writeln!(
            f,
            "captures: {} {}",
            &captured.attacker(),
            &captured.defender()
        )?;
        writeln!(f, "status: {}", self.status)?;
        writeln!(f, "turn: {}", self.turn)?;

        match &self.attacker_time {
            TimeSettings::Timed(time) => writeln!(f, "attacker time: {time}")?,
            TimeSettings::UnTimed => writeln!(f, "attacker time: infinite")?,
        }
        match &self.defender_time {
            TimeSettings::Timed(time) => writeln!(f, "defender time: {time}")?,
            TimeSettings::UnTimed => writeln!(f, "defender time: infinite")?,
        }

        write!(f, "plays: {}", self.plays)
    }
}

#[cfg(feature = "js")]
#[wasm_bindgen]
impl Game {
    #[must_use]
    #[wasm_bindgen(constructor)]
    pub fn new(board_size: BoardSize) -> Game {
        Game {
            board: Board::new(board_size),
            previous_board: PreviousBoard::new(board_size),
            previous_boards: PreviousBoards::new(board_size),
            ..Game::default()
        }
    }

    /// # Errors
    ///
    /// If the command is illegal or invalid.
    #[wasm_bindgen]
    pub fn read_line_js(&mut self, buffer: &str) -> String {
        let mut buffer = Cow::from(buffer);
        if let Some(comment_offset) = buffer.find('#') {
            buffer.to_mut().replace_range(comment_offset.., "");
        }

        match Message::from_str(buffer.as_ref()) {
            Ok(message) => match self.update(message) {
                Ok(update) => {
                    if let Some(update) = update {
                        format!("= {update}")
                    } else {
                        String::new()
                    }
                }
                Err(err) => format!("? {err}"),
            },
            Err(err) => format!("? {err}"),
        }
    }
}

impl Game {
    #[must_use]
    pub fn all_legal_moves(&self) -> LegalMoves {
        let size = self.board.size();
        let board_size_usize = size.into();

        let mut possible_vertexes = Vec::new();
        let mut legal_moves = LegalMoves {
            role: self.turn,
            moves: HashMap::new(),
        };

        for y in 0..board_size_usize {
            for x in 0..board_size_usize {
                let vertex = Vertex { size, x, y };
                if self.board.get(&vertex).role() == legal_moves.role {
                    possible_vertexes.push(vertex);
                }
            }
        }

        for vertex_from in possible_vertexes {
            let mut vertexes_to = Vec::new();

            for y in 0..board_size_usize {
                let vertex_to = Vertex {
                    size,
                    x: vertex_from.x,
                    y,
                };
                let play = Play {
                    role: self.turn,
                    from: vertex_from,
                    to: vertex_to,
                };

                if self
                    .board
                    .legal_move(&play, &self.status, &self.turn, &self.previous_boards)
                    .is_ok()
                {
                    vertexes_to.push(vertex_to);
                }
            }

            for x in 0..board_size_usize {
                let vertex_to = Vertex {
                    size,
                    x,
                    y: vertex_from.y,
                };
                let play = Play {
                    role: self.turn,
                    from: vertex_from,
                    to: vertex_to,
                };

                if self
                    .board
                    .legal_move(&play, &self.status, &self.turn, &self.previous_boards)
                    .is_ok()
                {
                    vertexes_to.push(vertex_to);
                }
            }

            if !vertexes_to.is_empty() {
                legal_moves.moves.insert(vertex_from, vertexes_to);
            }
        }

        legal_moves
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn all_legal_plays(&self) -> Vec<Plae> {
        let moves = self.all_legal_moves();

        if moves.moves.is_empty() && self.status == Status::Ongoing {
            match &moves.role {
                Role::Attacker => return vec![Plae::AttackerResigns],
                Role::Defender => return vec![Plae::DefenderResigns],
                Role::Roleless => return Vec::new(),
            }
        }

        let mut plays = Vec::new();
        for (from, tos) in moves.moves {
            for to in tos {
                plays.push(Plae::Play(Play {
                    role: moves.role,
                    from,
                    to,
                }));
            }
        }

        plays
    }

    #[must_use]
    pub fn calculate_hash(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.plays.hash(&mut s);
        s.finish()
    }

    #[must_use]
    pub fn exit_one(&self) -> bool {
        let size = self.board.size();
        let board_size_usize: usize = size.into();

        let exit_1 = Vertex { size, x: 0, y: 0 };
        let exit_2 = Vertex {
            size,
            x: board_size_usize - 1,
            y: 0,
        };
        let exit_3 = Vertex {
            size,
            x: 0,
            y: board_size_usize - 1,
        };
        let exit_4 = Vertex {
            size,
            x: board_size_usize - 1,
            y: board_size_usize - 1,
        };

        let mut game = self.clone();
        if let Some(king) = self.board.find_the_king() {
            for exit in [exit_1, exit_2, exit_3, exit_4] {
                if game
                    .play(&Plae::Play(Play {
                        role: self.turn,
                        from: king,
                        to: exit,
                    }))
                    .is_ok()
                {
                    return true;
                }
            }
        }

        false
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn obvious_play(&self) -> Option<Plae> {
        match self.turn {
            Role::Attacker => {
                if let Some(vertex) = self.board.capture_the_king_one_move() {
                    for plae in self.all_legal_plays() {
                        if let Plae::Play(ref play) = plae
                            && play.to == vertex
                        {
                            return Some(plae);
                        }
                    }
                }
            }
            Role::Defender => {
                let kings_position = self
                    .board
                    .find_the_king()
                    .expect("The king must still be on the board.");

                if let Some(plays) = self.all_legal_moves().moves.get(&kings_position) {
                    for play in plays {
                        if self.board.on_exit_square(play) {
                            return Some(Plae::Play(Play {
                                role: Role::Defender,
                                from: kings_position,
                                to: *play,
                            }));
                        }
                    }
                }
            }
            Role::Roleless => unreachable!(),
        }

        None
    }

    #[cfg(not(feature = "js"))]
    #[must_use]
    pub fn new(board_size: BoardSize) -> Game {
        Game {
            board: Board::new(board_size),
            previous_board: PreviousBoard::new(board_size),
            previous_boards: PreviousBoards::new(board_size),
            ..Game::default()
        }
    }

    /// # Errors
    ///
    /// If the game is already over or the move is illegal.
    #[allow(clippy::too_many_lines)]
    pub fn play(&mut self, play: &Plae) -> anyhow::Result<Captures> {
        if self.status == Status::Ongoing {
            if let (status, TimeSettings::Timed(timer), TimeUnix::Time(time)) = match self.turn {
                Role::Attacker => (
                    Status::DefenderWins,
                    &mut self.attacker_time,
                    &mut self.time,
                ),
                Role::Roleless => {
                    unreachable!("It can't be no one's turn when the game is ongoing!")
                }
                Role::Defender => (
                    Status::AttackerWins,
                    &mut self.defender_time,
                    &mut self.time,
                ),
            } {
                let now = Utc::now().timestamp_millis();
                timer.milliseconds_left -= now - *time;
                *time = now;

                if timer.milliseconds_left <= 0 {
                    self.status = status;
                    return Ok(Captures::default());
                }

                timer.milliseconds_left += timer.add_seconds * 1_000;
            }

            match play {
                Plae::AttackerResigns => {
                    if self.turn == Role::Attacker {
                        self.status = Status::DefenderWins;

                        match &mut self.plays {
                            Plays::PlayRecordsTimed(plays) => {
                                plays.push(PlayRecordTimed {
                                    play: Some(play.clone()),
                                    attacker_time: self.attacker_time.clone().try_into()?,
                                    defender_time: self.defender_time.clone().try_into()?,
                                });
                            }
                            Plays::PlayRecords(plays) => plays.push(Some(play.clone())),
                        }

                        Ok(Captures::default())
                    } else {
                        Err(anyhow::Error::msg("You can't resign for the other player."))
                    }
                }
                Plae::DefenderResigns => {
                    if self.turn == Role::Defender {
                        self.status = Status::AttackerWins;

                        match &mut self.plays {
                            Plays::PlayRecordsTimed(plays) => {
                                plays.push(PlayRecordTimed {
                                    play: Some(play.clone()),
                                    attacker_time: self.attacker_time.clone().try_into()?,
                                    defender_time: self.defender_time.clone().try_into()?,
                                });
                            }
                            Plays::PlayRecords(plays) => plays.push(Some(play.clone())),
                        }

                        Ok(Captures::default())
                    } else {
                        Err(anyhow::Error::msg("You can't resign for the other player."))
                    }
                }
                Plae::Play(play) => {
                    let piece_role = self.board.get(&play.from).role();
                    if piece_role != play.role {
                        return Err(anyhow::Error::msg(format!(
                            "play: you are trying to move {piece_role}, but it's {}'s turn",
                            play.role
                        )));
                    }

                    let (captures, status) = self.board.play(
                        &Plae::Play(play.clone()),
                        &self.status,
                        &self.turn,
                        &mut self.previous_boards,
                    )?;

                    self.status = status;

                    match &mut self.plays {
                        Plays::PlayRecordsTimed(plays) => {
                            plays.push(PlayRecordTimed {
                                play: Some(Plae::Play(play.clone())),
                                attacker_time: self.attacker_time.clone().try_into()?,
                                defender_time: self.defender_time.clone().try_into()?,
                            });
                        }
                        Plays::PlayRecords(plays) => plays.push(Some(Plae::Play(play.clone()))),
                    }

                    if self.status == Status::Ongoing {
                        self.turn = self.turn.opposite();

                        if !self.board.a_legal_move_exists(
                            &self.status,
                            &self.turn,
                            &self.previous_boards,
                        ) {
                            match self.turn {
                                Role::Attacker => self.status = Status::DefenderWins,
                                Role::Roleless => {}
                                Role::Defender => self.status = Status::AttackerWins,
                            }
                        }
                    }

                    let captures = Captures(captures);
                    Ok(captures)
                }
            }
        } else {
            Err(anyhow::Error::msg("play: the game is already over"))
        }
    }

    /// # Errors
    ///
    /// If the command is illegal or invalid.
    pub fn read_line(&mut self, buffer: &str) -> anyhow::Result<Option<String>> {
        let mut buffer = Cow::from(buffer);
        if let Some(comment_offset) = buffer.find('#') {
            buffer.to_mut().replace_range(comment_offset.., "");
        }

        self.update(Message::from_str(buffer.as_ref())?)
    }

    /// # Errors
    ///
    /// If the command is illegal or invalid.
    #[allow(clippy::missing_panics_doc)]
    #[allow(clippy::too_many_lines)]
    pub fn update(&mut self, message: Message) -> anyhow::Result<Option<String>> {
        // Fixme: use monte carlo?
        let mut ai: Box<dyn AI> = Box::new(AiBanal);

        match message {
            Message::BoardSize(board_size) => {
                let board_size = BoardSize::try_from(board_size)?;
                *self = Game::new(board_size);

                Ok(Some(String::new()))
            }
            Message::Empty => Ok(None),
            Message::FinalStatus => Ok(Some(format!("{}", self.status))),
            Message::GenerateMove => {
                let generate_move = ai.generate_move(self);
                if let Some(play) = generate_move.play {
                    Ok(Some(format!(
                        "{play}, score: {}, delay milliseconds: {}",
                        generate_move.score, generate_move.delay_milliseconds
                    )))
                } else {
                    Err(anyhow::Error::msg("failed to generate move"))
                }
            }
            Message::KnownCommand(command) => {
                if COMMANDS.contains(&command.as_str()) {
                    Ok(Some("true".to_string()))
                } else {
                    Ok(Some("false".to_string()))
                }
            }
            Message::ListCommands => {
                let mut commands = "\n".to_string();
                commands.push_str(&COMMANDS.join("\n"));
                Ok(Some(commands))
            }
            Message::Name => {
                let name = env!("CARGO_PKG_NAME");
                Ok(Some(name.to_string()))
            }
            Message::Play(play) => self.play(&play).map(|captures| Some(captures.to_string())),
            Message::PlayFrom => {
                let moves = self.all_legal_moves();
                Ok(Some(format!(
                    "{} {}",
                    moves.role,
                    moves
                        .moves
                        .keys()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" ")
                )))
            }
            Message::PlayTo(from) => {
                let (role, vertex) = from;
                let moves = self.all_legal_moves();
                if role != moves.role {
                    return Err(anyhow::Error::msg(format!(
                        "tried play_to {role}, but it's {} turn",
                        moves.role
                    )));
                }

                if let Some(moves) = moves.moves.get(&vertex) {
                    Ok(Some(
                        moves
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(" "),
                    ))
                } else {
                    Err(anyhow::Error::msg("invalid from vertex"))
                }
            }
            Message::ProtocolVersion => Ok(Some("1-beta".to_string())),
            Message::Quit => exit(0),
            Message::ShowBoard => Ok(Some(self.board.to_string())),
            Message::TimeSettings(time_settings) => {
                *self = Game::new(self.board.size());

                self.plays = Plays::new(&time_settings);
                match time_settings {
                    TimeSettings::Timed(time) => {
                        self.attacker_time = TimeSettings::Timed(time);
                        self.defender_time = TimeSettings::Timed(time);
                        self.time = TimeUnix::default();
                    }
                    TimeSettings::UnTimed => {
                        self.attacker_time = TimeSettings::UnTimed;
                        self.defender_time = TimeSettings::UnTimed;
                        self.time = TimeUnix::UnTimed;
                    }
                }

                Ok(Some(String::new()))
            }
            Message::Version => {
                let version = env!("CARGO_PKG_VERSION");
                Ok(Some(version.to_string()))
            }
        }
    }

    #[must_use]
    pub fn utility(&self) -> i32 {
        match self.status {
            Status::Ongoing => {}
            Status::AttackerWins => return i32::MIN,
            Status::Draw => return 0,
            Status::DefenderWins => return i32::MAX,
        }

        let mut utility = 0;

        let mut defender_left = 0;
        let mut attacker_left = 0;
        for space in &self.board.spaces {
            match space {
                Space::Attacker => attacker_left += 1,
                Space::Empty | Space::King => {}
                Space::Defender => defender_left += 1,
            }
        }

        utility += defender_left * 2;
        utility -= attacker_left;

        if self.exit_one() {
            utility += 100;
        }

        utility
    }
}

impl From<Node> for Game {
    fn from(node: Node) -> Self {
        Self {
            board: node.board,
            status: Status::Ongoing,
            turn: node.turn,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegalMoves {
    pub role: Role,
    pub moves: HashMap<Vertex, Vec<Vertex>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PreviousBoards(pub FxHashSet<PreviousBoard>);

impl PreviousBoards {
    fn new(board_size: BoardSize) -> Self {
        let hasher = FxBuildHasher;
        // Fixme?
        let mut boards = FxHashSet::with_capacity_and_hasher(128, hasher);

        boards.insert(PreviousBoard::new(board_size));
        Self(boards)
    }
}

impl Default for PreviousBoards {
    fn default() -> Self {
        PreviousBoards::new(BoardSize::_11)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum TimeUnix {
    Time(i64),
    UnTimed,
}

impl Default for TimeUnix {
    fn default() -> Self {
        Self::Time(Utc::now().timestamp_millis())
    }
}
