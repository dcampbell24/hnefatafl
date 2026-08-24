#! /bin/bash -e

export RUST_MIN_STACK=67108864

sed -i 's/cargo-/linux-steam-/' src/lib.rs;
cargo build --release --bin hnefatafl-client --features client --no-default-features
sed -i 's/linux-steam-/cargo-/' src/lib.rs;
