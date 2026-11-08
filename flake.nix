{
  description = "Moneta: dev shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # Builds the exact toolchain pinned in rust-toolchain.toml.
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, rust-overlay, ... }:
    let
      forSystems = nixpkgs.lib.genAttrs [
        "x86_64-linux"
        "aarch64-darwin"
      ];
    in
    {
      devShells = forSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
          rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        in
        {
          default = pkgs.mkShell {
            packages = [
              rust
              pkgs.pkg-config
              pkgs.openssl
              pkgs.nodejs
              # psql for tests/run-sql-tests.sh; the test databases themselves run in Docker.
              pkgs.postgresql
            ];
            shellHook = ''
              printf '💶 \033[1m\033[38;2;46;204;113mM\033[38;2;41;189;107mo\033[38;2;36;174;101mn\033[38;2;32;158;96me\033[38;2;27;143;90mt\033[38;2;22;128;84ma\033[0m  \033[2m%s\033[0m\n' 'Rust ${rust.version} · node ${pkgs.nodejs.version}'
              printf '  \033[32m%s\033[0m \033[2m·\033[0m \033[32m%s\033[0m\n' 'cd backend && cargo test' 'cd frontend && npm run dev'
            '';
          };
        }
      );
    };
}
