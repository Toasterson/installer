# installer-ui

A Rust web application that provides a minimal UI and HTTP API for the installer. It serves a small web page, exposes HTTP endpoints, and acts as a client of the `machined` gRPC API. It can also validate/generate installer configs using the local `machineconfig` crate.

## Why a Rust webapp for the UI?

Comprehensive research and rationale:

- Server framework: Axum (Tokio-based, modular, excellent ergonomics, widespread adoption). Alternatives considered:
  - Actix Web: High performance, mature; Axum was preferred for simpler tower-based middleware and alignment with Tokio and tonic client patterns.
  - Rocket: Batteries-included but heavier, and less commonly paired with tonic gRPC clients.
- Static assets: `tower-http::ServeDir` for serving a simple HTML interface. Alternatives:
  - SPA built with React/Vite or Svelte and reverse-proxied. Heavier toolchain, but viable for richer UIs.
  - Full Rust/WASM UIs (Yew, Leptos, Dioxus): Compelling but adds WASM toolchain and SSR complexity. For a minimal installer UI, server-rendered/basic static page is sufficient.
- gRPC client: `tonic` is the de facto Rust gRPC implementation and already used by `machined`. Reusing the same `.proto` ensures schema parity. Alternatives:
  - REST translation layer (grpc-gateway). Overhead and extra code; not needed as we can call gRPC directly from the server.
- Config generation: Reuse the local `machineconfig` library to parse/validate installer KDL and summarize output. This ensures consistent validation logic with the installer.
- Security/auth:
  - `machined` uses a claim workflow returning a JWT. The UI server provides an HTTP endpoint to claim, then attaches the token to gRPC metadata as `Authorization` when needed.
  - For production, deploy behind TLS (e.g., terminated by a reverse proxy like Nginx/Traefik/Caddy) and restrict network exposure. Consider CSRF and CORS if enabling cross-origin usage.
- Streaming/long-running ops:
  - `Install` is a server stream in gRPC. The UI server could expose Server-Sent Events (SSE) or WebSocket to stream progress to the browser by bridging from gRPC stream. The current MVP omits this but leaves a clear path to add it.

## Features (MVP)

- Serves a static index page at `/` with basic controls, including a Pool Setup section to compose vdevs from available devices (defaults to disks). Includes a “Generate Pool KDL” action that produces a KDL snippet for the configured pool.
- Multi-machine HTTP API:
  - `GET /health` → `{ "status": "ok" }`
  - `GET /api/machines` → list registered machines `[ { id, endpoint, name?, has_token } ]`
  - `POST /api/machines` with `{ endpoint: string, name?: string }` → `{ id: string }` (adds a machine to the registry)
  - `POST /api/machines/{id}/claim` with `{ claim_password?: string, claim_payload?: string }` → `{ claim_token: string }` (also stored server-side for subsequent calls)
  - `GET /api/machines/{id}/system-info?token=...` → Protobuf-encoded bytes of `SystemInfoResponse` (content-type `application/octet-stream`). If `token` is omitted, the stored token is used if present.
  - `GET /api/machines/{id}/storage?include_partitions=0|1` → JSON `{ disks: [...], partitions: [...] }` (partitions may be empty; disks are returned by default).
  - Backward-compat: `POST /api/claim` and `GET /api/system-info` operate only when exactly one machine is registered; otherwise they return a 400 with guidance.
  - `POST /api/generate-config` with `{ filename?: string, content: string }` → summary JSON of parsed config.
  - `POST /api/pool/config` with `{ pool: { name: string, vdevs: [ { type: string, devices: string[] } ] } }` → `{ kdl: string, warnings: string[] }`. Note: `type: "stripe"` isn’t a native machineconfig vdev; it’s mapped with warnings (<=1 device → `mirror`, >1 → `raidz`).

Note: `SystemInfoResponse` is returned as protobuf bytes since the prost-generated types don’t implement `serde::Serialize`. A future extension can map the fields into a serializable DTO.

## Configuration

- `UI_BIND` (default `127.0.0.1:8080`) – Address for the HTTP server.
- Machines are added at runtime via the API/UI (e.g., `POST /api/machines`).

## Building and running

```bash
cargo build --manifest-path installer-ui/Cargo.toml
UI_BIND=127.0.0.1:8080 cargo run --manifest-path installer-ui/Cargo.toml
```

Visit http://127.0.0.1:8080/ to use the demo UI.

## Future roadmap

- Add an endpoint to call `Install` and bridge the gRPC progress stream to the browser via SSE/WebSocket.
- Provide JSON-friendly DTOs for system info instead of raw protobuf bytes.
- Add templating (e.g., Askama or Tera) or integrate a richer SPA if needed.
- Harden auth/session handling and add TLS.
- Package the UI as a service and integrate with the overall installer deployment.


## OpenAPI

- `GET /api/openapi.json` → OpenAPI JSON specification generated from source annotations using the `utoipa` crate.

Example:

```bash
# Run the server, then fetch the OpenAPI spec as JSON
UI_BIND=127.0.0.1:8080 cargo run --manifest-path installer-ui/Cargo.toml
curl -s http://127.0.0.1:8080/api/openapi.json | jq .
```
