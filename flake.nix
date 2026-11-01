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
        in
        {
          default = pkgs.mkShell {
            packages = [
              (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
              pkgs.pkg-config
              pkgs.openssl
              pkgs.nodejs
              # pnpm switches itself to the version in frontend/package.json's packageManager.
              pkgs.pnpm
              # psql for tests/run-sql-tests.sh; the test databases themselves run in Docker.
              pkgs.postgresql
            ];
          };
        }
      );
    };
}
