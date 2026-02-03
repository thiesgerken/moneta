FROM rust:1.89-bullseye@sha256:8f72d971a31b278cebdb2eb64a44c3900ff27c716a4ef6f4db05946d10c9ae4e AS builder

RUN apt-get update -y && apt-get dist-upgrade -y \
  && apt-get update -y && apt-get install -y --no-install-recommends libc-dev libssl-dev pkg-config gcc clang \
  && rm -Rf /var/cache/apt

WORKDIR /build

COPY Cargo.toml .
COPY Cargo.lock .
RUN mkdir -p src/bin && touch src/lib.rs && touch src/bin/moneta.rs && touch src/bin/sandbox.rs && cargo build --lib --release

COPY src ./src

RUN touch src/lib.rs && touch src/bin/moneta.rs && touch src/bin/sandbox.rs
RUN cargo build --release
RUN strip -s target/release/moneta

FROM debian:bullseye-slim@sha256:f0dbd70ae23f6ffa17f8b816a1ba1a489f7e9b3c32328867f6b456dec869e031
COPY --from=builder /build/target/release/moneta /bin/moneta
CMD moneta
