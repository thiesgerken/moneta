FROM rust:1.89-bullseye@sha256:5b0defc2e3b26c97c917406f3d7c682028f87b2ae9adee184694b5c0279e2740 AS builder

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

FROM debian:bullseye-slim@sha256:6d3c63184632046054ae709964befc943ecffa140adc697ca955a10002a79c08
COPY --from=builder /build/target/release/moneta /bin/moneta
CMD moneta
