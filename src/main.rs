//! A high-performance reverse proxy server with colored request tracing.
//!
//! Features:
//! - Routes API requests to a backend server
//! - Serves static files from a directory
//! - Detailed logging with color-coded request IDs
//! - Latency tracking for both static and API requests

pub mod cli;
pub mod colors;
pub mod handlers;
pub mod middleware;
pub mod state;

use axum::{
    Router, middleware as axum_middleware,
    routing::{any, get},
};
use std::sync::Arc;
use tracing::{Level, info};

use crate::cli::Cli;
use crate::handlers::{proxy_api, serve_static};
use crate::middleware::log_requests;
use crate::state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let args: Cli = argh::from_env();
    let canonical_static_dir = args
        .static_dir
        .canonicalize()
        .expect("Failed to canonicalize static directory");

    let api_base_url = if args.api.starts_with("http") {
        args.api
    } else {
        format!("http://{}", args.api)
    };

    let state = Arc::new(AppState {
        api_base_url,
        api_path: args.api_path.trim_end_matches('/').to_string(),
        static_dir: canonical_static_dir.clone(),
    });

    let app = Router::new()
        .route(&format!("{}/{{*path}}", args.api_path), any(proxy_api))
        .fallback(get(serve_static))
        .layer(axum_middleware::from_fn(log_requests))
        .with_state(state.clone());

    info!("Serving static files from: {:?}", canonical_static_dir);
    info!(
        "Proxying {}/* to: {}{}/",
        args.api_path, state.api_base_url, args.api_path
    );
    info!("Server running on: http://{}", args.bind);

    axum::serve(tokio::net::TcpListener::bind(args.bind).await.unwrap(), app)
        .await
        .unwrap();
}
