// Don't open the terminal on Windows.
#![windows_subsystem = "windows"]

use std::{
    collections::{HashMap, HashSet, VecDeque},
    f64,
    fmt::{self, Write as fmt_write},
    fs::{self, File},
    io::{BufRead, BufReader, Cursor, ErrorKind, Read, Write},
    net::{Shutdown, TcpStream},
    ops::Not,
    process::exit,
    str::{FromStr, SplitAsciiWhitespace},
    sync::mpsc,
    thread,
};

use chrono::{Local, Utc};
use clap::{CommandFactory, Parser, command};
use futures::{SinkExt, executor};
use hnefatafl_copenhagen::{
    COPYRIGHT, Id, LONG_VERSION, SERVER_PORT, VERSION_ID,
    accounts::Email,
    ai::GenerateMove,
    board::{Board, BoardSize},
    characters::Characters,
    client::{Move, Size, Theme, User},
    draw::Draw,
    game::{Game, LegalMoves, TimeUnix},
    glicko::{CONFIDENCE_INTERVAL_95, Rating},
    heat_map::{Heat, HeatMap},
    locale::Locale,
    play::{BOARD_LETTERS, Plae, Plays, Vertex},
    rating::Rated,
    role::Role,
    server_game::{ArchivedGame, ArchivedGameHandle, ServerGameLight, ServerGamesLight},
    space::Space,
    status::Status,
    time::{Time, TimeEnum, TimeSettings},
    tree::{Node, Tree},
    utils::{self, choose_ai, data_file},
};
#[cfg(target_os = "linux")]
use iced::window::settings::PlatformSpecific;
use iced::{
    Color, Element, Event, Font, Pixels, Subscription, Task,
    alignment::{Horizontal, Vertical},
    color, event,
    futures::Stream,
    keyboard::{self, Key, key::Named},
    stream,
    theme::Palette,
    widget::{
        self, Column, Container, Row, Scrollable, button, checkbox, column, container,
        operation::{focus_next, focus_previous},
        pick_list, radio, row, scrollable, text, text_editor,
        text_input::Value,
        tooltip,
    },
    window::{self, icon},
};
use log::{debug, error, info, trace};
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use smol_str::ToSmolStr;

/*
SOLARIZED HEX       16/8 TERMCOL  XTERM/HEX     L*A*B      RGB         HSB
--------- -------   ---- -------  -----------   ---------- ----------- -----------
base03    #002b36  8/4 brblack  234 #1c1c1c 15 -12 -12   0  43  54 193 100  21
base02    #073642  0/4 black    235 #262626 20 -12 -12   7  54  66 192  90  26
base01    #586e75 10/7 brgreen  240 #585858 45 -07 -07  88 110 117 194  25  46
base00    #657b83 11/7 bryellow 241 #626262 50 -07 -07 101 123 131 195  23  51

base0     #839496 12/6 brblue   244 #808080 60 -06 -03 131 148 150 186  13  59
base1     #93a1a1 14/4 brcyan   245 #8a8a8a 65 -05 -02 147 161 161 180   9  63
base2     #eee8d5  7/7 white    254 #e4e4e4 92 -00  10 238 232 213  44  11  93
base3     #fdf6e3 15/7 brwhite  230 #ffffd7 97  00  10 253 246 227  44  10  99

yellow    #b58900  3/3 yellow   136 #af8700 60  10  65 181 137   0  45 100  71
orange    #cb4b16  9/3 brred    166 #d75f00 50  50  55 203  75  22  18  89  80
red       #dc322f  1/1 red      160 #d70000 50  65  45 220  50  47   1  79  86
magenta   #d33682  5/5 magenta  125 #af005f 50  65 -05 211  54 130 331  74  83
violet    #6c71c4 13/5 brmagenta 61 #5f5faf 50  15 -45 108 113 196 237  45  77
blue      #268bd2  4/4 blue      33 #0087ff 55 -10 -45  38 139 210 205  82  82
cyan      #2aa198  6/6 cyan      37 #00afaf 60 -35 -05  42 161 152 175  74  63
green     #859900  2/2 green     64 #5f8700 60 -20  65 133 153   0  68 100  60
*/

/// The [Tol] variant of a [`Palette`]. A palette for the color blind
///
/// [Tol]: https://www.nceas.ucsb.edu/sites/default/files/2022-06/Colorblind%20Safe%20Color%20Schemes.pdf
pub const TOL: Palette = Palette {
    background: color!(46, 37, 133), // Background
    text: color!(221, 221, 221),     // Foreground
    primary: color!(148, 203, 236),  // Blue
    success: color!(51, 117, 56),    // Green
    warning: color!(220, 205, 125),  // Yellow
    danger: color!(194, 106, 119),   // Red
};

const ALPHABET: [char; 26] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

const BLUE: Color = color!(38, 139, 210);
const BOARD_LETTERS_LOWERCASE: [char; 13] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
];

const USER_CONFIG_FILE_POSTCARD: &str = "hnefatafl.postcard";
const USER_CONFIG_FILE_RON: &str = "hnefatafl.ron";

const PADDING: u16 = 10;
const SPACING: Pixels = Pixels(10.0);
const SPACING_B: Pixels = Pixels(20.0);

rust_i18n::i18n!();

/// Hnefatafl Copenhagen Client
///
/// This is a TCP client that connects to a server.
#[allow(clippy::struct_excessive_bools)]
#[derive(Parser, Debug)]
#[command(long_version = LONG_VERSION, about = "Copenhagen Hnefatafl Client")]
struct Args {
    /// Connect to the server at host
    #[arg(default_value = "hnefatafl.org", long)]
    host: String,

    /// What AI to use for Heat Map
    #[arg(default_value = "monte-carlo", long)]
    ai: String,

    /// How many seconds to run the monte-carlo AI
    #[arg(long)]
    seconds: Option<u64>,

    /// How deep in the game tree to go with the AI
    #[arg(long)]
    depth: Option<u8>,

    /// Make the window size tiny
    #[arg(long)]
    tiny_window: bool,

    /// Make the window size appropriate for social preview
    #[arg(long)]
    social_preview: bool,

    /// Render everything in ASCII
    #[arg(long)]
    ascii: bool,

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
    strings.insert("Heat Map".to_string(), t!("Heat Map").to_string());
    strings.insert("Join Discord".to_string(), t!("Join Discord").to_string());
    strings.insert("Cancel".to_string(), t!("Cancel").to_string());

    strings
}

fn init_client() -> Client {
    let user_config_file_postcard = data_file(USER_CONFIG_FILE_POSTCARD);
    let user_config_file_ron = data_file(USER_CONFIG_FILE_RON);
    let mut error = Vec::new();

    let mut client: Client = match &fs::read_to_string(&user_config_file_ron) {
        Ok(string) => match ron::from_str(string) {
            Ok(client) => client,
            Err(err) => {
                error.push(format!(
                    "Error parsing the ron file {}: {err}",
                    user_config_file_ron.display()
                ));
                Client::default()
            }
        },
        Err(err) => {
            if err.kind() == ErrorKind::NotFound {
                error.push(format!(
                    "Unable to find User Configuration file: {}",
                    user_config_file_ron.display()
                ));
                Client::default()
            } else {
                error.push(format!(
                    "Error opening the file {}: {err}",
                    user_config_file_ron.display()
                ));
                Client::default()
            }
        }
    };

    rust_i18n::set_locale(&client.locale_selected.txt());
    client.strings = i18n_buttons();
    client.text_input.clone_from(&client.username);

    let archived_games: Vec<ArchivedGame> = match &fs::read(&user_config_file_postcard) {
        Ok(bytes) => match postcard::from_bytes(bytes) {
            Ok(client) => client,
            Err(err) => {
                error.push(format!(
                    "Error parsing the postcard file {}: {err}",
                    user_config_file_postcard.display()
                ));
                Vec::new()
            }
        },
        Err(err) => {
            if err.kind() == ErrorKind::NotFound {
                error.push(format!(
                    "{}: {}",
                    t!("Unable to find Archived Games file"),
                    user_config_file_postcard.display()
                ));
                Vec::new()
            } else {
                error.push(format!(
                    "{} {}: {err}",
                    t!("Error opening the file"),
                    user_config_file_postcard.display()
                ));
                Vec::new()
            }
        }
    };

    client.archived_games = archived_games;
    client.error_persistent = error;

    let args = Args::parse();
    if args.ascii {
        client.chars.ascii();
    }

    let mut letters = HashMap::new();
    for ch in BOARD_LETTERS_LOWERCASE {
        letters.insert(ch, false);
    }

    client
}

fn main() -> anyhow::Result<()> {
    utils::init_logger(false);
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
        .theme(Client::theme);

    // For screenshots.
    if args.tiny_window {
        application = application.window_size(iced::Size {
            width: 868.0,
            height: 541.0,
        });
    }

    if args.social_preview {
        application = application.window_size(iced::Size {
            width: 1148.0,
            height: 481.0,
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
    #[serde(default)]
    archived_games: Vec<ArchivedGame>,
    #[serde(skip)]
    archived_games_filtered: Option<Vec<ArchivedGame>>,
    #[serde(skip)]
    archived_game_selected: Option<ArchivedGame>,
    #[serde(skip)]
    archived_game_handle: Option<ArchivedGameHandle>,
    #[serde(default)]
    coordinates: Coordinates,
    #[serde(skip)]
    defender: String,
    #[serde(skip)]
    delete_account: bool,
    #[serde(skip)]
    email_everyone: bool,
    #[serde(skip)]
    estimate_score: bool,
    #[serde(skip)]
    estimate_score_tx: Option<mpsc::Sender<Tree>>,
    #[serde(skip)]
    captures: HashSet<Vertex>,
    #[serde(skip)]
    counter: u64,
    #[serde(skip)]
    chars: Characters,
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
    error_persistent: Vec<String>,
    #[serde(skip)]
    game: Option<Game>,
    #[serde(skip)]
    game_id: Id,
    #[serde(skip)]
    games_light: ServerGamesLight,
    #[serde(skip)]
    game_settings: NewGameSettings,
    #[serde(skip)]
    heat_map: Option<HeatMap>,
    #[serde(skip)]
    heat_map_display: bool,
    #[serde(default)]
    locale_selected: Locale,
    #[serde(default)]
    my_games_only: bool,
    #[serde(skip)]
    my_turn: bool,
    #[serde(skip)]
    now: i64,
    #[serde(skip)]
    now_diff: i64,
    #[serde(default)]
    password: String,
    #[serde(skip)]
    password_ends_with_whitespace: bool,
    #[serde(default)]
    password_save: bool,
    #[serde(default)]
    password_show: bool,
    #[serde(skip)]
    play_from: Option<Vertex>,
    #[serde(skip)]
    play_from_previous: Option<Vertex>,
    #[serde(skip)]
    play_to_previous: Option<Vertex>,
    #[serde(skip)]
    press_letters: HashSet<char>,
    #[serde(skip)]
    press_numbers: [bool; 13],
    #[serde(skip)]
    request_draw: bool,
    #[serde(skip)]
    screen: Screen,
    #[serde(skip)]
    screen_size: Size,
    #[serde(default)]
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
    board_size: BoardSize,
    #[serde(skip)]
    rated: Rated,
    #[serde(skip)]
    role_selected: Option<Role>,
    #[serde(skip)]
    timed: TimeSettings,
    #[serde(skip)]
    time: Option<TimeEnum>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
enum Screen {
    AccountSettings,
    EmailEveryone,
    #[default]
    Login,
    Game,
    GameNew,
    GameReview,
    Games,
    Users,
}

impl<'a> Client {
    fn archived_game_reset(&mut self) {
        self.archived_game_handle = None;
        self.archived_game_selected = None;
    }

    #[must_use]
    fn board(&self) -> Row<'_, Message> {
        let (board, heat_map) = self.board_and_heatmap();
        let board_size = board.size();
        let board_size_usize: usize = board_size.into();
        let d = Dimensions::new(board_size, &self.screen_size);
        let letters: Vec<_> = BOARD_LETTERS[..board_size_usize].chars().collect();
        let mut game_display = Row::new().spacing(2);
        let possible_moves = self.possible_moves();

        let coordinates: bool = self.coordinates.into();
        if coordinates {
            game_display =
                game_display.push(self.numbers(d.letter_size, d.spacing, board_size_usize));
        }

        for (x, letter) in letters.iter().enumerate() {
            let mut column = Column::new().spacing(2).align_x(Horizontal::Center);

            if coordinates {
                column = self.letter(*letter, column, d.letter_size);
            }

            for y in 0..board_size_usize {
                let vertex = Vertex {
                    size: board.size(),
                    x,
                    y,
                };

                let mut txt = match board.get(&vertex) {
                    Space::Attacker => text(&self.chars.attacker),
                    Space::Defender => text(&self.chars.defender),
                    Space::Empty => {
                        if let Some(arrow) = self.draw_arrow(y, x) {
                            text(arrow)
                        } else if self.captures.contains(&vertex) {
                            text(&self.chars.captured)
                        } else if board.on_restricted_square(&vertex) {
                            text(&self.chars.restricted_square)
                        } else {
                            text(" ")
                        }
                    }
                    Space::King => text(&self.chars.king),
                };

                if let Some((heat_map_from, heat_map_to)) = &heat_map
                    && possible_moves.is_some()
                {
                    if let Some(vertex_from) = self.play_from.as_ref() {
                        let space = board.get(vertex_from);
                        let turn = Role::from(space);
                        if let Some(heat_map_to) = heat_map_to.get(&(turn, *vertex_from)) {
                            let heat = heat_map_to[y * board_size_usize + x];

                            if heat == Heat::UnRanked {
                                txt = txt.color(Color::from_rgba(0.0, 0.0, 0.0, heat.into()));
                            } else {
                                let txt_char = match space {
                                    Space::Attacker => &self.chars.attacker,
                                    Space::Defender => &self.chars.defender,
                                    Space::Empty => "",
                                    Space::King => &self.chars.king,
                                };

                                txt = text(txt_char).color(Color::from_rgba(
                                    0.0,
                                    0.0,
                                    0.0,
                                    heat.into(),
                                ));
                            }
                        }
                    } else {
                        let heat = heat_map_from[y * board_size_usize + x];
                        txt = txt.color(Color::from_rgba(0.0, 0.0, 0.0, heat.into()));
                    }
                }

                txt = txt.font(Font::MONOSPACE).center().size(d.piece_size);
                let mut button = button(txt)
                    .width(d.board_dimension)
                    .height(d.board_dimension);

                match self.board_move(&vertex, possible_moves.as_ref()) {
                    Move::From => button = button.on_press(Message::PlayMoveFrom(vertex)),
                    Move::To => button = button.on_press(Message::PlayMoveTo(vertex)),
                    Move::Revert => button = button.on_press(Message::PlayMoveRevert),
                    Move::None => {}
                }

                column = column.push(button);
            }

            if coordinates {
                column = self.letter(*letter, column, d.letter_size);
            }

            game_display = game_display.push(column);
        }

        if coordinates {
            game_display =
                game_display.push(self.numbers(d.letter_size, d.spacing, board_size_usize));
        }

        game_display
    }

    fn board_move(&self, vertex: &Vertex, possible_moves: Option<&LegalMoves>) -> Move {
        if let Some(legal_moves) = possible_moves {
            if let Some(vertex_from) = self.play_from.as_ref() {
                if let Some(vertexes) = legal_moves.moves.get(vertex_from) {
                    if vertex == vertex_from {
                        Move::Revert
                    } else if vertexes.contains(vertex) {
                        Move::To
                    } else {
                        Move::None
                    }
                } else {
                    Move::None
                }
            } else if legal_moves.moves.contains_key(vertex) {
                Move::From
            } else {
                Move::None
            }
        } else {
            Move::None
        }
    }

    #[allow(clippy::type_complexity)]
    fn board_and_heatmap(
        &self,
    ) -> (
        Board,
        Option<(Vec<Heat>, HashMap<(Role, Vertex), Vec<Heat>>)>,
    ) {
        if let Some(game_handle) = &self.archived_game_handle {
            let node = game_handle.boards.here();

            if self.heat_map_display
                && let Some(heat_map) = &self.heat_map
            {
                (node.board.clone(), Some(heat_map.draw(node.turn)))
            } else {
                (node.board.clone(), None)
            }
        } else {
            let game = self.game.as_ref().expect("we should be in a game");

            (game.board.clone(), None)
        }
    }

    fn game_state(&self, game_id: u128) -> State {
        if let Some(game) = self.games_light.0.get(&game_id) {
            if game.challenge_accepted {
                return State::Spectator;
            }

            if game.attacker.is_none() || game.defender.is_none() {
                if let Some(attacker) = &game.attacker
                    && &self.username == attacker
                {
                    return State::CreatorOnly;
                }

                if let Some(defender) = &game.defender
                    && &self.username == defender
                {
                    return State::CreatorOnly;
                }
            }

            if let (Some(attacker), Some(defender)) = (&game.attacker, &game.defender)
                && (&self.username == attacker || &self.username == defender)
            {
                if let Some(challenger) = &game.challenger.0 {
                    if &self.username != challenger {
                        return State::Creator;
                    }
                } else {
                    return State::Challenger;
                }
            }
        }

        State::Spectator
    }

    fn clear_letters_except(&mut self, letter: char) {
        for l in BOARD_LETTERS_LOWERCASE {
            if l != letter {
                self.press_letters.remove(&l);
            }
        }
    }

    fn clear_numbers_except(&mut self, number: usize) {
        let (board, _) = self.board_and_heatmap();
        let board_size = board.size().into();

        for i in 0..board_size {
            let i = board_size - i;
            if i != number {
                self.press_numbers[i - 1] = false;
            }
        }
    }

    fn create_account(&mut self) {
        if !self.connected_tcp {
            self.send("tcp_connect\n".to_string());
            self.connected_tcp = true;
        }

        if self.screen == Screen::Login {
            if !self.text_input.trim().is_empty() {
                let username = self.text_input.clone();
                self.send(format!(
                    "{VERSION_ID} create_account {username} {}\n",
                    self.password,
                ));
                self.username = username;
            }
            self.text_input.clear();
            self.archived_game_reset();
            handle_error(self.save_client_ron());
        }
    }

    fn delete_account(&mut self) {
        if self.delete_account {
            self.send("delete_account\n".to_string());
            self.screen = Screen::Login;
        } else {
            self.delete_account = true;
        }
    }

    fn draw(&mut self) {
        let game = self.game.as_ref().expect("you should have a game by now");
        self.send(format!("request_draw {} {}\n", self.game_id, game.turn));
    }

    fn draw_arrow(&self, y: usize, x: usize) -> Option<&str> {
        if let (Some(from), Some(to)) = (&self.play_from_previous, &self.play_to_previous) {
            if (y, x) == (from.y, from.x) {
                let x_diff = from.x as i128 - to.x as i128;
                let y_diff = from.y as i128 - to.y as i128;
                let mut arrow = " ";

                if y_diff < 0 {
                    arrow = &self.chars.arrow_down;
                } else if y_diff > 0 {
                    arrow = &self.chars.arrow_up;
                } else if x_diff < 0 {
                    arrow = &self.chars.arrow_right;
                } else if x_diff > 0 {
                    arrow = &self.chars.arrow_left;
                }

                Some(arrow)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn estimate_score(&mut self) {
        if !self.estimate_score {
            info!("start running score estimator...");

            let handle = self
                .archived_game_handle
                .as_ref()
                .expect("we should have a game handle now");

            self.estimate_score = true;
            self.send_estimate_score(handle.boards.clone());
        }
    }

    fn game_new(&mut self) {
        self.game_settings = NewGameSettings::default();
        self.screen = Screen::GameNew;
    }

    fn game_submit(&mut self) {
        if let Some(role) = self.game_settings.role_selected {
            if let TimeSettings::Timed(_) = self.game_settings.timed {
                let (milliseconds_left, add_seconds) = match self.game_settings.time {
                    Some(TimeEnum::Classical) => (1_000 * 60 * 30, 20),
                    Some(TimeEnum::Rapid) => (1_000 * 60 * 15, 10),
                    Some(TimeEnum::Long) => (1_000 * 60 * 60 * 24 * 3, 60 * 60 * 6),
                    Some(TimeEnum::VeryLong) => (1_000 * 60 * 60 * 12 * 15, 60 * 60 * 15),
                    _ => unreachable!(),
                };

                self.game_settings.timed = TimeSettings::Timed(Time {
                    add_seconds,
                    milliseconds_left,
                });
            }

            self.screen = Screen::Games;

            let board_size = self.game_settings.board_size;

            // <- new_game (attacker | defender) (rated | unrated) (TIME_MINUTES | _) (ADD_SECONDS_AFTER_EACH_MOVE | _) board_size
            // -> game id rated attacker defender un-timed _ _ board_size challenger challenge_accepted spectators
            self.send(format!(
                "new_game {role} {} {:?} {board_size}\n",
                self.game_settings.rated, self.game_settings.timed,
            ));
        }
    }

    fn press_letter(&mut self, letter: char) {
        self.clear_letters_except(letter);
        self.press_letters.insert(letter);
    }

    fn press_letter_and_number(&mut self) {
        let mut number = None;
        let mut letter = None;

        for i in 0..13 {
            if self.press_numbers[i] {
                number = Some(i);
                break;
            }
        }
        for (i, l) in BOARD_LETTERS_LOWERCASE.iter().enumerate() {
            if self.press_letters.contains(l) {
                letter = Some(i);
                break;
            }
        }

        let size = if let Some(game) = self.game.as_ref() {
            Some(game.board.size())
        } else {
            self.archived_game_handle
                .as_ref()
                .map(|game| game.game.board_size)
        };

        if let (Some(size), Some(number), Some(letter)) = (size, number, letter) {
            let board_usize: usize = size.into();
            let vertex = Vertex {
                size,
                x: letter,
                y: board_usize - number - 1,
            };
            let possible_moves = self.possible_moves();

            match self.board_move(&vertex, possible_moves.as_ref()) {
                Move::From => self.play_from = Some(vertex),
                Move::To => self.play_to(vertex),
                Move::Revert => self.play_from = None,
                Move::None => {}
            }

            for i in 0..13 {
                self.press_numbers[i] = false;
            }
            self.press_letters.clear();
        }
    }

    fn join_game_press(&mut self, i: usize, shift: bool) {
        let mut server_games: Vec<&ServerGameLight> = self.games_light.0.values().collect();

        server_games.sort_by(|a, b| b.id.cmp(&a.id));

        if let Some(game) = server_games.get(i) {
            match self.join_game(game) {
                JoinGame::Cancel => self.send(format!("decline_game {} switch\n", game.id)),
                JoinGame::Join => self.join(game.id),
                JoinGame::None => match self.game_state(game.id) {
                    State::Challenger | State::Spectator => {}
                    State::Creator => {
                        if shift {
                            self.send(format!("decline_game {}\n", game.id));
                        } else {
                            self.send(format!("join_game {}\n", game.id));
                        }
                    }
                    State::CreatorOnly => self.send(format!("leave_game {}\n", game.id)),
                },
                JoinGame::Resume => self.resume(game.id),
                JoinGame::Watch => self.watch(game.id),
            }
        }
    }

    fn join_game(&self, game: &ServerGameLight) -> JoinGame {
        if game.challenge_accepted {
            if Some(&self.username) == game.attacker.as_ref()
                || Some(&self.username) == game.defender.as_ref()
            {
                JoinGame::Resume
            } else {
                JoinGame::Watch
            }
        } else if game.attacker.is_some()
            && game.defender.is_some()
            && Some(&self.username) == game.challenger.0.as_ref()
        {
            JoinGame::Cancel
        } else if (game.attacker.is_none() || game.defender.is_none())
            && !(Some(&self.username) == game.attacker.as_ref()
                || Some(&self.username) == game.defender.as_ref())
        {
            JoinGame::Join
        } else {
            JoinGame::None
        }
    }

    fn join(&mut self, id: u128) {
        self.game_id = id;
        self.send(format!("join_game_pending {id}\n"));

        let game = self.games_light.0.get(&id).expect("the game must exist");

        self.game_settings.role_selected = if game.attacker.is_some() {
            Some(Role::Defender)
        } else {
            Some(Role::Attacker)
        };
    }

    fn resume(&mut self, id: u128) {
        self.game_id = id;
        self.send(format!("resume_game {id}\n"));
    }

    fn watch(&mut self, id: u128) {
        self.game_id = id;
        self.send(format!("watch_game {id}\n"));
    }

    fn login(&mut self) {
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
            let username = self.text_input.clone();

            self.send(format!("{VERSION_ID} login {username} {}\n", self.password));
            self.username = username;
        }

        self.text_input.clear();
        self.archived_game_reset();
        handle_error(self.save_client_ron());
    }

    fn play_to(&mut self, to: Vertex) {
        let from = self
            .play_from
            .expect("you have to have a from to get to to");

        let mut turn = Role::Roleless;
        if let Some(game) = &self.game {
            turn = game.turn;
        }

        self.handle_play(None, &from.to_string(), &to.to_string());

        if self.archived_game_handle.is_none() {
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

            self.my_turn = false;
        }

        self.play_from_previous = self.play_from;
        self.play_to_previous = Some(to);
        self.play_from = None;
    }

    fn possible_moves(&self) -> Option<LegalMoves> {
        let mut possible_moves = None;

        if self.my_turn {
            if let Some(game) = self.game.as_ref() {
                possible_moves = Some(game.all_legal_moves());
            }
        } else if let Some(handle) = &self.archived_game_handle {
            let game = Game::from(&handle.boards);
            possible_moves = Some(game.all_legal_moves());
        }

        possible_moves
    }

    fn change_theme(&mut self, theme: Theme) {
        self.theme = theme;
        handle_error(self.save_client_ron());
    }

    fn coordinates(&mut self) {
        self.coordinates = !self.coordinates;
        handle_error(self.save_client_ron());
    }

    // Fixme: get the real status when exploring the game tree.
    #[allow(clippy::too_many_lines)]
    fn display_game(&self) -> Element<'_, Message> {
        let mut attacker_rating = String::new();
        let mut defender_rating = String::new();

        let (game_id, attacker, attacker_time, defender, defender_time, board, play, status, texts) =
            if let Some(game_handle) = &self.archived_game_handle {
                attacker_rating = game_handle.game.attacker_rating.to_string_rounded();
                defender_rating = game_handle.game.defender_rating.to_string_rounded();

                let status = if game_handle.play == game_handle.game.plays.len() - 1 {
                    &game_handle.game.status
                } else {
                    &Status::Ongoing
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
                    &game_handle.boards.here().board,
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

                let game = self.game.as_ref().expect("we should be in a game");

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
                    text(attacker),
                    text(attacker_rating).center(),
                    text(captured.defender(&self.chars).clone()),
                ]
                .spacing(SPACING),
                row![
                    text(attacker_time).size(35).center(),
                    text(&self.chars.dagger).size(35).center(),
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
                    text(defender),
                    text(defender_rating).center(),
                    text(captured.attacker(&self.chars).clone()),
                ]
                .spacing(SPACING),
                row![
                    text(defender_time).size(35).center(),
                    text(&self.chars.shield).size(35.0).center(),
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

        let mut user_area = column![text!("#{game_id} {}", &self.username)].spacing(SPACING);

        let is_rated = match self.game_settings.rated {
            Rated::No => t!("no"),
            Rated::Yes => t!("yes"),
        };

        user_area = user_area.push(text!("{}: {play} {}: {is_rated}", t!("move"), t!("rated")));

        match self.screen_size {
            Size::Large | Size::Medium | Size::Small | Size::Tiny => {
                user_area = user_area.push(column![attacker, defender].spacing(SPACING));
            }
            Size::Giant | Size::TinyWide => {
                user_area = user_area.push(row![attacker, defender].spacing(SPACING));
            }
        }

        let mut spectators = Column::new();
        for spectator in &self.spectators {
            if self.username.as_str() == spectator.as_str() {
                watching = true;
            }

            let mut spectator = spectator.clone();
            if let Some(user) = self.users.get(&spectator) {
                let _ok = write!(spectator, " ({})", user.rating.to_string_rounded());
            }
            spectators = spectators.push(text(spectator));
        }

        let resign =
            button(text!("{} (p)", self.strings["Resign"].as_str())).on_press(Message::PlayResign);

        let request_draw = button(text!("{} (q)", self.strings["Request Draw"].as_str()))
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
                    Size::TinyWide | Size::Medium | Size::Large | Size::Giant => {
                        user_area = user_area.push(row![resign, request_draw].spacing(SPACING));
                    }
                }
            } else {
                let row = if self.request_draw {
                    column![
                        row![
                            button(text(self.strings["Accept Draw"].as_str()))
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

        let coordinates_muted = row![
            checkbox(self.coordinates.into()).on_toggle(Message::Coordinates),
            text!("{} (n)", t!("Coordinates")),
            checkbox(self.sound_muted).on_toggle(Message::SoundMuted),
            text!("{} (o)", t!("Muted"))
        ]
        .spacing(SPACING);

        user_area = user_area.push(coordinates_muted);

        let leave =
            button(text!("{} (Esc)", self.strings["Leave"].as_str())).on_press(Message::Leave);

        match status {
            Status::AttackerWins => {
                user_area = user_area.push(text(t!("Attacker wins!")));
            }
            Status::Draw => {
                user_area = user_area.push(text(t!("It's a draw.")));
            }
            Status::Ongoing => {}
            Status::DefenderWins => {
                user_area = user_area.push(text(t!("Defender wins!")));
            }
        }

        if let Some(handle) = &self.archived_game_handle {
            let mut heat_map = checkbox(self.heat_map_display).size(32);
            if self.heat_map.is_some() {
                heat_map = heat_map.on_toggle(Message::HeatMap);
            }

            let mut heat_map_button =
                button(text!("{} (p) (q)", self.strings["Heat Map"].as_str()));

            if !self.estimate_score && *status == Status::Ongoing {
                heat_map_button = heat_map_button.on_press(Message::EstimateScore);
            }

            user_area = user_area.push(row![heat_map, heat_map_button].spacing(SPACING));
            user_area = user_area.push(leave);

            let mut left_all = button(text(&self.chars.double_arrow_left_full));
            let mut left = button(text(&self.chars.double_arrow_left));
            if handle.play > 0 {
                left_all = left_all.on_press(Message::ReviewGameBackwardAll);
                left = left.on_press(Message::ReviewGameBackward);
            }

            let mut right = button(text(&self.chars.double_arrow_right).center());
            let mut right_all = button(text(&self.chars.double_arrow_right_full).center());
            if handle.boards.has_children() {
                right = right.on_press(Message::ReviewGameForward);
                right_all = right_all.on_press(Message::ReviewGameForwardAll);
            }

            let child_number = text(handle.boards.next_child);
            let child_right = button(text(&self.chars.double_arrow_right).center())
                .on_press(Message::ReviewGameChildNext);

            user_area = user_area.push(
                row![left_all, left, right, right_all, child_right, child_number].spacing(SPACING),
            );
        } else {
            user_area = user_area.push(leave);

            let spectator = text!(
                "{} ({}) {}: {seconds:01}.{sub_second:03} s",
                &self.chars.people,
                self.spectators.len(),
                t!("lag"),
            );

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
        }

        if self.archived_game_handle.is_some() {
            user_area = user_area.push(self.texting(texts, false));
        } else {
            user_area = user_area.push(self.texting(texts, true));
        }

        let user_area = container(user_area)
            .padding(PADDING)
            .style(container::bordered_box);

        let board = container(self.board())
            .style(container::bordered_box)
            .padding(PADDING);

        row![board, user_area].spacing(SPACING).into()
    }

    fn my_games_only(&mut self) {
        let selected = !self.my_games_only;
        if selected {
            self.archived_games_filtered = Some(
                self.archived_games
                    .iter()
                    .filter(|game| game.attacker == self.username || game.defender == self.username)
                    .cloned()
                    .collect(),
            );
        } else {
            self.archived_games_filtered = None;
        }

        self.my_games_only = selected;
        handle_error(self.save_client_ron());
    }

    #[allow(clippy::too_many_lines)]
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
        let subscription_3 = Subscription::run(estimate_score);

        let subscription_4 = event::listen_with(|event, _status, _id| match event {
            Event::Window(iced::window::Event::Resized(size)) => {
                Some(Message::WindowResized((size.width, size.height)))
            }
            Event::Keyboard(event) => match event {
                keyboard::Event::KeyPressed {
                    key: Key::Character(ch),
                    modifiers,
                    ..
                } => {
                    let shift = modifiers.shift();

                    if modifiers.control() || modifiers.command() {
                        if *ch == *Value::new("a").to_smolstr() {
                            Some(Message::PressA(shift))
                        } else if *ch == *Value::new("b").to_smolstr() {
                            Some(Message::PressB(shift))
                        } else if *ch == *Value::new("c").to_smolstr() {
                            Some(Message::PressC(shift))
                        } else if *ch == *Value::new("d").to_smolstr() {
                            Some(Message::PressD(shift))
                        } else if *ch == *Value::new("e").to_smolstr() {
                            Some(Message::PressE(shift))
                        } else if *ch == *Value::new("f").to_smolstr() {
                            Some(Message::PressF(shift))
                        } else if *ch == *Value::new("g").to_smolstr() {
                            Some(Message::PressG(shift))
                        } else if *ch == *Value::new("h").to_smolstr() {
                            Some(Message::PressH(shift))
                        } else if *ch == *Value::new("i").to_smolstr() {
                            Some(Message::PressI(shift))
                        } else if *ch == *Value::new("j").to_smolstr() {
                            Some(Message::PressJ(shift))
                        } else if *ch == *Value::new("k").to_smolstr() {
                            Some(Message::PressK(shift))
                        } else if *ch == *Value::new("l").to_smolstr() {
                            Some(Message::PressL(shift))
                        } else if *ch == *Value::new("m").to_smolstr() {
                            Some(Message::PressM(shift))
                        } else if *ch == *Value::new("n").to_smolstr() {
                            Some(Message::PressN(shift))
                        } else if *ch == *Value::new("o").to_smolstr() {
                            Some(Message::PressO(shift))
                        } else if *ch == *Value::new("p").to_smolstr() {
                            Some(Message::PressP(shift))
                        } else if *ch == *Value::new("q").to_smolstr() {
                            Some(Message::PressQ(shift))
                        } else if *ch == *Value::new("r").to_smolstr() {
                            Some(Message::PressR(shift))
                        } else if *ch == *Value::new("s").to_smolstr() {
                            Some(Message::PressS(shift))
                        } else if *ch == *Value::new("t").to_smolstr() {
                            Some(Message::PressT(shift))
                        } else if *ch == *Value::new("u").to_smolstr() {
                            Some(Message::PressU(shift))
                        } else if *ch == *Value::new("v").to_smolstr() {
                            Some(Message::PressV(shift))
                        } else if *ch == *Value::new("w").to_smolstr() {
                            Some(Message::PressW(shift))
                        } else if *ch == *Value::new("x").to_smolstr() {
                            Some(Message::PressX(shift))
                        } else if *ch == *Value::new("y").to_smolstr() {
                            Some(Message::PressY(shift))
                        } else if *ch == *Value::new("z").to_smolstr() {
                            Some(Message::PressZ(shift))
                        } else if *ch == *Value::new("1").to_smolstr() {
                            Some(Message::Press1)
                        } else if *ch == *Value::new("2").to_smolstr() {
                            Some(Message::Press2)
                        } else if *ch == *Value::new("3").to_smolstr() {
                            Some(Message::Press3)
                        } else if *ch == *Value::new("4").to_smolstr() {
                            Some(Message::Press4)
                        } else if *ch == *Value::new("5").to_smolstr() {
                            Some(Message::Press5)
                        } else if *ch == *Value::new("6").to_smolstr() {
                            Some(Message::Press6)
                        } else if *ch == *Value::new("7").to_smolstr() {
                            Some(Message::Press7)
                        } else if *ch == *Value::new("8").to_smolstr() {
                            Some(Message::Press8)
                        } else if *ch == *Value::new("9").to_smolstr() {
                            Some(Message::Press9)
                        } else if *ch == *Value::new("0").to_smolstr() {
                            Some(Message::Press0)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                keyboard::Event::KeyPressed {
                    key: Key::Named(named),
                    modifiers,
                    ..
                } => {
                    if named == Named::Enter {
                        Some(Message::PressEnter)
                    } else if modifiers.shift() && named == Named::Tab {
                        Some(Message::FocusPrevious)
                    } else if named == Named::Tab {
                        Some(Message::FocusNext)
                    } else if named == Named::ArrowUp {
                        Some(Message::ReviewGameBackwardAll)
                    } else if named == Named::ArrowLeft {
                        Some(Message::ReviewGameBackward)
                    } else if named == Named::ArrowRight && modifiers.shift() {
                        Some(Message::ReviewGameChildNext)
                    } else if named == Named::ArrowRight {
                        Some(Message::ReviewGameForward)
                    } else if named == Named::ArrowDown {
                        Some(Message::ReviewGameForwardAll)
                    } else if named == Named::Escape {
                        Some(Message::Leave)
                    } else {
                        None
                    }
                }
                _ => None,
            },
            _ => None,
        });

        Subscription::batch(vec![
            subscription_1,
            subscription_2,
            subscription_3,
            subscription_4,
        ])
    }

    fn texting(
        &'a self,
        messages: &'a VecDeque<String>,
        enable_texting: bool,
    ) -> Container<'a, Message> {
        let text_input = if enable_texting {
            iced::widget::text_input(&format!("{}…", t!("message")), &self.text_input)
                .on_input(Message::TextChanged)
                .on_paste(Message::TextChanged)
                .on_submit(Message::TextSend)
        } else {
            iced::widget::text_input(&format!("{}…", t!("message")), "")
        };

        let mut text_box = column![text_input].spacing(SPACING);

        let mut texting = Column::new();
        for message in messages {
            texting = texting.push(text(message));
        }
        text_box = text_box.push(scrollable(texting));

        container(text_box)
            .padding(PADDING)
            .style(container::bordered_box)
    }

    pub fn theme(&self) -> iced::Theme {
        match self.theme {
            Theme::Dark => iced::Theme::SolarizedDark,
            Theme::Light => iced::Theme::SolarizedLight,
            Theme::Tol => iced::Theme::custom("Tol", TOL),
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn update(&mut self, message: Message) -> Task<Message> {
        self.error = None;

        match message {
            Message::AccountSettings => self.screen = Screen::AccountSettings,
            Message::ArchivedGames(mut archived_games) => {
                archived_games.reverse();
                self.archived_games = archived_games;
                self.archived_games_filtered = None;
                handle_error(self.save_client_postcard());
            }
            Message::ArchivedGamesGet => self.send("archived_games\n".to_string()),
            Message::ArchivedGameSelected(game) => self.archived_game_selected = Some(game),
            Message::CancelGame(id) => self.send(format!("leave_game {id}\n")),
            Message::ChangeTheme(theme) => self.change_theme(theme),
            Message::BoardSizeSelected(size) => self.game_settings.board_size = size,
            Message::ConnectedTo(address) => self.connected_to = address,
            Message::Coordinates(_coordinates) => self.coordinates(),
            Message::DeleteAccount => self.delete_account(),
            Message::EmailEveryone => {
                self.screen = Screen::EmailEveryone;
                self.send("emails_bcc\n".to_string());
            }
            Message::EmailReset => self.reset_email(),
            Message::EstimateScore => self.estimate_score(),
            Message::EstimateScoreConnected(tx) => self.estimate_score_tx = Some(tx),
            Message::EstimateScoreDisplay((node, generate_move)) => {
                info!("finish running score estimator...");

                if let Some(handle) = self.archived_game_handle.as_ref()
                    && handle.boards.here() == node
                {
                    info!("{generate_move}");
                    debug!("{}", generate_move.heat_map);
                    self.heat_map = Some(generate_move.heat_map);
                }

                self.estimate_score = false;
            }
            Message::FocusNext => return focus_next(),
            Message::FocusPrevious => return focus_previous(),
            Message::GameCancel(id) => self.send(format!("decline_game {id} switch\n")),
            Message::GameAccept(id) => self.send(format!("join_game {id}\n")),
            Message::GameDecline(id) => self.send(format!("decline_game {id}\n")),
            Message::GameJoin(id) => self.join(id),
            Message::GameWatch(id) => self.watch(id),
            Message::HeatMap(_display) => self.heat_map_display = !self.heat_map_display,
            Message::Leave => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
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
                Screen::Games => {
                    self.send("quit\n".to_string());
                    self.connected_tcp = false;
                    self.text_input = self.username.clone();
                    self.screen = Screen::Login;
                }
                Screen::GameReview => {
                    self.heat_map = None;
                    self.heat_map_display = false;
                    self.screen = Screen::Login;
                }
                Screen::Login => exit(0),
            },
            Message::LocaleSelected(locale) => {
                rust_i18n::set_locale(&locale.txt());

                let string_keys: Vec<_> = self.strings.keys().cloned().collect();
                for string in string_keys {
                    self.strings.insert(string.clone(), t!(string).to_string());
                }

                self.locale_selected = locale;
                handle_error(self.save_client_ron());
            }
            Message::MyGamesOnly(_selected) => {
                self.my_games_only();
            }
            Message::OpenUrl(string) => open_url(&string),
            Message::GameNew => self.game_new(),
            Message::GameResume(id) => self.resume(id),
            Message::GameSubmit => self.game_submit(),
            Message::PasswordChanged(password) => {
                let (password, ends_with_whitespace) = utils::split_whitespace_password(&password);
                self.password_ends_with_whitespace = ends_with_whitespace;
                if password.len() <= 32 {
                    self.password = password;
                }
            }
            Message::PasswordSave(_save) => self.toggle_save_password(),
            Message::PasswordShow(_show) => self.toggle_show_password(),
            Message::PlayDraw => self.draw(),
            Message::PlayDrawDecision(draw) => {
                self.send(format!("draw {} {draw}\n", self.game_id));
            }
            Message::PlayMoveFrom(vertex) => self.play_from = Some(vertex),
            Message::PlayMoveTo(to) => self.play_to(to),
            Message::PlayMoveRevert => self.play_from = None,
            Message::PlayResign => self.resign(),
            Message::PressEnter => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::Game
                | Screen::Games
                | Screen::GameReview
                | Screen::Users => {}
                Screen::GameNew => self.game_submit(),
                Screen::Login => self.login(),
            },
            Message::PressA(shift) => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::Users => {}
                Screen::Game | Screen::GameReview => {
                    self.press_letter('a');
                    self.press_letter_and_number();
                }
                Screen::Games => self.join_game_press(0, shift),
                Screen::Login => self.change_theme(Theme::Tol),
            },
            Message::PressB(shift) => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::Login
                | Screen::Users => {}
                Screen::Game | Screen::GameReview => {
                    self.press_letter('b');
                    self.press_letter_and_number();
                }
                Screen::Games => self.join_game_press(1, shift),
            },
            Message::PressC(shift) => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::Login
                | Screen::Users => {}
                Screen::Game | Screen::GameReview => {
                    self.press_letter('c');
                    self.press_letter_and_number();
                }
                Screen::Games => self.join_game_press(2, shift),
            },
            Message::PressD(shift) => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::Login
                | Screen::Users => {}
                Screen::Game | Screen::GameReview => {
                    self.press_letter('d');
                    self.press_letter_and_number();
                }
                Screen::Games => self.join_game_press(3, shift),
            },
            Message::PressE(shift) => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::Login
                | Screen::Users => {}
                Screen::Game | Screen::GameReview => {
                    self.press_letter('e');
                    self.press_letter_and_number();
                }
                Screen::Games => self.join_game_press(4, shift),
            },
            Message::PressF(shift) => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::Login
                | Screen::Users => {}
                Screen::Game | Screen::GameReview => {
                    self.press_letter('f');
                    self.press_letter_and_number();
                }
                Screen::Games => self.join_game_press(5, shift),
            },
            Message::PressG(shift) => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::Login
                | Screen::Users => {}
                Screen::Game | Screen::GameReview => {
                    self.press_letter('g');
                    self.press_letter_and_number();
                }
                Screen::Games => self.join_game_press(6, shift),
            },
            Message::PressH(shift) => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::Login
                | Screen::Users => {}
                Screen::Game | Screen::GameReview => {
                    self.press_letter('h');
                    self.press_letter_and_number();
                }
                Screen::Games => self.join_game_press(7, shift),
            },
            Message::PressI(shift) => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::Login
                | Screen::Users => {}
                Screen::Game | Screen::GameReview => {
                    self.press_letter('i');
                    self.press_letter_and_number();
                }
                Screen::Games => self.join_game_press(8, shift),
            },
            Message::PressJ(shift) => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::Login
                | Screen::Users => {}
                Screen::Game | Screen::GameReview => {
                    self.press_letter('j');
                    self.press_letter_and_number();
                }
                Screen::Games => self.join_game_press(9, shift),
            },
            Message::PressK(shift) => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::Login
                | Screen::Users => {}
                Screen::Game | Screen::GameReview => {
                    self.press_letter('k');
                    self.press_letter_and_number();
                }
                Screen::Games => self.join_game_press(10, shift),
            },
            Message::PressL(shift) => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::Login
                | Screen::Users => {}
                Screen::Game | Screen::GameReview => {
                    self.press_letter('l');
                    self.press_letter_and_number();
                }
                Screen::Games => self.join_game_press(11, shift),
            },
            Message::PressM(shift) => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::Login
                | Screen::Users => {}
                Screen::Game | Screen::GameReview => {
                    self.press_letter('m');
                    self.press_letter_and_number();
                }
                Screen::Games => self.join_game_press(12, shift),
            },
            Message::PressN(shift) => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::Login
                | Screen::Users => {}
                Screen::Game | Screen::GameReview => self.coordinates(),
                Screen::Games => self.join_game_press(13, shift),
            },
            Message::PressO(shift) => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::Login
                | Screen::Users => {}
                Screen::Game | Screen::GameReview => self.sound_muted(),
                Screen::Games => self.join_game_press(14, shift),
            },
            Message::PressP(shift) => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::Login
                | Screen::Users => {}
                Screen::Game => self.resign(),
                Screen::Games => self.join_game_press(1, shift),
                Screen::GameReview => self.estimate_score(),
            },
            Message::PressQ(shift) => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::GameNew
                | Screen::Login
                | Screen::Users => {}
                Screen::Game => self.draw(),
                Screen::Games => self.join_game_press(16, shift),
                Screen::GameReview => self.heat_map_display = !self.heat_map_display,
            },
            Message::PressR(shift) => match self.screen {
                Screen::AccountSettings | Screen::EmailEveryone | Screen::Game => self.draw(),
                Screen::GameNew | Screen::GameReview | Screen::Login | Screen::Users => {}
                Screen::Games => self.join_game_press(17, shift),
            },
            Message::PressS(shift) => match self.screen {
                Screen::AccountSettings | Screen::EmailEveryone | Screen::Game => self.draw(),
                Screen::GameNew | Screen::GameReview | Screen::Login | Screen::Users => {}
                Screen::Games => self.join_game_press(18, shift),
            },
            Message::PressT(shift) => match self.screen {
                Screen::AccountSettings | Screen::EmailEveryone | Screen::Game => self.draw(),
                Screen::GameNew | Screen::GameReview | Screen::Login | Screen::Users => {}
                Screen::Games => self.join_game_press(19, shift),
            },
            Message::PressU(shift) => match self.screen {
                Screen::AccountSettings | Screen::EmailEveryone | Screen::Game => self.draw(),
                Screen::GameNew | Screen::GameReview | Screen::Login | Screen::Users => {}
                Screen::Games => self.join_game_press(20, shift),
            },
            Message::PressV(shift) => match self.screen {
                Screen::AccountSettings | Screen::EmailEveryone | Screen::Game => self.draw(),
                Screen::GameNew | Screen::GameReview | Screen::Login | Screen::Users => {}
                Screen::Games => self.join_game_press(21, shift),
            },
            Message::PressW(shift) => match self.screen {
                Screen::AccountSettings | Screen::EmailEveryone | Screen::Game => self.draw(),
                Screen::GameNew | Screen::GameReview | Screen::Login | Screen::Users => {}
                Screen::Games => self.join_game_press(22, shift),
            },
            Message::PressX(shift) => match self.screen {
                Screen::AccountSettings | Screen::EmailEveryone | Screen::Game => self.draw(),
                Screen::GameNew | Screen::GameReview | Screen::Login | Screen::Users => {}
                Screen::Games => self.join_game_press(23, shift),
            },
            Message::PressY(shift) => match self.screen {
                Screen::AccountSettings | Screen::EmailEveryone | Screen::Game => self.draw(),
                Screen::GameNew | Screen::GameReview | Screen::Login | Screen::Users => {}
                Screen::Games => self.join_game_press(24, shift),
            },
            Message::PressZ(shift) => match self.screen {
                Screen::AccountSettings | Screen::EmailEveryone | Screen::Game => self.draw(),
                Screen::GameNew | Screen::GameReview | Screen::Login | Screen::Users => {}
                Screen::Games => self.join_game_press(25, shift),
            },
            Message::Press1 => match self.screen {
                Screen::AccountSettings | Screen::Login => self.reset_email(),
                Screen::EmailEveryone | Screen::Users => {}
                Screen::Games => self.send("archived_games\n".to_string()),
                Screen::GameNew => self.game_settings.role_selected = Some(Role::Attacker),
                Screen::Game | Screen::GameReview => {
                    if !(self.press_numbers[0]
                        || self.press_numbers[10]
                        || self.press_numbers[11]
                        || self.press_numbers[12])
                    {
                        self.clear_numbers_except(0);
                        self.press_numbers[0] = true;
                    } else if self.press_numbers[0] {
                        self.clear_numbers_except(10);
                        self.press_numbers[10] = true;
                    } else {
                        self.clear_numbers_except(11);
                        self.press_numbers[10] = false;
                    }

                    self.press_letter_and_number();
                }
            },
            Message::Press2 => match self.screen {
                Screen::AccountSettings => {
                    self.send(format!("change_password {}\n", self.password));
                }
                Screen::EmailEveryone | Screen::Users => {}
                Screen::Games => self.my_games_only(),
                Screen::GameNew => self.game_settings.role_selected = Some(Role::Defender),
                Screen::Game | Screen::GameReview => {
                    let (board, _) = self.board_and_heatmap();
                    match board.size() {
                        BoardSize::_11 => {
                            self.clear_numbers_except(2);
                            self.press_numbers[1] = !self.press_numbers[1];
                            self.press_letter_and_number();
                        }
                        BoardSize::_13 => {
                            if !self.press_numbers[0]
                                && !self.press_numbers[1]
                                && !self.press_numbers[11]
                            {
                                self.clear_numbers_except(2);
                                self.press_numbers[1] = true;
                            } else if self.press_numbers[1] {
                                self.press_numbers[1] = false;
                            } else if self.press_numbers[11] {
                                self.press_numbers[11] = false;
                            } else {
                                self.press_numbers[0] = false;
                                self.press_numbers[11] = true;
                            }
                        }
                    }

                    self.press_letter_and_number();
                }
                Screen::Login => self.toggle_save_password(),
            },
            Message::Press3 => match self.screen {
                Screen::AccountSettings => self.toggle_show_password(),
                Screen::EmailEveryone | Screen::Users => {}
                Screen::Games => self.game_new(),
                Screen::GameNew => self.game_settings.board_size = BoardSize::_11,
                Screen::Login => self.my_games_only(),
                Screen::Game | Screen::GameReview => {
                    let (board, _) = self.board_and_heatmap();
                    match board.size() {
                        BoardSize::_11 => {
                            self.clear_numbers_except(3);
                            self.press_numbers[2] = !self.press_numbers[2];
                            self.press_letter_and_number();
                        }
                        BoardSize::_13 => {
                            if !self.press_numbers[0]
                                && !self.press_numbers[2]
                                && !self.press_numbers[12]
                            {
                                self.clear_numbers_except(3);
                                self.press_numbers[2] = true;
                            } else if self.press_numbers[2] {
                                self.press_numbers[2] = false;
                            } else if self.press_numbers[12] {
                                self.press_numbers[12] = false;
                            } else {
                                self.press_numbers[0] = false;
                                self.press_numbers[12] = true;
                            }
                        }
                    }

                    self.press_letter_and_number();
                }
            },
            Message::Press4 => match self.screen {
                Screen::AccountSettings => self.delete_account(),
                Screen::EmailEveryone | Screen::Users => {}
                Screen::Games => self.screen = Screen::Users,
                Screen::GameNew => self.game_settings.board_size = BoardSize::_13,
                Screen::Login => self.create_account(),
                Screen::Game | Screen::GameReview => {
                    self.clear_numbers_except(4);
                    self.press_numbers[3] = !self.press_numbers[3];
                    self.press_letter_and_number();
                }
            },
            Message::Press5 => match self.screen {
                Screen::AccountSettings | Screen::EmailEveryone | Screen::Users => {}
                Screen::Games => self.screen = Screen::AccountSettings,
                Screen::GameNew => self.game_settings.time = Some(TimeEnum::Rapid),
                Screen::Login => self.reset_password(),
                Screen::Game | Screen::GameReview => {
                    self.clear_numbers_except(5);
                    self.press_numbers[4] = !self.press_numbers[4];
                    self.press_letter_and_number();
                }
            },
            Message::Press6 => match self.screen {
                Screen::AccountSettings | Screen::EmailEveryone | Screen::Users => {}
                Screen::GameNew => self.game_settings.time = Some(TimeEnum::Classical),
                Screen::Games => open_url("https://hnefatafl.org/rules.html"),
                Screen::Login => self.review_game(),
                Screen::Game | Screen::GameReview => {
                    if self.screen == Screen::Game || self.screen == Screen::GameReview {
                        self.clear_numbers_except(6);
                        self.press_numbers[5] = !self.press_numbers[5];
                        self.press_letter_and_number();
                    }
                }
            },
            Message::Press7 => match self.screen {
                Screen::AccountSettings | Screen::EmailEveryone | Screen::Games | Screen::Users => {
                }
                Screen::GameNew => self.game_settings.time = Some(TimeEnum::Long),
                Screen::Login => self.change_theme(Theme::Dark),
                Screen::Game | Screen::GameReview => {
                    self.clear_numbers_except(7);
                    self.press_numbers[6] = !self.press_numbers[6];
                    self.press_letter_and_number();
                }
            },
            Message::Press8 => match self.screen {
                Screen::AccountSettings | Screen::EmailEveryone | Screen::Games | Screen::Users => {
                }
                Screen::GameNew => self.game_settings.time = Some(TimeEnum::VeryLong),
                Screen::Login => self.change_theme(Theme::Light),
                Screen::Game | Screen::GameReview => {
                    self.clear_numbers_except(8);
                    self.press_numbers[7] = !self.press_numbers[7];
                    self.press_letter_and_number();
                }
            },
            Message::Press9 => match self.screen {
                Screen::AccountSettings
                | Screen::EmailEveryone
                | Screen::Games
                | Screen::GameNew
                | Screen::Users => {}
                Screen::Login => open_url("https://discord.gg/h56CAHEBXd"),
                Screen::Game | Screen::GameReview => {
                    self.clear_numbers_except(9);
                    self.press_numbers[8] = !self.press_numbers[8];
                    self.press_letter_and_number();
                }
            },
            Message::Press0 => match self.screen {
                Screen::AccountSettings | Screen::EmailEveryone | Screen::Games | Screen::Users => {
                }
                Screen::GameNew => self.game_settings.rated = !self.game_settings.rated,
                Screen::Login => open_url("https://hnefatafl.org"),
                Screen::Game | Screen::GameReview => {
                    self.clear_numbers_except(10);
                    self.press_numbers[9] = !self.press_numbers[9];
                    self.press_letter_and_number();
                }
            },
            Message::SoundMuted(_muted) => self.sound_muted(),
            Message::StreamConnected(tx) => self.tx = Some(tx),
            Message::RatedSelected(rated) => self.game_settings.rated = rated.into(),
            Message::ResetPassword => self.reset_password(),
            Message::ReviewGame => self.review_game(),
            Message::ReviewGameBackward => {
                if let Some(handle) = &mut self.archived_game_handle {
                    handle.play = handle.play.saturating_sub(1);
                    handle.boards.backward();
                    self.reset_markers();
                }
            }
            Message::ReviewGameBackwardAll => {
                if let Some(handle) = &mut self.archived_game_handle {
                    handle.play = 0;
                    handle.boards.backward_all();
                    self.reset_markers();
                }
            }
            Message::ReviewGameChildNext => {
                if let Some(handle) = &mut self.archived_game_handle {
                    handle.boards.next_child();
                    self.reset_markers();
                }
            }
            Message::ReviewGameForward => {
                if let Some(handle) = &mut self.archived_game_handle
                    && handle.boards.has_children()
                {
                    handle.play += 1;
                    handle.boards.forward();
                    self.reset_markers();
                }
            }
            Message::ReviewGameForwardAll => {
                if let Some(handle) = &mut self.archived_game_handle {
                    let count = handle.boards.forward_all();
                    handle.play += count;
                    self.reset_markers();
                }
            }
            Message::RoleSelected(role) => self.game_settings.role_selected = Some(role),
            Message::TextChanged(string) => {
                if self.screen == Screen::Login {
                    let string: Vec<_> = string.split_whitespace().collect();
                    if let Some(string) = string.first() {
                        if string.len() <= 16 {
                            self.text_input = string.to_ascii_lowercase();
                            self.username = self.text_input.clone();
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
                            Some(
                                "archived_games"
                                | "challenge_requested"
                                | "change_password"
                                | "decline_game"
                                | "email_reset"
                                | "game"
                                | "request_draw",
                            ) => {}
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
                                    let (mut rating, mut deviation) =
                                        rating.split_once("±").unwrap_or_else(|| {
                                            panic!("the ratings has this form: {rating}")
                                        });

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

                                        let game_over = include_bytes!("../sound/game_over.ogg");
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

                                let attacker =
                                    text.next().expect("the attacker should be supplied");
                                let defender =
                                    text.next().expect("the defender should be supplied");

                                self.attacker = attacker.to_string();
                                self.defender = defender.to_string();

                                let rated = text
                                    .next()
                                    .expect("there should be rated or unrated supplied");
                                let rated = Rated::from_str(rated).expect("rated should be valid");

                                self.game_settings.rated = rated;

                                let timed = text
                                    .next()
                                    .expect("there should be a time setting supplied");
                                let minutes =
                                    text.next().expect("there should be a minutes supplied");
                                let add_seconds =
                                    text.next().expect("there should be a add_seconds supplied");

                                let timed = TimeSettings::try_from(vec![
                                    "time_settings",
                                    timed,
                                    minutes,
                                    add_seconds,
                                ])
                                .expect("there should be a valid time settings");

                                let board_size =
                                    text.next().expect("there should be a valid board size");
                                let board_size = BoardSize::from_str(board_size)
                                    .expect("there should be a valid board size");

                                let board = Board::new(board_size);

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
                                let id = text.next().expect("there should be an id supplied");
                                let id = id.parse().expect("id should be a valid usize");

                                self.game_id = id;
                                self.challenger = true;
                            }
                            Some("leave_game") => self.game_id = 0,
                            Some("login") => {
                                if self.username == "david" {
                                    self.email_everyone = true;
                                }
                                self.screen = Screen::Games;
                            }
                            Some("new_game") => {
                                // = new_game game 15 none david rated fischer 900_000 10
                                if Some("game") == text.next() {
                                    let game_id = text.next().expect("the game id should be next");
                                    let game_id =
                                        game_id.parse().expect("the game_id should be a usize");

                                    self.game_id = game_id;
                                    self.challenger = false;
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
                        let id = text.next().expect("there should be a game id");
                        let id = id
                            .parse::<Id>()
                            .expect("the game_id should be a valid usize");

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
                        let id = text.next().expect("there should be a game id");
                        let id = id
                            .parse::<Id>()
                            .expect("the game_id should be a valid usize");

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
                    Screen::GameNew | Screen::GameReview | Screen::Login | Screen::Users => {}
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
            Message::TextSendCreateAccount => self.create_account(),
            Message::TextSendLogin => self.login(),
            Message::Tick => {
                self.counter = self.counter.wrapping_add(1);
                if self.counter.is_multiple_of(25) {
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
            Message::Time(time) => self.game_settings.time = Some(time),
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
                } else if width >= 1_100.0 {
                    self.screen_size = Size::TinyWide;
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
    fn games(&self) -> Scrollable<'_, Message> {
        let mut game_ids = Column::new().spacing(SPACING_B);
        let mut attackers = Column::new().spacing(SPACING_B);
        let mut defenders = Column::new().spacing(SPACING_B);
        let mut ratings = Column::new().spacing(SPACING_B);
        let mut timings = Column::new().spacing(SPACING_B);
        let mut sizes = Column::new().spacing(SPACING_B);
        let mut buttons = Column::new().spacing(SPACING);

        let mut server_games: Vec<&ServerGameLight> = self.games_light.0.values().collect();
        server_games.sort_by(|a, b| b.id.cmp(&a.id));

        for (i, game) in server_games.iter().enumerate() {
            if self.my_games_only {
                let mut includes_username = false;
                if let Some(attacker) = &game.attacker
                    && attacker == &self.username
                {
                    includes_username = true;
                }

                if let Some(defender) = &game.defender
                    && defender == &self.username
                {
                    includes_username = true;
                }

                if !includes_username {
                    continue;
                }
            }

            let id = game.id;
            game_ids = game_ids.push(text(id));

            attackers = if let Some(attacker) = &game.attacker {
                attackers.push(text(attacker))
            } else {
                attackers.push(text(t!("none")))
            };
            defenders = if let Some(defender) = &game.defender {
                defenders.push(text(defender))
            } else {
                defenders.push(text(t!("none")))
            };

            let rating: bool = game.rated.into();
            let rating = if rating { t!("yes") } else { t!("no") };
            ratings = ratings.push(text(rating));

            timings = timings.push(text(game.timed.to_string()));
            sizes = sizes.push(text(game.board_size.to_string()));

            let mut buttons_row = Row::new().spacing(SPACING);

            let i = if let Some(i) = ALPHABET.get(i) {
                format!(" ({i})")
            } else {
                String::new()
            };

            match self.join_game(game) {
                JoinGame::Cancel => {
                    buttons_row = buttons_row.push(
                        button(text!("{}{i}", self.strings["Cancel"].as_str()))
                            .on_press(Message::GameCancel(id)),
                    );
                }
                JoinGame::Join => {
                    buttons_row = buttons_row.push(
                        button(text!("{}{i}", self.strings["Join"].as_str()))
                            .on_press(Message::GameJoin(id)),
                    );
                }
                JoinGame::None => {}
                JoinGame::Resume => {
                    buttons_row = buttons_row.push(
                        button(text!("{}{i}", self.strings["Resume"].as_str()))
                            .on_press(Message::GameResume(id)),
                    );
                }
                JoinGame::Watch => {
                    buttons_row = buttons_row.push(
                        button(text!("{}{i}", self.strings["Watch"].as_str()))
                            .on_press(Message::GameWatch(id)),
                    );
                }
            }

            match self.game_state(id) {
                State::Challenger | State::Spectator => {}
                State::Creator => {
                    buttons_row = buttons_row.push(
                        button(text!("{}{i}", self.strings["Accept"].as_str()))
                            .on_press(Message::GameAccept(id)),
                    );
                    buttons_row = buttons_row.push(
                        button(text!(
                            "{}{}",
                            self.strings["Decline"].as_str(),
                            i.to_ascii_uppercase()
                        ))
                        .on_press(Message::GameDecline(id)),
                    );
                }
                State::CreatorOnly => {
                    buttons_row = buttons_row.push(
                        button(text!("{}{i}", self.strings["Cancel"].as_str()))
                            .on_press(Message::CancelGame(id)),
                    );
                }
            }
            buttons = buttons.push(buttons_row);
        }

        let game_id = t!("ID");
        let game_ids = column![
            text(game_id.to_string()),
            text("-".repeat(game_id.chars().count())).font(Font::MONOSPACE),
            game_ids
        ]
        .padding(PADDING);
        let attacker = t!("attacker");
        let attackers = column![
            text(attacker.to_string()),
            text("-".repeat(attacker.chars().count())).font(Font::MONOSPACE),
            attackers
        ]
        .padding(PADDING);
        let defender = t!("defender");
        let defenders = column![
            text(defender.to_string()),
            text("-".repeat(defender.chars().count())).font(Font::MONOSPACE),
            defenders
        ]
        .padding(PADDING);
        let rated = t!("rated");
        let ratings = column![
            text(rated.to_string()),
            text("-".repeat(rated.chars().count())).font(Font::MONOSPACE),
            ratings
        ]
        .padding(PADDING);
        let timed = t!("timed");
        let timings = column![
            text(timed.to_string()),
            text("-".repeat(timed.chars().count())).font(Font::MONOSPACE),
            timings
        ]
        .padding(PADDING);
        let size = t!("size");
        let sizes = column![
            text(size.to_string()),
            text("-".repeat(size.chars().count())).font(Font::MONOSPACE),
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

        let mut game_handle = None;
        if let Some(handle) = &mut self.archived_game_handle {
            game_handle = Some(Game::from(&handle.boards));
        }

        let game = if let Some(game) = &mut game_handle {
            game
        } else {
            self.game.as_mut().expect("you should have a game by now")
        };

        let role = if let Some(role) = role {
            Role::from_str(role).expect("there should be a valid role")
        } else {
            game.turn
        };

        let play = Plae::try_from(vec!["play", &role.to_string(), from, to]).unwrap();
        let captures = game.play(&play).unwrap();
        for vertex in captures.0 {
            self.captures.insert(vertex);
        }

        if let Some(handle) = &mut self.archived_game_handle {
            handle.boards.insert(&game.board);
            handle.play += 1;
        }

        if self.sound_muted {
            return;
        }

        let capture = !self.captures.is_empty();

        thread::spawn(move || {
            let mut stream = rodio::OutputStreamBuilder::open_default_stream()?;
            let cursor = if capture {
                let capture_ogg = include_bytes!("../sound/capture.ogg").to_vec();
                Cursor::new(capture_ogg)
            } else {
                let move_ogg = include_bytes!("../sound/move.ogg").to_vec();
                Cursor::new(move_ogg)
            };
            let sound = rodio::play(stream.mixer(), cursor)?;
            sound.set_volume(1.0);
            sound.sleep_until_end();

            stream.log_on_drop(false);
            Ok::<(), anyhow::Error>(())
        });
    }

    fn resign(&mut self) {
        let game = self.game.as_ref().expect("you should have a game by now");

        self.send(format!(
            "game {} play {} resigns _\n",
            self.game_id, game.turn
        ));
    }

    fn sound_muted(&mut self) {
        self.sound_muted = !self.sound_muted;
        handle_error(self.save_client_ron());
    }

    #[must_use]
    fn users(&self, logged_in: bool) -> Scrollable<'_, Message> {
        let mut ratings = Column::new();
        let mut usernames = Column::new();
        let mut wins = Column::new();
        let mut losses = Column::new();
        let mut draws = Column::new();

        for user in self.users_sorted() {
            if logged_in == user.logged_in {
                ratings = ratings.push(text(user.rating.to_string_rounded()));
                usernames = usernames.push(text(user.name));
                wins = wins.push(text(user.wins));
                losses = losses.push(text(user.losses));
                draws = draws.push(text(user.draws));
            }
        }

        let rating = t!("rating");
        let ratings = column![
            text(rating.to_string()),
            text("-".repeat(rating.chars().count())).font(Font::MONOSPACE),
            ratings
        ]
        .padding(PADDING);
        let username = t!("username");
        let usernames = column![
            text(username.to_string()),
            text("-".repeat(username.chars().count())).font(Font::MONOSPACE),
            usernames
        ]
        .padding(PADDING);
        let win = t!("wins");
        let wins = column![
            text(win.to_string()),
            text("-".repeat(win.chars().count())).font(Font::MONOSPACE),
            wins
        ]
        .padding(PADDING);
        let loss = t!("losses");
        let losses = column![
            text(loss.to_string()),
            text("-".repeat(loss.chars().count())).font(Font::MONOSPACE),
            losses
        ]
        .padding(PADDING);
        let draw = t!("draws");
        let draws = column![
            text(draw.to_string()),
            text("-".repeat(draw.chars().count())).font(Font::MONOSPACE),
            draws
        ]
        .padding(PADDING);

        scrollable(row![ratings, usernames, wins, losses, draws])
    }

    #[must_use]
    fn user_area(&self, in_game: bool) -> Container<'_, Message> {
        let texts = if in_game {
            &self.texts_game
        } else {
            &self.texts
        };

        let games = self.games();
        let texting = self.texting(texts, true).padding(PADDING);
        let users = self.users(true);

        let user_area = scrollable(column![games, users, texting]);
        container(user_area)
            .padding(PADDING)
            .style(container::bordered_box)
    }

    fn reset_password(&mut self) {
        if !self.connected_tcp {
            self.send("tcp_connect\n".to_string());
            self.connected_tcp = true;
        }

        if self.screen == Screen::Login {
            self.send(format!("{VERSION_ID} reset_password {}\n", self.text_input));
        }
    }

    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn view(&self) -> Element<'_, Message> {
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
                    ),
                    text!("{}: {}", t!("username"), &self.username),
                    text!("{}: {rating}", t!("rating")),
                    text!("{}: {wins}", t!("wins")),
                    text!("{}: {losses}", t!("losses")),
                    text!("{}: {draws}", t!("draws")),
                ]
                .padding(PADDING)
                .spacing(SPACING);

                if let Some(email) = &self.email {
                    let mut row = Row::new();
                    if email.verified {
                        row = row.push(text!(
                            "{}: [{}] {} ",
                            t!("email address"),
                            t!("verified"),
                            email.address,
                        ));
                        columns = columns.push(row);
                    } else {
                        row = row.push(text!(
                            "{}: [{}] {} ",
                            t!("email address"),
                            t!("unverified"),
                            email.address,
                        ));
                        columns = columns.push(row);

                        let mut row = Row::new();
                        row = row.push(text!("{}: ", t!("email code")));
                        row = row.push(
                            widget::text_input("", &self.text_input)
                                .on_input(Message::TextChanged)
                                .on_paste(Message::TextChanged)
                                .on_submit(Message::TextSendEmailCode),
                        );
                        columns = columns.push(row);
                    }
                } else {
                    let mut row = Row::new();
                    row = row.push(text!("{}: ", t!("email address")));
                    row = row.push(
                        widget::text_input("", &self.text_input)
                            .on_input(Message::TextChanged)
                            .on_paste(Message::TextChanged)
                            .on_submit(Message::TextSendEmail),
                    );

                    columns = columns.push(row);
                    columns = columns.push(row![text!("{}: ", t!("email code"))]);
                }

                columns = columns.push(row![
                    button(text!("{} (1)", self.strings["Reset Email"].as_str()))
                        .on_press(Message::EmailReset)
                ]);

                if let Some(error) = &self.error_email {
                    columns = columns.push(row![text!("error: {error}")]);
                }

                let mut change_password_button =
                    button(text!("{} (2)", self.strings["Change Password"].as_str()));

                if !self.password_ends_with_whitespace {
                    change_password_button = change_password_button.on_press(Message::TextSend);
                }

                columns = columns.push(
                    row![
                        change_password_button,
                        widget::text_input("", &self.password)
                            .secure(!self.password_show)
                            .on_input(Message::PasswordChanged)
                            .on_paste(Message::PasswordChanged),
                    ]
                    .spacing(SPACING),
                );

                columns = columns.push(
                    row![
                        checkbox(self.password_show).on_toggle(Message::PasswordShow),
                        text!("{} (3)", t!("show password")),
                    ]
                    .spacing(SPACING),
                );

                if self.delete_account {
                    columns = columns.push(
                        button(text!(
                            "{} (4)",
                            self.strings["REALLY DELETE ACCOUNT"].as_str()
                        ))
                        .on_press(Message::DeleteAccount),
                    );
                } else {
                    columns = columns.push(
                        button(text!("{} (4)", self.strings["Delete Account"].as_str()))
                            .on_press(Message::DeleteAccount),
                    );
                }

                columns = columns.push(
                    button(text!("{} (Esc)", self.strings["Leave"].as_str()))
                        .on_press(Message::Leave),
                );

                columns.into()
            }
            Screen::EmailEveryone => {
                let subject = row![
                    text("Subject: "),
                    widget::text_input("", &self.text_input)
                        .on_input(Message::TextChanged)
                        .on_paste(Message::TextChanged)
                        .on_submit(Message::TextSend),
                ];

                let editor = text_editor(&self.content)
                    .placeholder("Dear User, …")
                    .on_action(Message::TextEdit);

                let send_emails = button("Send Emails").on_press(Message::TextSend);
                let leave = button(text!("{} (Esc)", self.strings["Leave"].as_str()))
                    .on_press(Message::Leave);
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
                    format!("{} (1)", t!("attacker")),
                    Role::Attacker,
                    self.game_settings.role_selected,
                    Message::RoleSelected,
                );

                let defender = radio(
                    format!("{} (2)", t!("defender")),
                    Role::Defender,
                    self.game_settings.role_selected,
                    Message::RoleSelected,
                );

                let rated = row![
                    text!("{} (0):", t!("rated")),
                    checkbox(self.game_settings.rated.into()).on_toggle(Message::RatedSelected)
                ]
                .padding(PADDING)
                .spacing(SPACING);

                let mut new_game = button(text!("{} (Enter)", self.strings["New Game"].as_str()));
                if self.game_settings.role_selected.is_some() && self.game_settings.time.is_some() {
                    new_game = new_game.on_press(Message::GameSubmit);
                }

                let leave = button(text!("{} (Esc)", self.strings["Leave"].as_str()))
                    .on_press(Message::Leave);

                let size_11x11 = radio(
                    "11x11 (3)",
                    BoardSize::_11,
                    Some(self.game_settings.board_size),
                    Message::BoardSizeSelected,
                );

                let size_13x13 = radio(
                    "13x13 (4)",
                    BoardSize::_13,
                    Some(self.game_settings.board_size),
                    Message::BoardSizeSelected,
                );

                let row_1 = row![text!("{}:", t!("role")), attacker, defender]
                    .padding(PADDING)
                    .spacing(SPACING);

                let row_2 = row![text!("{}:", t!("board size")), size_11x11, size_13x13]
                    .padding(PADDING)
                    .spacing(SPACING);

                let rapid = radio(
                    format!("{} (5)", TimeEnum::Rapid),
                    TimeEnum::Rapid,
                    self.game_settings.time,
                    Message::Time,
                );

                let classical = radio(
                    format!("{} (6)", TimeEnum::Classical),
                    TimeEnum::Classical,
                    self.game_settings.time,
                    Message::Time,
                );

                let long = radio(
                    format!("{} (7)", TimeEnum::Long),
                    TimeEnum::Long,
                    self.game_settings.time,
                    Message::Time,
                );

                let very_long = radio(
                    format!("{} (8)", TimeEnum::VeryLong),
                    TimeEnum::VeryLong,
                    self.game_settings.time,
                    Message::Time,
                );

                let row_3 = row![text!("{}:", t!("time"))]
                    .padding(PADDING)
                    .spacing(SPACING);

                let row_4 = row![rapid, classical].padding(PADDING).spacing(SPACING);
                let row_5 = row![long, very_long].padding(PADDING).spacing(SPACING);
                let row_6 = row![new_game, leave].padding(PADDING).spacing(SPACING);

                column![rated, row_1, row_2, row_3, row_4, row_5, row_6].into()
            }
            Screen::Games => {
                let mut email_everyone = Row::new().spacing(SPACING);

                if self.email_everyone {
                    email_everyone = email_everyone
                        .push(button("Email Everyone").on_press(Message::EmailEveryone));
                }

                let username =
                    row![text!("{}: {}", t!("username"), &self.username)].spacing(SPACING);

                let username = container(username)
                    .padding(PADDING / 2)
                    .style(container::bordered_box);

                let my_games_text = text!("{} (2)", t!("My Games Only")).center();
                let my_games = checkbox(self.my_games_only)
                    .on_toggle(Message::MyGamesOnly)
                    .size(32);

                let get_archived_games =
                    button(text!("{} (1)", self.strings["Get Archived Games"].as_str()))
                        .on_press(Message::ArchivedGamesGet);

                let username =
                    row![username, get_archived_games, my_games, my_games_text].spacing(SPACING);

                let create_game = button(text!("{} (3)", self.strings["Create Game"].as_str()))
                    .on_press(Message::GameNew);

                let users = button(text!("{} (4)", self.strings["Users"].as_str()))
                    .on_press(Message::Users);

                let account_setting =
                    button(text!("{} (5)", self.strings["Account Settings"].as_str()))
                        .on_press(Message::AccountSettings);

                let website = button(text!("{} (6)", self.strings["Rules"].as_str())).on_press(
                    Message::OpenUrl("https://hnefatafl.org/rules.html".to_string()),
                );

                let quit = button(text!("{} (Esc)", self.strings["Leave"].as_str()))
                    .on_press(Message::Leave);

                let top = row![create_game, users, account_setting, website, quit].spacing(SPACING);
                let user_area = self.user_area(false);

                column![email_everyone, username, top, user_area]
                    .padding(PADDING)
                    .spacing(SPACING)
                    .into()
            }
            Screen::Login => {
                let username = row![
                    text!("{}:", t!("username")).size(20),
                    widget::text_input("", &self.text_input)
                        .on_input(Message::TextChanged)
                        .on_paste(Message::TextChanged),
                ]
                .spacing(SPACING);

                let username = container(username)
                    .padding(PADDING)
                    .style(container::bordered_box);

                let password = row![
                    text!("{}:", t!("password")).size(20),
                    widget::text_input("", &self.password)
                        .secure(!self.password_show)
                        .on_input(Message::PasswordChanged)
                        .on_paste(Message::PasswordChanged),
                ]
                .spacing(SPACING);

                let password = container(password)
                    .padding(PADDING)
                    .style(container::bordered_box);

                let show_password_text = text!("{} (1)", t!("show password"));
                let show_password = checkbox(self.password_show).on_toggle(Message::PasswordShow);

                let save_password_text = text!("{} (2)", t!("save password"));
                let save_password = checkbox(self.password_save).on_toggle(Message::PasswordSave);

                let mut login = button(text!("{} (Enter)", self.strings["Login"].as_str()));
                if !self.password_ends_with_whitespace {
                    login = login.on_press(Message::TextSendLogin);
                }

                let mut create_account =
                    button(text!("{} (4)", self.strings["Create Account"].as_str()));
                if !self.text_input.is_empty() && !self.password_ends_with_whitespace {
                    create_account = create_account.on_press(Message::TextSendCreateAccount);
                }

                let mut reset_password =
                    button(text!("{} (5)", self.strings["Reset Password"].as_str()));
                if !self.text_input.is_empty() {
                    reset_password = reset_password.on_press(Message::ResetPassword);
                }

                let mut error = text("");
                if let Some(error_) = &self.error {
                    error = text(error_);
                }

                let mut error_persistent = Column::new();
                for error in &self.error_persistent {
                    error_persistent = error_persistent.push(text(error));
                }

                let mut review_game = button(text!("{} (6)", self.strings["Review Game"].as_str()));
                if self.archived_game_selected.is_some() {
                    review_game = review_game.on_press(Message::ReviewGame);
                }

                let archived_games = if let Some(archived_games) = &self.archived_games_filtered {
                    archived_games.clone()
                } else {
                    self.archived_games.clone()
                };

                let my_games_text = text!("{} (3)", t!("My Games Only"));
                let my_games = checkbox(self.my_games_only).on_toggle(Message::MyGamesOnly);

                let buttons_1 =
                    row![login, create_account, reset_password, review_game,].spacing(SPACING);

                let review_game_pick = pick_list(
                    archived_games,
                    self.archived_game_selected.clone(),
                    Message::ArchivedGameSelected,
                )
                .placeholder(t!("Archived Games"));

                let review_game_pick = row![review_game_pick].spacing(SPACING);

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
                    text!("{}: ", t!("locale")).size(20),
                    pick_list(locale, Some(self.locale_selected), Message::LocaleSelected),
                ];

                let mut buttons_2 = if self.theme == Theme::Light {
                    row![
                        button(text!("{} (7)", self.strings["Dark"].as_str()))
                            .on_press(Message::ChangeTheme(Theme::Dark)),
                        button(text!("{} (8)", self.strings["Light"].as_str())),
                        button(text("Tol (a)")).on_press(Message::ChangeTheme(Theme::Tol)),
                    ]
                    .spacing(SPACING)
                } else if self.theme == Theme::Dark {
                    row![
                        button(text!("{} (7)", self.strings["Dark"].as_str())),
                        button(text!("{} (8)", self.strings["Light"].as_str()))
                            .on_press(Message::ChangeTheme(Theme::Light)),
                        button(text("Tol (a)")).on_press(Message::ChangeTheme(Theme::Tol)),
                    ]
                    .spacing(SPACING)
                } else {
                    row![
                        button(text!("{} (7)", self.strings["Dark"].as_str()))
                            .on_press(Message::ChangeTheme(Theme::Dark)),
                        button(text!("{} (8)", self.strings["Light"].as_str()))
                            .on_press(Message::ChangeTheme(Theme::Light)),
                        button(text("Tol (a)")),
                    ]
                    .spacing(SPACING)
                };

                let discord = button(text!("{} (9)", self.strings["Join Discord"].as_str()))
                    .on_press(Message::OpenUrl(
                        "https://discord.gg/h56CAHEBXd".to_string(),
                    ));

                let website = button("https://hnefatafl.org (0)")
                    .on_press(Message::OpenUrl("https://hnefatafl.org".to_string()));

                let quit = button(text!("{} (Esc)", self.strings["Quit"].as_str()))
                    .on_press(Message::Leave);

                buttons_2 = buttons_2.push(discord);
                buttons_2 = buttons_2.push(website);
                buttons_2 = buttons_2.push(quit);

                let help_text = container(text!(
                    "Tab: {}, Shift + Tab: {}",
                    self.chars.arrow_right,
                    self.chars.arrow_left
                ))
                .padding(PADDING)
                .style(container::bordered_box);

                let help_text_2 = text(t!(
                    "You must hold down the control (Ctrl) or command (⌘) key when pressing a lettered or numbered hotkey."
                ));

                let help_text_3 = text(t!(
                    "You can play on the board by pressing control (Ctrl) or command (⌘) and a letter then a number or vice versa."
                ));

                column![
                    username,
                    password,
                    row![
                        show_password,
                        show_password_text,
                        save_password,
                        save_password_text,
                        my_games,
                        my_games_text,
                    ]
                    .spacing(SPACING),
                    buttons_1,
                    review_game_pick,
                    locale,
                    buttons_2,
                    help_text,
                    help_text_2,
                    help_text_3,
                    error,
                    error_persistent
                ]
                .padding(PADDING)
                .spacing(SPACING)
                .into()
            }
            Screen::Users => scrollable(column![
                text(t!("logged in")),
                self.users(true),
                text(t!("logged out")),
                self.users(false),
                row![
                    button(text!("{} (Esc)", self.strings["Leave"].as_str()))
                        .on_press(Message::Leave)
                ]
                .padding(PADDING),
            ])
            .spacing(SPACING)
            .into(),
        }
    }

    fn reset_email(&mut self) {
        self.email = None;
        self.send("email_reset\n".to_string());
    }

    fn reset_markers(&mut self) {
        self.captures = HashSet::new();
        self.play_from = None;
        self.play_from_previous = None;
        self.play_to_previous = None;
    }

    fn review_game(&mut self) {
        if let Some(archived_game) = &self.archived_game_selected {
            self.archived_game_handle = Some(ArchivedGameHandle::new(archived_game));
            self.screen = Screen::GameReview;

            self.captures = HashSet::new();
            self.reset_markers();
        }
    }

    fn save_client_postcard(&self) -> anyhow::Result<()> {
        let postcard_bytes = postcard::to_allocvec(&self.archived_games)?;
        if !postcard_bytes.is_empty() {
            let mut file = File::create(data_file(USER_CONFIG_FILE_POSTCARD))?;
            file.write_all(&postcard_bytes)?;
        }

        Ok(())
    }

    fn save_client_ron(&self) -> anyhow::Result<()> {
        let password = if self.password_save {
            self.password.clone()
        } else {
            String::new()
        };

        let client = Client {
            archived_games: Vec::new(),
            coordinates: self.coordinates,
            locale_selected: self.locale_selected,
            my_games_only: self.my_games_only,
            password,
            password_save: self.password_save,
            password_show: self.password_show,
            sound_muted: self.sound_muted,
            theme: self.theme,
            username: self.username.clone(),
            ..Client::default()
        };

        let ron_string = ron::ser::to_string_pretty(&client, ron::ser::PrettyConfig::new())?;
        if !ron_string.is_empty() {
            let mut file = File::create(data_file(USER_CONFIG_FILE_RON))?;
            file.write_all(ron_string.as_bytes())?;
        }

        Ok(())
    }

    fn send(&mut self, string: String) {
        handle_error(
            self.tx
                .as_mut()
                .unwrap_or_else(|| {
                    panic!("error sending {string:?}: you should have a tx available by now")
                })
                .send(string),
        );
    }

    fn send_estimate_score(&mut self, tree: Tree) {
        handle_error(
            self.estimate_score_tx
                .as_mut()
                .unwrap_or_else(|| {
                    panic!("error sending {tree:?}: you should have a tx available by now")
                })
                .send(tree),
        );
    }

    fn toggle_save_password(&mut self) {
        self.password_save = !self.password_save;
        handle_error(self.save_client_ron());
    }

    fn toggle_show_password(&mut self) {
        self.password_show = !self.password_show;
        handle_error(self.save_client_ron());
    }

    fn letter(
        &self,
        letter: char,
        column: Column<'a, Message>,
        letter_size: u32,
    ) -> Column<'a, Message> {
        let mut text = text(letter).size(letter_size);
        if self.press_letters.contains(&letter.to_ascii_lowercase()) {
            text = text.color(BLUE);
        }

        column.push(text)
    }

    fn numbers(&self, letter_size: u32, spacing: u32, board_size: usize) -> Column<'a, Message> {
        let mut column = column![text(" ").size(letter_size)].spacing(spacing);

        for i in 0..board_size {
            let i = board_size - i;
            let mut text = text!("{i:2}").size(letter_size).align_y(Vertical::Center);
            if self.press_numbers[i - 1] {
                text = text.color(BLUE);
            }
            column = column.push(text);
        }

        column
    }
}

#[derive(Clone, Debug)]
enum Message {
    AccountSettings,
    ArchivedGames(Vec<ArchivedGame>),
    ArchivedGamesGet,
    ArchivedGameSelected(ArchivedGame),
    BoardSizeSelected(BoardSize),
    CancelGame(Id),
    ChangeTheme(Theme),
    ConnectedTo(String),
    Coordinates(bool),
    DeleteAccount,
    EmailEveryone,
    EmailReset,
    EstimateScore,
    EstimateScoreConnected(mpsc::Sender<Tree>),
    EstimateScoreDisplay((Node, GenerateMove)),
    FocusPrevious,
    FocusNext,
    GameAccept(Id),
    GameCancel(Id),
    GameDecline(Id),
    GameJoin(Id),
    GameNew,
    GameResume(Id),
    GameSubmit,
    GameWatch(Id),
    HeatMap(bool),
    Leave,
    LocaleSelected(Locale),
    MyGamesOnly(bool),
    OpenUrl(String),
    PasswordChanged(String),
    PasswordSave(bool),
    PasswordShow(bool),
    PlayDraw,
    PlayDrawDecision(Draw),
    PlayMoveFrom(Vertex),
    PlayMoveTo(Vertex),
    PlayMoveRevert,
    PlayResign,
    PressEnter,
    PressA(bool),
    PressB(bool),
    PressC(bool),
    PressD(bool),
    PressE(bool),
    PressF(bool),
    PressG(bool),
    PressH(bool),
    PressI(bool),
    PressJ(bool),
    PressK(bool),
    PressL(bool),
    PressM(bool),
    PressN(bool),
    PressO(bool),
    PressP(bool),
    PressQ(bool),
    PressR(bool),
    PressS(bool),
    PressT(bool),
    PressU(bool),
    PressV(bool),
    PressW(bool),
    PressX(bool),
    PressY(bool),
    PressZ(bool),
    Press1,
    Press2,
    Press3,
    Press4,
    Press5,
    Press6,
    Press7,
    Press8,
    Press9,
    Press0,
    SoundMuted(bool),
    RatedSelected(bool),
    ResetPassword,
    ReviewGame,
    ReviewGameBackward,
    ReviewGameBackwardAll,
    ReviewGameChildNext,
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
    Time(TimeEnum),
    Users,
    WindowResized((f32, f32)),
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
enum Coordinates {
    Hide,
    #[default]
    Show,
}

impl Not for Coordinates {
    type Output = Coordinates;

    fn not(self) -> Self::Output {
        match self {
            Self::Hide => Self::Show,
            Self::Show => Self::Hide,
        }
    }
}

impl From<bool> for Coordinates {
    fn from(value: bool) -> Self {
        if value { Self::Show } else { Self::Hide }
    }
}

impl From<Coordinates> for bool {
    fn from(coordinates: Coordinates) -> Self {
        match coordinates {
            Coordinates::Show => true,
            Coordinates::Hide => false,
        }
    }
}

#[derive(Clone, Debug)]
struct Dimensions {
    board_dimension: u32,
    letter_size: u32,
    piece_size: u32,
    spacing: u32,
}

impl Dimensions {
    fn new(board_size: BoardSize, screen_size: &Size) -> Self {
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

#[derive(Clone, Debug)]
enum JoinGame {
    Cancel,
    Join,
    None,
    Resume,
    Watch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum State {
    Challenger,
    Creator,
    CreatorOnly,
    Spectator,
}

fn estimate_score() -> impl Stream<Item = Message> {
    let args = Args::parse();

    stream::channel(
        100,
        move |mut sender: iced::futures::channel::mpsc::Sender<Message>| async move {
            let (tx, rx) = mpsc::channel();

            if let Err(error) = sender.send(Message::EstimateScoreConnected(tx)).await {
                error!("failed to send channel: {error}");
                exit(1);
            }

            thread::spawn(move || {
                let mut ai = match choose_ai(&args.ai, args.seconds, args.depth) {
                    Ok(ai) => ai,
                    Err(error) => {
                        error!("{error}");
                        exit(1);
                    }
                };

                loop {
                    let tree = handle_error(rx.recv());
                    let mut game = Game::from(&tree);
                    let generate_move = ai.generate_move(&mut game).expect("the game is ongoing");

                    if let Err(error) = executor::block_on(
                        sender.send(Message::EstimateScoreDisplay((tree.here(), generate_move))),
                    ) {
                        error!("failed to send channel: {error}");
                        exit(1);
                    }
                }
            });
        },
    )
}

fn pass_messages() -> impl Stream<Item = Message> {
    stream::channel(
        100,
        move |mut sender: iced::futures::channel::mpsc::Sender<Message>| async move {
            let mut args = Args::parse();
            args.host.push_str(SERVER_PORT);
            let address = args.host;

            thread::spawn(move || {
                'start_over: loop {
                    let (tx, rx) = mpsc::channel();

                    if let Err(error) =
                        executor::block_on(sender.send(Message::StreamConnected(tx)))
                    {
                        error!("failed to send channel: {error}");
                        exit(1);
                    }

                    loop {
                        let message = handle_error(rx.recv());
                        let message_trim = message.trim();

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

                            if message_trim == "quit" {
                                tcp_stream
                                    .shutdown(Shutdown::Both)
                                    .expect("shutdown call failed");

                                return;
                            }

                            handle_error(tcp_stream.write_all(message.as_bytes()));
                        }
                    });

                    let mut buffer = String::new();
                    handle_error(executor::block_on(
                        sender.send(Message::ConnectedTo(address.clone())),
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
                                let length = handle_error(buffer_trim_vec[2].parse());
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
                            continue 'start_over;
                        }
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

fn open_url(url: &str) {
    if let Err(error) = webbrowser::open(url) {
        error!("{error}");
    }
}

fn text_collect(text: SplitAsciiWhitespace<'_>) -> String {
    let text: Vec<&str> = text.collect();
    text.join(" ")
}
