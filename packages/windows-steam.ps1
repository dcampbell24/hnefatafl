sed -i 's/cargo-/windows-steam-/' src\bin\hnefatafl-client\main.rs;
cargo build --release --bin hnefatafl-client --features client --no-default-features
scp ..\..\..\target\release\hnefatafl-client.exe david@192.168.1.141:~
