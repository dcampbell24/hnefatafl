sed -i 's/cargo-/windows-steam-/' src\bin\hnefatafl-client\main.rs;
cargo build --release --bin hnefatafl-client --features client --no-default-features
