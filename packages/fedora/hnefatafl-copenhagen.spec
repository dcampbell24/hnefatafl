Name:          hnefatafl-copenhagen
Version:       5.6.1
Release:       4%{?dist}
Summary:       Copenhagen Hnefatafl client, engine, server and artificial intelligence
License:       AGPL-3.0-or-later
URL:           https://hnefatafl.org
Source:        https://codeberg.org/dcampbell/hnefatafl/archive/v%{version}-2.tar.gz

BuildRequires: clang
BuildRequires: llvm
BuildRequires: mold
BuildRequires: cargo
BuildRequires: openssl-devel
BuildRequires: alsa-lib-devel

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

%prep
%setup -q -n hnefatafl

%build
cargo build --release

./target/release/hnefatafl-ai --man --username ""
./target/release/hnefatafl-client --man
./target/release/hnefatafl-server --man
./target/release/hnefatafl-server-full --man
./target/release/hnefatafl-text-protocol --man

gzip --no-name --best hnefatafl-ai.1
gzip --no-name --best hnefatafl-server.1
gzip --no-name --best hnefatafl-server-full.1
gzip --no-name --best hnefatafl-text-protocol.1
gzip --no-name --best hnefatafl-client.1

sed -i 's/games/bin/' packages/hnefatafl-ai-attacker.service
sed -i 's/games/bin/' packages/hnefatafl-ai-defender.service
sed -i 's/games/bin/' packages/hnefatafl.service
sed -i 's/Exec=hnefatafl-client/Exec=hnefatafl-client --ascii/' packages/hnefatafl-client.desktop

%install
install -Dm755 "target/release/hnefatafl-ai" -t "%{buildroot}%{_bindir}"
install -Dm755 "target/release/hnefatafl-client" -t "%{buildroot}%{_bindir}"
install -Dm755 "target/release/hnefatafl-server" -t "%{buildroot}%{_bindir}"
install -Dm755 "target/release/hnefatafl-server-full" -t "%{buildroot}%{_bindir}"
install -Dm755 "target/release/hnefatafl-text-protocol" -t "%{buildroot}%{_bindir}"
install -Dm644 "packages/hnefatafl.service" -t "%{buildroot}/usr/lib/systemd/system"
install -Dm644 "packages/hnefatafl-ai-attacker.service" -t "%{buildroot}/usr/lib/systemd/system"
install -Dm644 "packages/hnefatafl-ai-defender.service" -t "%{buildroot}/usr/lib/systemd/system"
install -Dm644 "website/src/images/helmet.svg" "%{buildroot}/usr/share/icons/hicolor/scalable/apps/org.hnefatafl.hnefatafl_client.svg"
install -Dm644 "hnefatafl-ai.1.gz" -t "%{buildroot}/usr/share/man/man1"
install -Dm644 "hnefatafl-client.1.gz" -t "%{buildroot}/usr/share/man/man1"
install -Dm644 "hnefatafl-server.1.gz" -t "%{buildroot}/usr/share/man/man1"
install -Dm644 "hnefatafl-server-full.1.gz" -t "%{buildroot}/usr/share/man/man1"
install -Dm644 "hnefatafl-text-protocol.1.gz" -t "%{buildroot}/usr/share/man/man1"
install -Dm644 "packages/hnefatafl-client.desktop" -t "%{buildroot}/usr/share/applications"
install -Dm644 "packages/org.hnefatafl.hnefatafl_client.metainfo.xml" -t "%{buildroot}%{_metainfodir}"

%files
%{_bindir}/hnefatafl-ai
%{_bindir}/hnefatafl-client
%{_bindir}/hnefatafl-server
%{_bindir}/hnefatafl-server-full
%{_bindir}/hnefatafl-text-protocol
/usr/lib/systemd/system/hnefatafl-ai-attacker.service
/usr/lib/systemd/system/hnefatafl-ai-defender.service
/usr/lib/systemd/system/hnefatafl.service
/usr/share/applications/hnefatafl-client.desktop
/usr/share/icons/hicolor/scalable/apps/org.hnefatafl.hnefatafl_client.svg
/usr/share/man/man1/hnefatafl-ai.1.gz
/usr/share/man/man1/hnefatafl-client.1.gz
/usr/share/man/man1/hnefatafl-server-full.1.gz
/usr/share/man/man1/hnefatafl-server.1.gz
/usr/share/man/man1/hnefatafl-text-protocol.1.gz
%{_metainfodir}/org.hnefatafl.hnefatafl_client.metainfo.xml
%license LICENSES/AGPL-3.0-or-later.txt
%doc README.md

%changelog
%autochangelog

%check
