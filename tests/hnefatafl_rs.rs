use std::{fmt, io::Cursor, str::FromStr};

use rustc_hash::FxHashSet;

use hnefatafl_copenhagen::{
    game::{self, Game},
    play::{Plae, Play, Plays, Vertex},
    role::Role,
    status::Status,
    time,
};

/// # Errors
///
/// If the game records are invalid.
pub fn setup_hnefatafl_rs() -> anyhow::Result<Vec<(usize, GameRecord)>> {
    let copenhagen_csv = include_str!("copenhagen.csv");
    game_records_from_path(copenhagen_csv)
}

/// # Errors
///
/// If the captures or game status don't match for an engine game and a record
/// game.
#[allow(clippy::cast_precision_loss, clippy::missing_panics_doc)]
pub fn hnefatafl_rs(records: &[(usize, GameRecord)]) {
    let mut already_played = 0;
    let mut already_over = 0;

    records
        .iter()
        .map(|(i, record)| play_game(*i, record))
        .for_each(|result| match result {
            Ok((i, game)) => {
                if game.status != Status::Ongoing {
                    assert_eq!(game.status, records[i].1.status);
                }
            }
            Err(error) => {
                if &error.to_string() == "play: you already reached that position" {
                    already_played += 1;
                } else if &error.to_string() == "play: the game is already over" {
                    already_over += 1;
                } else {
                    panic!("{}", error.to_string());
                }
            }
        });

    assert_eq!(already_over, 0);
    assert_eq!(already_played, 36);

    let already_played_error = f64::from(already_played) / records.len() as f64;
    assert!(already_played_error > 0.020_5 && already_played_error < 0.020_6);
}

#[inline]
fn play_game(i: usize, record: &GameRecord) -> Result<(usize, Game), anyhow::Error> {
    let mut game = Game {
        plays: Plays::new(&time::TimeSettings::UnTimed),
        time: game::TimeUnix::UnTimed,
        attacker_time: time::TimeSettings::UnTimed,
        defender_time: time::TimeSettings::UnTimed,
        ..Game::default()
    };

    for (play, captures_1) in record.clone().plays {
        let mut captures_2 = FxHashSet::default();
        let play = Plae::Play(play);
        let captures = game.play(&play)?;

        for vertex in captures.0 {
            captures_2.insert(vertex);
        }

        if let Some(king) = game.board.find_the_king() {
            captures_2.remove(&king);
        }

        let captures_2 = Captures(captures_2);

        if !game.board.captured_the_king() {
            if let Some(captures_1) = captures_1 {
                assert_eq!(captures_1, captures_2);
            } else if !captures_2.0.is_empty() {
                panic!("The engine reports captures, but the record says there are none.");
            }
        }
    }

    Ok((i, game))
}

#[derive(Debug, serde::Deserialize)]
struct Record {
    moves: String,
    _attacker_captures: u64,
    _defender_captures: u64,
    status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Captures(pub FxHashSet<Vertex>);

impl fmt::Display for Captures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for vertex in &self.0 {
            write!(f, "{vertex} ")?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct GameRecord {
    pub plays: Vec<(Play, Option<Captures>)>,
    pub status: Status,
}

/// # Errors
///
/// If the game records are invalid.
pub fn game_records_from_path(string: &str) -> anyhow::Result<Vec<(usize, GameRecord)>> {
    let cursor = Cursor::new(string);
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(cursor);

    let mut game_records = Vec::with_capacity(1_800);
    for (i, result) in rdr.deserialize().enumerate() {
        let record: Record = result?;
        let mut role = Role::Defender;
        let mut plays = Vec::new();

        for play in record.moves.split_ascii_whitespace() {
            role = role.opposite();

            if play.contains('-') {
                let vertexes: Vec<_> = play.split('-').collect();
                let vertex_1_captures: Vec<_> = vertexes[1].split('x').collect();

                if let (Ok(from), Ok(to)) = (
                    Vertex::from_str(vertexes[0]),
                    Vertex::from_str(vertex_1_captures[0]),
                ) {
                    let play = Play { role, from, to };

                    if vertex_1_captures.get(1).is_some() {
                        let mut captures = FxHashSet::default();
                        for capture in vertex_1_captures.into_iter().skip(1) {
                            let vertex = Vertex::from_str(capture)?;
                            if !captures.contains(&vertex) {
                                captures.insert(vertex);
                            }
                        }

                        plays.push((play, Some(Captures(captures))));
                    } else {
                        plays.push((play, None));
                    }
                }
            }
        }

        let game_record = GameRecord {
            plays,
            status: Status::from_str(record.status.as_str())?,
        };

        game_records.push((i, game_record));
    }

    Ok(game_records)
}

#[test]
fn hnefatafl_games() -> anyhow::Result<()> {
    hnefatafl_rs(&setup_hnefatafl_rs()?);

    Ok(())
}
