# Install on Android

```sh
# fish
pkg upgrade
termux-change-repo
pkg install rust git x11-repo
pkg install xfce termux-x11-nightly
git clone https://github.com/termux/termux-packages termux-packages-hnefatafl-copenhagen-dest
git clone -b hnefatafl-copenhagen https://github.com/robertkirkman/termux-packages termux-packages-hnefatafl-copenhagen-src
cp -r termux-packages-hnefatafl-copenhagen-src/x11-packages/hnefatafl-copenhagen/ termux-packages-hnefatafl-copenhagen-dest/x11-packages/
cd termux-packages-hnefatafl-copenhagen-dest
scripts/setup-termux.sh
./build-package.sh -I -f hnefatafl-copenhagen
cd output/
apt reinstall ./hnefatafl-copenhagen*.deb
export LIBGL_ALWAYS_SOFTWARE=1 DISPLAY=:0
termux-x11 -xstartup xfce4-session
hnefatafl-client
```
