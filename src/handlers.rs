//! HTTP request handlers.

use axum::{
    body::{Body, Bytes},
    extract::{Extension, Path, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header},
    response::Response,
};
use owo_colors::OwoColorize;
use std::{sync::Arc, time::Instant};
use tokio::fs;
use tracing::info;

use crate::colors::colored_id;
use crate::state::AppState;

/// Handles static file requests with proper content-type detection and logging
///
/// Implements several key behaviors:
/// - Automatic index.html serving for directory requests
/// - Correct MIME type detection using file extension
/// - Detailed latency tracking from request start
/// - Color-coded logging with consistent request IDs
pub async fn serve_static(
    State(state): State<Arc<AppState>>,
    Extension(id): Extension<String>,
    Extension(start_time): Extension<Instant>,
    uri: Uri,
) -> Result<Response, StatusCode> {
    let path = uri.path().trim_start_matches('/');
    let mut file_path = state.static_dir.join(path);

    if file_path.is_dir() {
        file_path.push("index.html");
    }

    match fs::read(&file_path).await {
        Ok(content) => {
            let mime_type = mime_guess::from_path(&file_path).first_or_octet_stream();
            let mut response = Response::new(Body::from(content));
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime_type.as_ref()).unwrap(),
            );

            let latency = start_time.elapsed();
            info!(
                "{} ← {} {} ({}ms)",
                colored_id(&id),
                "STATIC".green(),
                response.status(),
                latency.as_millis()
            );
            Ok(response)
        }
        Err(_) => {
            let latency = start_time.elapsed();
            info!(
                "{} ← {} {} ({}ms)",
                colored_id(&id),
                "STATIC".green(),
                StatusCode::NOT_FOUND,
                latency.as_millis()
            );
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// Proxies API requests to the backend with full headers/body passthrough
///
/// This handler implements sophisticated proxy behavior including:
/// - Header filtering (removing hop-by-hop headers)
/// - Query parameter preservation
/// - Dual latency tracking (proxy time and total time)
/// - Error handling with proper status codes
/// - Streaming response bodies for efficiency
#[allow(clippy::too_many_arguments)]
pub async fn proxy_api(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    Extension(id): Extension<String>,
    Extension(start_time): Extension<Instant>,
    method: Method,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Result<Response, StatusCode> {
    let client = reqwest::Client::new();
    let api_url = format!(
        "{}{}/{}",
        state.api_base_url,
        state.api_path,
        path.trim_start_matches('/')
    );

    let full_url = uri
        .query()
        .map_or(api_url.clone(), |query| format!("{}?{}", api_url, query));

    let mut filtered_headers = HeaderMap::new();
    for (key, value) in headers.iter() {
        if !matches!(
            key.as_str(),
            "host" | "accept-encoding" | "connection" | "keep-alive"
        ) {
            filtered_headers.insert(key.clone(), value.clone());
        }
    }

    info!("{} → {} {}", colored_id(&id), "API".yellow(), full_url);
    let proxy_start_time = Instant::now();

    let response = client
        .request(method.clone(), &full_url)
        .headers(filtered_headers)
        .body(body)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("API request failed: {}", e);
            StatusCode::BAD_GATEWAY
        })?;

    let proxy_latency = proxy_start_time.elapsed();
    info!(
        "{} ← {} {} ({}ms)",
        colored_id(&id),
        "API".yellow(),
        response.status(),
        proxy_latency.as_millis()
    );

    let mut builder = Response::builder().status(response.status());
    for (key, value) in response.headers().iter() {
        if !matches!(
            key.as_str(),
            "transfer-encoding" | "content-encoding" | "connection" | "keep-alive"
        ) {
            builder = builder.header(key, value);
        }
    }

    let total_latency = start_time.elapsed();
    info!(
        "{} ← {} {} ({}ms)",
        colored_id(&id),
        method,
        response.status(),
        total_latency.as_millis()
    );

    builder
        .body(Body::from_stream(response.bytes_stream()))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
