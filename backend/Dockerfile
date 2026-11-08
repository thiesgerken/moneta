FROM rust:1.97-bookworm@sha256:0e2bcaef56d041a486784e54104a81aebe0da44bd03019bd70bc0401e42e4a97 AS builder

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

FROM debian:bookworm-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc

# Only for reaching renditier over https. In the cluster that call goes to a Service over plain
# http and this is dead weight, but without it an `https://` url fails with a certificate error
# that looks nothing like the missing root store it is.
RUN apt-get update -y && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/moneta /bin/moneta
CMD moneta
