FROM rust:1.96-bookworm@sha256:a339861ae23e9abb272cea45dfafde21760d2ce6577a70f8a926153677902663 AS builder

RUN apt-get update -y && apt-get dist-upgrade -y \
  && apt-get update -y && apt-get install -y --no-install-recommends libc-dev libssl-dev pkg-config gcc \
  && rm -Rf /var/cache/apt

WORKDIR /build

COPY Cargo.toml .
COPY Cargo.lock .
RUN mkdir -p src/bin && touch src/lib.rs && touch src/bin/moneta.rs && touch src/bin/sandbox.rs && cargo build --lib --release

COPY src ./src

RUN touch src/lib.rs && touch src/bin/moneta.rs && touch src/bin/sandbox.rs
RUN cargo build --release
RUN strip -s target/release/moneta

FROM debian:bookworm-slim@sha256:7b140f374b289a7c2befc338f42ebe6441b7ea838a042bbd5acbfca6ec875818
COPY --from=builder /build/target/release/moneta /bin/moneta
CMD moneta
