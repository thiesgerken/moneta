{
  description = "Moneta: dev shell and backend image";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # Builds the exact toolchain pinned in rust-toolchain.toml.
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      crane,
      ...
    }:
    let
      forSystems = nixpkgs.lib.genAttrs [
        "x86_64-linux"
        "aarch64-darwin"
      ];
      pkgsFor =
        system:
        import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
      rustFor = pkgs: pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
    in
    {
      devShells = forSystems (
        system:
        let
          pkgs = pkgsFor system;
          rust = rustFor pkgs;
        in
        {
          default = pkgs.mkShell {
            packages = [
              rust
              # Its own test suite fails in the nixpkgs sandbox, so there is no cached build to
              # fetch either way and skipping the tests is what makes it build at all.
              (pkgs.cargo-llvm-cov.overrideAttrs { doCheck = false; })
              pkgs.pkg-config
              pkgs.openssl
              pkgs.nodejs_26
              # Trails packageManager in frontend/package.json by a few minor versions, which
              # reads the same lockfile. Following it exactly would take a hash in here that
              # renovate cannot update.
              pkgs.pnpm
              # psql for tests/run-sql-tests.sh, and the server for the database tests in CI.
              # uuidv7() as a column default needs 18.
              pkgs.postgresql_18
            ];
            shellHook = ''
              printf '💶 \033[1m\033[38;2;46;204;113mM\033[38;2;41;189;107mo\033[38;2;36;174;101mn\033[38;2;32;158;96me\033[38;2;27;143;90mt\033[38;2;22;128;84ma\033[0m  \033[2m%s\033[0m\n' 'Rust ${rust.version} · node ${pkgs.nodejs_26.version}'
              printf '  \033[32m%s\033[0m \033[2m·\033[0m \033[32m%s\033[0m\n' 'cd backend && cargo test' 'cd frontend && pnpm dev'
            '';
          };
        }
      );

      packages.x86_64-linux =
        let
          pkgs = pkgsFor "x86_64-linux";
          inherit (pkgs) lib;
          craneLib = (crane.mkLib pkgs).overrideToolchain rustFor;

          cargoArgs = {
            src = lib.fileset.toSource {
              root = ./backend;
              fileset = lib.fileset.unions [
                (craneLib.fileset.commonCargoSources ./backend)
                # embed_migrations! reads them at compile time.
                ./backend/migrations
              ];
            };
            strictDeps = true;
          };

          moneta = craneLib.buildPackage (
            cargoArgs
            // {
              cargoArtifacts = craneLib.buildDepsOnly cargoArgs;
              cargoExtraArgs = "--locked --bin moneta";
              # The CI test jobs run the tests.
              doCheck = false;
            }
          );
        in
        {
          backend-image = pkgs.dockerTools.streamLayeredImage {
            # So that `./result | docker load` yields the image docker-compose.yaml runs.
            name = "gitea.gerken.haus/thies/moneta/backend";
            tag = "main";
            # No User: the deployment binds port 80, which needs root.
            config.Env = [
              "PATH=${lib.makeBinPath [ moneta ]}"
              # For reaching stockdb over https: rustls finds the root store through this.
              "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
            ];
          };
        };
    };
}
