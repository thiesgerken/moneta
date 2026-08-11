FROM rust:1.97-bookworm@sha256:593db038d36ea0d2e5b22a2e095eb95dbec607d88d658c2f35f65ee880b4f93a AS builder

RUN apt-get update -y && apt-get dist-upgrade -y \
  && apt-get update -y && apt-get install -y --no-install-recommends libc-dev libssl-dev pkg-config gcc \
  && rm -Rf /var/cache/apt

WORKDIR /build

COPY Cargo.toml .
COPY Cargo.lock .
RUN mkdir -p src/bin && touch src/lib.rs && touch src/bin/moneta.rs && cargo build --lib --release

# embed_migrations! reads these at compile time, so they have to be here before the real build
COPY migrations ./migrations
COPY src ./src

RUN touch src/lib.rs && touch src/bin/moneta.rs
RUN cargo build --release
RUN strip -s target/release/moneta

FROM debian:bookworm-slim@sha256:abd67ffcfa541b485a3dff59865ab629aa048a6c613e639d36e7456b0b229241
COPY --from=builder /build/target/release/moneta /bin/moneta
CMD moneta
