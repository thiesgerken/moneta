FROM rust:1.97-bookworm@sha256:14bc9c5966e7b3a385794b3d5389a8765668342025fbcc7b2e3d2866ac4bd8c3 AS builder

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
