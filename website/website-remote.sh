#! /bin/bash -ex

./packages/debian/deb.sh

cd packages/debian/
./generate-release.sh

scp -r ./apt/ root@hnefatafl.org:/var/www/html/
