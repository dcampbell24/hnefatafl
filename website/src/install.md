![Login](screenshots/runic_login.png)

<div id="google_translate_element"></div>
<script type="text/javascript" src="//translate.google.com/translate_a/element.js?cb=googleTranslateElementInit"></script>
<script type="text/javascript">
  function googleTranslateElementInit() {
    new google.translate.TranslateElement({pageLanguage: 'en'}, 'google_translate_element');
  }
</script>

The binaries `hnefatafl-ai`, `hnefatafl-client`, `hnefatafl-server`,
`hnefatafl-server-full` and `hnefatafl-text-protocol` are made available from
the [Arch](./install.html#arch), [Cargo](./install.html#cargo),
[Debian](./install.html#debian), and [Fedora](./install.html#fedora-copr)
sources. The [Windows](./install.html#windows) and
[Flathub](./install.html#flathub) sources only provide `hnefatafl-client`. The
[NPM](./install.html#npm) source only provides something akin to
`hnefatafl-text-protocol`.

You should run the [Flathub](./install.html#flathub) package if you want the
client on Linux to avoid font issues.

## Running the Applications

You can pass `--help` to any of the following commands to get all of the
runtime options.

You can run the engine with:

```sh
hnefatafl-text-protocol --display-game
```

or you can start a local server:

```sh
hnefatafl-server-full
```

and run a local client:

```sh
hnefatafl-client --host localhost
```

## All platforms

You need to install [Rust].

### Linux Dependencies

ALSA development files are needed to build `cpal` on Linux (`rodio` dependency,
`hnefatafl-client` dependency). These are provided as part of the
`alsa-lib` package on Arch, `libasound2-dev` package on Debian and Ubuntu, and
`alsa-lib-devel` on Fedora.

The package uses the `mold` linker. This is provided via the `mold`
package on Arch, Debian, Ubuntu, and Fedora.

### Cargo [![crates-io]][crates-io-link]

In your terminal run:

```sh
# On Windows pass `--features console` to see console output.
cargo install hnefatafl-copenhagen
```

[crates-io]: https://img.shields.io/crates/v/hnefatafl-copenhagen.svg?logo=rust
[crates-io-link]: https://crates.io/crates/hnefatafl-copenhagen

### Source [![codeberg]][codeberg-link]

In your terminal run:

```sh
git clone https://codeberg.org/dcampbell/hnefatafl.git
```

[codeberg]: https://img.shields.io/badge/Codeberg-eeeeee?logo=codeberg
[codeberg-link]: https://codeberg.org/dcampbell/hnefatafl
[Rust]: https://rust-lang.org/learn/get-started/

## Linux

### Arch [![AUR]][aur-link]

In your terminal run:

```sh
sudo pacman --sync yay
yay --sync hnefatafl-copenhagen
```

[AUR]: https://img.shields.io/aur/version/hnefatafl-copenhagen?logo=archlinux
[aur-link]: https://aur.archlinux.org/packages/hnefatafl-copenhagen

### Debian ![deb]

[deb]: https://img.shields.io/badge/-111111?logo=debian

In your terminal run:

```sh
echo 'deb [arch=amd64 signed-by=/etc/apt/keyrings/packages.hnefatafl.org.asc] https://hnefatafl.org/apt stable main' \
| sudo tee /etc/apt/sources.list.d/hnefatafl.list
```

or if using the newer `/etc/apt/sources.list.d/debian.sources`, add:

```plaintext
Types: deb
URIs: https://hnefatafl.org/apt
Suites: stable
Components: main
Signed-By: /etc/apt/keyrings/packages.hnefatafl.org.asc
```

Then, Download [packages.hnefatafl.org.asc][1] to `/etc/apt/keyrings/` or:

```sh
wget https://hnefatafl.org/packages.hnefatafl.org.asc
sudo mv packages.hnefatafl.org.asc /etc/apt/keyrings/
```

Then:

```sh
sudo apt update && sudo apt install hnefatafl-copenhagen
```

[1]: packages.hnefatafl.org.asc

### Fedora COPR [![fedora]][fedora-copr]

In your terminal run:

```sh
sudo dnf copr enable dcampbell24/hnefatafl-copenhagen
sudo dnf install hnefatafl-copenhagen
```

[fedora]: https://img.shields.io/badge/-111111?logo=fedora
[fedora-copr]: https://copr.fedorainfracloud.org/coprs/dcampbell24/hnefatafl-copenhagen

### Flathub [![flathub]][flathub-link]

In your terminal run:

```sh
flatpak install flathub org.hnefatafl.hnefatafl_client
flatpak run org.hnefatafl.hnefatafl_client
```

or to see all your options run:

```sh
flatpak run org.hnefatafl.hnefatafl_client --help
```

or under KDE go to:

```sh
Application Launcher -> Games -> Hnefatafl Copenhagen
```

[flathub]: https://img.shields.io/flathub/v/org.hnefatafl.hnefatafl_client?logo=flathub
[flathub-link]: https://flathub.org/apps/org.hnefatafl.hnefatafl_client

## Windows

### Chocolatey [![chocolatey]][choco-link]

Install [Chocolatey](https://chocolatey.org/install).

Then install [hnefatafl-copenhagen](https://community.chocolatey.org/packages/hnefatafl-copenhagen/).

Go to the Start menu and run Hnefatafl.

[chocolatey]: https://img.shields.io/chocolatey/v/hnefatafl-copenhagen?logo=chocolatey
[choco-link]: https://community.chocolatey.org/packages/hnefatafl-copenhagen/

## Android

### F-Droid

1. Go to [Google](https://myaccount.google.com/security). Scroll down to `Enhanced
   Safe Browsing for your account`. If it is on, turn it off.
2. Go to [F-Droid](https://f-droid.org) and download F-Droid.
3. Open F-Droid, go to search, and install `Termux: Terminal emulator with packages`.
4. Open Termux, then run the following:

```sh
# Only need to run this once per update.

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
```

```sh
# Repeat this every time you want to run the program.

export LIBGL_ALWAYS_SOFTWARE=1 DISPLAY=:0
termux-x11 -xstartup xfce4-session &
hnefatafl-client --ascii
```

Then go to `Termux:X11` from the main menu. You may need to hold `Termux:11` to
go to the `Preferences` -> `Output`, then increase the `Display scale %`.

## NPM [![npm]][npm-link]

[npm]: https://img.shields.io/npm/v/hnefatafl-copenhagen?logo=npm
[npm-link]: https://www.npmjs.com/package/hnefatafl-copenhagen

```sh
# With Apache and Firefox on Debian.
sudo npm install -g hnefatafl-copenhagen
sudo mkdir --parent /var/www/html/pkg
sudo cp /usr/lib/node_modules/hnefatafl-copenhagen/ -r pkg /var/www/html
```

Then load the javascript on a webpage:

```sh
cat << EOF > /var/www/html/index.html
<!DOCTYPE html>
<html>
<head>
    <title>Copenhagen Hnefatafl</title>
</head>
<body>
    <h1>Copenhagen Hnefatafl</h1>
    <script type="module">
        import init, { Game } from '../pkg/hnefatafl_copenhagen.js';

        init().then(() => {
            const game = new Game();
            const output = game.read_line_js("show_board");
            console.log(output);
        });
    </script>
</body>
</html>
EOF
```

See the [message protocol] for all of the commands available.

[message protocol]: https://docs.rs/hnefatafl-copenhagen/latest/hnefatafl_copenhagen/message/enum.Message.html

```sh
# With Apache running.
firefox localhost
```

## Redox [@](https://www.redox-os.org/)

Edit your user config to include:

```toml
[packages]
hnefatafl-copenhagen = "source"
```

You should be at least using the packages supplied via
`include = ["desktop-minimal.toml"]`. Go to games and open Hnefatafl or run
`/usr/games/hnefatafl-client`. After logging in and exiting the application,
you'll have to restart Redox, because the application is buggy.

There are also at least these bugs:

1. There are no CJK fonts.
2. On a button press two characters are read in instead of one.
3. Backspace does not work.
4. I think you have to restart Redox after entering the application, because
   TcpStream shutdown is not implemented.
