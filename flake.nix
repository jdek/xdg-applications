{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-parts.inputs.nixpkgs-lib.follows = "nixpkgs";
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [];
      systems = ["aarch64-darwin" "aarch64-linux" "riscv64-linux" "x86_64-darwin" "x86_64-linux"];
      perSystem = {
        config,
        self',
        inputs',
        pkgs,
        system,
        ...
      }: {
        packages.default = let
          toml = pkgs.lib.importTOML ./Cargo.toml;
        in
          pkgs.rustPlatform.buildRustPackage rec {
            inherit (toml.package) name version;

            pname = name;
            src = ./.;

            cargoLock.lockFile = ./Cargo.lock;
          };
        devShells.default = pkgs.mkShell {
          name = "rust";
          nativeBuildInputs = with pkgs; [
            cargo
            clippy
            gdb
            rust-analyzer
            rustc
            rustfmt
          ];
        };
        formatter = pkgs.alejandra;
      };
    };
}
