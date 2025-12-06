#[cfg(feature = "bench")]
use std::time::Duration;

#[cfg(feature = "bench")]
use criterion::{Criterion, criterion_group, criterion_main};

#[cfg(feature = "bench")]
use hnefatafl_copenhagen::{hnefatafl_rs, setup_hnefatafl_rs};

#[cfg(feature = "bench")]
fn game_play_outs(c: &mut Criterion) {
    let game_records = setup_hnefatafl_rs().unwrap();
    c.bench_function("game_play_outs", move |b| {
        b.iter(|| hnefatafl_rs(&game_records));
    });
}

#[cfg(feature = "bench")]
criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = game_play_outs
}

#[cfg(feature = "bench")]
criterion_main!(benches);

#[cfg(not(feature = "bench"))]
fn main() {
    eprintln!("You must enable pass `--features=bench`");
}
