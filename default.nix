{ pkgs ? import <nixpkgs> { } }:

let manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
in
pkgs.rustPlatform.buildRustPackage rec {
  pname = manifest.name;
  version = manifest.version;
  cargoLock = {
    lockFile = ./Cargo.lock;
  };
  src = pkgs.lib.cleanSource ./.;
  buildPhase = ''
    cargo build --release --bin hnefatafl-client --features client --no-default-features
    # install -Dm755 target/release/examples/hnefatafl-client /app/bin/$FLATPAK_ID
    # install -Dm644 icons/king.svg /app/share/icons/hicolor/scalable/apps/$FLATPAK_ID.svg
    # install -Dm644 packages/$FLATPAK_ID.desktop /app/share/applications/$FLATPAK_ID.desktop
    # install -Dm644 packages/$FLATPAK_ID.metainfo.xml /app/share/metainfo/$FLATPAK_ID.metainfo.xml
  '';
}
