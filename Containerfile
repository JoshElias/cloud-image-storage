# syntax=docker/dockerfile:1.7

FROM docker.io/library/rust:1.92-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    rustup target add wasm32-unknown-unknown \
    && cargo install cargo-leptos --version 0.3.6 --locked
COPY src ./src
COPY public ./public
COPY style ./style
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo leptos build --release \
    && mkdir -p /app/build-output \
    && cp /app/target/release/cis /app/build-output/cis \
    && cp -r /app/target/site /app/build-output/site

FROM docker.io/library/debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/build-output/cis /usr/local/bin/cis
COPY --from=build /app/build-output/site /srv/cis/site
ENTRYPOINT ["/usr/local/bin/cis"]
