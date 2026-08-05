FROM rust:1.97-bookworm@sha256:77fac8b98f9f46062bb680b6d25d5bcaabfc400143952ebc572e924bcbedc3fa AS builder

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

FROM debian:bookworm-slim@sha256:7b140f374b289a7c2befc338f42ebe6441b7ea838a042bbd5acbfca6ec875818
COPY --from=builder /build/target/release/moneta /bin/moneta
CMD moneta
