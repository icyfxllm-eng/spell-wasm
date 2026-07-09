# Stage 1: compile the Rust/WASM frontend from source.
FROM rust:1-bookworm AS build
RUN rustup target add wasm32-unknown-unknown \
    && cargo install wasm-bindgen-cli --version 0.2.126
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --target wasm32-unknown-unknown \
    && wasm-bindgen target/wasm32-unknown-unknown/release/spell_wasm.wasm \
       --out-dir /out/pkg --target web --no-typescript

# Stage 2: serve the static site with Caddy (automatic HTTPS included).
FROM caddy:2-alpine
COPY Caddyfile /etc/caddy/Caddyfile
COPY index.html privacy.html ocr-shim.js audio-native.js manifest.json sw.js /srv/
COPY icons /srv/icons
# Android App Links verification file, served at /.well-known/assetlinks.json.
COPY .well-known /srv/.well-known
COPY --from=build /out/pkg /srv/pkg
