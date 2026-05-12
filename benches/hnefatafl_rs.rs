// SPDX-FileCopyrightText: 2025 David Campbell <david@hnefatafl.or>
// SPDX-License-Identifier: MIT

use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};

#[path = "../tests/hnefatafl_rs.rs"]
mod test;

use test::{aagenielsen_dk_game_records, play_games};

fn game_play_outs(c: &mut Criterion) {
    let game_records = aagenielsen_dk_game_records().unwrap();

    c.bench_function("game_play_outs", move |b| {
        b.iter(|| play_games(&game_records));
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = game_play_outs
}

criterion_main!(benches);
