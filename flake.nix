# This file is part of hnefatafl-copenhagen.
#
# hnefatafl-copenhagen is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# hnefatafl-copenhagen is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-FileCopyrightText: 2026 Anton Büttner (ruarq) <anton@ruarq.co>
# SPDX-License-Identifier: AGPL-3.0-or-later
{
  description = "flake for hnefatafl-copenhagen";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    flake-utils.url = "github:numtide/flake-utils";

    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      crane,
      ...
    }:
    let
      supportedSystems = [ "x86_64-linux" ];
    in
    flake-utils.lib.eachSystem supportedSystems (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        lib = pkgs.lib;

        craneLib = crane.mkLib pkgs;

        nativeBuildInputs = with pkgs; [
          clang
          mold
          pkg-config
          wget
        ];

        buildInputs = with pkgs; [
          openssl
          alsa-lib
          onnxruntime
        ];

        commonArgs = {
          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              (craneLib.fileset.commonCargoSources ./.)
              ./src/bin/hnefatafl-client/assets
              ./tests
            ];
          };

          strictDeps = true;

          nativeBuildInputs = nativeBuildInputs;
          buildInputs = buildInputs;

          doCheck = false;

          env = {
            ORT_STRATEGY = "system";
            ORT_LIB_LOCATION = "${pkgs.onnxruntime}/lib";
            ORT_PREFER_DYNAMIC_LINK = "1";
          };
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      in
      {
        packages.default = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
          }
        );

        checks = {
          clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            }
          );

          fmt = craneLib.cargoFmt {
            inherit (commonArgs) src;
          };

          # This is currently broken. The tests pass in the development environment (if you manually run `cargo test`),
          # so it's a flake configuration issue. If the testing problems have been fixed, we can try uncommenting this to
          # see if it fixed any problems.
          # tests = craneLib.cargoTest (
          #   commonArgs
          #   // {
          #     inherit cargoArtifacts;
          #     doCheck = true;
          #
          #     # tests don't pass without building explicitly and omitting the --release flag
          #     checkPhase = ''
          #       runHook preCheck
          #
          #       cargoWithProfile build
          #       cargoWithProfile test
          #
          #       runHook postCheck
          #     '';
          #   }
          # );
        };

        devShells.default = craneLib.devShell {
          packages = buildInputs ++ nativeBuildInputs;
        };
      }
    );

}
