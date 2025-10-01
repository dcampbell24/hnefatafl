use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};

use hnefatafl_copenhagen::{board::BoardSize, game_tree::Tree, hnefatafl_rs, setup_hnefatafl_rs};

fn game_play_outs(c: &mut Criterion) {
    let game_records = setup_hnefatafl_rs().unwrap();
    c.bench_function("game_play_outs", move |b| {
        b.iter(|| hnefatafl_rs(&game_records));
    });
}

fn monte_carlo(c: &mut Criterion) {
    c.bench_function("monte_carlo", move |b| {
        b.iter(|| {
            let mut tree = Tree::new(BoardSize::_11);
            let _plays = tree.monte_carlo_tree_search(10);
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(15));
    targets = game_play_outs, monte_carlo
}

criterion_main!(benches);
