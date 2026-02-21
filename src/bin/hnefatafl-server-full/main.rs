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

#![deny(clippy::expect_used)]
#![deny(clippy::indexing_slicing)]
#![deny(clippy::panic)]
#![deny(clippy::unwrap_used)]

mod accounts;
mod command_line;
mod remove_connection;
mod smtp;
mod tests;
mod unix_timestamp;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    process::exit,
    str::FromStr,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, sleep},
    time::Duration,
};

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{DateTime, Days, Local, Utc};
use clap::Parser;
use hnefatafl_copenhagen::{
    Id, SERVER_PORT, VERSION_ID,
    board::BoardSize,
    draw::Draw,
    email::Email,
    game::TimeUnix,
    glicko::Outcome,
    rating::Rated,
    role::Role,
    server_game::{
        ArchivedGame, Challenger, Messenger, ServerGame, ServerGameLight, ServerGameSerialized,
        ServerGames, ServerGamesLight,
    },
    status::Status,
    time::{Time, TimeEnum, TimeSettings},
    tournament::{Group, Record, Standing, Tournament},
    utils::{self, create_data_folder, data_file},
};
use itertools::Itertools;
use lettre::{
    SmtpTransport, Transport,
    message::{Mailbox, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use log::{debug, error, info, trace};
use old_rand::rngs::OsRng;
use password_hash::SaltString;
use rand::{random, seq::SliceRandom};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

use crate::{
    accounts::{Account, Accounts},
    command_line::Args,
    remove_connection::RemoveConnection,
    smtp::Smtp,
    unix_timestamp::UnixTimestamp,
};

const ACTIVE_GAMES_FILE: &str = "active-games.postcard";
const ARCHIVED_GAMES_FILE: &str = "archived-games.ron";
const KEEP_TEXTS: usize = 256;

const HOUR_IN_SECONDS: u64 = 60 * 60;
const DAY_IN_SECONDS: u64 = HOUR_IN_SECONDS * 24;

/// Seconds in two months: `60.0 * 60.0 * 24.0 * 30.417 * 2.0 = 5_256_057.6`
const TWO_MONTHS: i64 = 5_256_058;
const SEVEN_DAYS: i64 = 1_000 * 60 * 60 * 24 * 7;
const USERS_FILE: &str = "users.ron";
const MESSAGE_FILE: &str = "message.txt";

fn main() -> anyhow::Result<()> {
    // println!("{:x}", rand::random::<u32>());
    // return Ok(());

    let args = Args::parse();
    utils::init_logger("hnefatafl_server_full", args.debug, args.systemd);

    if args.man {
        return Args::generate_man_page();
    }

    create_data_folder()?;

    let (tx, rx) = mpsc::channel();
    let mut server = Server {
        tx: Some(tx.clone()),
        ..Server::default()
    };

    if args.skip_the_data_file {
        server.skip_the_data_files = true;
    } else {
        server.load_data_files(tx.clone(), args.systemd)?;
    }

    thread::spawn(move || handle_error(server.handle_messages(&rx)));

    if !args.skip_advertising_updates {
        Server::advertise_updates(tx.clone());
    }

    Server::check_update_rd_send(tx.clone());
    Server::new_tournament(tx.clone());
    Server::save(tx.clone());

    let mut address = "[::]".to_string();
    address.push_str(SERVER_PORT);

    let listener = match TcpListener::bind(&address) {
        Ok(listener) => listener,
        Err(error) => {
            error!("TcpLister::bind: {error}");

            address = "0.0.0.0".to_string();
            address.push_str(SERVER_PORT);
            TcpListener::bind(&address)?
        }
    };

    info!("listening on {address} ...");

    for (index, stream) in (1..).zip(listener.incoming()) {
        let stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                error!("stream: {error}");
                continue;
            }
        };

        if args.secure {
            let peer_address = match stream.peer_addr() {
                Ok(peer_address) => peer_address.ip(),
                Err(error) => {
                    error!("peer_address: {error}");
                    continue;
                }
            };

            let (tx_close, rx_close) = mpsc::channel();
            tx.send((
                format!("0 server connection_add {peer_address}"),
                Some(tx_close),
            ))?;

            match rx_close.recv() {
                Ok(close) => match close.parse() {
                    Ok(close) => {
                        if close {
                            continue;
                        }
                    }
                    Err(error) => {
                        error!("close 2: {error}");
                        continue;
                    }
                },
                Err(error) => {
                    error!("close 1: {error}");
                    continue;
                }
            }
        }

        let tx = tx.clone();

        thread::spawn(move || {
            if let Err(error) = login(index, stream, &tx) {
                error!("login: {error}");
            }
        });
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn login(
    id: Id,
    mut stream: TcpStream,
    tx: &mpsc::Sender<(String, Option<mpsc::Sender<String>>)>,
) -> anyhow::Result<()> {
    let args = Args::parse();

    let _remove_connection;
    if args.secure {
        _remove_connection = RemoveConnection {
            address: stream.peer_addr()?.ip(),
            tx: tx.clone(),
        };
    }

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut buf = String::new();
    let (client_tx, client_rx) = mpsc::channel();
    let mut username_proper = "_".to_string();
    let mut login_successful = false;

    for _ in 0..100 {
        reader.read_line(&mut buf)?;

        for ch in buf.trim().chars() {
            if ch.is_control() || ch == '\0' {
                return Err(anyhow::Error::msg(
                    "there are control characters in the username or password",
                ));
            }
        }

        if buf.trim().is_empty() {
            return Err(anyhow::Error::msg(
                "The user sent a command without logging in, then quit.",
            ));
        }

        let buf_clone = buf.clone();
        let mut username_password_etc = buf_clone.split_ascii_whitespace();

        let version_id = username_password_etc.next();
        let create_account_login = username_password_etc.next();
        let username_option = username_password_etc.next();

        if let (Some(version_id), Some(create_account_login), Some(username)) =
            (version_id, create_account_login, username_option)
        {
            username_proper = username.to_string();
            if version_id != VERSION_ID {
                stream.write_all(b"? login wrong_version\n")?;
                buf.clear();
                continue;
            }

            let password: Vec<&str> = username_password_etc.collect();
            let password = password.join(" ");

            if username.len() > 16 {
                stream.write_all(b"? login _ username is more than 16 characters\n")?;
                buf.clear();
                continue;
            }
            if password.len() > 32 {
                stream.write_all(b"? login _ password is more than 32 characters\n")?;
                buf.clear();
                continue;
            }

            debug!("{id} {username} {create_account_login} {password}");

            if create_account_login == "reset_password" {
                tx.send((
                    format!("0 {username} reset_password"),
                    Some(client_tx.clone()),
                ))?;

                stream.write_all(b"? login reset_password\n")?;

                buf.clear();
                continue;
            }

            tx.send((
                format!("{id} {username} {create_account_login} {password}"),
                Some(client_tx.clone()),
            ))?;

            let message = client_rx.recv()?;
            buf.clear();
            if create_account_login == "login" {
                if "= login" == message.as_str() {
                    login_successful = true;
                    break;
                }

                stream.write_all(b"? login multiple_possible_errors\n")?;
                continue;
            } else if create_account_login == "create_account" {
                if "= create_account" == message.as_str() {
                    login_successful = true;
                    break;
                }

                stream.write_all(b"? create_account\n")?;
                continue;
            }

            stream.write_all(b"? login _\n")?;
        }

        buf.clear();
    }

    if !login_successful {
        return Err(anyhow::Error::msg("the user failed to login"));
    }
    stream.write_all(b"= login\n")?;

    thread::spawn(move || {
        if let Err(error) = receiving_and_writing(stream, &client_rx) {
            error!("receiving_and_writing: {error}");
        }
    });

    tx.send((format!("{id} {username_proper} email_get"), None))?;
    tx.send((format!("{id} {username_proper} texts"), None))?;
    tx.send((format!("{id} {username_proper} message"), None))?;
    tx.send((format!("{id} {username_proper} display_games"), None))?;
    tx.send((format!("{id} {username_proper} tournament_status_0"), None))?;
    tx.send((format!("{id} {username_proper} admin"), None))?;

    let mut game_id = None;
    'outer: for _ in 0..1_000_000 {
        if let Err(err) = reader.read_line(&mut buf) {
            error!("reader.read_line(): {err}");
            break 'outer;
        }

        let buf_str = buf.trim();

        if buf_str.is_empty() {
            break 'outer;
        }

        for char in buf_str.chars() {
            if char.is_control() || char == '\0' {
                break 'outer;
            }
        }

        let words: Vec<_> = buf_str.split_whitespace().collect();
        if let Some(first) = words.first() {
            if (*first == "join_game" || *first == "join_game_pending" || *first == "resume_game")
                && let Some(second) = words.get(1)
                && let Ok(id) = u128::from_str(second)
            {
                game_id = Some(id);
            }

            if *first == "leave_game" {
                game_id = None;
            }
        }

        tx.send((format!("{id} {username_proper} {buf_str}"), None))?;
        buf.clear();
    }

    if let Some(game_id) = game_id {
        tx.send((format!("{id} {username_proper} leave_game {game_id}"), None))?;
    }

    tx.send((format!("{id} {username_proper} logout"), None))?;
    Ok(())
}

fn receiving_and_writing<T: Send + Write>(
    mut stream: T,
    client_rx: &Receiver<String>,
) -> anyhow::Result<()> {
    for mut message in client_rx {
        match message.as_str() {
            "= archived_games" => {
                let ron_archived_games = client_rx.recv()?;
                let archived_games: Vec<ArchivedGame> = ron::from_str(&ron_archived_games)?;
                let postcard_archived_games = &postcard::to_allocvec(&archived_games)?;

                writeln!(message, " {}", postcard_archived_games.len())?;
                stream.write_all(message.as_bytes())?;
                stream.write_all(postcard_archived_games)?;
            }
            "= logout" => return Ok(()),
            _ => {
                message.push('\n');
                if let Err(error) = stream.write_all(message.as_bytes()) {
                    return Err(anyhow::Error::msg(format!("{message}: {error}")));
                }
            }
        }
    }

    Ok(())
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

fn hash_password(password: &str) -> Option<String> {
    let ctx = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    Some(
        ctx.hash_password(password.as_bytes(), &salt)
            .ok()?
            .to_string(),
    )
}

fn timestamp() -> String {
    Utc::now().format("[%F %T UTC]").to_string()
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Server {
    #[serde(default)]
    game_id: Id,
    #[serde(default)]
    ran_update_rd: UnixTimestamp,
    #[serde(default)]
    admins: HashSet<String>,
    #[serde(default)]
    smtp: Smtp,
    #[serde(default)]
    tournament: Option<Tournament>,
    #[serde(default)]
    accounts: Accounts,
    #[serde(skip)]
    accounts_old: Accounts,
    #[serde(skip)]
    archived_games: Vec<ArchivedGame>,
    #[serde(skip)]
    clients: HashMap<usize, mpsc::Sender<String>>,
    #[serde(skip)]
    connections: HashMap<String, u128>,
    #[serde(skip)]
    games: ServerGames,
    #[serde(skip)]
    games_light: ServerGamesLight,
    #[serde(skip)]
    games_light_old: ServerGamesLight,
    #[serde(skip)]
    skip_the_data_files: bool,
    #[serde(default)]
    texts: VecDeque<String>,
    #[serde(skip)]
    tx: Option<mpsc::Sender<(String, Option<mpsc::Sender<String>>)>>,
}

impl Server {
    fn advertise_updates(tx: Sender<(String, Option<Sender<String>>)>) {
        thread::spawn(move || {
            loop {
                handle_error(tx.send(("0 server display_server".to_string(), None)));
                thread::sleep(Duration::from_secs(1));
            }
        });
    }

    fn append_archived_game(&mut self, game: ServerGame) -> anyhow::Result<()> {
        let Some(attacker) = self.accounts.0.get(&game.attacker) else {
            return Err(anyhow::Error::msg("failed to get rating!"));
        };
        let Some(defender) = self.accounts.0.get(&game.defender) else {
            return Err(anyhow::Error::msg("failed to get rating!"));
        };
        let game = ArchivedGame::new(game, attacker.rating.clone(), defender.rating.clone());

        let archived_games_file = data_file(ARCHIVED_GAMES_FILE);
        let mut game_string = ron::ser::to_string(&game)?;
        game_string.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(archived_games_file)?;

        file.write_all(game_string.as_bytes())?;

        self.archived_games.push(game);

        Ok(())
    }

    fn bcc_mailboxes(&self, username: &str) -> Vec<Mailbox> {
        let mut emails = Vec::new();

        if let Some(account) = self.accounts.0.get(username)
            && account.send_emails
        {
            for account in self.accounts.0.values() {
                if let Some(email) = &account.email
                    && email.verified
                    && let Some(email) = email.to_mailbox()
                {
                    emails.push(email);
                }
            }
        }

        emails
    }

    fn bcc_send(&self, username: &str) -> String {
        let mut emails = Vec::new();

        if let Some(account) = self.accounts.0.get(username)
            && account.send_emails
        {
            for account in self.accounts.0.values() {
                if let Some(email) = &account.email
                    && email.verified
                {
                    emails.push(email.tx());
                }
            }
        }

        emails.sort();
        emails.join(" ")
    }

    /// ```sh
    /// # PASSWORD can be the empty string.
    /// <- change_password PASSWORD
    /// -> = change_password
    /// ```
    fn change_password(
        &mut self,
        username: &str,
        index_supplied: usize,
        command: &str,
        the_rest: &[&str],
    ) -> Option<(mpsc::Sender<String>, bool, String)> {
        info!("{index_supplied} {username} change_password");

        let account = self.accounts.0.get_mut(username)?;
        let password = the_rest.join(" ");

        if password.len() > 32 {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                format!("{command} password is greater than 32 characters"),
            ));
        }

        let hash = hash_password(&password)?;
        account.password = hash;

        Some((
            self.clients.get(&index_supplied)?.clone(),
            true,
            (*command).to_string(),
        ))
    }

    /// ```sh
    /// # server internal
    /// ```
    ///
    /// c = 63.2
    ///
    /// This assumes 30 2 month periods must pass before one's rating
    /// deviation is the same as a new player and that a typical RD is 50.
    #[must_use]
    fn check_update_rd(&mut self) -> bool {
        let now = Local::now().to_utc().timestamp();
        if now - self.ran_update_rd.0 >= TWO_MONTHS {
            for account in self.accounts.0.values_mut() {
                account.rating.update_rd();
            }
            self.ran_update_rd = UnixTimestamp(now);
            true
        } else {
            false
        }
    }

    fn check_update_rd_send(tx: Sender<(String, Option<Sender<String>>)>) {
        thread::spawn(move || {
            loop {
                handle_error(tx.send(("0 server check_update_rd".to_string(), None)));
                thread::sleep(Duration::from_secs(DAY_IN_SECONDS));
            }
        });
    }

    /// ```sh
    /// # PASSWORD can be the empty string.
    /// <- VERSION_ID create_account player-1 PASSWORD
    /// -> = login
    /// ```
    fn create_account(
        &mut self,
        username: &str,
        index_supplied: usize,
        command: &str,
        the_rest: &[&str],
        option_tx: Option<Sender<String>>,
    ) -> Option<(mpsc::Sender<String>, bool, String)> {
        let password = the_rest.join(" ");
        let tx = option_tx?;

        if self.accounts.0.contains_key(username) || username == "server" {
            info!("{index_supplied} {username} is already in the database");
            Some((tx, false, (*command).to_string()))
        } else {
            info!("{index_supplied} {username} created user account");

            let hash = hash_password(&password)?;
            self.clients.insert(index_supplied, tx);
            self.accounts.0.insert(
                (*username).to_string(),
                Account {
                    password: hash,
                    logged_in: Some(index_supplied),
                    ..Default::default()
                },
            );

            Some((
                self.clients.get(&index_supplied)?.clone(),
                true,
                (*command).to_string(),
            ))
        }
    }

    fn decline_game(
        &mut self,
        username: &str,
        index_supplied: usize,
        mut command: String,
        the_rest: &[&str],
    ) -> Option<(mpsc::Sender<String>, bool, String)> {
        let channel = self.clients.get(&index_supplied)?;

        let Some(id) = the_rest.first() else {
            return Some((channel.clone(), false, command));
        };
        let Ok(id) = id.parse::<Id>() else {
            return Some((channel.clone(), false, command));
        };

        let mut switch = false;
        if let Some(&"switch") = the_rest.get(1) {
            switch = true;
        }

        info!("{index_supplied} {username} decline_game {id} switch={switch}");

        if let Some(game_old) = self.games_light.0.remove(&id) {
            let mut attacker = None;
            let mut defender = None;

            if switch {
                if Some(username.to_string()) == game_old.attacker {
                    defender = game_old.defender;
                } else if Some(username.to_string()) == game_old.defender {
                    attacker = game_old.attacker;
                }
            } else if Some(username.to_string()) == game_old.attacker {
                attacker = game_old.attacker;
            } else if Some(username.to_string()) == game_old.defender {
                defender = game_old.defender;
            }

            let game = ServerGameLight {
                id,
                attacker,
                defender,
                challenger: Challenger::default(),
                rated: game_old.rated,
                timed: game_old.timed,
                board_size: game_old.board_size,
                spectators: game_old.spectators,
                challenge_accepted: false,
                game_over: false,
            };

            command = format!("{command} {game:?}");
            self.games_light.0.insert(id, game);
        }

        Some((channel.clone(), true, command))
    }

    fn delete_account(&mut self, username: &str, index_supplied: usize) {
        info!("{index_supplied} {username} delete_account");

        self.accounts.0.remove(username);
    }

    #[allow(clippy::too_many_lines)]
    fn display_server(&mut self, username: &str) -> Option<(mpsc::Sender<String>, bool, String)> {
        if self.games_light != self.games_light_old {
            debug!("0 {username} display_games");
            self.games_light_old = self.games_light.clone();

            let mut names = HashMap::new();
            for (name, account) in &self.accounts.0 {
                if let Some(id) = account.logged_in {
                    names.insert(id, name);
                }
            }

            for (id, tx) in &mut self.clients {
                let Ok(games) = self
                    .games_light
                    .display_games(names.get(id).map(|s| s.as_str()))
                else {
                    continue;
                };

                let _ok = tx.send(format!("= display_games {games}"));
            }
        }

        if self.accounts != self.accounts_old {
            debug!("0 {username} display_users");
            self.accounts_old = self.accounts.clone();

            for tx in &mut self.clients.values() {
                let _ok = tx.send(format!("= display_users {}", &self.accounts));
            }
        }

        for game in self.games.0.values_mut() {
            match game.game.turn {
                Role::Attacker => {
                    if game.game.status == Status::Ongoing
                        && let TimeUnix::Time(game_time) = &mut game.game.time
                    {
                        let now = Local::now().to_utc().timestamp_millis();
                        let elapsed_time = now - *game_time;
                        game.elapsed_time += elapsed_time;
                        *game_time = now;

                        if game.elapsed_time > SEVEN_DAYS
                            && let Some(tx) = &mut self.tx
                        {
                            let _ok = tx.send((
                                format!(
                                    "0 {} game {} play attacker resigns _",
                                    game.attacker, game.id
                                ),
                                None,
                            ));
                            return None;
                        }

                        if let TimeSettings::Timed(attacker_time) = &mut game.game.attacker_time {
                            if attacker_time.milliseconds_left > 0 {
                                attacker_time.milliseconds_left -= elapsed_time;
                            } else if let Some(tx) = &mut self.tx {
                                let _ok = tx.send((
                                    format!(
                                        "0 {} game {} play attacker resigns _",
                                        game.attacker, game.id
                                    ),
                                    None,
                                ));
                            }
                        }
                    }
                }
                Role::Roleless => {}
                Role::Defender => {
                    if game.game.status == Status::Ongoing
                        && let TimeUnix::Time(game_time) = &mut game.game.time
                    {
                        let now = Local::now().to_utc().timestamp_millis();
                        let elapsed_time = now - *game_time;
                        game.elapsed_time += elapsed_time;
                        *game_time = now;

                        if game.elapsed_time > SEVEN_DAYS
                            && let Some(tx) = &mut self.tx
                        {
                            let _ok = tx.send((
                                format!(
                                    "0 {} game {} play defender resigns _",
                                    game.defender, game.id
                                ),
                                None,
                            ));
                            return None;
                        }

                        if let TimeSettings::Timed(defender_time) = &mut game.game.defender_time {
                            if defender_time.milliseconds_left > 0 {
                                defender_time.milliseconds_left -= elapsed_time;
                            } else if let Some(tx) = &mut self.tx {
                                let _ok = tx.send((
                                    format!(
                                        "0 {} game {} play defender resigns _",
                                        game.defender, game.id
                                    ),
                                    None,
                                ));
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn draw(
        &mut self,
        index_supplied: usize,
        command: &str,
        the_rest: &[&str],
    ) -> Option<(mpsc::Sender<String>, bool, String)> {
        let Some(id) = the_rest.first() else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };
        let Ok(id) = id.parse::<Id>() else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };

        let Some(draw) = the_rest.get(1) else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };
        let Ok(draw) = Draw::from_str(draw) else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };

        let Some(mut game) = self.games.0.remove(&id) else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };

        let message = format!("= draw {draw}");
        game.attacker_tx.send(message.clone());
        game.defender_tx.send(message.clone());

        if draw == Draw::Accept {
            let Some(game_light) = self.games_light.0.get(&id) else {
                return Some((
                    self.clients.get(&index_supplied)?.clone(),
                    false,
                    (*command).to_string(),
                ));
            };

            for spectator in game_light.spectators() {
                if let Some(sender) = self.clients.get(&spectator) {
                    let _ok = sender.send(message.clone());
                }
            }

            game.game.status = Status::Draw;

            let accounts = &mut self.accounts.0;
            let (attacker_rating, defender_rating) = if let (Some(attacker), Some(defender)) =
                (accounts.get(&game.attacker), accounts.get(&game.defender))
            {
                (attacker.rating.rating, defender.rating.rating)
            } else {
                unreachable!();
            };

            if let Some(attacker) = accounts.get_mut(&game.attacker) {
                attacker.draws += 1;

                if game.rated.into() {
                    attacker
                        .rating
                        .update_rating(defender_rating, &Outcome::Draw);
                }
            }
            if let Some(defender) = accounts.get_mut(&game.defender) {
                defender.draws += 1;

                if game.rated.into() {
                    defender
                        .rating
                        .update_rating(attacker_rating, &Outcome::Draw);
                }
            }

            if let Some(game) = self.games_light.0.get_mut(&id) {
                game.game_over = true;
            }

            if !self.skip_the_data_files {
                self.append_archived_game(game)
                    .map_err(|err| {
                        error!("append_archived_games: {err}");
                    })
                    .ok()?;
            }
        }

        None
    }

    #[allow(clippy::too_many_lines)]
    fn game(
        &mut self,
        index_supplied: usize,
        username: &str,
        command: &str,
        the_rest: &[&str],
    ) -> Option<(mpsc::Sender<String>, bool, String)> {
        if the_rest.len() < 5 {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        }

        let index = the_rest.first()?;
        let Ok(index) = index.parse() else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };
        let role = the_rest.get(2)?;
        let Ok(role) = Role::from_str(role) else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };
        let from = the_rest.get(3)?;
        let to = the_rest.get(4)?;
        let mut to = (*to).to_string();
        if to == "_" {
            to = String::new();
        }

        let Some(game) = self.games.0.get_mut(&index) else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };

        let Some(game_light) = self.games_light.0.get_mut(&index) else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };

        game.elapsed_time = 0;
        game.draw_requested = Role::Roleless;

        let mut attackers_turn_next = true;
        if role == Role::Attacker {
            if *username == game.attacker {
                game.game
                    .read_line(&format!("play attacker {from} {to}"))
                    .map_err(|error| {
                        error!("play attacker {from} {to}: {error}");
                        error
                    })
                    .ok()?;

                attackers_turn_next = false;

                let message = format!("game {index} play attacker {from} {to}");
                for spectator in game_light.spectators() {
                    if let Some(client) = self.clients.get(&spectator) {
                        let _ok = client.send(message.clone());
                    }
                }
                game.defender_tx.send(message);
            } else {
                return Some((
                    self.clients.get(&index_supplied)?.clone(),
                    false,
                    (*command).to_string(),
                ));
            }
        } else if *username == game.defender {
            game.game
                .read_line(&format!("play defender {from} {to}"))
                .map_err(|error| {
                    error!("play defender {from} {to}: {error}");
                    error
                })
                .ok()?;

            let message = format!("game {index} play defender {from} {to}");
            for spectator in game_light.spectators() {
                if let Some(client) = self.clients.get(&spectator) {
                    let _ok = client.send(message.clone());
                }
            }
            game.attacker_tx.send(message);
        } else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        }

        let mut game_over = false;

        match game.game.status {
            Status::AttackerWins => {
                let accounts = &mut self.accounts.0;
                let (attacker_rating, defender_rating) = if let (Some(attacker), Some(defender)) =
                    (accounts.get(&game.attacker), accounts.get(&game.defender))
                {
                    (attacker.rating.rating, defender.rating.rating)
                } else {
                    unreachable!();
                };

                if let Some(attacker) = accounts.get_mut(&game.attacker) {
                    attacker.wins += 1;

                    if game.rated.into() {
                        attacker
                            .rating
                            .update_rating(defender_rating, &Outcome::Win);
                    }
                }
                if let Some(defender) = accounts.get_mut(&game.defender) {
                    defender.losses += 1;

                    if game.rated.into() {
                        defender
                            .rating
                            .update_rating(attacker_rating, &Outcome::Loss);
                    }
                }

                let message = format!("= game_over {index} attacker_wins");
                game.attacker_tx.send(message.clone());
                game.defender_tx.send(message.clone());

                for spectator in game_light.spectators() {
                    if let Some(sender) = self.clients.get(&spectator) {
                        let _ok = sender.send(message.clone());
                    }
                }

                game_over = true;
            }
            Status::Draw => {
                // Handled in the draw fn.
            }
            Status::Ongoing => {
                if attackers_turn_next {
                    game.attacker_tx
                        .send(format!("game {index} generate_move attacker"));
                } else {
                    game.defender_tx
                        .send(format!("game {index} generate_move defender"));
                }
            }
            Status::DefenderWins => {
                let accounts = &mut self.accounts.0;
                let (attacker_rating, defender_rating) = if let (Some(attacker), Some(defender)) =
                    (accounts.get(&game.attacker), accounts.get(&game.defender))
                {
                    (attacker.rating.rating, defender.rating.rating)
                } else {
                    unreachable!();
                };

                if let Some(attacker) = accounts.get_mut(&game.attacker) {
                    attacker.losses += 1;

                    if game.rated.into() {
                        attacker
                            .rating
                            .update_rating(defender_rating, &Outcome::Loss);
                    }
                }
                if let Some(defender) = accounts.get_mut(&game.defender) {
                    defender.wins += 1;

                    if game.rated.into() {
                        defender
                            .rating
                            .update_rating(attacker_rating, &Outcome::Win);
                    }
                }

                let message = format!("= game_over {index} defender_wins");
                game.attacker_tx.send(message.clone());
                game.defender_tx.send(message.clone());

                for id in game_light.spectators() {
                    if let Some(sender) = self.clients.get(&id) {
                        let _ok = sender.send(message.clone());
                    }
                }

                game_over = true;
            }
        }

        if game_over {
            let Some(game) = self.games.0.remove(&index) else {
                unreachable!()
            };

            if let Some(game_light) = self.games_light.0.get_mut(&index) {
                game_light.game_over = true;
            }

            if let Some(tournament) = &mut self.tournament
                && let Some(group) = tournament.tournament_games.get_mut(&game.id)
            {
                if let Ok(mut group) = group.lock() {
                    match game.game.status {
                        Status::AttackerWins => {
                            if let Some(record) = group.records.get_mut(game.attacker.as_str()) {
                                record.wins += 1;
                            }
                            if let Some(record) = group.records.get_mut(game.defender.as_str()) {
                                record.losses += 1;
                            }
                        }
                        Status::Draw => {
                            if let Some(record) = group.records.get_mut(game.attacker.as_str()) {
                                record.draws += 1;
                            }
                            if let Some(record) = group.records.get_mut(game.defender.as_str()) {
                                record.draws += 1;
                            }
                        }
                        Status::Ongoing => {}
                        Status::DefenderWins => {
                            if let Some(record) = group.records.get_mut(game.attacker.as_str()) {
                                record.losses += 1;
                            }
                            if let Some(record) = group.records.get_mut(game.defender.as_str()) {
                                record.wins += 1;
                            }
                        }
                    }

                    let mut group_finished = true;
                    for record in group.records.values() {
                        if group.total_games != record.games_count() {
                            group_finished = false;
                        }
                    }

                    if group_finished {
                        let mut standings = Vec::new();
                        let mut players = Vec::new();
                        let mut previous_score = -1.0;

                        for (name, record) in &group.records {
                            players.push(name.to_string());
                            let score = record.score();
                            if score != previous_score {
                                standings.push(Standing {
                                    score: record.score(),
                                    players: players.clone(),
                                });
                            } else if let Some(standing) = standings.last_mut() {
                                standing.players.push(name.to_string());
                            }

                            previous_score = score;
                        }

                        group.finishing_standings = standings;

                        println!("{:#?}", group.finishing_standings);
                    }
                }

                tournament.tournament_games.remove(&game.id);
                self.tournament_status_all();
            }

            if !self.skip_the_data_files {
                self.append_archived_game(game)
                    .map_err(|err| {
                        error!("append_archived_game: {err}");
                    })
                    .ok()?;
            }

            return None;
        }

        Some((
            self.clients.get(&index_supplied)?.clone(),
            true,
            (*command).to_string(),
        ))
    }

    fn generate_first_round(&mut self) {
        let mut round = None;

        if let Some(tournament) = &mut self.tournament {
            let mut players_vec = Vec::new();

            for player in &tournament.players {
                let mut rating = 1500.0;

                if let Some(account) = self.accounts.0.get(player.as_str()) {
                    rating = account.rating.rating.round_ties_even();
                }

                players_vec.push((player.to_string(), rating));
            }

            let players_len = players_vec.len();
            let mut rng = rand::rng();
            players_vec.shuffle(&mut rng);
            players_vec.sort_unstable_by(|a, b| a.1.total_cmp(&b.1));

            // Or if all players had the same number of wins, losses, and draws in the last round.
            // if players_len == 1: The tournament is over...

            let groups_number = players_len / 8 + 1;
            let mut group_size = 0;

            while (group_size + 1) * groups_number <= players_len {
                group_size += 1;
            }

            let mut remainder = if group_size > 0 {
                players_len % group_size
            } else {
                0
            };

            let mut groups = Vec::new();
            for _ in 0..groups_number {
                let mut group = Group::default();

                for _ in 0..group_size {
                    group.records.insert(
                        players_vec
                            .pop()
                            .expect("There should be a player to pop.")
                            .0,
                        Record::default(),
                    );
                }

                if remainder > 0 {
                    group.records.insert(
                        players_vec
                            .pop()
                            .expect("There should be a player to pop.")
                            .0,
                        Record::default(),
                    );
                    remainder = remainder.saturating_sub(1);
                }

                groups.push(group);
            }

            round = Some(groups);
        }

        let mut ids = VecDeque::new();
        let mut groups_arc_mutex = Vec::new();

        if let Some(groups) = round {
            for mut group in groups.into_iter() {
                for combination in group.records.iter().map(|record| record.0).combinations(2) {
                    if let (Some(first), Some(second)) = (combination.first(), combination.get(1)) {
                        ids.push_back(self.new_tournament_game(first, second));
                        ids.push_back(self.new_tournament_game(second, first));
                        group.total_games += 2;
                    }
                }

                groups_arc_mutex.push(Arc::new(Mutex::new(group)));
            }
        }

        if !groups_arc_mutex.is_empty()
            && let Some(tournament) = &mut self.tournament
        {
            for group in &groups_arc_mutex {
                if let Some(id) = ids.pop_front() {
                    tournament.tournament_games.insert(id, group.clone());
                }
                if let Some(id) = ids.pop_front() {
                    tournament.tournament_games.insert(id, group.clone());
                }
            }

            tournament.groups = Some(vec![groups_arc_mutex]);
        }
    }

    fn set_email(
        &mut self,
        index_supplied: usize,
        username: &str,
        command: &str,
        email: Option<&str>,
    ) -> Option<(mpsc::Sender<String>, bool, String)> {
        let Some(address) = email else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };

        let Some(account) = self.accounts.0.get_mut(username) else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };

        let random_u32 = random();
        let email = Email {
            address: address.to_string(),
            code: Some(random_u32),
            username: username.to_string(),
            verified: false,
        };

        info!("{index_supplied} {username} email {}", email.tx());

        let email_send = lettre::Message::builder()
            .from("Hnefatafl Org <no-reply@hnefatafl.org>".parse().ok()?)
            .to(email.to_mailbox()?)
            .subject("Account Verification")
            .header(ContentType::TEXT_PLAIN)
            .body(format!(
                "Dear {username},\nyour email verification code is as follows: {random_u32:x}",
            ))
            .ok()?;

        let credentials = Credentials::new(self.smtp.username.clone(), self.smtp.password.clone());

        let mailer = SmtpTransport::relay(&self.smtp.service)
            .ok()?
            .credentials(credentials)
            .build();

        match mailer.send(&email_send) {
            Ok(_) => {
                info!("email sent to {address} successfully!");

                account.email = Some(email);

                let reply = format!("email {address} false");
                Some((self.clients.get(&index_supplied)?.clone(), true, reply))
            }
            Err(err) => {
                let reply = format!("could not send email to {address}");
                error!("{reply}: {err}");

                Some((self.clients.get(&index_supplied)?.clone(), false, reply))
            }
        }
    }

    fn handle_messages(
        &mut self,
        rx: &mpsc::Receiver<(String, Option<mpsc::Sender<String>>)>,
    ) -> anyhow::Result<()> {
        loop {
            if let Some((tx, ok, command)) = self.handle_messages_internal(rx) {
                if ok {
                    tx.send(format!("= {command}"))?;
                } else {
                    tx.send(format!("? {command}"))?;
                }
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn handle_messages_internal(
        &mut self,
        rx: &mpsc::Receiver<(String, Option<mpsc::Sender<String>>)>,
    ) -> Option<(mpsc::Sender<String>, bool, String)> {
        let args = Args::parse();

        let (message, option_tx) = rx.recv().ok()?;
        let index_username_command: Vec<_> = message.split_ascii_whitespace().collect();

        if let (Some(index_supplied), Some(username), Some(command)) = (
            index_username_command.first(),
            index_username_command.get(1),
            index_username_command.get(2),
        ) {
            if *command != "check_update_rd"
                && *command != "create_account"
                && *command != "display_server"
                && *command != "join_game_pending"
                && *command != "leave_game"
                && *command != "login"
                && *command != "logout"
                && *command != "ping"
                && *command != "resume_game"
            {
                debug!("{index_supplied} {username} {command}");
            }

            let index_supplied = index_supplied.parse::<usize>().ok()?;
            let the_rest: Vec<_> = index_username_command.clone().into_iter().skip(3).collect();

            match *command {
                "admin" => {
                    if self.admins.contains(*username) {
                        self.clients
                            .get(&index_supplied)?
                            .send("= admin".to_string())
                            .ok()?;
                    }

                    None
                }
                "archived_games" => {
                    self.clients
                        .get(&index_supplied)?
                        .send("= archived_games".to_string())
                        .ok()?;

                    self.clients
                        .get(&index_supplied)?
                        .send(ron::ser::to_string(&self.archived_games).ok()?)
                        .ok()?;

                    None
                }
                "change_password" => {
                    self.change_password(username, index_supplied, command, the_rest.as_slice())
                }
                "check_update_rd" => {
                    let bool = self.check_update_rd();
                    info!("0 {username} check_update_rd {bool}");
                    None
                }
                "connection_add" => {
                    if let Some(address) = the_rest.first()
                        && let Some(tx) = option_tx
                    {
                        if let Some(connections) = self.connections.get(*address)
                            && *connections > 2_000
                        {
                            tx.send("true".to_string()).ok()?;
                        } else {
                            tx.send("false".to_string()).ok()?;

                            let entry = self.connections.entry(address.to_string());
                            entry.and_modify(|value| *value += 1).or_insert(1);
                        }
                    }

                    debug!("connections: {:?}", self.connections);

                    None
                }
                "connection_remove" => {
                    if let Some(connection) = the_rest.first() {
                        let entry = self.connections.entry(connection.to_string());
                        entry.and_modify(|value| *value = value.saturating_sub(1));

                        if let Some(value) = self.connections.get(*connection)
                            && *value == 0
                        {
                            self.connections.remove(*connection);
                        }
                    }

                    debug!("connections: {:?}", self.connections);

                    None
                }
                "create_account" => self.create_account(
                    username,
                    index_supplied,
                    command,
                    the_rest.as_slice(),
                    option_tx,
                ),
                "decline_game" => self.decline_game(
                    username,
                    index_supplied,
                    (*command).to_string(),
                    the_rest.as_slice(),
                ),
                "delete_account" => {
                    self.delete_account(username, index_supplied);
                    None
                }
                "display_games" => {
                    if args.skip_advertising_updates {
                        None
                    } else {
                        self.clients.get(&index_supplied).map(|tx| {
                            (
                                tx.clone(),
                                true,
                                format!("display_games {:?}", &self.games_light),
                            )
                        })
                    }
                }
                "display_server" => self.display_server(username),
                "draw" => self.draw(index_supplied, command, the_rest.as_slice()),
                "game" => self.game(index_supplied, username, command, the_rest.as_slice()),
                "email" => {
                    self.set_email(index_supplied, username, command, the_rest.first().copied())
                }
                "email_everyone" => {
                    if self.admins.contains(*username) {
                        info!("{index_supplied} {username} email_everyone");
                    } else {
                        error!("{index_supplied} {username} email_everyone");
                        return None;
                    }

                    let emails_bcc = self.bcc_mailboxes(username);
                    let subject = the_rest.first()?;
                    let email_string = the_rest.get(1..)?.join(" ").replace("\\n", "\n");
                    let mut email = lettre::Message::builder();

                    for email_bcc in emails_bcc {
                        email = email.bcc(email_bcc);
                    }

                    let email = email
                        .from("Hnefatafl Org <no-reply@hnefatafl.org>".parse().ok()?)
                        .subject(*subject)
                        .header(ContentType::TEXT_PLAIN)
                        .body(email_string)
                        .ok()?;

                    let credentials =
                        Credentials::new(self.smtp.username.clone(), self.smtp.password.clone());

                    let mailer = SmtpTransport::relay(&self.smtp.service)
                        .ok()?
                        .credentials(credentials)
                        .build();

                    match mailer.send(&email) {
                        Ok(_) => {
                            info!("emails sent successfully!");

                            Some((
                                self.clients.get(&index_supplied)?.clone(),
                                true,
                                (*command).to_string(),
                            ))
                        }
                        Err(err) => {
                            let reply = "could not send emails";
                            error!("{reply}: {err}");

                            Some((
                                self.clients.get(&index_supplied)?.clone(),
                                false,
                                reply.to_string(),
                            ))
                        }
                    }
                }
                "emails_bcc" => {
                    let emails_bcc = self.bcc_send(username);

                    if !emails_bcc.is_empty() {
                        self.clients
                            .get(&index_supplied)?
                            .send(format!("= emails_bcc {emails_bcc}"))
                            .ok()?;
                    }

                    None
                }
                "email_code" => {
                    if let Some(account) = self.accounts.0.get_mut(*username)
                        && let Some(email) = &mut account.email
                        && let (Some(code_1), Some(code_2)) = (email.code, the_rest.first())
                    {
                        if format!("{code_1:x}") == *code_2 {
                            email.verified = true;

                            self.clients
                                .get(&index_supplied)?
                                .send("= email_code".to_string())
                                .ok()?;
                        } else {
                            email.verified = false;

                            self.clients
                                .get(&index_supplied)?
                                .send("? email_code".to_string())
                                .ok()?;
                        }
                    }

                    None
                }
                "email_get" => {
                    if let Some(account) = self.accounts.0.get(*username)
                        && let Some(email) = &account.email
                    {
                        self.clients
                            .get(&index_supplied)?
                            .send(format!("= email {} {}", email.address, email.verified))
                            .ok()?;
                    }

                    None
                }
                "email_reset" => {
                    if let Some(account) = self.accounts.0.get_mut(*username) {
                        account.email = None;

                        Some((
                            self.clients.get(&index_supplied)?.clone(),
                            true,
                            (*command).to_string(),
                        ))
                    } else {
                        None
                    }
                }
                "exit" => {
                    info!("saving active games...");
                    let mut active_games = Vec::new();
                    for game in self.games.0.values() {
                        let mut serialized_game = ServerGameSerialized::from(game);

                        if let Some(game_light) = self.games_light.0.get(&game.id) {
                            serialized_game.timed = game_light.timed.clone();
                        }

                        active_games.push(serialized_game);
                    }

                    let mut file = handle_error(File::create(data_file(ACTIVE_GAMES_FILE)));
                    handle_error(
                        file.write_all(
                            handle_error(postcard::to_allocvec(&active_games)).as_slice(),
                        ),
                    );

                    exit(0);
                }
                "join_game" => self.join_game(
                    username,
                    index_supplied,
                    (*command).to_string(),
                    the_rest.as_slice(),
                ),
                "join_game_pending" => self.join_game_pending(
                    (*username).to_string(),
                    index_supplied,
                    (*command).to_string(),
                    the_rest.as_slice(),
                ),
                "join_tournament" => {
                    if let Some(tournament) = &mut self.tournament {
                        tournament.players.insert(username.to_string());
                        self.tournament_status_all();
                    }

                    None
                }
                "leave_game" => self.leave_game(
                    username,
                    index_supplied,
                    (*command).to_string(),
                    the_rest.as_slice(),
                ),
                "leave_tournament" => {
                    if let Some(tournament) = &mut self.tournament {
                        tournament.players.remove(*username);
                        self.tournament_status_all();
                    }

                    None
                }
                "login" => self.login(
                    username,
                    index_supplied,
                    command,
                    the_rest.as_slice(),
                    option_tx,
                ),
                "logout" => self.logout(username, index_supplied, command),
                "message" => {
                    if Args::parse().skip_message {
                        return None;
                    }

                    let message_file = data_file(MESSAGE_FILE);
                    let mut message = String::new();

                    match fs::read_to_string(&message_file) {
                        Ok(new_message) => message = new_message.trim().replace('\n', "\\n"),
                        Err(err) => match err.kind() {
                            ErrorKind::NotFound => {}
                            _ => error!("Error loading message: {err}"),
                        },
                    }

                    if message.trim().is_empty() {
                        return None;
                    }

                    self.clients
                        .get(&index_supplied)?
                        .send(format!("= message {message}"))
                        .ok()?;

                    None
                }
                "new_game" => self.new_game(username, index_supplied, command, the_rest.as_slice()),
                "ping" => Some((
                    self.clients.get(&index_supplied)?.clone(),
                    true,
                    (*command).to_string(),
                )),
                "reset_password" => {
                    let account = self.accounts.0.get_mut(*username)?;
                    if let Some(email) = &account.email {
                        if email.verified {
                            let day = 60 * 60 * 24;
                            let now = Utc::now().timestamp();
                            if now - account.email_sent > day {
                                let password = format!("{:x}", random::<u32>());
                                account.password = hash_password(&password)?;

                                let message = lettre::Message::builder()
                                .from("Hnefatafl Org <no-reply@hnefatafl.org>".parse().ok()?)
                                .to(email.to_mailbox()?)
                                .subject("Password Reset")
                                .header(ContentType::TEXT_PLAIN)
                                .body(format!(
                                    "Dear {username},\nyour new password is as follows: {password}",
                                ))
                                .ok()?;

                                let credentials = Credentials::new(
                                    self.smtp.username.clone(),
                                    self.smtp.password.clone(),
                                );

                                let mailer = SmtpTransport::relay(&self.smtp.service)
                                    .ok()?
                                    .credentials(credentials)
                                    .build();

                                match mailer.send(&message) {
                                    Ok(_) => {
                                        info!("email sent to {} successfully!", email.address);
                                        account.email_sent = now;
                                    }
                                    Err(err) => {
                                        error!("could not send email to {}: {err}", email.address);
                                    }
                                }
                            }
                            {
                                error!(
                                    "a password reset email was sent less than a day ago for {username}"
                                );
                            }
                        } else {
                            error!("the email address for account {username} is unverified");
                        }
                    } else {
                        error!("no email exists for account {username}");
                    }

                    None
                }
                "resume_game" => self.resume_game(username, index_supplied, command, &the_rest),
                "request_draw" => self.request_draw(username, index_supplied, command, &the_rest),
                "save" => {
                    debug!("saving users file...");
                    self.save_server();

                    None
                }
                "text" => {
                    let timestamp = timestamp();
                    let the_rest = the_rest.join(" ");
                    info!("{index_supplied} {timestamp} {username} text {the_rest}");

                    let text = format!("= text {timestamp} {username}: {the_rest}");
                    if self.texts.len() >= KEEP_TEXTS {
                        self.texts.pop_front();
                    }

                    for tx in &mut self.clients.values() {
                        let _ok = tx.send(text.clone());
                    }

                    self.texts.push_back(text);

                    None
                }
                "texts" => {
                    if !self.texts.is_empty() {
                        let string = Vec::from(self.texts.clone()).join("\n");

                        self.clients.get(&index_supplied)?.send(string).ok()?;
                    }

                    None
                }
                "text_game" => self.text_game(username, index_supplied, command, the_rest),
                "tournament_delete" => {
                    if self.admins.contains(*username) {
                        self.tournament = None;
                        self.tournament_status_all();
                    }

                    None
                }
                "tournament_groups_delete" => {
                    if self.admins.contains(*username)
                        && let Some(tournament) = &mut self.tournament
                    {
                        tournament.groups = None;
                        self.tournament_status_all();
                    }

                    None
                }
                "tournament_date" => {
                    if self.admins.contains(*username) {
                        if let Err(error) = self.tournament_date(&the_rest) {
                            error!("tournament_date: {error}");
                        } else {
                            self.tournament_status_all();
                        }
                    }

                    None
                }
                "tournament_status_0" => {
                    trace!("tournament_status: {:#?}", self.tournament);

                    if args.skip_advertising_updates {
                        None
                    } else {
                        let tx = self.clients.get(&index_supplied)?;
                        let tournament = ron::ser::to_string(&self.tournament).ok()?;

                        Some((
                            tx.clone(),
                            true,
                            format!("tournament_status_0 {tournament}"),
                        ))
                    }
                }
                "tournament_start" => {
                    let mut start_tournament = false;

                    if let Some(tournament) = &self.tournament
                        && tournament.groups.is_none()
                        && Utc::now() >= tournament.date
                    {
                        start_tournament = true;
                    }

                    if start_tournament {
                        info!("Starting tournament...");

                        self.generate_first_round();
                        self.tournament_status_all();
                    }

                    None
                }
                "watch_game" => self.watch_game(
                    username,
                    index_supplied,
                    (*command).to_string(),
                    the_rest.as_slice(),
                ),
                "=" => None,
                _ => self.clients.get(&index_supplied).map(|channel| {
                    error!("{index_supplied} {username} {command}");
                    (channel.clone(), false, (*command).to_string())
                }),
            }
        } else {
            error!("{index_username_command:?}");
            None
        }
    }

    fn join_game(
        &mut self,
        username: &str,
        index_supplied: usize,
        command: String,
        the_rest: &[&str],
    ) -> Option<(mpsc::Sender<String>, bool, String)> {
        let Some(id) = the_rest.first() else {
            return Some((self.clients.get(&index_supplied)?.clone(), false, command));
        };
        let Ok(id) = id.parse::<Id>() else {
            return Some((self.clients.get(&index_supplied)?.clone(), false, command));
        };

        info!("{index_supplied} {username} join_game {id}");
        let Some(game) = self.games_light.0.get_mut(&id) else {
            unreachable!();
        };

        game.challenge_accepted = true;

        let mut attacker_channel_id = 0;
        if let Some(account) = self.accounts.0.get(game.attacker.as_ref()?)
            && let Some(id) = account.logged_in
        {
            attacker_channel_id = id;
        }

        let mut defender_channel_id = 0;
        if let Some(account) = self.accounts.0.get(game.defender.as_ref()?)
            && let Some(id) = account.logged_in
        {
            defender_channel_id = id;
        }

        game.spectators
            .insert(game.attacker.clone()?, attacker_channel_id);
        game.spectators
            .insert(game.defender.clone()?, defender_channel_id);

        let (Some(attacker), Some(defender)) = (&game.attacker, &game.defender) else {
            unreachable!();
        };

        let (Some(attacker_account), Some(defender_account)) =
            (self.accounts.0.get(attacker), self.accounts.0.get(defender))
        else {
            unreachable!()
        };

        let mut attacker_channel = None;
        if let Some(channel_id) = attacker_account.logged_in
            && let Some(channel) = self.clients.get(&channel_id)
        {
            attacker_channel = Some(channel);
        }

        let mut defender_channel = None;
        if let Some(channel_id) = defender_account.logged_in
            && let Some(channel) = self.clients.get(&channel_id)
        {
            defender_channel = Some(channel);
        }

        for channel in [&attacker_channel, &defender_channel].into_iter().flatten() {
            channel
                .send(format!(
                    "= join_game {} {} {} {:?} {}",
                    game.attacker.clone()?,
                    game.defender.clone()?,
                    game.rated,
                    game.timed,
                    game.board_size,
                ))
                .ok()?;
        }

        let new_game = ServerGame::new(
            attacker_channel.cloned(),
            defender_channel.cloned(),
            game.clone(),
        );
        self.games.0.insert(id, new_game);

        if let Some(account) = self.accounts.0.get_mut(username) {
            account.pending_games.remove(&id);
        }

        if let Some(channel) = attacker_channel {
            channel
                .send(format!("game {id} generate_move attacker"))
                .ok()?;
        }

        None
    }

    fn join_game_pending(
        &mut self,
        username: String,
        index_supplied: usize,
        mut command: String,
        the_rest: &[&str],
    ) -> Option<(mpsc::Sender<String>, bool, String)> {
        let channel = self.clients.get(&index_supplied)?;

        let Some(id) = the_rest.first() else {
            return Some((channel.clone(), false, command));
        };
        let Ok(id) = id.parse::<Id>() else {
            return Some((channel.clone(), false, command));
        };

        info!("{index_supplied} {username} join_game_pending {id}");
        let Some(game) = self.games_light.0.get_mut(&id) else {
            command.push_str(" the id doesn't refer to a pending game");
            return Some((channel.clone(), false, command));
        };

        if game.attacker.is_none() {
            game.attacker = Some(username.clone());

            if let Some(defender) = &game.defender
                && let Some(account) = self.accounts.0.get(defender)
                && let Some(channel_id) = account.logged_in
                && let Some(channel) = self.clients.get(&channel_id)
            {
                let _ok = channel.send(format!("= challenge_requested {id}"));
            }
        } else if game.defender.is_none() {
            game.defender = Some(username.clone());

            if let Some(attacker) = &game.attacker
                && let Some(account) = self.accounts.0.get(attacker)
                && let Some(channel_id) = account.logged_in
                && let Some(channel) = self.clients.get(&channel_id)
            {
                let _ok = channel.send(format!("= challenge_requested {id}"));
            }
        }
        game.challenger.0 = Some(username);

        command.push(' ');
        command.push_str(the_rest.first()?);

        Some((channel.clone(), true, command))
    }

    fn leave_game(
        &mut self,
        username: &str,
        index_supplied: usize,
        mut command: String,
        the_rest: &[&str],
    ) -> Option<(mpsc::Sender<String>, bool, String)> {
        let Some(id) = the_rest.first() else {
            return Some((self.clients.get(&index_supplied)?.clone(), false, command));
        };
        let Ok(id) = id.parse::<Id>() else {
            return Some((self.clients.get(&index_supplied)?.clone(), false, command));
        };
        if let Some(account) = self.accounts.0.get_mut(username) {
            account.pending_games.remove(&id);
        }

        info!("{index_supplied} {username} leave_game {id}");

        let mut remove = false;
        match self.games_light.0.get_mut(&id) {
            Some(game) => {
                if !game.challenge_accepted {
                    if let Some(attacker) = &game.attacker
                        && username == attacker
                    {
                        game.attacker = None;
                    }

                    if let Some(defender) = &game.defender
                        && username == defender
                    {
                        game.defender = None;
                    }

                    if let Some(challenger) = &game.challenger.0
                        && username == challenger
                    {
                        game.challenger.0 = None;
                    }
                }

                game.spectators.remove(username);

                if game.attacker.is_none() && game.defender.is_none() {
                    remove = true;
                }
            }
            None => return Some((self.clients.get(&index_supplied)?.clone(), false, command)),
        }

        if remove {
            self.games_light.0.remove(&id);
        }

        command.push(' ');
        command.push_str(the_rest.first()?);
        Some((self.clients.get(&index_supplied)?.clone(), true, command))
    }

    fn login(
        &mut self,
        username: &str,
        index_supplied: usize,
        command: &str,
        the_rest: &[&str],
        option_tx: Option<Sender<String>>,
    ) -> Option<(mpsc::Sender<String>, bool, String)> {
        let password_1 = the_rest.join(" ");
        let tx = option_tx?;
        if let Some(account) = self.accounts.0.get_mut(username) {
            // The username is in the database and already logged in.
            if let Some(index_database) = account.logged_in {
                info!("{index_supplied} {username} login failed, {index_database} is logged in");

                Some(((tx), false, (*command).to_string()))
            // The username is in the database, but not logged in yet.
            } else {
                let hash_2 = PasswordHash::try_from(account.password.as_str()).ok()?;
                if let Err(_error) =
                    Argon2::default().verify_password(password_1.as_bytes(), &hash_2)
                {
                    info!("{index_supplied} {username} provided the wrong password");
                    return Some((tx, false, (*command).to_string()));
                }
                info!("{index_supplied} {username} logged in");

                self.clients.insert(index_supplied, tx);
                account.logged_in = Some(index_supplied);

                Some((
                    self.clients.get(&index_supplied)?.clone(),
                    true,
                    (*command).to_string(),
                ))
            }
        // The username is not in the database.
        } else {
            info!("{index_supplied} {username} is not in the database");
            Some((tx, false, (*command).to_string()))
        }
    }

    fn load_data_files(
        &mut self,
        tx: Sender<(String, Option<Sender<String>>)>,
        systemd: bool,
    ) -> anyhow::Result<()> {
        let users_file = data_file(USERS_FILE);
        match &fs::read_to_string(&users_file) {
            Ok(string) => match ron::from_str(string.as_str()) {
                Ok(server_ron) => {
                    *self = server_ron;
                    self.tx = Some(tx.clone());
                }
                Err(err) => {
                    return Err(anyhow::Error::msg(format!(
                        "RON: {}: {err}",
                        users_file.display(),
                    )));
                }
            },
            Err(err) => match err.kind() {
                ErrorKind::NotFound => {}
                _ => return Err(anyhow::Error::msg(err.to_string())),
            },
        }

        let archived_games_file = data_file(ARCHIVED_GAMES_FILE);
        match fs::read_to_string(&archived_games_file) {
            Ok(archived_games_string) => {
                let mut archived_games = Vec::new();

                for line in archived_games_string.lines() {
                    let archived_game: ArchivedGame = match ron::from_str(line) {
                        Ok(archived_game) => archived_game,
                        Err(err) => {
                            return Err(anyhow::Error::msg(format!(
                                "RON: {}: {err}",
                                archived_games_file.display(),
                            )));
                        }
                    };
                    archived_games.push(archived_game);
                }

                self.archived_games = archived_games;
            }
            Err(err) => {
                error!("archived games file not found: {err}");
            }
        }

        let active_games_file = data_file(ACTIVE_GAMES_FILE);
        if fs::exists(&active_games_file)? {
            let mut file = File::open(active_games_file)?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;

            let games: Vec<ServerGameSerialized> = postcard::from_bytes(data.as_slice())?;
            for game in games {
                let id = game.id;
                let server_game_light = ServerGameLight::from(&game);
                let server_game = ServerGame::from(game);

                self.games_light.0.insert(id, server_game_light);
                self.games.0.insert(id, server_game);
            }
        }

        ctrlc::set_handler(move || {
            if !systemd {
                println!();
            }
            handle_error(tx.send(("0 server save".to_string(), None)));
            handle_error(tx.send(("0 server exit".to_string(), None)));
        })?;

        Ok(())
    }

    fn logout(
        &mut self,
        username: &str,
        index_supplied: usize,
        command: &str,
    ) -> Option<(mpsc::Sender<String>, bool, String)> {
        // The username is in the database and already logged in.
        if let Some(account) = self.accounts.0.get_mut(username) {
            for id in &account.pending_games {
                if let Some(tx) = &self.tx
                    && let Some(game) = self.games_light.0.get(id)
                    && let TimeSettings::Timed(Time {
                        milliseconds_left, ..
                    }) = game.timed
                    && milliseconds_left < 1_000 * 60 * 60 * 24
                {
                    let _ok =
                        tx.send((format!("{index_supplied} {username} leave_game {id}"), None));
                }
            }

            if let Some(index_database) = account.logged_in
                && index_database == index_supplied
            {
                info!("{index_supplied} {username} logged out");
                account.logged_in = None;

                self.clients
                    .get(&index_supplied)?
                    .send("= logout".to_string())
                    .ok()?;

                self.clients.remove(&index_database);

                return None;
            }
        }

        self.clients
            .get(&index_supplied)
            .map(|sender| (sender.clone(), false, (*command).to_string()))
    }

    /// ```sh
    /// <- new_game attacker rated fischer 900000 10 13
    /// -> = new_game game 6 player-1 _ rated fischer 900000 10 _ false {}
    /// ```
    fn new_game(
        &mut self,
        username: &str,
        index_supplied: usize,
        command: &str,
        the_rest: &[&str],
    ) -> Option<(mpsc::Sender<String>, bool, String)> {
        if the_rest.len() < 6 {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        }

        let role = the_rest.first()?;
        let Ok(role) = Role::from_str(role) else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };

        let rated = the_rest.get(1)?;
        let Ok(rated) = Rated::from_str(rated) else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };

        let timed = the_rest.get(2)?;
        let minutes = the_rest.get(3)?;
        let add_seconds = the_rest.get(4)?;

        let Ok(timed) = TimeSettings::try_from(vec!["time-settings", timed, minutes, add_seconds])
        else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };

        let board_size = the_rest.get(5)?;
        let board_size = BoardSize::from_str(board_size).ok()?;

        info!(
            "{index_supplied} {username} new_game {} {role} {rated} {timed:?} {board_size}",
            self.game_id
        );

        let game = ServerGameLight::new(
            self.game_id,
            (*username).to_string(),
            rated,
            timed,
            board_size,
            role,
        );

        let command = format!("{command} {game:?}");
        self.games_light.0.insert(self.game_id, game);

        if let Some(account) = self.accounts.0.get_mut(username) {
            account.pending_games.insert(self.game_id);
        }

        self.game_id += 1;

        Some((self.clients.get(&index_supplied)?.clone(), true, command))
    }

    fn new_tournament(tx: Sender<(String, Option<Sender<String>>)>) {
        thread::spawn(move || {
            handle_error(tx.send(("0 server tournament_start".to_string(), None)));

            loop {
                let now_utc = Utc::now();

                let tomorrow_midnight_utc = (now_utc + Days::new(1))
                    .date_naive()
                    .and_hms_opt(0, 0, 0)
                    .unwrap_or_else(|| {
                        error!("and_hms_opt failed");
                        exit(1)
                    })
                    .and_local_timezone(Utc)
                    .single()
                    .unwrap_or_else(|| {
                        error!("single failed");
                        exit(1)
                    });

                let duration_until_midnight = tomorrow_midnight_utc.signed_duration_since(now_utc);
                debug!(
                    "seconds until midnight UTC: {}",
                    duration_until_midnight.num_seconds()
                );

                let std_duration = duration_until_midnight.to_std().unwrap_or_else(|error| {
                    error!("to_std failed: {error}");
                    exit(1)
                });

                sleep(std_duration);
                sleep(Duration::from_secs(1));

                handle_error(tx.send(("0 server tournament_start".to_string(), None)));
            }
        });
    }

    #[must_use]
    fn new_tournament_game(&mut self, attacker: &str, defender: &str) -> Id {
        let id = self.game_id;

        self.game_id += 1;

        let game_light = ServerGameLight {
            id,
            attacker: Some(attacker.to_string()),
            defender: Some(defender.to_string()),
            challenger: Challenger(None),
            rated: Rated::Yes,
            timed: TimeEnum::Long.into(),
            spectators: HashMap::new(),
            challenge_accepted: true,
            game_over: false,
            board_size: BoardSize::_11,
        };

        info!(
            "0 server new_tournament_game {id} {} {:?} {}",
            game_light.rated, game_light.timed, game_light.board_size
        );

        let game = ServerGame::new(None, None, game_light.clone());

        self.games_light.0.insert(id, game_light);
        self.games.0.insert(id, game);

        id
    }

    fn resume_game(
        &mut self,
        username: &str,
        index_supplied: usize,
        command: &str,
        the_rest: &[&str],
    ) -> Option<(mpsc::Sender<String>, bool, String)> {
        let Some(id) = the_rest.first() else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };
        let Ok(id) = id.parse::<Id>() else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };

        let Some(server_game) = self.games.0.get(&id) else {
            unreachable!()
        };

        let game = &server_game.game;
        let Ok(board) = ron::ser::to_string(game) else {
            unreachable!()
        };
        let texts = &server_game.texts;
        let Ok(texts) = ron::ser::to_string(&texts) else {
            unreachable!()
        };

        info!("{index_supplied} {username} {command} {id}");

        let Some(game_light) = self.games_light.0.get_mut(&id) else {
            unreachable!();
        };

        let mut channel_id = 0;
        if let Some(account) = self.accounts.0.get(username)
            && let Some(id) = account.logged_in
        {
            channel_id = id;
        }
        game_light
            .spectators
            .insert(username.to_string(), channel_id);

        let mut request_draw = Role::Roleless;
        if let Some(server_game) = self.games.0.get_mut(&id) {
            if Some(username) == game_light.attacker.as_deref() {
                server_game.attacker_tx =
                    Messenger::new(self.clients.get(&index_supplied)?.clone());

                if server_game.draw_requested == Role::Defender {
                    request_draw = Role::Attacker;
                }
            } else if Some(username) == game_light.defender.as_deref() {
                server_game.defender_tx =
                    Messenger::new(self.clients.get(&index_supplied)?.clone());

                if server_game.draw_requested == Role::Attacker {
                    request_draw = Role::Defender;
                }
            }
        }

        let client = self.clients.get(&index_supplied)?;

        client
            .send(format!(
                "= resume_game {} {} {} {:?} {} {board} {texts}",
                game_light.attacker.clone()?,
                game_light.defender.clone()?,
                game_light.rated,
                game_light.timed,
                game_light.board_size,
            ))
            .ok()?;

        if request_draw != Role::Roleless {
            client
                .send(format!("request_draw {} {request_draw}", game_light.id))
                .ok()?;
        }

        None
    }

    fn request_draw(
        &mut self,
        username: &str,
        index_supplied: usize,
        command: &str,
        the_rest: &[&str],
    ) -> Option<(mpsc::Sender<String>, bool, String)> {
        let Some(id) = the_rest.first() else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };
        let Ok(id) = id.parse::<Id>() else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };

        let Some(role) = the_rest.get(1) else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };
        let Ok(role) = Role::from_str(role) else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };

        info!("{index_supplied} {username} request_draw {id} {role}");

        if let Some(server_game) = self.games.0.get_mut(&id) {
            server_game.draw_requested = role;
        }

        let message = format!("request_draw {id} {role}");
        if let Some(game) = self.games.0.get(&id) {
            match role {
                Role::Attacker => {
                    game.defender_tx.send(message);
                }
                Role::Roleless => {}
                Role::Defender => {
                    game.attacker_tx.send(message);
                }
            }
        }

        Some((
            self.clients.get(&index_supplied)?.clone(),
            true,
            (*command).to_string(),
        ))
    }

    fn save(tx: Sender<(String, Option<Sender<String>>)>) {
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(HOUR_IN_SECONDS));
                handle_error(tx.send(("0 server save".to_string(), None)));
            }
        });
    }

    fn save_server(&self) {
        if !self.skip_the_data_files {
            let mut server = self.clone();

            for account in server.accounts.0.values_mut() {
                account.logged_in = None;
            }

            match ron::ser::to_string_pretty(&server, ron::ser::PrettyConfig::default()) {
                Ok(string) => {
                    if !string.trim().is_empty() {
                        let users_file = data_file(USERS_FILE);

                        match File::create(&users_file) {
                            Ok(mut file) => {
                                if let Err(error) = file.write_all(string.as_bytes()) {
                                    error!("save file (3): {error}");
                                }
                            }
                            Err(error) => error!("save file (2): {error}"),
                        }
                    }
                }
                Err(error) => error!("save file (1): {error}"),
            }
        }
    }

    fn text_game(
        &mut self,
        username: &str,
        index_supplied: usize,
        command: &str,
        mut the_rest: Vec<&str>,
    ) -> Option<(mpsc::Sender<String>, bool, String)> {
        let Some(id) = the_rest.first() else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };
        let Ok(id) = id.parse::<Id>() else {
            return Some((
                self.clients.get(&index_supplied)?.clone(),
                false,
                (*command).to_string(),
            ));
        };

        let timestamp = timestamp();
        let text = the_rest.split_off(1);
        let mut text = text.join(" ");
        text = format!("{timestamp} {username}: {text}");
        info!("{index_supplied} {username} text_game {id} {text}");

        if let Some(game) = self.games.0.get_mut(&id) {
            game.texts.push_front(text.clone());
        }

        text = format!("= text_game {text}");

        if let Some(game) = self.games_light.0.get(&id) {
            for index in game.spectators.values() {
                if let Some(sender) = self.clients.get(index) {
                    let _ok = sender.send(text.clone());
                }
            }
        }

        None
    }

    fn tournament_date(&mut self, the_rest: &[&str]) -> anyhow::Result<()> {
        let mut tournament = Tournament::default();

        let Some(date) = the_rest.first() else {
            return Err(anyhow::Error::msg("tournament_date: date is empty"));
        };

        let datetime = match DateTime::parse_from_str(
            &format!("{date} 00:00:00 +0000"),
            "%Y-%m-%d %H:%M:%S %z",
        ) {
            Ok(datetime) => datetime,
            Err(error) => return Err(anyhow::Error::msg(format!("tournament_date: {error}"))),
        };

        tournament.date = datetime.to_utc();
        self.tournament = Some(tournament);

        Ok(())
    }

    fn tournament_status_all(&self) {
        trace!("tournament_status: {:#?}", self.tournament);

        if let Ok(mut tournament) = ron::ser::to_string(&self.tournament) {
            tournament = format!("= tournament_status_0 {tournament}");

            for tx in self.clients.values() {
                let _ok = tx.send(tournament.clone());
            }
        }
    }

    fn watch_game(
        &mut self,
        username: &str,
        index_supplied: usize,
        command: String,
        the_rest: &[&str],
    ) -> Option<(mpsc::Sender<String>, bool, String)> {
        let Some(id) = the_rest.first() else {
            return Some((self.clients.get(&index_supplied)?.clone(), false, command));
        };
        let Ok(id) = id.parse::<Id>() else {
            return Some((self.clients.get(&index_supplied)?.clone(), false, command));
        };

        if let Some(game) = self.games_light.0.get_mut(&id) {
            game.spectators.insert(username.to_string(), index_supplied);
        }

        let Some(server_game) = self.games.0.get(&id) else {
            unreachable!()
        };

        let game = &server_game.game;
        let Ok(board) = ron::ser::to_string(game) else {
            unreachable!()
        };
        let texts = &server_game.texts;
        let Ok(texts) = ron::ser::to_string(&texts) else {
            unreachable!()
        };

        info!("{index_supplied} {username} watch_game {id}");
        let Some(game) = self.games_light.0.get_mut(&id) else {
            unreachable!()
        };

        self.clients
            .get(&index_supplied)?
            .send(format!(
                "= watch_game {} {} {} {:?} {} {board} {texts}",
                game.attacker.clone()?,
                game.defender.clone()?,
                game.rated,
                game.timed,
                game.board_size,
            ))
            .ok()?;

        None
    }
}
