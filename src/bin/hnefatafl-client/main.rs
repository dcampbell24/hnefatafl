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

// Don't open the terminal on Windows.
#![cfg_attr(all(windows, not(feature = "console")), windows_subsystem = "windows")]
#![deny(clippy::panic)]
#![deny(clippy::unwrap_used)]

mod archived_game_handle;
mod client;
mod command_line;
mod dimensions;
mod enums;
mod new_game_settings;
mod user;

use std::{
    collections::HashMap,
    fmt, fs,
    io::{BufRead, BufReader, ErrorKind, Read, Write},
    net::{Shutdown, TcpStream},
    process::exit,
    str::SplitAsciiWhitespace,
    sync::mpsc,
    thread::{self, sleep},
    time::Duration,
};

use clap::{CommandFactory, Parser};
use futures::{SinkExt, executor};
use hnefatafl_copenhagen::{
    COPYRIGHT, SERVER_PORT,
    game::Game,
    server_game::ArchivedGame,
    utils::{self, choose_ai, data_file},
};
#[cfg(target_os = "linux")]
use iced::window::settings::PlatformSpecific;
use iced::{
    Pixels, color,
    futures::Stream,
    stream,
    theme::Palette,
    window::{self, icon},
};
use log::{debug, error, info, trace};
use rust_i18n::t;

use crate::{
    client::Client,
    command_line::Args,
    dimensions::Dimensions,
    enums::{Message, Size},
};

/// The Muted qualitative color scheme of [Tol]. A color scheme for the
/// color blind.
///
/// [Tol]: https://sronpersonalpages.nl/~pault/#sec:qualitative
pub const TOL: Palette = Palette {
    background: color!(0xDD, 0xDD, 0xDD), // PALE_GREY
    text: color!(0x00, 0x00, 0x00),       // BLACK
    primary: color!(0x88, 0xCC, 0xEE),    // CYAN
    success: color!(0x11, 0x77, 0x33),    // GREEN
    warning: color!(0xDD, 0xCC, 0x77),    // SAND
    danger: color!(0xCC, 0x66, 0x77),     // ROSE
};

const ALPHABET: [char; 26] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

#[cfg(all(target_os = "linux", not(feature = "icon_2")))]
const APPLICATION_ID: &str = "hnefatafl-client";

#[cfg(all(target_os = "linux", feature = "icon_2"))]
const APPLICATION_ID: &str = "org.hnefatafl.hnefatafl_client";

const BOARD_LETTERS_LOWERCASE: [char; 13] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
];

#[cfg(not(feature = "icon_2"))]
const KING: &[u8] = include_bytes!("icons/king_1_256x256.rgba");

#[cfg(feature = "icon_2")]
const KING: &[u8] = include_bytes!("icons/king_2_256x256.rgba");

const USER_CONFIG_FILE_POSTCARD: &str = "hnefatafl.postcard";
const USER_CONFIG_FILE_RON: &str = "hnefatafl.ron";

const PADDING: u16 = 10;
const PADDING_SMALL: u16 = 2;
const SPACING: Pixels = Pixels(10.0);
const SPACING_B: Pixels = Pixels(20.0);

const SOUND_CAPTURE: &[u8] = include_bytes!("sound/capture.ogg");
const SOUND_GAME_OVER: &[u8] = include_bytes!("sound/game_over.ogg");
const SOUND_MOVE: &[u8] = include_bytes!("sound/move.ogg");

const TROPHY_SIZE: u32 = 32;

rust_i18n::i18n!();

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
    strings.insert("Tournament".to_string(), t!("Tournament").to_string());
    strings.insert(
        "Join Tournament".to_string(),
        t!("Join Tournament").to_string(),
    );
    strings.insert(
        "Leave Tournament".to_string(),
        t!("Leave Tournament").to_string(),
    );

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
    let args = Args::parse();
    utils::init_logger("hnefatafl_client", args.debug, false);

    if args.man {
        let mut buffer: Vec<u8> = Vec::default();
        let cmd = Args::command().name("hnefatafl-client").long_version(None);
        let man = clap_mangen::Man::new(cmd).date("2025-06-23");

        man.render(&mut buffer)?;
        write!(buffer, "{COPYRIGHT}")?;

        std::fs::write("hnefatafl-client.1", buffer)?;
        return Ok(());
    }

    let mut application = iced::application(init_client, Client::update, Client::view)
        .title("Hnefatafl Copenhagen")
        .subscription(Client::subscriptions)
        .window(window::Settings {
            #[cfg(target_os = "linux")]
            platform_specific: PlatformSpecific {
                application_id: APPLICATION_ID.to_string(),
                ..PlatformSpecific::default()
            },
            icon: Some(icon::from_rgba(KING.to_vec(), 256, 256)?),
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
                let mut ai = match choose_ai(&args.ai, args.seconds, args.depth, true) {
                    Ok(ai) => ai,
                    Err(error) => {
                        error!("{error}");
                        exit(1);
                    }
                };

                for tree in &rx {
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

#[allow(clippy::too_many_lines)]
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
                        executor::block_on(sender.send(Message::StreamConnected(tx.clone())))
                    {
                        error!("failed to send channel: {error}");
                        exit(1);
                    }

                    loop {
                        let message = match rx.recv() {
                            Ok(message) => message,
                            Err(error) => {
                                error!("rx: {error}");

                                if let Err(error) = executor::block_on(sender.send(Message::Exit)) {
                                    error!("{error}");
                                }

                                return;
                            }
                        };
                        let message_trim = message.trim();

                        if message_trim == "tcp_connect" {
                            break;
                        }
                    }

                    let Ok(mut tcp_stream) = TcpStream::connect(&address) else {
                        handle_error(executor::block_on(sender.send(Message::TcpDisconnect)));
                        handle_error(executor::block_on(sender.send(Message::ServerShutdown)));
                        continue 'start_over;
                    };

                    let mut reader = BufReader::new(handle_error(tcp_stream.try_clone()));
                    info!("connected to {address} ...");

                    let mut sender_clone = sender.clone();
                    thread::spawn(move || {
                        for message in rx {
                            let message_trim = message.trim();

                            if message_trim == "ping" {
                                trace!("<- {message_trim}");
                            } else {
                                debug!("<- {message_trim}");
                            }

                            if message_trim == "quit" {
                                if cfg!(not(target_os = "redox")) {
                                    tcp_stream
                                        .shutdown(Shutdown::Both)
                                        .expect("shutdown call failed");
                                }

                                return;
                            }

                            handle_error(tcp_stream.write_all(message.as_bytes()));
                        }

                        for _ in 0..2 {
                            if let Err(error) =
                                executor::block_on(sender_clone.send(Message::Leave))
                            {
                                error!("{error}");
                            }
                        }

                        if let Err(error) =
                            executor::block_on(sender_clone.send(Message::ServerShutdown))
                        {
                            error!("{error}");
                        }
                    });

                    let mut buffer = String::new();
                    handle_error(executor::block_on(
                        sender.send(Message::ConnectedTo(address.clone())),
                    ));

                    if cfg!(target_os = "redox") {
                        sleep(Duration::from_secs(1));
                    }

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

fn text_collect(text: SplitAsciiWhitespace<'_>) -> String {
    let text: Vec<&str> = text.collect();
    text.join(" ")
}
