#! /bin/bash -e

export RUST_MIN_STACK=67108864

cargo run --release --bin hnefatafl-ai -- --man --username ""
cargo run --release --bin hnefatafl-text-protocol -- --man
cargo run --release --example hnefatafl-client -- --man
cargo run --release -- --man

gzip --no-name --best hnefatafl-ai.1
gzip --no-name --best hnefatafl-text-protocol.1
gzip --no-name --best hnefatafl-client.1
gzip --no-name --best hnefatafl-server-full.1

PACKAGE=$(cargo deb)

rm hnefatafl-ai.1.gz
rm hnefatafl-text-protocol.1.gz
rm hnefatafl-client.1.gz
rm hnefatafl-server-full.1.gz

echo $PACKAGE
lintian $PACKAGE

if [ -z $1 ]; then
    exit
fi

if [ $1 = 'install' ]; then
    sudo dpkg --remove hnefatafl-copenhagen
    sudo dpkg --install $PACKAGE
    sudo systemctl restart hnefatafl.service
    sudo systemctl daemon-reload
fi
