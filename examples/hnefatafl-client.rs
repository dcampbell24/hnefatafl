// Don't open the terminal on Windows.
#![windows_subsystem = "windows"]

#[macro_use]
extern crate rust_i18n;

use std::io::Cursor;
use std::io::Read;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    env, f64,
    fmt::{self, Write as fmt_write},
    fs::{self, File},
    io::{BufRead, BufReader, ErrorKind, Write},
    net::{Shutdown, TcpStream},
    path::PathBuf,
    process::exit,
    str::{FromStr, SplitAsciiWhitespace},
    sync::mpsc,
    thread,
};

use chrono::{Local, Utc};
use clap::{CommandFactory, Parser, command};
use env_logger::Builder;
use futures::{SinkExt, executor};
use hnefatafl_copenhagen::board::BoardSize;
use hnefatafl_copenhagen::play::Plays;
use hnefatafl_copenhagen::server_game::{ArchivedGame, ArchivedGameHandle};
use hnefatafl_copenhagen::{
    COPYRIGHT, VERSION_ID,
    accounts::Email,
    draw::Draw,
    game::{Game, TimeUnix},
    glicko::{CONFIDENCE_INTERVAL_95, Rating},
    play::{BOARD_LETTERS, Vertex},
    rating::Rated,
    role::Role,
    server_game::{ServerGameLight, ServerGamesLight},
    space::Space,
    status::Status,
    time::{Time, TimeSettings},
};
use hnefatafl_copenhagen::{LONG_VERSION, board};
use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::{focus_next, focus_previous};
#[cfg(target_os = "linux")]
use iced::window::settings::PlatformSpecific;
use iced::{
    Element, Event, Pixels, Subscription,
    alignment::{Horizontal, Vertical},
    event,
    font::Font,
    futures::Stream,
    stream,
    widget::{
        Column, Container, Row, Scrollable, button, checkbox, column, container, pick_list, radio,
        row, scrollable, text, text_editor, text_input, tooltip,
    },
    window::{self, icon},
};
use iced::{Task, keyboard};
use log::{LevelFilter, debug, error, info, trace};
use rust_i18n::t;
use serde::{Deserialize, Serialize};

const PORT: &str = ":49152";
const PADDING: u16 = 10;
const SPACING: Pixels = Pixels(10.0);
const SPACING_B: Pixels = Pixels(20.0);

i18n!();

/// Hnefatafl Copenhagen Client
///
/// This is a TCP client that connects to a server.
#[derive(Parser, Debug)]
#[command(long_version = LONG_VERSION, about = "Copenhagen Hnefatafl Client")]
struct Args {
    /// Connect to the server at host
    #[arg(default_value = "hnefatafl.org", long)]
    host: String,

    /// Make the window size tiny
    #[arg(long)]
    tiny_window: bool,

    /// Build the manpage
    #[arg(long)]
    man: bool,
}

fn i18n_buttons() -> HashMap<String, String> {
    let mut strings = HashMap::new();

    strings.insert("Login".to_string(), t!("Login").to_string());

    strings.insert(
        "Create Account".to_string(),
        t!("Create Account").to_string(),
    );

    strings.insert(
        "Reset Password".to_string(),
        t!("Reset Password").to_string(),
    );

    strings.insert("Leave".to_string(), t!("Leave").to_string());
    strings.insert("Quit".to_string(), t!("Quit").to_string());
    strings.insert("Dark".to_string(), t!("Dark").to_string());
    strings.insert("Light".to_string(), t!("Light").to_string());
    strings.insert("Create Game".to_string(), t!("Create Game").to_string());
    strings.insert("Users".to_string(), t!("Users").to_string());

    strings.insert(
        "Account Settings".to_string(),
        t!("Account Settings").to_string(),
    );

    strings.insert("Rules".to_string(), t!("Rules").to_string());
    strings.insert("Reset Email".to_string(), t!("Reset Email").to_string());

    strings.insert(
        "Change Password".to_string(),
        t!("Change Password").to_string(),
    );

    strings.insert(
        "Delete Account".to_string(),
        t!("Delete Account").to_string(),
    );

    strings.insert(
        "REALLY DELETE ACCOUNT".to_string(),
        t!("REALLY DELETE ACCOUNT").to_string(),
    );

    strings.insert("New Game".to_string(), t!("New Game").to_string());
    strings.insert("Accept".to_string(), t!("Accept").to_string());
    strings.insert("Decline".to_string(), t!("Decline").to_string());
    strings.insert("Watch".to_string(), t!("Watch").to_string());
    strings.insert("Join".to_string(), t!("Join").to_string());
    strings.insert("Resume".to_string(), t!("Resume").to_string());

    strings.insert("Resign".to_string(), t!("Resign").to_string());
    strings.insert("Request Draw".to_string(), t!("Request Draw").to_string());
    strings.insert("Accept Draw".to_string(), t!("Accept Draw").to_string());
    strings.insert("Review Game".to_string(), t!("Review Game").to_string());
    strings.insert(
        "Get Archived Games".to_string(),
        t!("Get Archived Games").to_string(),
    );

    strings
}

fn init_client() -> Client {
    let data_file = data_file();
    let mut error = None;
    let mut client: Client = match &fs::read_to_string(&data_file) {
        Ok(string) => match ron::from_str(string.as_str()) {
            Ok(client) => client,
            Err(err) => {
                error = Some(format!(
                    "error parsing the ron file {}: {err}",
                    data_file.display()
                ));
                Client::default()
            }
        },
        Err(err) => {
            if err.kind() == ErrorKind::NotFound {
                Client::default()
            } else {
                error = Some(format!(
                    "error opening the file {}: {err}",
                    data_file.display()
                ));
                Client::default()
            }
        }
    };

    if error.is_some() {
        client.error_persistent = error;
    }

    rust_i18n::set_locale(&client.locale_selected.txt());

    client.strings = i18n_buttons();
    client.text_input = client.username.clone();

    client
}

fn main() -> anyhow::Result<()> {
    init_logger();
    let args = Args::parse();

    if args.man {
        let mut buffer: Vec<u8> = Vec::default();
        let cmd = Args::command().name("hnefatafl-client").long_version(None);
        let man = clap_mangen::Man::new(cmd).date("2025-06-23");

        man.render(&mut buffer)?;
        write!(buffer, "{COPYRIGHT}")?;

        std::fs::write("hnefatafl-client.1", buffer)?;
        return Ok(());
    }

    #[cfg(not(feature = "icon_2"))]
    let king = include_bytes!("king_1_256x256.rgba").to_vec();

    #[cfg(feature = "icon_2")]
    let king = include_bytes!("king_2_256x256.rgba").to_vec();

    let mut application = iced::application(init_client, Client::update, Client::view)
        .title("Hnefatafl Copenhagen")
        .subscription(Client::subscriptions)
        .window(window::Settings {
            #[cfg(target_os = "linux")]
            platform_specific: PlatformSpecific {
                #[cfg(feature = "icon_2")]
                application_id: "org.hnefatafl.hnefatafl_client".to_string(),
                #[cfg(not(feature = "icon_2"))]
                application_id: "hnefatafl-client".to_string(),
                ..PlatformSpecific::default()
            },
            icon: Some(icon::from_rgba(king, 256, 256)?),
            ..window::Settings::default()
        })
        .theme(Client::theme)
        .default_font(Font::MONOSPACE);

    // For screenshots.
    if args.tiny_window {
        application = application.window_size(iced::Size {
            width: 870.0,
            height: 541.0,
        });
    }

    application.run()?;
    Ok(())
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, Deserialize, Serialize)]
struct Client {
    #[serde(skip)]
    attacker: String,
    #[serde(skip)]
    archived_games: Vec<ArchivedGame>,
    #[serde(skip)]
    archived_games_filtered: Vec<ArchivedGame>,
    #[serde(skip)]
    archived_game_selected: Option<ArchivedGame>,
    #[serde(skip)]
    archived_game_handle: Option<ArchivedGameHandle>,
    #[serde(skip)]
    defender: String,
    #[serde(skip)]
    delete_account: bool,
    #[serde(default)]
    email_everyone: bool,
    #[serde(skip)]
    captures: HashSet<Vertex>,
    #[serde(skip)]
    counter: u64,
    #[serde(skip)]
    challenger: bool,
    #[serde(skip)]
    connected_tcp: bool,
    #[serde(skip)]
    connected_to: String,
    #[serde(skip)]
    content: text_editor::Content,
    #[serde(skip)]
    email: Option<Email>,
    #[serde(skip)]
    emails_bcc: Vec<String>,
    #[serde(skip)]
    error: Option<String>,
    #[serde(skip)]
    error_email: Option<String>,
    #[serde(skip)]
    error_persistent: Option<String>,
    #[serde(skip)]
    game: Option<Game>,
    #[serde(skip)]
    game_id: usize,
    #[serde(skip)]
    games_light: ServerGamesLight,
    #[serde(skip)]
    game_settings: NewGameSettings,
    #[serde(default)]
    locale_selected: Locale,
    #[serde(skip)]
    my_games_only: bool,
    #[serde(skip)]
    my_turn: bool,
    #[serde(skip)]
    now: i64,
    #[serde(skip)]
    now_diff: i64,
    #[serde(skip)]
    password: String,
    #[serde(skip)]
    password_no_save: bool,
    #[serde(skip)]
    password_show: bool,
    #[serde(skip)]
    play_from: Option<Vertex>,
    #[serde(skip)]
    play_from_previous: Option<Vertex>,
    #[serde(skip)]
    play_to_previous: Option<Vertex>,
    #[serde(skip)]
    request_draw: bool,
    #[serde(skip)]
    screen: Screen,
    #[serde(skip)]
    screen_size: Size,
    #[serde(skip)]
    sound_muted: bool,
    #[serde(skip)]
    spectators: Vec<String>,
    #[serde(skip)]
    status: Status,
    #[serde(skip)]
    texts: VecDeque<String>,
    #[serde(skip)]
    texts_game: VecDeque<String>,
    #[serde(skip)]
    text_input: String,
    #[serde(default)]
    theme: Theme,
    #[serde(skip)]
    time_attacker: TimeSettings,
    #[serde(skip)]
    time_defender: TimeSettings,
    #[serde(skip)]
    tx: Option<mpsc::Sender<String>>,
    #[serde(default)]
    username: String,
    #[serde(skip)]
    users: HashMap<String, User>,
    #[serde(skip)]
    strings: HashMap<String, String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct NewGameSettings {
    #[serde(skip)]
    board_size: Option<BoardSize>,
    #[serde(skip)]
    rated: Rated,
    #[serde(skip)]
    role_selected: Option<Role>,
    #[serde(skip)]
    timed: TimeSettings,
    #[serde(skip)]
    time_days: String,
    #[serde(skip)]
    time_hours: String,
    #[serde(skip)]
    time_minutes: String,
    #[serde(skip)]
    time_add_hours: String,
    #[serde(skip)]
    time_add_minutes: String,
    #[serde(skip)]
    time_add_seconds: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
enum Screen {
    AccountSettings,
    EmailEveryone,
    #[default]
    Login,
    Game,
    GameNew,
    GameNewFrozen,
    GameReview,
    Games,
    Users,
}

impl<'a> Client {
    fn archived_game_reset(&mut self) {
        self.archived_game_handle = None;
        self.archived_game_selected = None;
    }

    #[allow(clippy::too_many_lines)]
    #[must_use]
    fn board(&self) -> Row<Message> {
        let board = if let Some(game_handle) = &self.archived_game_handle {
            &game_handle.boards.here().unwrap()
        } else {
            let Some(game) = &self.game else {
                panic!("we should be in a game");
            };
            &game.board
        };

        let board_size = board.size();
        let board_size_usize: usize = board_size.into();

        let (board_dimension, letter_size, piece_size, spacing) = match board_size {
            BoardSize::_11 => match self.screen_size {
                Size::Large | Size::Giant => (75, 55, 60, 6),
                Size::Medium => (65, 45, 50, 8),
                Size::Small => (55, 35, 40, 11),
                Size::Tiny => (40, 20, 25, 16),
            },
            BoardSize::_13 => match self.screen_size {
                Size::Large | Size::Giant => (65, 45, 50, 8),
                Size::Medium => (58, 38, 43, 10),
                Size::Small => (50, 30, 35, 12),
                Size::Tiny => (40, 20, 25, 15),
            },
        };

        let letters: Vec<_> = BOARD_LETTERS[..board_size_usize].chars().collect();
        let mut game_display = Row::new().spacing(2);

        let mut possible_moves = None;
        if self.my_turn {
            if let Some(game) = self.game.as_ref() {
                possible_moves = Some(game.all_legal_moves());
            }
        }

        let mut column = column![text(" ").size(letter_size)].spacing(spacing);

        for i in 0..board_size_usize {
            let i = board_size_usize - i;
            column = column.push(text!("{i:2}").size(letter_size).align_y(Vertical::Center));
        }
        game_display = game_display.push(column);

        for (x, letter) in letters.iter().enumerate() {
            let mut column = Column::new().spacing(2).align_x(Horizontal::Center);
            column = column.push(text(letter).size(letter_size));

            for y in 0..board_size_usize {
                let vertex = Vertex {
                    size: board.size(),
                    x,
                    y,
                };

                let mut text_ = match board.get(&vertex) {
                    Space::Empty => {
                        if (board_size == BoardSize::_11
                            && ((y, x) == (0, 0)
                                || (y, x) == (10, 0)
                                || (y, x) == (0, 10)
                                || (y, x) == (10, 10)
                                || (y, x) == (5, 5)))
                            || (board_size == BoardSize::_13
                                && ((y, x) == (0, 0)
                                    || (y, x) == (12, 0)
                                    || (y, x) == (0, 12)
                                    || (y, x) == (12, 12)
                                    || (y, x) == (6, 6)))
                        {
                            text("⌘")
                        } else {
                            text(" ")
                        }
                    }
                    Space::Attacker => text("♟"),
                    Space::King => text("♔"),
                    Space::Defender => text("♙"),
                };

                text_ = text_
                    .size(piece_size)
                    .shaping(text::Shaping::Advanced)
                    .center();

                if let (Some(from), Some(to)) = (&self.play_from_previous, &self.play_to_previous) {
                    let x_diff = from.x as i128 - to.x as i128;
                    let y_diff = from.y as i128 - to.y as i128;
                    let mut arrow = " ";

                    if y_diff < 0 {
                        arrow = "↓";
                    } else if y_diff > 0 {
                        arrow = "↑";
                    } else if x_diff < 0 {
                        arrow = "→";
                    } else if x_diff > 0 {
                        arrow = "←";
                    }

                    if (y, x) == (from.y, from.x) {
                        text_ = text(arrow)
                            .size(piece_size)
                            .shaping(text::Shaping::Advanced)
                            .center();
                    }
                }

                if self.captures.contains(&vertex) {
                    text_ = text("X").size(piece_size).center();
                }

                let mut button_ = button(text_).width(board_dimension).height(board_dimension);

                if let Some(legal_moves) = &possible_moves {
                    if let Some(vertex_from) = self.play_from.as_ref() {
                        if let Some(vertexes) = legal_moves.moves.get(vertex_from) {
                            if vertex == *vertex_from {
                                button_ = button_.on_press(Message::PlayMoveRevert);
                            }
                            if vertexes.contains(&vertex) {
                                button_ = button_.on_press(Message::PlayMoveTo(vertex));
                            }
                        }
                    } else if legal_moves.moves.contains_key(&vertex) {
                        button_ = button_.on_press(Message::PlayMoveFrom(vertex));
                    }
                }

                column = column.push(button_);
            }

            column = column.push(
                text(letters[x])
                    .size(letter_size)
                    .align_x(Horizontal::Center),
            );
            game_display = game_display.push(column);
        }

        let mut column = column![text(" ").size(letter_size)].spacing(spacing);
        for i in 0..board_size_usize {
            let i = board_size_usize - i;
            column = column.push(text!("{i:2}").size(letter_size).align_y(Vertical::Center));
        }

        game_display = game_display.push(column);
        game_display
    }

    #[allow(clippy::too_many_lines)]
    fn display_game(&self) -> Element<Message> {
        let mut attacker_rating = String::new();
        let mut defender_rating = String::new();

        let (game_id, attacker, attacker_time, defender, defender_time, board, play, status, texts) =
            if let Some(game_handle) = &self.archived_game_handle {
                attacker_rating = game_handle.game.attacker_rating.to_string_rounded();
                defender_rating = game_handle.game.defender_rating.to_string_rounded();

                let status = if game_handle.play < game_handle.game.plays.len() - 1 {
                    &Status::Ongoing
                } else {
                    &game_handle.game.status
                };

                (
                    &game_handle.game.id,
                    &game_handle.game.attacker,
                    game_handle
                        .game
                        .plays
                        .time_left(Role::Attacker, game_handle.play),
                    &game_handle.game.defender,
                    game_handle
                        .game
                        .plays
                        .time_left(Role::Defender, game_handle.play),
                    &game_handle.boards.here().unwrap(),
                    game_handle.play,
                    status,
                    &game_handle.game.texts,
                )
            } else {
                for user in self.users.values() {
                    if self.attacker == user.name {
                        attacker_rating = user.rating.to_string_rounded();
                    }
                    if self.defender == user.name {
                        defender_rating = user.rating.to_string_rounded();
                    }
                }

                let Some(game) = &self.game else {
                    panic!("we should be in a game");
                };

                (
                    &self.game_id,
                    &self.attacker,
                    self.time_attacker.time_left(),
                    &self.defender,
                    self.time_defender.time_left(),
                    &game.board,
                    game.previous_boards.0.len() - 1,
                    &self.status,
                    &self.texts_game,
                )
            };

        for user in self.users.values() {
            if self.attacker == user.name {
                attacker_rating = user.rating.to_string_rounded();
            }
            if self.defender == user.name {
                defender_rating = user.rating.to_string_rounded();
            }
        }

        let captured = board.captured();
        let attacker = container(
            column![
                row![
                    text(attacker).shaping(text::Shaping::Advanced),
                    text(attacker_rating).center(),
                    text(captured.defender().to_string()).shaping(text::Shaping::Advanced)
                ]
                .spacing(SPACING),
                row![
                    text(attacker_time).size(35).center(),
                    text("🗡").shaping(text::Shaping::Advanced).size(35).center(),
                ]
                .spacing(SPACING),
            ]
            .spacing(SPACING),
        )
        .padding(PADDING)
        .style(container::bordered_box);

        let defender = container(
            column![
                row![
                    text(defender).shaping(text::Shaping::Advanced),
                    text(defender_rating).center(),
                    text(captured.attacker().to_string()).shaping(text::Shaping::Advanced)
                ]
                .spacing(SPACING),
                row![
                    text(defender_time).size(35).center(),
                    text("⛨")
                        .shaping(text::Shaping::Advanced)
                        .size(35.0)
                        .center(),
                ]
                .spacing(SPACING),
            ]
            .spacing(SPACING),
        )
        .padding(PADDING)
        .style(container::bordered_box);

        let mut watching = false;

        let sub_second = self.now_diff % 1_000;
        let seconds = self.now_diff / 1_000;

        let mut user_area = column![
            text!(
                "#{game_id} {} {}: {seconds:01}.{sub_second:03} s",
                &self.username,
                t!("lag"),
            )
            .shaping(text::Shaping::Advanced)
        ]
        .spacing(SPACING);

        let is_rated = match self.game_settings.rated {
            Rated::No => t!("no"),
            Rated::Yes => t!("yes"),
        };

        user_area = user_area.push(
            text!("{}: {play} {}: {is_rated}", t!("move"), t!("rated"),)
                .shaping(text::Shaping::Advanced),
        );

        match self.screen_size {
            Size::Tiny | Size::Small | Size::Medium | Size::Large => {
                user_area = user_area.push(column![attacker, defender].spacing(SPACING));
            }
            Size::Giant => {
                user_area = user_area.push(row![attacker, defender].spacing(SPACING));
            }
        }

        let mut spectators = Column::new();
        for spectator in &self.spectators {
            if self.username.as_str() == spectator.as_str() {
                watching = true;
            }

            let mut spectator = spectator.to_string();
            if let Some(user) = self.users.get(&spectator) {
                let _ok = write!(spectator, " ({})", user.rating.to_string_rounded());
            }
            spectators = spectators.push(text(spectator));
        }

        let resign = button(text(self.strings["Resign"].as_str()).shaping(text::Shaping::Advanced))
            .on_press(Message::PlayResign);

        let request_draw =
            button(text(self.strings["Request Draw"].as_str()).shaping(text::Shaping::Advanced))
                .on_press(Message::PlayDraw);

        if !watching {
            if self.my_turn {
                match self.screen_size {
                    Size::Tiny | Size::Small => {
                        user_area = user_area.push(
                            column![
                                row![resign].spacing(SPACING),
                                row![request_draw].spacing(SPACING),
                            ]
                            .spacing(SPACING),
                        );
                    }
                    Size::Medium | Size::Large | Size::Giant => {
                        user_area = user_area.push(row![resign, request_draw].spacing(SPACING));
                    }
                }
            } else {
                let row = if self.request_draw {
                    column![
                        row![
                            button(
                                text(self.strings["Accept Draw"].as_str())
                                    .shaping(text::Shaping::Advanced)
                            )
                            .on_press(Message::PlayDrawDecision(Draw::Accept)),
                        ]
                        .spacing(SPACING)
                    ]
                } else {
                    Column::new()
                };
                user_area = user_area.push(row.spacing(SPACING));
            }
        }

        let muted = checkbox(t!("Muted"), self.sound_muted)
            .text_shaping(text::Shaping::Advanced)
            .on_toggle(Message::SoundMuted)
            .size(32);

        let leave = button(text(self.strings["Leave"].as_str()).shaping(text::Shaping::Advanced))
            .on_press(Message::Leave);

        user_area = user_area.push(row![muted, leave].spacing(SPACING));

        match status {
            Status::AttackerWins => {
                user_area =
                    user_area.push(text(t!("Attacker wins!")).shaping(text::Shaping::Advanced));
            }
            Status::Draw => {
                user_area =
                    user_area.push(text(t!("It's a draw.")).shaping(text::Shaping::Advanced));
            }
            Status::Ongoing => {}
            Status::DefenderWins => {
                user_area =
                    user_area.push(text(t!("Defender wins!")).shaping(text::Shaping::Advanced));
            }
        }

        let spectator =
            column![text!("👥 ({})", self.spectators.len()).shaping(text::Shaping::Advanced)];

        if self.spectators.is_empty() {
            user_area = user_area.push(spectator);
        } else {
            user_area = user_area.push(tooltip(
                spectator,
                container(spectators)
                    .style(container::bordered_box)
                    .padding(PADDING),
                tooltip::Position::Bottom,
            ));
        }

        if let Some(handle) = &self.archived_game_handle {
            let mut left_all = button(text("⏮").shaping(text::Shaping::Advanced));
            let mut left = button(text("⏪").shaping(text::Shaping::Advanced));
            if handle.play > 0 {
                left_all = left_all.on_press(Message::ReviewGameBackwardAll);
                left = left.on_press(Message::ReviewGameBackward);
            }

            let mut right = button(text("⏩").center().shaping(text::Shaping::Advanced));
            let mut right_all = button(text("⏭").center().shaping(text::Shaping::Advanced));
            if handle.play < handle.game.plays.len() - 1 {
                right = right.on_press(Message::ReviewGameForward);
                right_all = right_all.on_press(Message::ReviewGameForwardAll);
            }

            user_area = user_area.push(row![left_all, left, right, right_all].spacing(SPACING));
        }

        user_area = user_area.push(self.texting(texts));

        let user_area = container(user_area)
            .padding(PADDING)
            .style(container::bordered_box);

        row![self.board(), user_area].spacing(SPACING).into()
    }

    #[allow(clippy::collapsible_match)]
    fn subscriptions(&self) -> Subscription<Message> {
        let subscription_1 = if let Some(game) = &self.game {
            if let TimeUnix::Time(_) = game.time {
                iced::time::every(iced::time::Duration::from_millis(100))
                    .map(|_instant| Message::Tick)
            } else {
                Subscription::none()
            }
        } else {
            Subscription::none()
        };

        let subscription_2 = Subscription::run(pass_messages);

        let subscription_3 = event::listen_with(|event, _status, _id| match event {
            Event::Window(iced::window::Event::Resized(size)) => {
                Some(Message::WindowResized((size.width, size.height)))
            }
            Event::Keyboard(event) => match event {
                keyboard::Event::KeyPressed {
                    key: Key::Named(Named::Tab),
                    modifiers,
                    ..
                } => Some(if modifiers.shift() {
                    Message::FocusPrevious
                } else {
                    Message::FocusNext
                }),
                keyboard::Event::KeyPressed {
                    key: Key::Named(Named::ArrowLeft),
                    ..
                } => Some(Message::ReviewGameBackward),
                keyboard::Event::KeyPressed {
                    key: Key::Named(Named::ArrowRight),
                    ..
                } => Some(Message::ReviewGameForward),
                _ => None,
            },
            _ => None,
        });

        Subscription::batch(vec![subscription_1, subscription_2, subscription_3])
    }

    fn texting(&'a self, messages: &'a VecDeque<String>) -> Container<'a, Message> {
        let text_input = text_input(&format!("{}…", t!("message")), &self.text_input)
            .on_input(Message::TextChanged)
            .on_paste(Message::TextChanged)
            .on_submit(Message::TextSend);

        let mut texting = Column::new();

        for message in messages {
            texting = texting.push(text(message).shaping(text::Shaping::Advanced));
        }

        container(column![text_input, scrollable(texting)].spacing(SPACING))
            .padding(PADDING)
            .style(container::bordered_box)
    }

    pub fn theme(&self) -> iced::Theme {
        match self.theme {
            Theme::Dark => iced::Theme::SolarizedDark,
            Theme::Light => iced::Theme::SolarizedLight,
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn update(&mut self, message: Message) -> Task<Message> {
        self.error = None;

        match message {
            Message::AccountSettings => self.screen = Screen::AccountSettings,
            Message::ArchivedGames(mut archived_games) => {
                archived_games.reverse();
                self.archived_games = archived_games.clone();
                self.archived_games_filtered = archived_games;
            }
            Message::ArchivedGamesGet => self.send("archived_games\n".to_string()),
            Message::ArchivedGameSelected(game) => self.archived_game_selected = Some(game),
            Message::ChangeTheme(theme) => {
                self.theme = theme;
                self.save_client();
            }
            Message::BoardSizeSelected(size) => self.game_settings.board_size = Some(size),
            Message::ConnectedTo(address) => self.connected_to = address,
            Message::DeleteAccount => {
                if self.delete_account {
                    self.send("delete_account\n".to_string());
                } else {
                    self.delete_account = true;
                }
            }
            Message::EmailEveryone => {
                self.screen = Screen::EmailEveryone;
                self.send("emails_bcc\n".to_string());
            }
            Message::EmailReset => {
                self.email = None;
                self.send("email_reset\n".to_string());
            }
            Message::FocusNext => return focus_next(),
            Message::FocusPrevious => return focus_previous(),
            Message::GameAccept(id) => {
                self.send(format!("join_game {id}\n"));
                self.game_id = id;
            }
            Message::GameDecline(id) => {
                self.send(format!("decline_game {id}\n"));
            }
            Message::GameJoin(id) => {
                self.game_id = id;
                self.send(format!("join_game_pending {id}\n"));

                let Some(game) = self.games_light.0.get(&id) else {
                    panic!("the game must exist");
                };

                self.game_settings.role_selected = if game.attacker.is_some() {
                    Some(Role::Defender)
                } else {
                    Some(Role::Attacker)
                };

                self.screen = Screen::GameNewFrozen;
            }
            Message::GameWatch(id) => {
                self.game_id = id;
                self.send(format!("watch_game {id}\n"));
            }
            Message::Leave => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::GameReview
                | Screen::Users => {
                    self.screen = Screen::Games;
                    self.text_input = String::new();
                }
                Screen::Game => {
                    self.screen = Screen::Games;
                    self.my_turn = false;
                    self.request_draw = false;

                    if self.spectators.contains(&self.username) {
                        self.send(format!("leave_game {}\n", self.game_id));
                    }
                    self.spectators = Vec::new();
                }
                Screen::GameNewFrozen => {
                    self.send(format!("leave_game {}\n", self.game_id));
                    self.screen = Screen::Games;
                }
                Screen::Games => self.send("logout\n".to_string()),
                Screen::Login => self.send("quit\n".to_string()),
            },
            Message::LocaleSelected(locale) => {
                rust_i18n::set_locale(&locale.txt());

                let string_keys: Vec<_> = self.strings.keys().cloned().collect();
                for string in string_keys {
                    self.strings.insert(string.clone(), t!(string).to_string());
                }

                self.locale_selected = locale;
                self.save_client();
            }
            Message::MyGamesOnly(selected) => {
                if selected {
                    self.archived_games_filtered = self
                        .archived_games
                        .iter()
                        .filter(|game| {
                            game.attacker == self.username || game.defender == self.username
                        })
                        .cloned()
                        .collect();
                } else {
                    self.archived_games_filtered = self.archived_games.clone();
                }

                self.my_games_only = selected;
            }
            Message::OpenUrl(string) => open_url(&string),
            Message::GameNew => {
                self.game_settings = NewGameSettings::default();
                self.screen = Screen::GameNew;
            }
            Message::GameResume(id) => {
                self.game_id = id;
                self.send(format!("resume_game {id}\n"));
            }
            Message::GameSubmit => {
                if let Some(role) = self.game_settings.role_selected {
                    if let TimeSettings::Timed(_) = self.game_settings.timed {
                        let days: i64 = self.game_settings.time_days.parse().unwrap_or_default();
                        let hours: i64 = self.game_settings.time_hours.parse().unwrap_or_default();
                        let minutes: i64 = self.game_settings.time_minutes.parse().unwrap_or(15);
                        let milliseconds_left = (days * 24 * 60 * 60 * 1_000)
                            + (hours * 60 * 60 * 1_000)
                            + (minutes * 60 * 1_000);

                        let add_hours: i64 = self
                            .game_settings
                            .time_add_hours
                            .parse()
                            .unwrap_or_default();

                        let add_minutes: i64 = self
                            .game_settings
                            .time_add_minutes
                            .parse()
                            .unwrap_or_default();

                        let mut add_seconds: i64 =
                            self.game_settings.time_add_seconds.parse().unwrap_or(10);

                        add_seconds += (add_hours * 60 * 60) + (add_minutes * 60);

                        self.game_settings.timed = TimeSettings::Timed(Time {
                            add_seconds,
                            milliseconds_left,
                        });
                    }

                    self.screen = Screen::GameNewFrozen;

                    let Some(board_size) = self.game_settings.board_size else {
                        unreachable!();
                    };

                    // <- new_game (attacker | defender) (rated | unrated) (TIME_MINUTES | _) (ADD_SECONDS_AFTER_EACH_MOVE | _) board_size
                    // -> game id rated attacker defender un-timed _ _ board_size challenger challenge_accepted spectators
                    self.send(format!(
                        "new_game {role} {} {:?} {board_size}\n",
                        self.game_settings.rated, self.game_settings.timed,
                    ));
                }
            }
            Message::PasswordChanged(password) => {
                let (password, ends_with_whitespace) = split_whitespace(&password);
                self.password_no_save = ends_with_whitespace;
                if password.len() <= 32 {
                    self.password = password;
                }
            }
            Message::PasswordShow(show_password) => {
                self.password_show = show_password;
            }
            Message::PlayDraw => {
                let game = self.game.as_ref().expect("you should have a game by now");
                self.send(format!("request_draw {} {}\n", self.game_id, game.turn));
            }
            Message::PlayDrawDecision(draw) => {
                self.send(format!("draw {} {draw}\n", self.game_id));
            }
            Message::PlayMoveFrom(vertex) => self.play_from = Some(vertex),
            Message::PlayMoveTo(to) => {
                let Some(from) = self.play_from.clone() else {
                    panic!("you have to have a from to get to to");
                };

                let mut turn = Role::Roleless;
                if let Some(game) = &self.game {
                    turn = game.turn;
                }

                self.handle_play(None, &from.to_string(), &to.to_string());

                self.send(format!("game {} play {} {from} {to}\n", self.game_id, turn));

                let game = self.game.as_ref().expect("you should have a game by now");
                if game.status == Status::Ongoing {
                    match game.turn {
                        Role::Attacker => {
                            if let TimeSettings::Timed(time) = &mut self.time_defender {
                                time.milliseconds_left += time.add_seconds * 1_000;
                            }
                        }
                        Role::Roleless => {}
                        Role::Defender => {
                            if let TimeSettings::Timed(time) = &mut self.time_attacker {
                                time.milliseconds_left += time.add_seconds * 1_000;
                            }
                        }
                    }
                }

                self.play_from_previous = self.play_from.clone();
                self.play_to_previous = Some(to);
                self.play_from = None;
                self.my_turn = false;
            }
            Message::PlayMoveRevert => self.play_from = None,
            Message::PlayResign => {
                let game = self.game.as_ref().expect("you should have a game by now");

                self.send(format!(
                    "game {} play {} resigns _\n",
                    self.game_id, game.turn
                ));
            }
            Message::SoundMuted(muted) => self.sound_muted = muted,
            Message::StreamConnected(tx) => self.tx = Some(tx),
            Message::RatedSelected(rated) => {
                self.game_settings.rated = if rated { Rated::Yes } else { Rated::No };
            }
            Message::ResetPassword(account) => {
                if !self.connected_tcp {
                    self.send("tcp_connect\n".to_string());
                    self.connected_tcp = true;
                }
                self.send(format!("{VERSION_ID} reset_password {account}\n"));
            }
            Message::ReviewGame => {
                if let Some(archived_game) = &self.archived_game_selected {
                    self.archived_game_handle = Some(ArchivedGameHandle::new(archived_game));
                    self.screen = Screen::GameReview;

                    self.captures = HashSet::new();
                    self.play_from = None;
                    self.play_from_previous = None;
                    self.play_to_previous = None;
                }
            }
            Message::ReviewGameBackward => {
                if let Some(handle) = &mut self.archived_game_handle {
                    if handle.play > 0 {
                        handle.play -= 1;
                        handle.boards.backward();
                    }
                }
            }
            Message::ReviewGameBackwardAll => {
                if let Some(handle) = &mut self.archived_game_handle {
                    handle.play = 0;
                    handle.boards.backward_all();
                }
            }
            Message::ReviewGameForward => {
                if let Some(handle) = &mut self.archived_game_handle {
                    if handle.play < handle.game.plays.len() - 1 {
                        handle.play += 1;
                        handle.boards.forward();
                    }
                }
            }
            Message::ReviewGameForwardAll => {
                if let Some(handle) = &mut self.archived_game_handle {
                    handle.play = handle.game.plays.len() - 1;
                    handle.boards.forward_all();
                }
            }
            Message::RoleSelected(role) => {
                self.game_settings.role_selected = Some(role);
            }
            Message::TextChanged(string) => {
                if self.screen == Screen::Login {
                    let string: Vec<_> = string.split_whitespace().collect();
                    if let Some(string) = string.first() {
                        if string.len() <= 16 {
                            self.text_input = string.to_ascii_lowercase();
                        }
                    } else {
                        self.text_input = String::new();
                    }
                } else {
                    self.text_input = string;
                }
            }
            Message::TextEdit(action) => {
                self.content.perform(action);
            }
            Message::TextReceived(string) => {
                let mut text = string.split_ascii_whitespace();
                match text.next() {
                    Some("=") => {
                        let text_next = text.next();
                        match text_next {
                            Some("archived_games" | "challenge_requested" | "game") => {}
                            Some("display_games") => {
                                self.games_light.0.clear();
                                let games: Vec<&str> = text.collect();
                                for chunks in games.chunks_exact(12) {
                                    let game = ServerGameLight::try_from(chunks)
                                        .expect("the value should be a valid ServerGameLight");

                                    self.games_light.0.insert(game.id, game);
                                }

                                if let Some(game) = self.games_light.0.get(&self.game_id) {
                                    self.spectators = game.spectators.keys().cloned().collect();
                                    self.spectators.sort();
                                }
                            }
                            Some("display_users") => {
                                let users: Vec<&str> = text.collect();
                                self.users.clear();
                                for user_wins_losses_rating in users.chunks_exact(6) {
                                    let rating = user_wins_losses_rating[4];
                                    let Some((mut rating, mut deviation)) = rating.split_once("±")
                                    else {
                                        panic!("the ratings has this form: {rating}");
                                    };

                                    rating = rating.trim();
                                    deviation = deviation.trim();

                                    let (Ok(rating), Ok(deviation)) =
                                        (rating.parse::<f64>(), deviation.parse::<f64>())
                                    else {
                                        panic!(
                                            "the ratings has this form: ({rating}, {deviation})"
                                        );
                                    };

                                    let logged_in = "logged_in" == user_wins_losses_rating[5];

                                    self.users.insert(
                                        user_wins_losses_rating[0].to_string(),
                                        User {
                                            name: user_wins_losses_rating[0].to_string(),
                                            wins: user_wins_losses_rating[1].to_string(),
                                            losses: user_wins_losses_rating[2].to_string(),
                                            draws: user_wins_losses_rating[3].to_string(),
                                            rating: Rating {
                                                rating,
                                                rd: deviation / CONFIDENCE_INTERVAL_95,
                                            },
                                            logged_in,
                                        },
                                    );
                                }
                            }
                            Some("draw") => {
                                self.request_draw = false;
                                if let Some("accept") = text.next() {
                                    self.my_turn = false;
                                    self.status = Status::Draw;

                                    if let Some(game) = &mut self.game {
                                        game.turn = Role::Roleless;
                                    }
                                }
                            }
                            Some("email") => {
                                if let (Some(address), Some(verified)) = (text.next(), text.next())
                                {
                                    self.email = Some(Email {
                                        username: String::new(),
                                        address: address.to_string(),
                                        code: None,
                                        verified: handle_error(verified.parse()),
                                    });
                                }
                            }
                            Some("emails_bcc") => {
                                self.emails_bcc = text.map(ToString::to_string).collect();
                            }
                            Some("email_code") => {
                                if let Some(email) = &mut self.email {
                                    email.verified = true;
                                }
                                self.error_email = None;
                            }
                            Some("game_over") => {
                                self.my_turn = false;
                                if let Some(game) = &mut self.game {
                                    game.turn = Role::Roleless;
                                }

                                text.next();
                                match text.next() {
                                    Some("attacker_wins") => self.status = Status::AttackerWins,
                                    Some("defender_wins") => self.status = Status::DefenderWins,
                                    _ => error!("(1) unexpected text: {}", string.trim()),
                                }

                                if !self.sound_muted {
                                    thread::spawn(move || {
                                        let mut stream =
                                            rodio::OutputStreamBuilder::open_default_stream()?;

                                        let game_over = include_bytes!("game_over.ogg");
                                        let cursor = Cursor::new(game_over);
                                        let sound = rodio::play(stream.mixer(), cursor)?;
                                        sound.set_volume(1.0);
                                        sound.sleep_until_end();

                                        stream.log_on_drop(false);
                                        Ok::<(), anyhow::Error>(())
                                    });
                                }
                            }
                            // = join_game david abby rated fischer 900_000 10
                            Some("join_game" | "resume_game" | "watch_game") => {
                                self.screen = Screen::Game;
                                self.status = Status::Ongoing;
                                self.captures = HashSet::new();
                                self.play_from = None;
                                self.play_from_previous = None;
                                self.play_to_previous = None;
                                self.texts_game = VecDeque::new();
                                self.archived_game_handle = None;

                                let Some(attacker) = text.next() else {
                                    panic!("the attacker should be supplied");
                                };
                                let Some(defender) = text.next() else {
                                    panic!("the defender should be supplied");
                                };
                                self.attacker = attacker.to_string();
                                self.defender = defender.to_string();

                                let Some(rated) = text.next() else {
                                    panic!("there should be rated or unrated supplied");
                                };
                                let Ok(rated) = Rated::from_str(rated) else {
                                    panic!("rated should be valid");
                                };
                                self.game_settings.rated = rated;

                                let Some(timed) = text.next() else {
                                    panic!("there should be a time setting supplied");
                                };
                                let Some(minutes) = text.next() else {
                                    panic!("there should be a minutes supplied");
                                };
                                let Some(add_seconds) = text.next() else {
                                    panic!("there should be a add_seconds supplied");
                                };
                                let Ok(timed) = TimeSettings::try_from(vec![
                                    "time_settings",
                                    timed,
                                    minutes,
                                    add_seconds,
                                ]) else {
                                    panic!("there should be a valid time settings");
                                };

                                let Some(board_size) = text.next() else {
                                    panic!("there should be a valid board size");
                                };
                                let Ok(board_size) = BoardSize::from_str(board_size) else {
                                    panic!("there should be a valid board size");
                                };

                                let board = match board_size {
                                    BoardSize::_11 => board::board_11x11(),
                                    BoardSize::_13 => board::board_13x13(),
                                };

                                let mut game = Game {
                                    attacker_time: timed.clone(),
                                    defender_time: timed.clone(),
                                    plays: Plays::new(&timed),
                                    board,
                                    ..Game::default()
                                };

                                self.time_attacker = timed.clone();
                                self.time_defender = timed;

                                if let Some(game_serialized) = text.next() {
                                    let game_deserialized = ron::from_str(game_serialized)
                                        .expect("we should be able to deserialize the game");

                                    game = game_deserialized;

                                    self.time_attacker = game.attacker_time.clone();
                                    self.time_defender = game.defender_time.clone();

                                    match game.turn {
                                        Role::Attacker => {
                                            if let (
                                                TimeSettings::Timed(time),
                                                TimeUnix::Time(time_ago),
                                            ) = (&mut self.time_attacker, &game.time)
                                            {
                                                let now = Local::now().to_utc().timestamp_millis();
                                                time.milliseconds_left -= now - time_ago;
                                                if time.milliseconds_left < 0 {
                                                    time.milliseconds_left = 0;
                                                }
                                            }
                                        }
                                        Role::Roleless => {}
                                        Role::Defender => {
                                            if let (
                                                TimeSettings::Timed(time),
                                                TimeUnix::Time(time_ago),
                                            ) = (&mut self.time_defender, &game.time)
                                            {
                                                let now = Local::now().to_utc().timestamp_millis();
                                                time.milliseconds_left -= now - time_ago;
                                                if time.milliseconds_left < 0 {
                                                    time.milliseconds_left = 0;
                                                }
                                            }
                                        }
                                    }
                                }

                                let texts: Vec<&str> = text.collect();
                                let texts = texts.join(" ");
                                if !texts.is_empty() {
                                    let texts = ron::from_str(&texts)
                                        .expect("we should be able to deserialize the text");
                                    println!("{texts:?}");
                                    self.texts_game = texts;
                                }

                                if (self.username == attacker && game.turn == Role::Attacker)
                                    || (self.username == defender && game.turn == Role::Defender)
                                {
                                    self.my_turn = true;
                                }

                                self.game = Some(game);
                            }
                            Some("join_game_pending") => {
                                self.challenger = true;
                                let Some(id) = text.next() else {
                                    panic!("there should be an id supplied");
                                };
                                let Ok(id) = id.parse() else {
                                    panic!("id should be a valid usize");
                                };
                                self.game_id = id;
                            }
                            Some("leave_game") => self.game_id = 0,
                            Some("login") => self.screen = Screen::Games,
                            Some("new_game") => {
                                // = new_game game 15 none david rated fischer 900_000 10
                                if Some("game") == text.next() {
                                    self.challenger = false;
                                    let Some(game_id) = text.next() else {
                                        panic!("the game id should be next");
                                    };
                                    let Ok(game_id) = game_id.parse() else {
                                        panic!("the game_id should be a usize")
                                    };
                                    self.game_id = game_id;
                                }
                            }
                            Some("ping") => {
                                let after = Utc::now().timestamp_millis();
                                self.now_diff = after - self.now;
                            }
                            Some("text") => self.texts.push_front(text_collect(text)),
                            Some("text_game") => self.texts_game.push_front(text_collect(text)),
                            _ => error!("(2) unexpected text: {}", string.trim()),
                        }
                    }
                    Some("?") => {
                        let text_next = text.next();
                        match text_next {
                            Some("create_account" | "login") => {
                                let text: Vec<_> = text.collect();
                                let text = text.join(" ");
                                self.error = Some(text);
                            }
                            Some("email") => {
                                let text: Vec<_> = text.collect();
                                let text = text.join(" ");
                                self.error_email = Some(text);
                            }
                            Some("email_code") => {
                                self.error_email = Some("invalid email code".to_string());
                            }
                            _ => error!("(3) unexpected text: {}", string.trim()),
                        }
                    }
                    Some("game") => {
                        // Plays the move then sends the result back.
                        let Some(id) = text.next() else {
                            panic!("there should be a game id");
                        };
                        let Ok(id) = id.parse::<usize>() else {
                            panic!("the game_id should be a valid usize");
                        };
                        self.game_id = id;

                        // game 0 generate_move attacker
                        let text_word = text.next();
                        if text_word == Some("generate_move") {
                            self.request_draw = false;
                            self.my_turn = true;
                        // game 0 play attacker a3 a4
                        } else if text_word == Some("play") {
                            let role = text.next().expect("this should be a role string");
                            let role = Role::from_str(role).expect("this should be a role");
                            let from = text.next().expect("this should be from");

                            if from == "resigns" {
                                return Task::none();
                            }

                            let to = text.next().expect("this should be to");

                            if let (Ok(from), Ok(to)) =
                                (Vertex::from_str(from), Vertex::from_str(to))
                            {
                                self.play_from_previous = Some(from);
                                self.play_to_previous = Some(to);
                            }

                            self.handle_play(Some(&role.to_string()), from, to);
                            let game = self.game.as_ref().expect("you should have a game by now");

                            if game.status == Status::Ongoing {
                                match game.turn {
                                    Role::Attacker => {
                                        if let TimeSettings::Timed(time) = &mut self.time_defender {
                                            time.milliseconds_left += time.add_seconds * 1_000;
                                        }
                                    }
                                    Role::Roleless => {}
                                    Role::Defender => {
                                        if let TimeSettings::Timed(time) = &mut self.time_attacker {
                                            time.milliseconds_left += time.add_seconds * 1_000;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Some("request_draw") => {
                        let Some(id) = text.next() else {
                            panic!("there should be a game id");
                        };
                        let Ok(id) = id.parse::<usize>() else {
                            panic!("the game_id should be a valid usize");
                        };

                        if id == self.game_id {
                            self.request_draw = true;
                        }
                    }
                    _ => error!("(4) unexpected text: {}", string.trim()),
                }
            }
            Message::TextSend => {
                match self.screen {
                    Screen::AccountSettings => {
                        self.send(format!("change_password {}\n", self.password));
                        self.password.clear();
                    }
                    Screen::EmailEveryone => {
                        // subject == self.text_input
                        let email = self.content.text().replace('\n', "\\n");
                        self.send(format!("email_everyone {} {email}\n", self.text_input));
                    }
                    Screen::Game => {
                        if !self.text_input.trim().is_empty() {
                            self.text_input.push('\n');
                            self.send(format!("text_game {} {}", self.game_id, self.text_input));
                        }
                    }
                    Screen::Games => {
                        if !self.text_input.trim().is_empty() {
                            self.text_input.push('\n');
                            self.send(format!("text {}", self.text_input));
                        }
                    }
                    Screen::GameNew
                    | Screen::GameNewFrozen
                    | Screen::GameReview
                    | Screen::Login
                    | Screen::Users => {}
                }

                self.text_input.clear();
            }
            Message::TextSendEmail => {
                self.error_email = None;

                self.send(format!("email {}\n", self.text_input));
                self.text_input.clear();
            }
            Message::TextSendEmailCode => {
                self.error_email = None;

                self.send(format!("email_code {}\n", self.text_input));
            }
            Message::TextSendCreateAccount => {
                if !self.connected_tcp {
                    self.send("tcp_connect\n".to_string());
                    self.connected_tcp = true;
                }

                if !self.text_input.trim().is_empty() {
                    let username = self.text_input.to_string();
                    self.send(format!(
                        "{VERSION_ID} create_account {username} {}\n",
                        self.password,
                    ));
                    self.username = username;
                    self.password.clear();
                }
                self.text_input.clear();
                self.archived_game_reset();
                self.save_client();
            }
            Message::TextSendLogin => {
                if !self.connected_tcp {
                    self.send("tcp_connect\n".to_string());
                    self.connected_tcp = true;
                }

                if self.text_input.trim().is_empty() {
                    let username = format!("user-{:x}", rand::random::<u16>());

                    self.send(format!(
                        "{VERSION_ID} create_account {username} {}\n",
                        self.password
                    ));
                    self.username = username;
                } else {
                    let username = self.text_input.to_string();

                    self.send(format!("{VERSION_ID} login {username} {}\n", self.password));
                    self.username = username;
                }

                self.password.clear();
                self.text_input.clear();
                self.archived_game_reset();
                self.save_client();
            }
            Message::Tick => {
                self.counter = self.counter.wrapping_add(1);
                if self.counter % 25 == 0 {
                    self.now = Utc::now().timestamp_millis();
                    self.send("ping\n".to_string());
                }

                if let Some(game) = &mut self.game {
                    match game.turn {
                        Role::Attacker => {
                            if let TimeSettings::Timed(time) = &mut self.time_attacker {
                                time.milliseconds_left -= 100;
                                if time.milliseconds_left < 0 {
                                    time.milliseconds_left = 0;
                                }
                            }
                        }
                        Role::Roleless => {}
                        Role::Defender => {
                            if let TimeSettings::Timed(time) = &mut self.time_defender {
                                time.milliseconds_left -= 100;
                                if time.milliseconds_left < 0 {
                                    time.milliseconds_left = 0;
                                }
                            }
                        }
                    }
                }
            }
            Message::TimeAddHours(string) => {
                if string.parse::<u8>().is_ok() {
                    self.game_settings.time_add_hours = string;
                }
            }
            Message::TimeAddMinutes(string) => {
                if let Ok(minutes) = string.parse::<u8>() {
                    if minutes < 60 {
                        self.game_settings.time_add_minutes = string;
                    }
                }
            }
            Message::TimeAddSeconds(string) => {
                if let Ok(seconds) = string.parse::<u8>() {
                    if seconds < 60 {
                        self.game_settings.time_add_seconds = string;
                    }
                }
            }
            Message::TimeCheckbox(time_selected) => {
                if time_selected {
                    self.game_settings.timed = TimeSettings::default();
                } else {
                    self.game_settings.timed = TimeSettings::UnTimed;
                }
            }
            Message::TimeDays(string) => {
                if string.parse::<u8>().is_ok() {
                    self.game_settings.time_days = string;
                }
            }
            Message::TimeHours(string) => {
                if let Ok(hours) = string.parse::<u8>() {
                    if hours < 24 {
                        self.game_settings.time_hours = string;
                    }
                }
            }
            Message::TimeMinutes(string) => {
                if let Ok(minutes) = string.parse::<u8>() {
                    if minutes < 60 {
                        self.game_settings.time_minutes = string;
                    }
                }
            }
            Message::Users => self.screen = Screen::Users,
            Message::WindowResized((width, height)) => {
                if width >= 1_500.0 && height >= 1_000.0 {
                    self.screen_size = Size::Giant;
                } else if width >= 1_300.0 && height >= 1_000.0 {
                    self.screen_size = Size::Large;
                } else if width >= 1_200.0 && height >= 850.0 {
                    self.screen_size = Size::Medium;
                } else if width >= 1_000.0 && height >= 750.0 {
                    self.screen_size = Size::Small;
                } else {
                    self.screen_size = Size::Tiny;
                }
            }
        }

        Task::none()
    }

    #[must_use]
    fn users_sorted(&self) -> Vec<User> {
        let mut users: Vec<_> = self.users.values().cloned().collect();

        users.sort_by(|a, b| b.name.cmp(&a.name));
        users.sort_by(|a, b| b.rating.rating.partial_cmp(&a.rating.rating).unwrap());

        users
    }

    #[allow(clippy::too_many_lines)]
    #[must_use]
    fn games(&self) -> Scrollable<Message> {
        let mut game_ids = Column::new().spacing(SPACING_B);
        let mut attackers = Column::new().spacing(SPACING_B);
        let mut defenders = Column::new().spacing(SPACING_B);
        let mut ratings = Column::new().spacing(SPACING_B);
        let mut timings = Column::new().spacing(SPACING_B);
        let mut sizes = Column::new().spacing(SPACING_B);
        let mut buttons = Column::new().spacing(SPACING);

        let mut server_games: Vec<&ServerGameLight> = self.games_light.0.values().collect();
        server_games.sort_by(|a, b| b.id.cmp(&a.id));

        for game in server_games {
            let id = game.id;
            game_ids = game_ids.push(text(id));

            attackers = if let Some(attacker) = &game.attacker {
                attackers.push(text(attacker).shaping(text::Shaping::Advanced))
            } else {
                attackers.push(text(t!("none")).shaping(text::Shaping::Advanced))
            };
            defenders = if let Some(defender) = &game.defender {
                defenders.push(text(defender).shaping(text::Shaping::Advanced))
            } else {
                defenders.push(text(t!("none")).shaping(text::Shaping::Advanced))
            };

            let rating: bool = game.rated.into();
            let rating = if rating { t!("yes") } else { t!("no") };
            ratings = ratings.push(text(rating).shaping(text::Shaping::Advanced));

            timings = timings.push(text(game.timed.to_string()));
            sizes = sizes.push(text(game.board_size.to_string()));

            let mut buttons_row = Row::new().spacing(SPACING);

            if game.challenge_accepted
                && !(Some(&self.username) == game.attacker.as_ref()
                    || Some(&self.username) == game.defender.as_ref())
            {
                buttons_row = buttons_row.push(
                    button(text(self.strings["Watch"].as_str()).shaping(text::Shaping::Advanced))
                        .on_press(Message::GameWatch(id)),
                );
            } else if game.attacker.is_none() || game.defender.is_none() {
                buttons_row = buttons_row.push(
                    button(text(self.strings["Join"].as_str()).shaping(text::Shaping::Advanced))
                        .on_press(Message::GameJoin(id)),
                );
            }

            if game.attacker.as_ref() == Some(&self.username)
                || game.defender.as_ref() == Some(&self.username)
            {
                buttons_row = buttons_row.push(
                    button(text(self.strings["Resume"].as_str()).shaping(text::Shaping::Advanced))
                        .on_press(Message::GameResume(id)),
                );
            }

            buttons = buttons.push(buttons_row);
        }

        let game_id = t!("ID");
        let game_ids = column![
            text(game_id.to_string()).shaping(text::Shaping::Advanced),
            text("-".repeat(game_id.chars().count())),
            game_ids
        ]
        .padding(PADDING);
        let attacker = t!("attacker");
        let attackers = column![
            text(attacker.to_string()).shaping(text::Shaping::Advanced),
            text("-".repeat(attacker.chars().count())),
            attackers
        ]
        .padding(PADDING);
        let defender = t!("defender");
        let defenders = column![
            text(defender.to_string()).shaping(text::Shaping::Advanced),
            text("-".repeat(defender.chars().count())),
            defenders
        ]
        .padding(PADDING);
        let rated = t!("rated");
        let ratings = column![
            text(rated.to_string()).shaping(text::Shaping::Advanced),
            text("-".repeat(rated.chars().count())),
            ratings
        ]
        .padding(PADDING);
        let timed = t!("timed");
        let timings = column![
            text(timed.to_string()).shaping(text::Shaping::Advanced),
            text("-".repeat(timed.chars().count())),
            timings
        ]
        .padding(PADDING);
        let size = t!("size");
        let sizes = column![
            text(size.to_string()).shaping(text::Shaping::Advanced),
            text("-".repeat(size.chars().count())),
            sizes
        ]
        .padding(PADDING);
        let buttons = column![text(""), text(""), buttons].padding(PADDING);

        scrollable(row![
            game_ids, attackers, defenders, ratings, timings, sizes, buttons
        ])
    }

    fn handle_play(&mut self, role: Option<&str>, from: &str, to: &str) {
        self.captures = HashSet::new();

        let game = self.game.as_mut().expect("you should have a game by now");

        match role {
            Some(role) => match game.read_line(&format!("play {role} {from} {to}\n")) {
                Ok(vertexes) => {
                    if let Some(vertexes) = vertexes {
                        for vertex in vertexes.split_ascii_whitespace() {
                            let Ok(vertex) = Vertex::from_str(vertex) else {
                                panic!("this should be a valid vertex");
                            };
                            self.captures.insert(vertex);
                        }
                    }
                }
                Err(error) => {
                    error!("{error}");
                    exit(1)
                }
            },
            None => match game.read_line(&format!("play {} {from} {to}\n", game.turn)) {
                Ok(vertexes) => {
                    if let Some(vertexes) = vertexes {
                        for vertex in vertexes.split_ascii_whitespace() {
                            let Ok(vertex) = Vertex::from_str(vertex) else {
                                panic!("this should be a valid vertex");
                            };
                            self.captures.insert(vertex);
                        }
                    }
                }
                Err(error) => {
                    error!("{error}");
                    exit(1)
                }
            },
        }

        if self.sound_muted {
            return;
        }

        let capture = !self.captures.is_empty();

        thread::spawn(move || {
            let mut stream = rodio::OutputStreamBuilder::open_default_stream()?;
            let cursor = if capture {
                let capture_ogg = include_bytes!("capture.ogg").to_vec();
                Cursor::new(capture_ogg)
            } else {
                let move_ogg = include_bytes!("move.ogg").to_vec();
                Cursor::new(move_ogg)
            };
            let sound = rodio::play(stream.mixer(), cursor)?;
            sound.set_volume(1.0);
            sound.sleep_until_end();

            stream.log_on_drop(false);
            Ok::<(), anyhow::Error>(())
        });
    }

    #[must_use]
    fn users(&self, logged_in: bool) -> Scrollable<Message> {
        let mut ratings = Column::new();
        let mut usernames = Column::new();
        let mut wins = Column::new();
        let mut losses = Column::new();
        let mut draws = Column::new();

        for user in self.users_sorted() {
            if logged_in == user.logged_in {
                ratings = ratings.push(text(user.rating.to_string_rounded()));
                usernames = usernames.push(text(user.name).shaping(text::Shaping::Advanced));
                wins = wins.push(text(user.wins));
                losses = losses.push(text(user.losses));
                draws = draws.push(text(user.draws));
            }
        }

        let rating = t!("rating");
        let ratings = column![
            text(rating.to_string()).shaping(text::Shaping::Advanced),
            text("-".repeat(rating.chars().count())),
            ratings
        ]
        .padding(PADDING);
        let username = t!("username");
        let usernames = column![
            text(username.to_string()).shaping(text::Shaping::Advanced),
            text("-".repeat(username.chars().count())),
            usernames
        ]
        .padding(PADDING);
        let win = t!("wins");
        let wins = column![
            text(win.to_string()).shaping(text::Shaping::Advanced),
            text("-".repeat(win.chars().count())),
            wins
        ]
        .padding(PADDING);
        let loss = t!("losses");
        let losses = column![
            text(loss.to_string()).shaping(text::Shaping::Advanced),
            text("-".repeat(loss.chars().count())),
            losses
        ]
        .padding(PADDING);
        let draw = t!("draws");
        let draws = column![
            text(draw.to_string()).shaping(text::Shaping::Advanced),
            text("-".repeat(draw.chars().count())),
            draws
        ]
        .padding(PADDING);

        scrollable(row![ratings, usernames, wins, losses, draws])
    }

    #[must_use]
    fn user_area(&self, in_game: bool) -> Container<Message> {
        let texts = if in_game {
            &self.texts_game
        } else {
            &self.texts
        };

        let games = self.games();
        let texting = self.texting(texts).padding(PADDING);
        let users = self.users(true);

        let user_area = scrollable(column![games, users, texting]);
        container(user_area)
            .padding(PADDING)
            .style(container::bordered_box)
    }

    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn view(&self) -> Element<Message> {
        match self.screen {
            Screen::AccountSettings => {
                let mut rating = String::new();
                let mut wins = String::new();
                let mut draws = String::new();
                let mut losses = String::new();

                for user in self.users.values() {
                    if self.username == user.name {
                        rating = user.rating.to_string_rounded();
                        wins.clone_from(&user.wins);
                        losses.clone_from(&user.losses);
                        draws.clone_from(&user.draws);
                    }
                }

                let mut columns = column![
                    text!(
                        "{} {} {} TCP",
                        t!("connected to"),
                        &self.connected_to,
                        t!("via")
                    )
                    .shaping(text::Shaping::Advanced),
                    text!("{}: {}", t!("username"), &self.username)
                        .shaping(text::Shaping::Advanced),
                    text!("{}: {rating}", t!("rating")).shaping(text::Shaping::Advanced),
                    text!("{}: {wins}", t!("wins")).shaping(text::Shaping::Advanced),
                    text!("{}: {losses}", t!("losses")).shaping(text::Shaping::Advanced),
                    text!("{}: {draws}", t!("draws")).shaping(text::Shaping::Advanced),
                ]
                .padding(PADDING)
                .spacing(SPACING);

                if let Some(email) = &self.email {
                    let mut row = Row::new();
                    if email.verified {
                        row = row.push(
                            text!(
                                "{}: [{}] {} ",
                                t!("email address"),
                                t!("verified"),
                                email.address,
                            )
                            .shaping(text::Shaping::Advanced),
                        );
                        columns = columns.push(row);
                    } else {
                        row = row.push(
                            text!(
                                "{}: [{}] {} ",
                                t!("email address"),
                                t!("unverified"),
                                email.address,
                            )
                            .shaping(text::Shaping::Advanced),
                        );
                        columns = columns.push(row);

                        let mut row = Row::new();
                        row = row.push(
                            text!("{}: ", t!("email code)")).shaping(text::Shaping::Advanced),
                        );
                        row = row.push(
                            text_input("", &self.text_input)
                                .on_input(Message::TextChanged)
                                .on_paste(Message::TextChanged)
                                .on_submit(Message::TextSendEmailCode),
                        );
                        columns = columns.push(row);
                    }
                } else {
                    let mut row = Row::new();
                    row = row
                        .push(text!("{}: ", t!("email address")).shaping(text::Shaping::Advanced));
                    row = row.push(
                        text_input("", &self.text_input)
                            .on_input(Message::TextChanged)
                            .on_paste(Message::TextChanged)
                            .on_submit(Message::TextSendEmail),
                    );

                    columns = columns.push(row);
                    columns = columns.push(row![
                        text!("{}: ", t!("email code")).shaping(text::Shaping::Advanced)
                    ]);
                }

                columns = columns.push(row![
                    button(
                        text(self.strings["Reset Email"].as_str()).shaping(text::Shaping::Advanced)
                    )
                    .on_press(Message::EmailReset)
                ]);

                if let Some(error) = &self.error_email {
                    columns = columns.push(row![text!("error: {error}")]);
                }

                let mut change_password_button = button(
                    text(self.strings["Change Password"].as_str()).shaping(text::Shaping::Advanced),
                );

                if !self.password_no_save {
                    change_password_button = change_password_button.on_press(Message::TextSend);
                }

                columns = columns.push(
                    row![
                        change_password_button,
                        text_input("", &self.password)
                            .secure(!self.password_show)
                            .on_input(Message::PasswordChanged)
                            .on_paste(Message::PasswordChanged),
                    ]
                    .spacing(SPACING),
                );

                columns = columns.push(
                    checkbox(t!("show password"), self.password_show)
                        .text_shaping(text::Shaping::Advanced)
                        .on_toggle(Message::PasswordShow),
                );

                if self.delete_account {
                    columns = columns.push(
                        button(
                            text(self.strings["REALLY DELETE ACCOUNT"].as_str())
                                .shaping(text::Shaping::Advanced),
                        )
                        .on_press(Message::DeleteAccount),
                    );
                } else {
                    columns = columns.push(
                        button(
                            text(self.strings["Delete Account"].as_str())
                                .shaping(text::Shaping::Advanced),
                        )
                        .on_press(Message::DeleteAccount),
                    );
                }

                columns = columns.push(
                    button(text(self.strings["Leave"].as_str()).shaping(text::Shaping::Advanced))
                        .on_press(Message::Leave),
                );

                columns.into()
            }
            Screen::EmailEveryone => {
                let subject = row![
                    text("Subject: "),
                    text_input("", &self.text_input)
                        .on_input(Message::TextChanged)
                        .on_paste(Message::TextChanged)
                        .on_submit(Message::TextSend),
                ];

                let editor = text_editor(&self.content)
                    .placeholder("Dear User, …")
                    .on_action(Message::TextEdit);

                let send_emails = button("Send Emails").on_press(Message::TextSend);
                let leave = button("Leave").on_press(Message::Leave);
                let mut column = column![
                    subject,
                    text("From: Hnefatafl Org <no-reply@hnefatafl.org>"),
                    text("Content-Type: text/plain; charset=utf-8"),
                    text("Content-Transfer-Encoding: 7bit"),
                    text!("Date: {}", Utc::now().to_rfc2822()),
                    text("Body:"),
                    editor,
                    send_emails,
                    leave,
                    text("Bcc:")
                ]
                .spacing(SPACING)
                .padding(PADDING);

                for email in &self.emails_bcc {
                    column = column.push(text(email));
                }

                scrollable(column).into()
            }
            Screen::Game | Screen::GameReview => self.display_game(),
            Screen::GameNew => {
                let attacker = radio(
                    t!("attacker"),
                    Role::Attacker,
                    self.game_settings.role_selected,
                    Message::RoleSelected,
                )
                .text_shaping(text::Shaping::Advanced);

                let defender = radio(
                    format!("{},", t!("defender")),
                    Role::Defender,
                    self.game_settings.role_selected,
                    Message::RoleSelected,
                )
                .text_shaping(text::Shaping::Advanced);

                let rated = checkbox(t!("rated"), self.game_settings.rated.into())
                    .on_toggle(Message::RatedSelected)
                    .text_shaping(text::Shaping::Advanced);

                let mut new_game = button(
                    text(self.strings["New Game"].as_str()).shaping(text::Shaping::Advanced),
                );
                if self.game_settings.role_selected.is_some()
                    && self.game_settings.board_size.is_some()
                {
                    new_game = new_game.on_press(Message::GameSubmit);
                }

                let leave =
                    button(text(self.strings["Leave"].as_str()).shaping(text::Shaping::Advanced))
                        .on_press(Message::Leave);

                let size_11x11 = radio(
                    "11x11",
                    BoardSize::_11,
                    self.game_settings.board_size,
                    Message::BoardSizeSelected,
                );

                let size_13x13 = radio(
                    "13x13,",
                    BoardSize::_13,
                    self.game_settings.board_size,
                    Message::BoardSizeSelected,
                );

                let time = row![
                    checkbox(
                        format!("{}:", t!("timed")),
                        self.game_settings.timed.clone().into()
                    )
                    .text_shaping(text::Shaping::Advanced)
                    .on_toggle(Message::TimeCheckbox)
                ]
                .spacing(SPACING);

                let row_1 = row![
                    text!("{}:", t!("role")).shaping(text::Shaping::Advanced),
                    attacker,
                    defender,
                    rated,
                ]
                .padding(PADDING)
                .spacing(SPACING);

                let row_2 = row![
                    text!("{}:", t!("board size")).shaping(text::Shaping::Advanced),
                    size_11x11,
                    size_13x13,
                    time,
                ]
                .padding(PADDING)
                .spacing(SPACING);

                let mut row_3 = Row::new().spacing(SPACING).padding(PADDING);
                let mut row_4 = Row::new().spacing(SPACING).padding(PADDING);

                if let TimeSettings::Timed(_) = self.game_settings.timed {
                    row_3 = row_3.push(text(t!("days")).shaping(text::Shaping::Advanced));
                    row_3 = row_3.push(
                        text_input("0", &self.game_settings.time_days)
                            .on_input(Message::TimeDays)
                            .on_paste(Message::TimeDays),
                    );
                    row_3 = row_3.push(text(t!("hours")).shaping(text::Shaping::Advanced));
                    row_3 = row_3.push(
                        text_input("0", &self.game_settings.time_hours)
                            .on_input(Message::TimeHours)
                            .on_paste(Message::TimeHours),
                    );
                    row_3 = row_3.push(text(t!("minutes")).shaping(text::Shaping::Advanced));
                    row_3 = row_3.push(
                        text_input("15", &self.game_settings.time_minutes)
                            .on_input(Message::TimeMinutes)
                            .on_paste(Message::TimeMinutes),
                    );

                    row_4 = row_4.push(text(t!("add hours")).shaping(text::Shaping::Advanced));
                    row_4 = row_4.push(
                        text_input("0", &self.game_settings.time_add_hours)
                            .on_input(Message::TimeAddHours)
                            .on_paste(Message::TimeAddHours),
                    );
                    row_4 = row_4.push(text(t!("add minutes")).shaping(text::Shaping::Advanced));
                    row_4 = row_4.push(
                        text_input("0", &self.game_settings.time_add_minutes)
                            .on_input(Message::TimeAddMinutes)
                            .on_paste(Message::TimeAddMinutes),
                    );
                    row_4 = row_4.push(text(t!("add seconds")).shaping(text::Shaping::Advanced));
                    row_4 = row_4.push(
                        text_input("10", &self.game_settings.time_add_seconds)
                            .on_input(Message::TimeAddSeconds)
                            .on_paste(Message::TimeAddSeconds),
                    );
                }

                let row_5 = row![new_game, leave].padding(PADDING).spacing(SPACING);
                column![row_1, row_2, row_3, row_4, row_5].into()
            }
            Screen::GameNewFrozen => {
                let mut buttons_live = false;
                let mut game_display = Column::new().padding(PADDING);

                if let Some(game) = self.games_light.0.get(&self.game_id) {
                    game_display = game_display
                        .push(text(display_game(game)).shaping(text::Shaping::Advanced));

                    if game.attacker.is_some() && game.defender.is_some() {
                        buttons_live = true;
                    }
                }

                let mut buttons = if self.challenger {
                    row![
                        button(
                            text(self.strings["Leave"].as_str()).shaping(text::Shaping::Advanced)
                        )
                        .on_press(Message::Leave)
                    ]
                } else if buttons_live {
                    row![
                        button(
                            text(self.strings["Accept"].as_str()).shaping(text::Shaping::Advanced)
                        )
                        .on_press(Message::GameAccept(self.game_id)),
                        button(
                            text(self.strings["Decline"].as_str()).shaping(text::Shaping::Advanced)
                        )
                        .on_press(Message::GameDecline(self.game_id)),
                        button(
                            text(self.strings["Leave"].as_str()).shaping(text::Shaping::Advanced)
                        )
                        .on_press(Message::Leave),
                    ]
                } else {
                    row![
                        button(
                            text(self.strings["Accept"].as_str()).shaping(text::Shaping::Advanced)
                        ),
                        button(
                            text(self.strings["Decline"].as_str()).shaping(text::Shaping::Advanced)
                        ),
                        button(
                            text(self.strings["Leave"].as_str()).shaping(text::Shaping::Advanced)
                        )
                        .on_press(Message::Leave),
                    ]
                };
                buttons = buttons.padding(PADDING).spacing(SPACING);

                game_display.push(buttons).into()
            }
            Screen::Games => {
                let mut theme = if self.theme == Theme::Light {
                    row![
                        button(
                            text(self.strings["Dark"].as_str()).shaping(text::Shaping::Advanced)
                        )
                        .on_press(Message::ChangeTheme(Theme::Dark)),
                        button(
                            text(self.strings["Light"].as_str()).shaping(text::Shaping::Advanced)
                        ),
                    ]
                } else {
                    row![
                        button(
                            text(self.strings["Dark"].as_str()).shaping(text::Shaping::Advanced)
                        ),
                        button(
                            text(self.strings["Light"].as_str()).shaping(text::Shaping::Advanced)
                        )
                        .on_press(Message::ChangeTheme(Theme::Light)),
                    ]
                };

                if self.email_everyone {
                    let email_everyone = button("Email Everyone").on_press(Message::EmailEveryone);
                    theme = theme.push(email_everyone);
                }

                let theme = theme.spacing(SPACING);

                let username = row![
                    text!("{}: {}", t!("username"), &self.username)
                        .shaping(text::Shaping::Advanced)
                ]
                .spacing(SPACING);

                let username = container(username)
                    .padding(PADDING / 2)
                    .style(container::bordered_box);

                let my_games = checkbox(t!("My Games Only"), self.my_games_only)
                    .size(32)
                    .text_shaping(text::Shaping::Advanced)
                    .on_toggle(Message::MyGamesOnly);

                let get_archived_games = button(
                    text(self.strings["Get Archived Games"].as_str())
                        .shaping(text::Shaping::Advanced),
                )
                .on_press(Message::ArchivedGamesGet);

                let username = row![username, get_archived_games, my_games].spacing(SPACING);

                let create_game = button(
                    text(self.strings["Create Game"].as_str()).shaping(text::Shaping::Advanced),
                )
                .on_press(Message::GameNew);

                let users =
                    button(text(self.strings["Users"].as_str()).shaping(text::Shaping::Advanced))
                        .on_press(Message::Users);

                let account_setting = button(
                    text(self.strings["Account Settings"].as_str())
                        .shaping(text::Shaping::Advanced),
                )
                .on_press(Message::AccountSettings);

                let website =
                    button(text(self.strings["Rules"].as_str()).shaping(text::Shaping::Advanced))
                        .on_press(Message::OpenUrl(
                            "https://hnefatafl.org/rules.html".to_string(),
                        ));

                let quit =
                    button(text(self.strings["Quit"].as_str()).shaping(text::Shaping::Advanced))
                        .on_press(Message::Leave);

                let top = row![create_game, users, account_setting, website, quit].spacing(SPACING);
                let user_area = self.user_area(false);

                let mut review_game = button(
                    text(self.strings["Review Game"].as_str()).shaping(text::Shaping::Advanced),
                );
                if self.archived_game_selected.is_some() {
                    review_game = review_game.on_press(Message::ReviewGame);
                }

                let review_game_pick = pick_list(
                    self.archived_games_filtered.clone(),
                    self.archived_game_selected.clone(),
                    Message::ArchivedGameSelected,
                )
                .text_shaping(text::Shaping::Advanced);

                let review_game = row![review_game, review_game_pick].spacing(SPACING);

                column![theme, username, review_game, top, user_area]
                    .padding(PADDING)
                    .spacing(SPACING)
                    .into()
            }
            Screen::Login => {
                let username = row![
                    text!("{}:", t!("username"))
                        .shaping(text::Shaping::Advanced)
                        .size(20),
                    text_input("", &self.text_input)
                        .on_input(Message::TextChanged)
                        .on_paste(Message::TextChanged),
                ]
                .spacing(SPACING);

                let username = container(username)
                    .padding(PADDING)
                    .style(container::bordered_box);

                let password = row![
                    text!("{}:", t!("password"))
                        .shaping(text::Shaping::Advanced)
                        .size(20),
                    text_input("", &self.password)
                        .secure(!self.password_show)
                        .on_input(Message::PasswordChanged)
                        .on_paste(Message::PasswordChanged),
                ]
                .spacing(SPACING);

                let password = container(password)
                    .padding(PADDING)
                    .style(container::bordered_box);

                let show_password = checkbox(t!("show password"), self.password_show)
                    .text_shaping(text::Shaping::Advanced)
                    .on_toggle(Message::PasswordShow);

                let mut login =
                    button(text(self.strings["Login"].as_str()).shaping(text::Shaping::Advanced));
                if !self.password_no_save {
                    login = login.on_press(Message::TextSendLogin);
                }

                let mut create_account = button(
                    text(self.strings["Create Account"].as_str()).shaping(text::Shaping::Advanced),
                );
                if !self.text_input.is_empty() && !self.password_no_save {
                    create_account = create_account.on_press(Message::TextSendCreateAccount);
                }

                let mut reset_password = button(
                    text(self.strings["Reset Password"].as_str()).shaping(text::Shaping::Advanced),
                );
                if !self.text_input.is_empty() {
                    reset_password =
                        reset_password.on_press(Message::ResetPassword(self.text_input.clone()));
                }

                let website = button("https://hnefatafl.org")
                    .on_press(Message::OpenUrl("https://hnefatafl.org".to_string()));

                let quit =
                    button(text(self.strings["Quit"].as_str()).shaping(text::Shaping::Advanced))
                        .on_press(Message::Leave);

                let buttons =
                    row![login, create_account, reset_password, website, quit].spacing(SPACING);

                let mut error = text("");
                if let Some(error_) = &self.error {
                    error = text(error_);
                }

                let mut error_persistent = text("");
                if let Some(error_) = &self.error_persistent {
                    error_persistent = text(error_);
                }

                let locale = [
                    Locale::English,
                    Locale::Chinese,
                    Locale::Spanish,
                    Locale::Arabic,
                    Locale::Indonesian,
                    Locale::PortugueseBr,
                    Locale::PortuguesePt,
                    Locale::French,
                    Locale::Japanese,
                    Locale::Russian,
                    Locale::German,
                    Locale::Icelandic,
                    Locale::IcelandicRunic,
                    Locale::Swedish,
                ];

                let locale = row![
                    text!("{}: ", t!("locale"))
                        .shaping(text::Shaping::Advanced)
                        .size(20),
                    pick_list(locale, Some(self.locale_selected), Message::LocaleSelected)
                        .text_shaping(text::Shaping::Advanced),
                ];

                column![
                    username,
                    password,
                    show_password,
                    buttons,
                    locale,
                    error,
                    error_persistent
                ]
                .padding(PADDING)
                .spacing(SPACING)
                .into()
            }
            Screen::Users => scrollable(column![
                text(t!("logged in")).shaping(text::Shaping::Advanced),
                self.users(true),
                text(t!("logged out")).shaping(text::Shaping::Advanced),
                self.users(false),
                row![
                    button(text(self.strings["Leave"].as_str()).shaping(text::Shaping::Advanced))
                        .on_press(Message::Leave)
                ]
                .padding(PADDING),
            ])
            .spacing(SPACING)
            .into(),
        }
    }

    fn save_client(&self) {
        if let Ok(string) = ron::ser::to_string_pretty(&self, ron::ser::PrettyConfig::default()) {
            if !string.trim().is_empty() {
                let data_file = data_file();

                if let Ok(mut file) = File::create(&data_file) {
                    if let Err(error) = file.write_all(string.as_bytes()) {
                        error!("{error}");
                    }
                }
            }
        }
    }

    fn send(&mut self, string: String) {
        handle_error(
            self.tx
                .as_mut()
                .expect("you should have a tx available by now")
                .send(string),
        );
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
enum Locale {
    #[default]
    English,
    Chinese,
    Spanish,
    Arabic,
    Indonesian,
    PortugueseBr,
    PortuguesePt,
    French,
    Japanese,
    Russian,
    German,
    Icelandic,
    IcelandicRunic,
    Swedish,
}

impl Locale {
    fn txt(self) -> String {
        match self {
            Self::English => "en-US".to_string(),
            Self::Chinese => "zh-CN".to_string(),
            Self::Spanish => "es".to_string(),
            Self::Arabic => "ar".to_string(),
            Self::Indonesian => "id".to_string(),
            Self::PortugueseBr => "pt-BR".to_string(),
            Self::PortuguesePt => "pt-PT".to_string(),
            Self::French => "fr".to_string(),
            Self::Japanese => "ja".to_string(),
            Self::Russian => "ru".to_string(),
            Self::German => "de".to_string(),
            Self::Icelandic => "is-IS".to_string(),
            Self::IcelandicRunic => "is-RU".to_string(),
            Self::Swedish => "sv-SE".to_string(),
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::English => write!(f, "English (United States)"),
            Self::Chinese => write!(f, "中文 (中国)"),
            Self::Spanish => write!(f, "Español"),
            Self::Arabic => write!(f, "العربية"),
            Self::Indonesian => write!(f, "bahasa Indonesia"),
            Self::PortugueseBr => write!(f, "Português (Brazil)"),
            Self::PortuguesePt => write!(f, "Português (Portugal)"),
            Self::French => write!(f, "Français"),
            Self::Japanese => write!(f, "日本人"),
            Self::Russian => write!(f, "Русский"),
            Self::German => write!(f, "Deutsch"),
            Self::Icelandic => write!(f, "Íslenska"),
            Self::IcelandicRunic => write!(f, "ᛇᛋᛚᛂᚿᛋᚴᛁ ᚱᚤᛐᚢᚱᛁᚿᚿ (Íslenska Rúturinn)"),
            Self::Swedish => write!(f, "Svenska"),
        }
    }
}

#[derive(Clone, Debug)]
enum Message {
    AccountSettings,
    ArchivedGames(Vec<ArchivedGame>),
    ArchivedGamesGet,
    ArchivedGameSelected(ArchivedGame),
    BoardSizeSelected(BoardSize),
    ChangeTheme(Theme),
    ConnectedTo(String),
    DeleteAccount,
    EmailEveryone,
    EmailReset,
    FocusPrevious,
    FocusNext,
    GameAccept(usize),
    GameDecline(usize),
    GameJoin(usize),
    GameNew,
    GameResume(usize),
    GameSubmit,
    GameWatch(usize),
    Leave,
    LocaleSelected(Locale),
    MyGamesOnly(bool),
    OpenUrl(String),
    PasswordChanged(String),
    PasswordShow(bool),
    PlayDraw,
    PlayDrawDecision(Draw),
    PlayMoveFrom(Vertex),
    PlayMoveTo(Vertex),
    PlayMoveRevert,
    PlayResign,
    SoundMuted(bool),
    RatedSelected(bool),
    ResetPassword(String),
    ReviewGame,
    ReviewGameBackward,
    ReviewGameBackwardAll,
    ReviewGameForward,
    ReviewGameForwardAll,
    RoleSelected(Role),
    StreamConnected(mpsc::Sender<String>),
    TextChanged(String),
    TextEdit(text_editor::Action),
    TextReceived(String),
    TextSend,
    TextSendEmail,
    TextSendEmailCode,
    TextSendCreateAccount,
    TextSendLogin,
    Tick,
    TimeAddHours(String),
    TimeAddMinutes(String),
    TimeAddSeconds(String),
    TimeCheckbox(bool),
    TimeDays(String),
    TimeHours(String),
    TimeMinutes(String),
    Users,
    WindowResized((f32, f32)),
}

fn data_file() -> PathBuf {
    let mut data_file = if let Some(data_file) = dirs::data_dir() {
        data_file
    } else {
        PathBuf::new()
    };

    data_file.push("hnefatafl.ron");
    data_file
}

fn pass_messages() -> impl Stream<Item = Message> {
    stream::channel(
        100,
        move |mut sender: iced::futures::channel::mpsc::Sender<Message>| async move {
            let (tx, rx) = mpsc::channel();
            let _ = sender.send(Message::StreamConnected(tx)).await;

            let mut args = Args::parse();
            args.host.push_str(PORT);
            let address = args.host;

            thread::spawn(move || {
                loop {
                    let message = handle_error(rx.recv());
                    let message_trim = message.trim();

                    if message_trim == "quit" {
                        exit(0)
                    }

                    if message_trim == "tcp_connect" {
                        break;
                    }
                }

                let mut tcp_stream = handle_error(TcpStream::connect(&address));
                let mut reader = BufReader::new(handle_error(tcp_stream.try_clone()));
                info!("connected to {address} ...");

                thread::spawn(move || {
                    loop {
                        let message = handle_error(rx.recv());
                        let message_trim = message.trim();

                        if message_trim == "ping" {
                            trace!("<- {message_trim}");
                        } else {
                            debug!("<- {message_trim}");
                        }

                        if message_trim != "quit" {
                            handle_error(tcp_stream.write_all(message.as_bytes()));
                        }

                        if message_trim == "delete_account"
                            || message_trim == "logout"
                            || message_trim == "quit"
                        {
                            #[cfg(not(target_os = "redox"))]
                            tcp_stream
                                .shutdown(Shutdown::Both)
                                .expect("shutdown call failed");

                            exit(0);
                        }
                    }
                });

                let mut buffer = String::new();
                handle_error(executor::block_on(
                    sender.send(Message::ConnectedTo(address.to_string())),
                ));

                loop {
                    let bytes = handle_error(reader.read_line(&mut buffer));
                    if bytes > 0 {
                        let buffer_trim = buffer.trim();
                        let buffer_trim_vec: Vec<_> =
                            buffer_trim.split_ascii_whitespace().collect();

                        if buffer_trim_vec[1] == "display_users"
                            || buffer_trim_vec[1] == "display_games"
                            || buffer_trim_vec[1] == "ping"
                        {
                            trace!("-> {buffer_trim}");
                        } else {
                            debug!("-> {buffer_trim}");
                        }

                        handle_error(executor::block_on(
                            sender.send(Message::TextReceived(buffer.clone())),
                        ));

                        if buffer_trim_vec[1] == "archived_games" {
                            let length = handle_error(buffer_trim_vec[2].parse::<usize>());
                            let mut buf = vec![0; length];
                            handle_error(reader.read_exact(&mut buf));
                            let archived_games: Vec<ArchivedGame> =
                                handle_error(postcard::from_bytes(&buf));

                            handle_error(executor::block_on(
                                sender.send(Message::ArchivedGames(archived_games)),
                            ));
                        }

                        buffer.clear();
                    } else {
                        info!("the TCP stream has closed");
                        break;
                    }
                }
            });
        },
    )
}

fn handle_error<T, E: fmt::Display>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => {
            error!("{error}");
            exit(1)
        }
    }
}

fn init_logger() {
    let mut builder = Builder::new();

    builder.format(|formatter, record| {
        writeln!(
            formatter,
            "{} [{}] ({}): {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S %z"),
            record.level(),
            record.target(),
            record.args()
        )
    });

    if let Ok(var) = env::var("RUST_LOG") {
        builder.parse_filters(&var);
    } else {
        // if no RUST_LOG provided, default to logging at the Info level
        builder.filter(None, LevelFilter::Info);
    }

    builder.init();
}

fn open_url(url: &str) {
    if let Err(error) = webbrowser::open(url) {
        error!("{error}");
    }
}

fn split_whitespace(string: &str) -> (String, bool) {
    let mut ends_with_whitespace = false;

    if string.ends_with(|ch: char| ch.is_whitespace()) {
        ends_with_whitespace = true;
    }

    let mut string: String = string.split_whitespace().collect();

    if string.is_empty() {
        ends_with_whitespace = false;
    }

    if ends_with_whitespace {
        string.push(' ');
    }

    (string, ends_with_whitespace)
}

fn text_collect(text: SplitAsciiWhitespace<'_>) -> String {
    let text: Vec<&str> = text.collect();
    text.join(" ")
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
enum Size {
    Tiny,
    #[default]
    Small,
    Medium,
    Large,
    Giant,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
enum Theme {
    #[default]
    Dark,
    Light,
}

#[derive(Clone, Debug)]
struct User {
    name: String,
    wins: String,
    losses: String,
    draws: String,
    rating: Rating,
    logged_in: bool,
}

fn display_game(game: &ServerGameLight) -> String {
    let attacker = t!("attacker");
    let defender = t!("defender");
    let rated = t!(game.rated.to_string());
    let none = t!("none");

    let attacker = if let Some(name) = &game.attacker {
        &format!("{attacker}: {name}")
    } else {
        &format!("{attacker}: {none}")
    };

    let defender = if let Some(name) = &game.defender {
        &format!("{defender}: {name}")
    } else {
        &format!("{defender}: {none}")
    };

    format!(
        "# {}\n{attacker}, {defender}, {rated}\n{}: {}, {}: {}",
        game.id,
        t!("time"),
        game.timed,
        t!("board size"),
        game.board_size,
    )
}
