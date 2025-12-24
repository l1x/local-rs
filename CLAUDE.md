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

Single-file design (`src/main.rs`) with these key components:

1. **Color Palette** (lines 25-77): 32 ANSI colors for request ID visualization with deterministic hash-based assignment

2. **CLI Configuration** (lines 95-136): Clap-based argument parsing
   - Required: `--static-dir`, `--api`
   - Optional: `--api-path` (default: `/pz`), `--bind` (default: `127.0.0.1:8000`)

3. **AppState** (lines 138-147): Thread-safe shared state via `Arc<AppState>`

4. **Request Logging Middleware** (lines 149-167): Generates colored nanoid per request, tracks latency

5. **Static File Handler** (lines 169-222): Serves files with auto `index.html` and MIME detection

6. **API Proxy Handler** (lines 224-313): Full reverse proxy with:
   - Hop-by-hop header filtering
   - Query parameter preservation
   - Streaming request/response bodies
   - Dual latency tracking (proxy + total)

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

- **axum** 0.8: Web framework and routing
- **tokio**: Async runtime
- **reqwest**: HTTP client for proxy requests
- **clap**: CLI argument parsing
- **tracing**: Structured logging
- **nanoid**: Short unique request IDs
- **mime_guess**: Content-Type detection

## Linting

Strict Clippy enforcement: all warnings treated as errors. Run `mise run lint` before committing.
