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
mod server;
mod smtp;
mod unix_timestamp;

use std::{
    collections::VecDeque,
    fmt,
    fs::{self, File},
    io::{BufRead, BufReader, ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    process::exit,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use argon2::{Argon2, PasswordHasher};
use chrono::Utc;
use clap::{CommandFactory, Parser};
use hnefatafl_copenhagen::{
    COPYRIGHT, Id, SERVER_PORT, VERSION_ID,
    server_game::{ArchivedGame, ServerGame, ServerGameLight, ServerGameSerialized},
    tournament::{self, Player},
    utils::{self, data_file},
};
use log::{debug, error, info};
use old_rand::rngs::OsRng;
use password_hash::SaltString;
use std::fmt::Write as _;

use crate::{command_line::Args, remove_connection::RemoveConnection, server::Server};

const ACTIVE_GAMES_FILE: &str = "hnefatafl-games-active.postcard";
const ARCHIVED_GAMES_FILE: &str = "hnefatafl-games.ron";
/// Seconds in two months: `60.0 * 60.0 * 24.0 * 30.417 * 2.0 = 5_256_057.6`
const TWO_MONTHS: i64 = 5_256_058;
const SEVEN_DAYS: i64 = 1_000 * 60 * 60 * 24 * 7;
const USERS_FILE: &str = "hnefatafl-copenhagen.ron";
const MESSAGE_FILE: &str = "hnefatafl-message.txt";

#[allow(clippy::too_many_lines)]
fn main() -> anyhow::Result<()> {
    // println!("{:x}", rand::random::<u32>());
    // return Ok(());

    let args = Args::parse();
    utils::init_logger("hnefatafl_server_full", args.debug, args.systemd);

    if args.man {
        let mut buffer: Vec<u8> = Vec::default();
        let cmd = Args::command()
            .name("hnefatafl-server-full")
            .long_version(None);
        let man = clap_mangen::Man::new(cmd).date("2025-06-23");

        man.render(&mut buffer)?;
        write!(buffer, "{COPYRIGHT}")?;

        std::fs::write("hnefatafl-server-full.1", buffer)?;
        return Ok(());
    }

    let (tx, rx) = mpsc::channel();
    let mut server = Server {
        tx: Some(tx.clone()),
        ..Server::default()
    };

    if !args.skip_the_data_file {
        let users_file = data_file(USERS_FILE);
        match &fs::read_to_string(&users_file) {
            Ok(string) => match ron::from_str(string.as_str()) {
                Ok(server_ron) => {
                    server = server_ron;
                    server.tx = Some(tx.clone());
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

                server.archived_games = archived_games;
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

                server.games_light.0.insert(id, server_game_light);
                server.games.0.insert(id, server_game);
            }
        }

        let tx_signals = tx.clone();
        ctrlc::set_handler(move || {
            if !args.systemd {
                println!();
            }
            handle_error(tx_signals.send(("0 server exit".to_string(), None)));
        })?;
    }

    if args.skip_the_data_file {
        server.skip_the_data_file = true;
    }

    thread::spawn(move || handle_error(server.handle_messages(&rx)));

    if !args.skip_advertising_updates {
        let tx_messages_1 = tx.clone();
        thread::spawn(move || {
            loop {
                handle_error(tx_messages_1.send(("0 server display_server".to_string(), None)));
                thread::sleep(Duration::from_secs(1));
            }
        });
    }

    let tx_messages_2 = tx.clone();
    thread::spawn(move || {
        loop {
            handle_error(tx_messages_2.send(("0 server check_update_rd".to_string(), None)));
            thread::sleep(Duration::from_secs(60 * 60 * 24));
        }
    });

    let mut address = "0.0.0.0".to_string();
    address.push_str(SERVER_PORT);

    let listener = TcpListener::bind(&address)?;
    info!("listening on {address} ...");

    for (index, stream) in (1..).zip(listener.incoming()) {
        let stream = stream?;

        if args.secure {
            let peer_address = stream.peer_addr()?.ip();

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
                        error!("{error}");
                        continue;
                    }
                },
                Err(error) => {
                    error!("{error}");
                    continue;
                }
            }
        }

        let tx = tx.clone();
        let reader = BufReader::new(stream.try_clone()?);

        thread::spawn(move || log_error(login(index, stream, reader, &tx)));
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn login(
    id: Id,
    mut stream: TcpStream,
    mut reader: BufReader<TcpStream>,
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
                "the user tried to login with whitespace alone",
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
    thread::spawn(move || log_error(receiving_and_writing(stream, &client_rx)));

    tx.send((format!("{id} {username_proper} email_get"), None))?;
    tx.send((format!("{id} {username_proper} texts"), None))?;
    tx.send((format!("{id} {username_proper} message"), None))?;
    tx.send((format!("{id} {username_proper} display_games"), None))?;
    tx.send((format!("{id} {username_proper} tournament_status"), None))?;
    tx.send((format!("{id} {username_proper} admin"), None))?;

    'outer: for _ in 0..1_000_000 {
        if let Err(err) = reader.read_line(&mut buf) {
            error!("{err}");
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

        tx.send((format!("{id} {username_proper} {buf_str}"), None))?;
        buf.clear();
    }

    tx.send((format!("{id} {username_proper} logout"), None))?;
    Ok(())
}

fn receiving_and_writing<T: Send + Write>(
    mut stream: T,
    client_rx: &Receiver<String>,
) -> anyhow::Result<()> {
    loop {
        match client_rx.recv() {
            Ok(mut message) => {
                if message == "= archived_games" {
                    let ron_archived_games = client_rx.recv()?;
                    let archived_games: Vec<ArchivedGame> = ron::from_str(&ron_archived_games)?;
                    let postcard_archived_games = &postcard::to_allocvec(&archived_games)?;

                    writeln!(message, " {}", postcard_archived_games.len())?;
                    stream.write_all(message.as_bytes())?;
                    stream.write_all(postcard_archived_games)?;
                } else {
                    message.push('\n');
                    stream.write_all(message.as_bytes())?;
                }
            }
            Err(_) => {
                // The channel must be closed.
                return Ok(());
            }
        }
    }
}

fn generate_round_one(players: Vec<Player>) -> Vec<tournament::Status> {
    let players_len = players.len();

    if players_len == 1
        && let Some(player) = players.first()
    {
        return vec![tournament::Status::Won(player.clone())];
    }

    let mut power = 1;
    while power < players_len {
        power *= 2;
    }

    let mut tournament_players = VecDeque::new();
    for player in players {
        tournament_players.push_front(tournament::Status::Ready(player));
    }
    for _ in 0..(power - players_len) {
        tournament_players.push_back(tournament::Status::None);
    }

    let mut round = Vec::new();
    for i in 0..tournament_players.len() {
        if i % 2 == 0 {
            let Some(player) = tournament_players.pop_back() else {
                unreachable!()
            };

            round.push(player);
        } else {
            let Some(player) = tournament_players.pop_front() else {
                unreachable!()
            };

            round.push(player);
        }
    }

    let mut round_new = Vec::new();
    for statuses in round.chunks(2) {
        let (Some(status_1), Some(status_2)) = (statuses.first(), statuses.get(1)) else {
            return round_new;
        };

        let status_1 = status_1.clone();
        let status_2 = status_2.clone();

        let (status_1, status_2) = match (status_1, status_2) {
            (tournament::Status::Ready(player), tournament::Status::None) => (
                tournament::Status::Won(player.clone()),
                tournament::Status::None,
            ),
            (tournament::Status::None, tournament::Status::Ready(player)) => (
                tournament::Status::None,
                tournament::Status::Won(player.clone()),
            ),
            (status_1, status_2) => (status_1, status_2),
        };

        round_new.push(status_1);
        round_new.push(status_2);
    }

    round = round_new;
    round
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

fn log_error<T, E: fmt::Display>(result: Result<T, E>) {
    if let Err(error) = result {
        error!("{error}");
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

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use argon2::{PasswordHash, PasswordVerifier};

    use crate::accounts::{Account, Accounts};

    use super::*;

    use std::net::TcpStream;
    use std::process::{Child, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    const ADDRESS: &str = "localhost:49152";

    struct Server(Child);

    impl Server {
        fn new(release: bool) -> anyhow::Result<Server> {
            let server = if release {
                std::process::Command::new("./target/release/hnefatafl-server-full")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .arg("--skip-the-data-file")
                    .arg("--skip-advertising-updates")
                    .arg("--skip-message")
                    .spawn()?
            } else {
                std::process::Command::new("./target/debug/hnefatafl-server-full")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .arg("--skip-the-data-file")
                    .arg("--skip-advertising-updates")
                    .arg("--skip-message")
                    .spawn()?
            };

            Ok(Server(server))
        }
    }

    impl Drop for Server {
        fn drop(&mut self) {
            self.0.kill().unwrap();
        }
    }

    #[test]
    fn capital_letters_fail() {
        let mut accounts = Accounts::default();

        let password = "A".to_string();
        let ctx = Argon2::default();

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = ctx
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        let account = Account {
            password: password_hash,
            logged_in: Some(0),
            ..Default::default()
        };

        accounts.0.insert("testing".to_string(), account);
        {
            let account = accounts.0.get_mut("testing").unwrap();

            let salt = SaltString::generate(&mut OsRng);
            let password_hash = ctx
                .hash_password(password.as_bytes(), &salt)
                .unwrap()
                .to_string();

            account.password = password_hash;
        }

        {
            let account = accounts.0.get_mut("testing").unwrap();
            let hash = PasswordHash::try_from(account.password.as_str()).unwrap();

            assert!(
                Argon2::default()
                    .verify_password(password.as_bytes(), &hash)
                    .is_ok()
            );
        }
    }

    #[test]
    fn server_full() -> anyhow::Result<()> {
        std::process::Command::new("cargo")
            .arg("build")
            .arg("--bin")
            .arg("hnefatafl-server-full")
            .output()?;

        let _server = Server::new(false);
        thread::sleep(Duration::from_millis(10));

        let mut buf = String::new();
        let mut socket_1 = TcpStream::connect(ADDRESS)?;
        let mut reader_1 = BufReader::new(socket_1.try_clone()?);

        socket_1.write_all(format!("{VERSION_ID} create_account player-1\n").as_bytes())?;
        reader_1.read_line(&mut buf)?;
        assert_eq!(buf, "= login\n");
        buf.clear();

        socket_1.write_all(b"change_password\n")?;
        reader_1.read_line(&mut buf)?;
        assert_eq!(buf, "= change_password\n");
        buf.clear();

        socket_1.write_all(b"new_game attacker rated fischer 900000 10 11\n")?;
        reader_1.read_line(&mut buf)?;
        assert_eq!(
            buf,
            "= new_game game 0 player-1 _ rated fischer 900000 10 11 _ false {}\n"
        );
        buf.clear();

        let mut socket_2 = TcpStream::connect(ADDRESS)?;
        let mut reader_2 = BufReader::new(socket_2.try_clone()?);

        socket_2.write_all(format!("{VERSION_ID} create_account player-2\n").as_bytes())?;
        reader_2.read_line(&mut buf)?;
        assert_eq!(buf, "= login\n");
        buf.clear();

        socket_2.write_all(b"join_game_pending 0\n")?;
        reader_2.read_line(&mut buf)?;
        assert_eq!(buf, "= join_game_pending 0\n");
        buf.clear();

        reader_1.read_line(&mut buf)?;
        assert_eq!(buf, "= challenge_requested 0\n");
        buf.clear();

        // Fixme: "join_game_pending 0\n" should not be allowed!
        socket_1.write_all(b"join_game 0\n")?;
        reader_1.read_line(&mut buf)?;
        assert_eq!(
            buf,
            "= join_game player-1 player-2 rated fischer 900000 10 11\n"
        );
        buf.clear();

        reader_2.read_line(&mut buf)?;
        assert_eq!(
            buf,
            "= join_game player-1 player-2 rated fischer 900000 10 11\n"
        );
        buf.clear();

        reader_1.read_line(&mut buf)?;
        assert_eq!(buf, "game 0 generate_move attacker\n");
        buf.clear();

        socket_1.write_all(b"game 0 play attacker resigns _\n")?;
        reader_1.read_line(&mut buf)?;
        assert_eq!(buf, "= game_over 0 defender_wins\n");
        buf.clear();

        reader_2.read_line(&mut buf)?;
        assert_eq!(buf, "game 0 play attacker resigns \n");
        buf.clear();

        reader_2.read_line(&mut buf)?;
        assert_eq!(buf, "= game_over 0 defender_wins\n");
        buf.clear();

        Ok(())
    }

    // echo "* soft nofile 1000000" >> /etc/security/limits.conf
    // echo "* hard nofile 1000000" >> /etc/security/limits.conf
    // fish
    // ulimit --file-descriptor-count 1000000
    #[ignore = "too slow, too many tcp connections"]
    #[test]
    fn many_clients() -> anyhow::Result<()> {
        std::process::Command::new("cargo")
            .arg("build")
            .arg("--release")
            .arg("--bin")
            .arg("hnefatafl-server-full")
            .output()?;

        let _server = Server::new(true);
        thread::sleep(Duration::from_millis(10));

        let t0 = Instant::now();

        let mut handles = Vec::new();
        for i in 0..1_000 {
            handles.push(thread::spawn(move || {
                let mut buf = String::new();
                let mut tcp = TcpStream::connect(ADDRESS).unwrap();
                let mut reader = BufReader::new(tcp.try_clone().unwrap());

                tcp.write_all(format!("{VERSION_ID} create_account q-player-{i}\n").as_bytes())
                    .unwrap();

                reader.read_line(&mut buf).unwrap();
                buf.clear();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let t1 = Instant::now();
        println!("many clients: {:?}", t1 - t0);

        Ok(())
    }
}
