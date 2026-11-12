#! /bin/bash -e

export RUST_MIN_STACK=67108864

sed -i 's/cargo-/debian-/' src/lib.rs;

cargo build

./target/debug/taflzero --man --username ""
./target/debug/hnefatafl-client --man
./target/debug/hnefatafl-server --man
./target/debug/hnefatafl-text-protocol --man

gzip --no-name --best taflzero.1
gzip --no-name --best hnefatafl-server.1
gzip --no-name --best hnefatafl-text-protocol.1
gzip --no-name --best hnefatafl-client.1

PACKAGE=$(cargo deb)

sed -i 's/debian-/cargo-/' src/lib.rs;

rm taflzero.1.gz
rm hnefatafl-server.1.gz
rm hnefatafl-text-protocol.1.gz
rm hnefatafl-client.1.gz

echo $PACKAGE
# lintian -EviIL +pedantic $PACKAGE
