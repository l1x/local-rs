# CLAUDE.md

This file provides guidance to agentic coding tools when working in this repository.

## Project Overview

local-rs is a high-performance reverse proxy server in Rust that combines static file serving with API proxying. It's designed for local development workflows where you want a single server to serve frontend assets and proxy API requests to a backend.

## Build & Development Commands

```bash
# Using mise (preferred)
mise run build-dev          # Debug build
mise run build-prod         # Release build
mise run lint               # Clippy with strict warnings (-D warnings)
mise run tests              # Run tests with output capture

# Using cargo directly
cargo build                 # Debug build
cargo build --release       # Release build
cargo clippy -- -D warnings # Lint (warnings = errors)
cargo test -- --nocapture   # Run tests
```

## Running the Server

```bash
./target/release/local-rs \
  --static-dir ./dist \
  --api 127.0.0.1:8081 \
  --api-path /api \
  --bind 127.0.0.1:8000
```

## Architecture

The project is structured into several modules:

1. **`src/main.rs`**: Entry point, initializes tracing, sets up the Axum router and starts the server.
2. **`src/cli.rs`**: Defines the `Cli` struct for argument parsing using `argh`.
3. **`src/state.rs`**: Defines the `AppState` shared state.
4. **`src/colors.rs`**: Implements deterministic colored IDs for requests.
5. **`src/middleware.rs`**: Request logging middleware that assigns IDs and tracks latency.
6. **`src/handlers.rs`**: Contains the core logic for:
   - `serve_static`: Serves files with auto `index.html` and MIME detection.
   - `proxy_api`: Full reverse proxy with hop-by-hop header filtering, query preservation, and streaming bodies.

## Request Flow

```
Incoming Request
    ↓
log_requests() middleware (assign ID, start timer)
    ↓
Router decision:
  - Path starts with {api_path}/* → proxy_api()
  - All other paths → serve_static() (fallback)
    ↓
Response logged with latency
```

## Key Dependencies

- **axum**: Web framework and routing
- **tokio**: Async runtime
- **reqwest**: HTTP client for proxy requests
- **argh**: Lightweight CLI argument parsing
- **tracing**: Structured logging
- **nanoid**: Short unique request IDs
- **mime_guess**: Content-Type detection
- **owo-colors**: Terminal styling

## Linting

Strict Clippy enforcement: all warnings treated as errors. Run `mise run lint` before committing.
