{
  description = "An event stream store for personal, local-first use, specializing in event sourcing.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ { flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      perSystem = { config, self', inputs', pkgs, system, ... }: let
        pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [ (import inputs.rust-overlay) ];
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default;

        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;

        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            (builtins.match ".*xs\.nu$" path != null) ||
            (craneLib.filterCargoSources path type);
        };

        commonArgs = {
          inherit src;
          strictDeps = true;
          buildInputs = with pkgs; [
            openssl
          ];
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        cross-stream = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          doCheck = false;
        });
      in {
        checks = {
          inherit cross-stream;
        };

        packages = {
          default = cross-stream;
          cross-stream = cross-stream;
        };

        apps.default = {
          type = "app";
          program = "${cross-stream}/bin/xs";
        };

        devShells = {
          default = craneLib.devShell {
            checks = self'.checks;
            packages = with pkgs; [
              rust-analyzer
              rustfmt
              clippy
              nushell
              cross-stream
            ];
            shellHook = ''
              if [ -z "$CI" ]; then
                nu
              fi
            '';
          };

          bash = craneLib.devShell {
            checks = self'.checks;
            packages = with pkgs; [
              rust-analyzer
              rustfmt
              clippy
              cross-stream
            ];
          };
        };
      };
    };
}
