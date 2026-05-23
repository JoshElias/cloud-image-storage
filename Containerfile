FROM docker.io/library/rust:1.92-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY public ./public
COPY style ./style
RUN rustup target add wasm32-unknown-unknown \
    && cargo install cargo-leptos --version 0.3.6 --locked \
    && cargo leptos build --release

FROM docker.io/library/debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/cis /usr/local/bin/cis
COPY --from=build /app/target/site /srv/cis/site
ENTRYPOINT ["/usr/local/bin/cis"]
