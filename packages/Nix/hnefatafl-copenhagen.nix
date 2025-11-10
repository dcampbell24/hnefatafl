{
  fetchzip,
  stdenv,
}:

{ pkgs ? import <nixpkgs> {} }:

stdenv.mkDerivation {
  pname = "hnefatafl-copenhagen";
  version = "4.2.2";
  src = ../../.;

  unpackPhase = ''
    ls -la $srcs/
    cp --recursive $srcs/* .
    cp $srcs/packages/flathub/cargo-sources.json .
  '';

  nativeBuildInputs = [ pkgs.cargo ];

  buildPhase = ''
    ls -la
    cargo build --offline --release --example hnefatafl-client --no-default-features
  '';

  installPhase = ''

  '';
}
