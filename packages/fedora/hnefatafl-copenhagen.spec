Name:          hnefatafl-copenhagen
Version:       5.5.1
Release:       %autorelease
Summary:       Copenhagen Hnefatafl client, engine, server and artificial intelligence
License:       AGPL-3.0-or-later
URL:           https://hnefatafl.org
Source:        v%{version}-1.tar.gz

BuildRequires: clang
BuildRequires: llvm
BuildRequires: mold
BuildRequires: cargo
BuildRequires: openssl-devel
BuildRequires: alsa-lib-devel

Requires:      glibc
Requires:      hicolor-icon-theme
Requires:      openssl

%description
%{summary}

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

%install
install	-Dm755 "target/release/hnefatafl-ai" -t	"%{buildroot}/%{_bindir}"
install -Dm755 "target/release/hnefatafl-client" -t "%{buildroot}/%{_bindir}"
install -Dm755 "target/release/hnefatafl-server" -t "%{buildroot}/%{_bindir}"
install -Dm755 "target/release/hnefatafl-server-full" -t "%{buildroot}/%{_bindir}"
install -Dm755 "target/release/hnefatafl-text-protocol" -t "%{buildroot}/%{_bindir}"
install -Dm644 "packages/hnefatafl.service" -t "%{buildroot}/usr/lib/systemd/system"
install -Dm644 "packages/hnefatafl-ai-attacker.service" -t "%{buildroot}/usr/lib/systemd/system"
install -Dm644 "packages/hnefatafl-ai-defender.service" -t "%{buildroot}/usr/lib/systemd/system"
install -Dm644 "LICENSE.txt" "%{buildroot}/usr/share/licenses/$pkgname/LICENSE.txt"
install -Dm644 "website/src/images/helmet.svg" "%{buildroot}/usr/share/icons/hicolor/scalable/apps/org.hnefatafl.hnefatafl_client.svg"
install -Dm644 "hnefatafl-ai.1.gz" "%{buildroot}/usr/share/man/man1/hnefatafl-ai.1.gz"
install -Dm644 "hnefatafl-client.1.gz" "%{buildroot}/usr/share/man/man1/hnefatafl-client.1.gz"
install -Dm644 "hnefatafl-server.1.gz" "%{buildroot}/usr/share/man/man1/hnefatafl-server.1.gz"
install -Dm644 "hnefatafl-server-full.1.gz" "%{buildroot}/usr/share/man/man1/hnefatafl-server-full.1.gz"
install -Dm644 "hnefatafl-text-protocol.1.gz" "%{buildroot}/usr/share/man/man1/hnefatafl-text-protocol.1.gz"
install -Dm644 "packages/hnefatafl-client.desktop" "%{buildroot}/usr/share/applications/hnefatafl-client.desktop"

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
/usr/share/licenses/LICENSE.txt
/usr/share/man/man1/hnefatafl-ai.1.gz
/usr/share/man/man1/hnefatafl-client.1.gz
/usr/share/man/man1/hnefatafl-server-full.1.gz
/usr/share/man/man1/hnefatafl-server.1.gz
/usr/share/man/man1/hnefatafl-text-protocol.1.gz
%license LICENSE.txt
%doc README.md

%changelog
%autochangelog
