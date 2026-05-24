# Cloud Image Storage

Private AWS-backed image archive for Google Takeout imports and manual Pixel-origin uploads.

The current implementation is the v1 foundation:

- Rust `cis` binary with `serve`, `worker`, `import takeout`, and `upload` commands.
- Shared ingest code for image discovery, SHA-256 hashing, MIME detection, and duplicate collapse.
- Leptos-rendered admin pages and JSON ingest endpoints.
- OpenTofu infrastructure for EC2, S3 lifecycle, host WireGuard, IAM, and encrypted EBS.
- Podman Quadlet units for Caddy, the app, worker, Postgres, and backups.

## Local Development

```sh
cargo test
cargo run -- serve --config config/example.toml
```

Open `http://127.0.0.1:8080` locally. In production Caddy exposes the portal over HTTPS while WireGuard remains available for host administration.

The UI is built with Leptos. To verify the server and wasm build path:

```sh
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos --version 0.3.6 --locked
cargo leptos build
```

## Ingest Commands

Preview a Google Takeout import:

```sh
cargo run -- import takeout --path /path/to/takeout --collection "Google Takeout 2026-05"
```

Preview a manual upload batch:

```sh
cargo run -- upload --path /path/to/photos --collection "Pixel uploads"
```

Both commands currently build an ingest manifest and print a summary. The API types and server endpoints are in place for the next step: uploading originals, previews, and metadata through presigned S3 URLs.

Generate a production admin password hash:

```sh
cargo run -- auth hash-password --password "replace-me"
```

## Production Access

The production plan uses public HTTPS for the portal and WireGuard rather than AWS SSM for host administration:

- One public UDP port, default `51820`.
- Public HTTP/HTTPS for Caddy and Let's Encrypt.
- No public SSH or Postgres ingress.
- Host administration stays behind WireGuard.
- The EC2 host bootstraps itself with cloud-init and pulls the app image from GHCR.

See `deploy/README.md` for the infrastructure and host layout.
