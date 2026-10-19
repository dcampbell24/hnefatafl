Name:          hnefatafl-copenhagen
Version:       7.0.0
Release:       1%{?dist}
Summary:       Copenhagen Hnefatafl client, engine, server and artificial intelligence
License:       AGPL-3.0-or-later
URL:           https://hnefatafl.org
Source0:       https://codeberg.org/dcampbell/hnefatafl/archive/v%{version}.tar.gz
Source1:       https://codeberg.org/dcampbell/hnefatafl/media/branch/main/default_nn.onnx
BuildRequires: clang
BuildRequires: llvm
BuildRequires: mold
BuildRequires: cargo
BuildRequires: openssl-devel
BuildRequires: alsa-lib-devel
BuildRequires: libappstream-glib
BuildRequires: desktop-file-utils

Requires:      glibc
Requires:      hicolor-icon-theme
Requires:      openssl

%global _description %{expand:
This package contains an engine, server, client, artificial intelligence and
systemd services to run them.

Their are two server binaries. One which is the full version and another just
for running hnefatafl text protocol clients. The pack includes a systemd
service for running the full server.

The artificial intelligence comes with a service for running it as an attacker
and as a defender. When you run the service, the player will repeatedly create
a game and wait for a challenger.

The hnefatafl-text-protocol can be piped to the server or run as a standalone
binary. It has various options such as displaying the game with
`--display-game` for a user friendly interface.

The client is a graphical user interface that connects to the server-full. By
default it connects to the server running at hnefatafl.org.}

%description %_description

%define toolchain clang
%prep
%setup -q -n hnefatafl

%build
sed -i 's/cargo-/fedora-/' src/lib.rs;

RUST_MIN_STACK=2147483648 cargo build --release

./target/release/taflzero --man --username ""
./target/release/hnefatafl-client --man
./target/release/hnefatafl-server --man
./target/release/hnefatafl-text-protocol --man

sed -i 's/games/bin/' packages/hnefatafl-ai-attacker.service
sed -i 's/games/bin/' packages/hnefatafl-ai-defender.service
sed -i 's/games/bin/' packages/hnefatafl.service
sed -i 's/Exec=hnefatafl-client/Exec=hnefatafl-client --ascii/' packages/hnefatafl-client.desktop

%install
install -Dm755 "target/release/taflzero" -t "%{buildroot}%{_bindir}"
install -Dm755 "target/release/hnefatafl-client" -t "%{buildroot}%{_bindir}"
install -Dm755 "target/release/hnefatafl-server" -t "%{buildroot}%{_bindir}"
install -Dm755 "target/release/hnefatafl-text-protocol" -t "%{buildroot}%{_bindir}"
install -Dm644 "packages/hnefatafl.service" -t "%{buildroot}%{_unitdir}"
install -Dm644 "packages/hnefatafl-ai-attacker.service" -t "%{buildroot}%{_unitdir}"
install -Dm644 "packages/hnefatafl-ai-defender.service" -t "%{buildroot}%{_unitdir}"
install -Dm644 "website/src/images/helmet.svg" "%{buildroot}%{_datadir}/icons/hicolor/scalable/apps/org.hnefatafl.hnefatafl_client.svg"
install -Dm644 "taflzero.1*" -t "%{buildroot}%{_mandir}/man1"
install -Dm644 "hnefatafl-client.1*" -t "%{buildroot}%{_mandir}/man1"
install -Dm644 "hnefatafl-server.1*" -t "%{buildroot}%{_mandir}/man1"
install -Dm644 "hnefatafl-text-protocol.1.*" -t "%{buildroot}%{_mandir}/man1"
install -Dm644 "packages/hnefatafl-client.desktop" -t "%{buildroot}%{_datadir}/applications"
install -Dm644 "packages/org.hnefatafl.hnefatafl_client.metainfo.xml" -t "%{buildroot}%{_metainfodir}"
install -Dm644 "%{SOURCE1}" -t "%{buildroot}%{_datadir}/taflzero"

%files
%{_bindir}/hnefatafl-client
%{_bindir}/hnefatafl-server
%{_bindir}/hnefatafl-text-protocol
%{_bindir}/taflzero
%{_datadir}/taflzero/default_nn.onnx
%{_unitdir}/hnefatafl-ai-attacker.service
%{_unitdir}/hnefatafl-ai-defender.service
%{_unitdir}/hnefatafl.service
%{_datadir}/applications/hnefatafl-client.desktop
%{_datadir}/icons/hicolor/scalable/apps/org.hnefatafl.hnefatafl_client.svg
%{_mandir}/man1/hnefatafl-client.1*
%{_mandir}/man1/hnefatafl-server.1*
%{_mandir}/man1/hnefatafl-text-protocol.1*
%{_mandir}/man1/taflzero.1*
%{_metainfodir}/org.hnefatafl.hnefatafl_client.metainfo.xml
%license LICENSES/AGPL-3.0-or-later.txt
%doc README.md

%changelog
%autochangelog

%check
desktop-file-validate %{buildroot}/%{_datadir}/applications/hnefatafl-client.desktop
appstream-util validate-relax --nonet %{buildroot}%{_metainfodir}/*.metainfo.xml
