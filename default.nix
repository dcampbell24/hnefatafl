{ pkgs ? import <nixpkgs> { } }:

let manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
in
pkgs.rustPlatform.buildRustPackage rec {
  pname = manifest.name;
  version = manifest.version;
  tag = "4.2.2";
  cargoLock = {
    lockFile = ./Cargo.lock;
    outputHashes = {
        "cryoglyph-0.1.0" = "sha256-PyPizF+kgC7wUBOd/UEbPpUvW7oj8tEz/QHaIsk5+t0=";
        "dpi-0.1.1" = "sha256-pQn1lCFSJMkjUfHoggEzMHnm5k+Chnzi5JEDjahnjUA=";
        "iced-0.14.0-dev" = "sha256-dNJ6pv+WEIrJfi2OZIkmF5KDqhd4b/JH7EIcIYlG1qg=";
    };
  };
  src = pkgs.lib.cleanSource ./.;
  buildPhase = ''
    cargo build --release --example hnefatafl-client --no-default-features
    # install -Dm755 target/release/examples/hnefatafl-client /app/bin/$FLATPAK_ID
    # install -Dm644 icons/king.svg /app/share/icons/hicolor/scalable/apps/$FLATPAK_ID.svg
    # install -Dm644 packages/$FLATPAK_ID.desktop /app/share/applications/$FLATPAK_ID.desktop
    # install -Dm644 packages/$FLATPAK_ID.metainfo.xml /app/share/metainfo/$FLATPAK_ID.metainfo.xml
  '';
}
