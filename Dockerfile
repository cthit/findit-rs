FROM rust:1.93-trixie AS client-builder

ARG CARGO_BINSTALL_VERSION=v1.17.9
ARG DX_VERSION=0.7.3

RUN apt-get update && apt-get install -y --no-install-recommends \
    binaryen \
    ca-certificates \
    curl \
    libsqlite3-dev \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL "https://github.com/cargo-bins/cargo-binstall/releases/download/${CARGO_BINSTALL_VERSION}/cargo-binstall-x86_64-unknown-linux-gnu.tgz" \
    | tar -xz -C /usr/local/cargo/bin cargo-binstall

RUN cargo binstall -y --force "dioxus-cli@${DX_VERSION}"

RUN rustup target add wasm32-unknown-unknown

WORKDIR /app
COPY Cargo.toml Cargo.lock Dioxus.toml ./

RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo fetch
RUN rm src/main.rs

COPY src ./src/
COPY assets ./assets/

RUN touch src/main.rs
RUN dx build --release --fullstack --force-sequential

FROM rust:1.93-alpine AS server-builder

RUN apk add --no-cache \
    build-base \
    ca-certificates \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconfig \
    sqlite-dev

WORKDIR /app
COPY Cargo.toml Cargo.lock ./

RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo fetch
RUN cargo build --release --no-default-features --features server
RUN rm src/main.rs

COPY src ./src/
COPY assets ./assets/

RUN touch src/main.rs
RUN cargo build --release --no-default-features --features server

FROM client-builder AS asset-patcher

COPY --from=server-builder /app/target/release/find-it /app/find-it

RUN mkdir -p /app/patched-assets
RUN dx tools assets /app/find-it /app/patched-assets

##########################
#    PRODUCTION STAGE    #
##########################
FROM scratch

WORKDIR /app

COPY --from=server-builder /etc/ssl /etc/ssl
COPY --from=asset-patcher /app/find-it ./find-it
COPY --from=client-builder /app/target/dx/find-it/release/web/public ./public
COPY --from=asset-patcher /app/patched-assets/ ./public/assets/

ENV IP=0.0.0.0
ENV PORT=8080
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt

ENTRYPOINT ["/app/find-it"]
