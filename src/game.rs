use std::{
    borrow::Cow,
    collections::{BinaryHeap, HashMap},
    fmt,
    hash::{DefaultHasher, Hash, Hasher},
    process::exit,
    str::FromStr,
};

use chrono::Utc;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
#[cfg(feature = "js")]
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{
    ai::{AI, AiBasic},
    board::{Board, BoardSize, InvalidMove},
    characters::Characters,
    message::{COMMANDS, Message},
    play::{Captures, Plae, Play, PlayRecordTimed, Plays, Vertex},
    role::Role,
    status::Status,
    time::TimeSettings,
    tree::Tree,
};

#[cfg(not(feature = "js"))]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Game {
    pub board: Board,
    pub plays: Plays,
    pub previous_boards: PreviousBoards,
    pub status: Status,
    pub time: TimeUnix,
    pub attacker_time: TimeSettings,
    pub defender_time: TimeSettings,
    pub turn: Role,
    #[serde(skip)]
    pub chars: Characters,
}

#[cfg(feature = "js")]
#[wasm_bindgen]
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Game {
    #[wasm_bindgen(skip)]
    pub board: Board,
    #[wasm_bindgen(skip)]
    pub plays: Plays,
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
    #[wasm_bindgen(skip)]
    #[serde(skip)]
    pub chars: Characters,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PreviousBoards(pub FxHashSet<Board>);

impl PreviousBoards {
    #[must_use]
    pub fn new(board_size: BoardSize) -> Self {
        let mut boards = FxHashSet::with_capacity_and_hasher(64, FxBuildHasher);

        boards.insert(Board::new(board_size));
        Self(boards)
    }
}

impl Default for PreviousBoards {
    fn default() -> Self {
        Self::new(BoardSize::default())
    }
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let captured = self.board.captured();

        writeln!(f, "{}\n", self.board)?;
        writeln!(f, "move: {}", self.plays.len() + 1)?;
        writeln!(
            f,
            "captures: {} {}",
            &captured.attacker(&self.chars),
            &captured.defender(&self.chars)
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
    pub fn new() -> Game {
        Game::default()
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
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn alpha_beta(
        &self,
        original_depth: usize,
        depth: u8,
        mut play_option: Option<Plae>,
        mut alpha: f64,
        mut beta: f64,
    ) -> (Option<Plae>, f64, Option<EscapeVec>) {
        if let Some(result) = self.alpha_beta_duplicated(original_depth, depth) {
            return result;
        }

        if self.turn == Role::Attacker {
            let mut value = -f64::INFINITY;
            let mut escape_vec = None;
            for plae in self.all_legal_plays() {
                let mut child = self.clone();
                child.play(&plae).expect("this play should be valid");
                let (play_option_2, value_2, escape_vec_2) =
                    child.alpha_beta(original_depth, depth - 1, Some(plae.clone()), alpha, beta);

                if value_2 > value {
                    value = value_2;
                    play_option.clone_from(&play_option_2);
                    escape_vec.clone_from(&escape_vec_2);
                }

                if value >= beta {
                    break;
                }

                if value > alpha {
                    alpha = value;
                    play_option = play_option_2;
                    escape_vec = escape_vec_2;
                }
            }

            (play_option, value, escape_vec)
        } else {
            let mut value = f64::INFINITY;
            let mut escape_vec = None;
            for plae in self.all_legal_plays() {
                let mut child = self.clone();
                child.play(&plae).expect("this play should be valid");
                let (play_option_2, value_2, escape_vec_2) =
                    child.alpha_beta(original_depth, depth - 1, Some(plae.clone()), alpha, beta);

                if value_2 < value {
                    value = value_2;
                    play_option.clone_from(&play_option_2);
                    escape_vec.clone_from(&escape_vec_2);
                }

                if value <= alpha {
                    break;
                }

                if value < beta {
                    beta = value;
                    play_option = play_option_2;
                    escape_vec = escape_vec_2;
                }
            }

            (play_option, value, escape_vec)
        }
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn alpha_beta_parallel(
        &self,
        original_depth: usize,
        depth: u8,
        mut play_option: Option<Plae>,
        mut alpha: f64,
        mut beta: f64,
    ) -> (Option<Plae>, f64, Option<EscapeVec>) {
        if let Some(result) = self.alpha_beta_duplicated(original_depth, depth) {
            return result;
        }

        if self.turn == Role::Attacker {
            let mut value = -f64::INFINITY;
            let mut escape_vec = None;
            let results: Vec<_> = self
                .all_legal_plays()
                .par_iter()
                .map(|plae| {
                    let mut child = self.clone();
                    child.play(plae).expect("this play should be valid");
                    child.alpha_beta(original_depth, depth - 1, Some(plae.clone()), alpha, beta)
                })
                .collect();

            for (play_option_2, value_2, escape_vec_2) in results {
                if value_2 > value {
                    value = value_2;
                    play_option.clone_from(&play_option_2);
                    escape_vec.clone_from(&escape_vec_2);
                }

                if value >= beta {
                    break;
                }

                if value > alpha {
                    alpha = value;
                    play_option = play_option_2;
                    escape_vec = escape_vec_2;
                }
            }

            (play_option, value, escape_vec)
        } else {
            let mut value = f64::INFINITY;
            let mut escape_vec = None;

            let results: Vec<_> = self
                .all_legal_plays()
                .par_iter()
                .map(|plae| {
                    let mut child = self.clone();
                    child.play(plae).expect("this play should be valid");
                    child.alpha_beta(original_depth, depth - 1, Some(plae.clone()), alpha, beta)
                })
                .collect();

            for (play_option_2, value_2, escape_vec_2) in results {
                if value_2 < value {
                    value = value_2;
                    play_option.clone_from(&play_option_2);
                    escape_vec.clone_from(&escape_vec_2);
                }

                if value <= alpha {
                    break;
                }

                if value < beta {
                    beta = value;
                    play_option = play_option_2;
                    escape_vec = escape_vec_2;
                }
            }

            (play_option, value, escape_vec)
        }
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn alpha_beta_duplicated(
        &self,
        original_depth: usize,
        depth: u8,
    ) -> Option<(Option<Plae>, f64, Option<EscapeVec>)> {
        match self.status {
            Status::AttackerWins => {
                return Some((
                    self.play_n(self.plays.len() - original_depth + usize::from(depth) - 1),
                    f64::INFINITY,
                    None,
                ));
            }
            Status::DefenderWins => {
                return Some((
                    self.play_n(self.plays.len() - original_depth + usize::from(depth) - 1),
                    -f64::INFINITY,
                    None,
                ));
            }
            Status::Draw => unreachable!(),
            Status::Ongoing => {}
        }

        if depth == 0 {
            let (utility, escape_vec) = self.utility();

            return Some((
                self.play_n(self.plays.len() - original_depth),
                utility,
                Some(escape_vec),
            ));
        }

        None
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn play_n(&self, n: usize) -> Option<Plae> {
        match &self.plays {
            Plays::PlayRecordsTimed(plaes_timed) => plaes_timed
                .iter()
                .map(|plae_timed| plae_timed.play.clone().unwrap())
                .nth(n),
            Plays::PlayRecords(plaes) => plaes.iter().map(|plae| plae.clone().unwrap()).nth(n),
        }
    }

    #[must_use]
    pub fn new_game(board_size: BoardSize, time_settings: Option<TimeSettings>) -> Self {
        let board = Board::new(board_size);
        let previous_boards = PreviousBoards::new(board_size);

        if let Some(time_settings) = time_settings {
            let mut game = Self {
                board,
                plays: Plays::new(&time_settings),
                previous_boards,
                ..Self::default()
            };

            match time_settings {
                TimeSettings::Timed(time) => {
                    game.attacker_time = TimeSettings::Timed(time);
                    game.defender_time = TimeSettings::Timed(time);
                }
                TimeSettings::UnTimed => {
                    game.attacker_time = TimeSettings::UnTimed;
                    game.defender_time = TimeSettings::UnTimed;
                }
            }

            game
        } else {
            Self {
                board,
                previous_boards,
                ..Self::default()
            }
        }
    }

    #[must_use]
    pub fn all_legal_moves(&self) -> LegalMoves {
        let size = self.board.size();
        let board_size_usize = size.into();
        let vec_capacity = match size {
            BoardSize::_11 => 20,
            BoardSize::_13 => 24,
        };

        let mut possible_vertexes = Vec::new();
        let mut legal_moves = LegalMoves {
            role: self.turn,
            moves: FxHashMap::default(),
        };

        for y in 0..board_size_usize {
            for x in 0..board_size_usize {
                let vertex = Vertex { size, x, y };
                if Role::from(self.board.get(&vertex)) == legal_moves.role {
                    possible_vertexes.push(vertex);
                }
            }
        }

        for vertex_from in possible_vertexes {
            let mut vertexes_to = Vec::with_capacity(vec_capacity);

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
    pub fn kings_legal_moves(&self) -> (Vertex, Vec<Vertex>) {
        let size = self.board.size();
        let board_size_usize = size.into();
        let kings_position = self
            .board
            .find_the_king()
            .expect("The king must still be on the board.");

        let mut vertexes_to = Vec::new();
        for y in 0..board_size_usize {
            let vertex_to = Vertex {
                size,
                x: kings_position.x,
                y,
            };
            let play = Play {
                role: self.turn,
                from: kings_position,
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
                y: kings_position.y,
            };
            let play = Play {
                role: self.turn,
                from: kings_position,
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

        (kings_position, vertexes_to)
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

    #[must_use]
    pub fn moves_to_escape(&self) -> (MovesToEscape, EscapeVec) {
        let Some(start) = self.board.find_the_king() else {
            return (MovesToEscape::GameOver, EscapeVec::new(self.board.size()));
        };

        let mut priority_queue = BinaryHeap::new();
        priority_queue.push((None, vec![start]));

        let mut escape_vec = EscapeVec::new(self.board.size());
        escape_vec.set(&start, 0);

        let mut visited = HashMap::new();
        visited.insert(start, (0, None));

        while let Some((current_cost, current_nodes)) = priority_queue.pop() {
            let neighbors = self.board.get_neighbors(&current_nodes, &visited);

            let cost = if let Some(neighbor) = neighbors.first() {
                escape_vec.get(neighbor)
            } else {
                continue;
            };
            let cost = cost.unwrap_or_default();

            let total_cost = if let Some(Some(current_cost)) = current_cost {
                current_cost + cost
            } else {
                cost
            };

            for neighbor in &neighbors {
                if !visited.contains_key(neighbor) || total_cost < visited[neighbor].0 {
                    let mut moves = escape_vec.get(&current_nodes[0]).unwrap_or_default();
                    moves += 1;
                    escape_vec.set(neighbor, moves);

                    if self.board.exit_squares().contains(neighbor) {
                        return (MovesToEscape::Moves(moves), escape_vec);
                    }

                    for current_node in &current_nodes {
                        visited.insert(*neighbor, (total_cost, Some(*current_node)));
                    }
                }
            }

            priority_queue.push((Some(Some(total_cost)), neighbors));
        }

        (MovesToEscape::CanNotEscape, escape_vec)
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
                let (kings_position, move_to) = self.kings_legal_moves();

                for play in move_to {
                    if self.board.on_exit_square(&play) {
                        return Some(Plae::Play(Play {
                            role: Role::Defender,
                            from: kings_position,
                            to: play,
                        }));
                    }
                }
            }
            Role::Roleless => unreachable!(),
        }

        None
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
                    let piece_role = Role::from(self.board.get(&play.from));
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
                            self.status = match self.turn {
                                Role::Attacker => Status::DefenderWins,
                                Role::Roleless => unreachable!(),
                                Role::Defender => Status::AttackerWins,
                            }
                        }
                    }

                    let captures = Captures(captures);
                    Ok(captures)
                }
            }
        } else {
            Err(InvalidMove::GameOver.into())
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
    #[allow(clippy::too_many_lines)]
    pub fn update(&mut self, message: Message) -> anyhow::Result<Option<String>> {
        match message {
            Message::BoardSize(size) => {
                let board_size = BoardSize::try_from(size)?;
                *self = Self::new_game(board_size, None);
                Ok(Some(String::new()))
            }
            Message::Empty => Ok(None),
            Message::FinalStatus => Ok(Some(format!("{}", self.status))),
            Message::GenerateMove => {
                let mut ai = AiBasic::new(4, true);
                let generate_move = ai.generate_move(self)?;
                Ok(Some(generate_move.to_string()))
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
                *self = Self::new_game(self.board.size(), Some(time_settings));
                Ok(Some(String::new()))
            }
            Message::Version => {
                let version = env!("CARGO_PKG_VERSION");
                Ok(Some(version.to_string()))
            }
        }
    }

    #[must_use]
    pub fn utility(&self) -> (f64, EscapeVec) {
        let mut utility = 0.0;

        let captured = self.board.captured();
        utility -= f64::from(captured.attacker) * 100_000.0;

        let (moves_to_escape, escape_vec) = self.moves_to_escape();
        utility += match moves_to_escape {
            MovesToEscape::CanNotEscape => 20_000.0,
            MovesToEscape::GameOver => 0.0,
            MovesToEscape::Moves(moves) => f64::from(moves) * 1_000.0,
        };

        utility += f64::from(self.board.closed_off_exits()) * 100.0;
        // Todo: An extra 100.0 points for each corner that touches another corner.
        utility += f64::from(captured.defender) * 10.0;
        utility -= f64::from(self.board.spaces_around_the_king());

        (utility, escape_vec)
    }
}

#[derive(Clone, Debug)]
pub struct EscapeVec {
    spaces: Vec<Option<u8>>,
}

impl EscapeVec {
    fn new(board_size: BoardSize) -> Self {
        let size: usize = board_size.into();

        EscapeVec {
            spaces: vec![None; size * size],
        }
    }

    fn get(&self, vertex: &Vertex) -> Option<u8> {
        self.spaces[vertex.y * usize::from(vertex.size) + vertex.x]
    }

    fn set(&mut self, vertex: &Vertex, moves: u8) {
        self.spaces[vertex.y * usize::from(vertex.size) + vertex.x] = Some(moves);
    }
}

impl fmt::Display for EscapeVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let board_size = if self.spaces.len() == 11 * 11 { 11 } else { 13 };

        match board_size {
            11 => writeln!(f, "   A  B  C  D  E  F  G  H  I  J  K")?,
            13 => writeln!(f, "   A  B  C  D  E  F  G  H  I  J  K  L  M")?,
            _ => unreachable!(),
        }

        for y in 0..board_size {
            match board_size {
                11 => write!(f, "{:2} ", 11 - y)?,
                13 => write!(f, "{:2} ", 13 - y)?,
                _ => unreachable!(),
            }

            for x in 0..board_size {
                let moves = self.spaces[y * board_size + x];
                if let Some(moves) = moves {
                    write!(f, "{moves:02} ")?;
                } else {
                    write!(f, "-- ")?;
                }
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum MovesToEscape {
    CanNotEscape,
    GameOver,
    Moves(u8),
}

impl From<&Tree> for Game {
    fn from(tree: &Tree) -> Self {
        let node = tree.here();
        let (plays, previous_boards) = tree.previous_boards();

        Self {
            board: node.board,
            plays,
            previous_boards,
            status: Status::Ongoing,
            turn: node.turn,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegalMoves {
    pub role: Role,
    pub moves: FxHashMap<Vertex, Vec<Vertex>>,
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
