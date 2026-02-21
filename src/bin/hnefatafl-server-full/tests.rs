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

#![allow(clippy::unwrap_used)]
#![cfg(test)]

use argon2::{PasswordHash, PasswordVerifier};

use hnefatafl_copenhagen::accounts::{Account, Accounts};

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
