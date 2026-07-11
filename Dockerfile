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
COPY index.html privacy.html audio-native.js manifest.json sw.js /srv/
COPY icons /srv/icons
# Android App Links verification file, served at /.well-known/assetlinks.json.
COPY .well-known /srv/.well-known
COPY --from=build /out/pkg /srv/pkg
# Cache-bust the WASM/glue by content hash so a new deploy can never serve a
# stale glue against a fresh .wasm (or vice versa) through Cloudflare/browser
# caches — same-named files with a ?v=<hash> query are distinct cache keys.
# Replaces the ?v=DEV placeholder in index.html + sw.js with the wasm's hash.
RUN VER=$(sha256sum /srv/pkg/spell_wasm_bg.wasm | cut -c1-12) \
    && sed -i "s/?v=DEV/?v=$VER/g" /srv/index.html /srv/sw.js \
    && echo "cache-bust version: $VER"
