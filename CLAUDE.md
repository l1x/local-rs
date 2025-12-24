# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

local-rs is a Rust reverse proxy server built with Axum that serves static files and proxies API requests to a backend server. It features color-coded request tracing with unique request IDs.

## Build Commands

This project uses mise for task management (Rust 1.90.0).

```bash
# Build (development)
mise run build-dev

# Build (release)
mise run build-prod

# Lint (fails on warnings)
mise run lint

# Run tests
mise run tests

# Run (requires arguments)
cargo run -- --static-dir dist/ --api 127.0.0.1:8081 --api-path /api

# Format code
cargo fmt
```

## CLI Arguments

- `--static-dir <DIR>` - Directory containing static files (required)
- `--api <ADDR>` - Backend API address, e.g., `127.0.0.1:8081` (required)
- `--api-path <PATH>` - Path prefix for API requests (default: `/pz`)
- `--bind <ADDR>` - Server bind address (default: `127.0.0.1:8000`)

## Architecture

The application is a single-file Rust program (`src/main.rs`) using:

- **Axum** for HTTP routing and middleware
- **reqwest** for proxying requests to the backend API
- **tower-http** for HTTP utilities
- **tracing** for structured logging with color-coded output

### Request Flow

1. All requests pass through `log_requests` middleware which assigns a unique nanoid and records start time
2. Requests matching `{api_path}/*` are handled by `proxy_api` which forwards to the backend
3. All other requests fall through to `serve_static` which serves files from `static_dir`

### Key Components

- `AppState` - Shared state containing API base URL, API path prefix, and static directory
- `colored_id()` - Generates ANSI-colored request IDs for log readability
- `proxy_api()` - Handles API proxying with header filtering and response streaming
- `serve_static()` - Serves static files with MIME type detection
