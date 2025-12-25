//! Request logging middleware.

use axum::{body::Body, http::Request, middleware::Next, response::Response};
use nanoid::nanoid;
use std::time::Instant;
use tracing::info;

use crate::colors::colored_id;

/// Middleware that logs incoming requests and assigns them unique colored IDs
///
/// This middleware:
/// 1. Generates a short nanoid for each request
/// 2. Records the start time for latency calculation
/// 3. Logs the initial request with colored ID
/// 4. Stores the ID and start time in request extensions for downstream handlers
pub async fn log_requests(mut req: Request<Body>, next: Next) -> Response {
    let id = nanoid!(5);
    let method = req.method().clone();
    let uri = req.uri().clone();

    req.extensions_mut().insert(id.clone());
    req.extensions_mut().insert(Instant::now());

    info!("{} → {} {}", colored_id(&id), method, uri.path());
    next.run(req).await
}
