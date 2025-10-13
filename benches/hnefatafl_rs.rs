use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};

use hnefatafl_copenhagen::{hnefatafl_rs, setup_hnefatafl_rs};

fn game_play_outs(c: &mut Criterion) {
    let game_records = setup_hnefatafl_rs().unwrap();
    c.bench_function("game_play_outs", move |b| {
        b.iter(|| hnefatafl_rs(&game_records));
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = game_play_outs
}

criterion_main!(benches);
