#! /bin/bash -e

export RUST_MIN_STACK=67108864

cargo build --release

./target/release/hnefatafl-ai --man --username ""
./target/release/hnefatafl-client --man
./target/release/hnefatafl-server --man
./target/release/hnefatafl-server-full --man
./target/release/hnefatafl-text-protocol --man

gzip --no-name --best hnefatafl-ai.1
gzip --no-name --best hnefatafl-server.1
gzip --no-name --best hnefatafl-text-protocol.1
gzip --no-name --best hnefatafl-client.1
gzip --no-name --best hnefatafl-server-full.1

PACKAGE=$(cargo deb)

rm hnefatafl-ai.1.gz
rm hnefatafl-server.1.gz
rm hnefatafl-text-protocol.1.gz
rm hnefatafl-client.1.gz
rm hnefatafl-server-full.1.gz

echo $PACKAGE
lintian -EviIL +pedantic $PACKAGE
