let
  nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-24.05";
  pkgs = import nixpkgs { config = {}; overlays = []; };
in
{
  hnefatafl-copenhagen = pkgs.callPackage ./hnefatafl-copenhagen.nix { };
}
