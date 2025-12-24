//! HTTP request handlers.

use axum::{
    body::{Body, Bytes},
    extract::{Extension, Path, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header},
    response::Response,
};
use owo_colors::OwoColorize;
use std::{path::{Path as FsPath, PathBuf}, sync::Arc, time::Instant};
use tokio::fs;
use tracing::info;

use crate::colors::colored_id;
use crate::state::AppState;

/// Headers that should not be forwarded in proxy requests
const HOP_BY_HOP_REQUEST_HEADERS: &[&str] = &["host", "accept-encoding", "connection", "keep-alive"];

/// Headers that should not be forwarded in proxy responses
const HOP_BY_HOP_RESPONSE_HEADERS: &[&str] =
    &["transfer-encoding", "content-encoding", "connection", "keep-alive"];

/// Resolves a URI path to a file system path, handling index.html fallback
///
/// # Arguments
/// * `static_dir` - The root directory for static files
/// * `uri_path` - The URI path from the request (e.g., "/foo/bar")
///
/// # Returns
/// The resolved file path, with index.html appended for directory paths
pub fn resolve_static_path(static_dir: &FsPath, uri_path: &str) -> PathBuf {
    let path = uri_path.trim_start_matches('/');
    let mut file_path = static_dir.join(path);

    if file_path.is_dir() {
        file_path.push("index.html");
    }

    file_path
}

/// Filters out hop-by-hop headers from request headers
///
/// These headers are connection-specific and should not be forwarded
/// through a proxy to the backend server.
pub fn filter_request_headers(headers: &HeaderMap) -> HeaderMap {
    let mut filtered = HeaderMap::new();
    for (key, value) in headers.iter() {
        if !HOP_BY_HOP_REQUEST_HEADERS.contains(&key.as_str()) {
            filtered.insert(key.clone(), value.clone());
        }
    }
    filtered
}

/// Filters out hop-by-hop headers from response headers
///
/// These headers are connection-specific and should not be forwarded
/// from the backend server to the client.
pub fn filter_response_headers(headers: &HeaderMap) -> HeaderMap {
    let mut filtered = HeaderMap::new();
    for (key, value) in headers.iter() {
        if !HOP_BY_HOP_RESPONSE_HEADERS.contains(&key.as_str()) {
            filtered.insert(key.clone(), value.clone());
        }
    }
    filtered
}

/// Builds the full API URL from components
///
/// # Arguments
/// * `api_base_url` - The base URL (e.g., "http://localhost:8081")
/// * `api_path` - The API path prefix (e.g., "/api")
/// * `request_path` - The path from the request (e.g., "users/123")
/// * `query` - Optional query string
///
/// # Returns
/// The complete URL with query string if present
pub fn build_api_url(api_base_url: &str, api_path: &str, request_path: &str, query: Option<&str>) -> String {
    let base_url = format!(
        "{}{}/{}",
        api_base_url,
        api_path,
        request_path.trim_start_matches('/')
    );

    match query {
        Some(q) => format!("{}?{}", base_url, q),
        None => base_url,
    }
}

/// Handles static file requests with proper content-type detection and logging
pub async fn serve_static(
    State(state): State<Arc<AppState>>,
    Extension(id): Extension<String>,
    Extension(start_time): Extension<Instant>,
    uri: Uri,
) -> Result<Response, StatusCode> {
    let file_path = resolve_static_path(&state.static_dir, uri.path());

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
    let full_url = build_api_url(&state.api_base_url, &state.api_path, &path, uri.query());
    let filtered_headers = filter_request_headers(&headers);

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

    let filtered_response_headers = filter_response_headers(response.headers());
    let mut builder = Response::builder().status(response.status());
    for (key, value) in filtered_response_headers.iter() {
        builder = builder.header(key, value);
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderName;

    #[test]
    fn test_resolve_static_path_simple() {
        let static_dir = PathBuf::from("/var/www");
        let result = resolve_static_path(&static_dir, "/foo/bar.html");
        assert_eq!(result, PathBuf::from("/var/www/foo/bar.html"));
    }

    #[test]
    fn test_resolve_static_path_strips_leading_slash() {
        let static_dir = PathBuf::from("/var/www");
        let result = resolve_static_path(&static_dir, "///multiple/slashes.js");
        assert_eq!(result, PathBuf::from("/var/www//multiple/slashes.js"));
    }

    #[test]
    fn test_resolve_static_path_empty() {
        let static_dir = PathBuf::from("/var/www");
        let result = resolve_static_path(&static_dir, "/");
        // When path is empty, we just get the static dir (index.html added if it's a dir)
        assert_eq!(result, PathBuf::from("/var/www"));
    }

    #[test]
    fn test_filter_request_headers_removes_hop_by_hop() {
        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_static("host"), HeaderValue::from_static("example.com"));
        headers.insert(HeaderName::from_static("connection"), HeaderValue::from_static("keep-alive"));
        headers.insert(HeaderName::from_static("x-custom"), HeaderValue::from_static("value"));
        headers.insert(HeaderName::from_static("accept-encoding"), HeaderValue::from_static("gzip"));

        let filtered = filter_request_headers(&headers);

        assert!(!filtered.contains_key("host"));
        assert!(!filtered.contains_key("connection"));
        assert!(!filtered.contains_key("accept-encoding"));
        assert!(filtered.contains_key("x-custom"));
        assert_eq!(filtered.get("x-custom").unwrap(), "value");
    }

    #[test]
    fn test_filter_response_headers_removes_hop_by_hop() {
        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_static("transfer-encoding"), HeaderValue::from_static("chunked"));
        headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("application/json"));
        headers.insert(HeaderName::from_static("connection"), HeaderValue::from_static("close"));

        let filtered = filter_response_headers(&headers);

        assert!(!filtered.contains_key("transfer-encoding"));
        assert!(!filtered.contains_key("connection"));
        assert!(filtered.contains_key("content-type"));
        assert_eq!(filtered.get("content-type").unwrap(), "application/json");
    }

    #[test]
    fn test_build_api_url_without_query() {
        let url = build_api_url("http://localhost:8081", "/api", "users/123", None);
        assert_eq!(url, "http://localhost:8081/api/users/123");
    }

    #[test]
    fn test_build_api_url_with_query() {
        let url = build_api_url("http://localhost:8081", "/api", "users", Some("page=1&limit=10"));
        assert_eq!(url, "http://localhost:8081/api/users?page=1&limit=10");
    }

    #[test]
    fn test_build_api_url_strips_leading_slash() {
        let url = build_api_url("http://localhost:8081", "/api", "/users/123", None);
        assert_eq!(url, "http://localhost:8081/api/users/123");
    }
}
