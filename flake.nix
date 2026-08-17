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

          tests = craneLib.cargoTest (
            commonArgs
            // {
              inherit cargoArtifacts;
              doCheck = true;

              # tests don't pass without building explicitly and omitting the --release flag
              checkPhase = ''
                runHook preCheck

                cargoWithProfile build
                cargoWithProfile test

                runHook postCheck
              '';
            }
          );
        };

        devShells.default = craneLib.devShell {
          packages = buildInputs ++ nativeBuildInputs;
        };
      }
    );

}
