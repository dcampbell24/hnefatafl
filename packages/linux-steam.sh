#! /bin/bash -e

export RUST_MIN_STACK=67108864

sed -i 's/cargo-/linux-steam-/' src/bin/hnefatafl-client/main.rs;
cargo build --release --bin hnefatafl-client --features client --no-default-features
