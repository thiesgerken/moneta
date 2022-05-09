FROM ubuntu:jammy as build-frontend

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -y && apt-get dist-upgrade -y \
    && apt-get install -y --no-install-recommends npm  \
    && rm -Rf /var/cache/apt

WORKDIR /build
COPY web/package.json web/package-lock.json ./
RUN npm install
COPY web .
RUN npm run build

FROM ubuntu:jammy as build-backend

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -y && apt-get dist-upgrade -y \
    && apt-get install -y --no-install-recommends ca-certificates curl pkg-config gcc clang make libc-dev libssl-dev  \
    && rm -Rf /var/cache/apt

RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y
ENV PATH="/root/.cargo/bin:$PATH"

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin && touch src/lib.rs && echo "fn main() {}" > src/bin/moneta.rs
RUN cargo build --target=x86_64-unknown-linux-gnu --release
RUN rm -Rf ./src

COPY src ./src
COPY --from=build-frontend /build/dist/ ./web/dist
RUN touch src/bin/moneta.rs src/lib.rs
RUN cargo build --target=x86_64-unknown-linux-gnu --release

RUN cp target/x86_64-unknown-linux-gnu/release/moneta .
RUN strip -s moneta

FROM ubuntu:jammy as run
COPY --from=build-backend /build/moneta /bin/moneta
CMD /bin/moneta
