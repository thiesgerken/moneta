{
  description = "Moneta: dev shell and container images";

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
      self,
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
              # psql for tests/run-sql-tests.sh, and the server for the database tests in CI.
              # uuidv7() as a column default needs 18.
              pkgs.postgresql_18
            ];
            shellHook = ''
              printf '💶 \033[1m\033[38;2;46;204;113mM\033[38;2;41;189;107mo\033[38;2;36;174;101mn\033[38;2;32;158;96me\033[38;2;27;143;90mt\033[38;2;22;128;84ma\033[0m  \033[2m%s\033[0m\n' 'Rust ${rust.version} · node ${pkgs.nodejs_26.version}'
              printf '  \033[32m%s\033[0m \033[2m·\033[0m \033[32m%s\033[0m\n' 'cd backend && cargo test' 'cd frontend && npm run dev'
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

          frontend = pkgs.buildNpmPackage {
            pname = "moneta-frontend";
            version = "0.0.1";
            src = ./frontend;
            nodejs = pkgs.nodejs_26;
            npmDeps = pkgs.importNpmLock { npmRoot = ./frontend; };
            inherit (pkgs.importNpmLock) npmConfigHook;
            # houdini's codegen is a prebuilt binary from npm, linked against an FHS loader the
            # sandbox does not have, and the build runs it.
            nativeBuildInputs = [ pkgs.autoPatchelfHook ];
            preBuild = "autoPatchelf node_modules/houdini-*";
            # The menu links the commit the bundle was built from, and the sandbox has no git
            # history.
            env.GIT_SHA = self.rev or self.dirtyRev;
            # adapter-node's output imports the runtime dependencies from node_modules.
            installPhase = ''
              runHook preInstall
              npm prune --omit=dev
              mkdir $out
              cp -r build node_modules package.json $out
              runHook postInstall
            '';
          };

          # Both images run as root, because both deployments bind port 80. The name and tag are
          # what docker-compose.yaml runs, so `./result | docker load` drops a local build in.
          image =
            component: config:
            pkgs.dockerTools.streamLayeredImage {
              name = "gitea.gerken.haus/thies/moneta/${component}";
              tag = "main";
              inherit config;
            };
        in
        {
          backend-image = image "backend" {
            Env = [
              "PATH=${lib.makeBinPath [ moneta ]}"
              # For reaching stockdb over https: rustls finds the root store through this.
              "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
            ];
          };

          frontend-image = image "frontend" {
            Cmd = [
              (lib.getExe pkgs.nodejs-slim_26)
              "${frontend}/build"
            ];
            Env = [
              "HOST=0.0.0.0"
              "PORT=80"
            ];
          };
        };
    };
}
