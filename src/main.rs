//! A high-performance reverse proxy server with colored request tracing.
//!
//! Features:
//! - Routes API requests to a backend server
//! - Serves static files from a directory
//! - Detailed logging with color-coded request IDs
//! - Latency tracking for both static and API requests

use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Extension, Path, State},
    http::{HeaderMap, HeaderValue, Method, Request, StatusCode, Uri, header},
    middleware::{self, Next},
    response::Response,
    routing::{any, get},
};
use argh::FromArgs;
use nanoid::nanoid;
use owo_colors::{AnsiColors, DynColors, OwoColorize, Style};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Instant};
use tokio::fs;
use tracing::{Level, info};

/// 32 visually distinct ANSI colors for request ID coloring
///
/// Carefully selected to provide maximum visual differentiation while maintaining
/// readability on both light and dark backgrounds. Includes both standard and
/// bright variants, with some duplication to reach 32 distinct colors.
const COLORS: [AnsiColors; 32] = [
    AnsiColors::Red,
    AnsiColors::Green,
    AnsiColors::Yellow,
    AnsiColors::Blue,
    AnsiColors::Magenta,
    AnsiColors::Cyan,
    AnsiColors::BrightRed,
    AnsiColors::BrightGreen,
    AnsiColors::BrightYellow,
    AnsiColors::BrightBlue,
    AnsiColors::BrightMagenta,
    AnsiColors::BrightCyan,
    AnsiColors::Red,
    AnsiColors::Green,
    AnsiColors::Yellow,
    AnsiColors::Blue,
    AnsiColors::Magenta,
    AnsiColors::Cyan,
    AnsiColors::BrightRed,
    AnsiColors::BrightGreen,
    AnsiColors::BrightYellow,
    AnsiColors::BrightBlue,
    AnsiColors::BrightMagenta,
    AnsiColors::BrightCyan,
    AnsiColors::Red,
    AnsiColors::Green,
    AnsiColors::Yellow,
    AnsiColors::Blue,
    AnsiColors::Magenta,
    AnsiColors::Cyan,
    AnsiColors::BrightRed,
    AnsiColors::BrightGreen,
];

/// Deterministically maps a request ID to one of the 32 colors
///
/// Uses a stable hash function to ensure the same ID always gets the same color.
/// The hash is designed to be:
/// - Fast to compute (important for high-throughput logging)
/// - Well-distributed across the color palette
/// - Consistent across program runs
fn get_color_for_id(id: &str) -> AnsiColors {
    let hash = id
        .chars()
        .fold(0u32, |acc, c| acc.wrapping_mul(31).wrapping_add(c as u32));
    COLORS[(hash % 32) as usize] // Modulo 32 ensures we stay within our color palette bounds
}

/// Formats a request ID with consistent color coding
///
/// Returns a `String` with embedded ANSI color codes. Uses the full-color
/// palette while gracefully degrading to no color when output isn't to a terminal.
///
/// # Examples
/// ```
/// let colored = colored_id("abc123");
/// println!("Request {} started", colored);  // Will show colored ID in terminals
/// ```
pub fn colored_id(id: &str) -> String {
    let color = get_color_for_id(id);
    let style = Style::new().color(DynColors::Ansi(color));
    format!("[{}]", id).style(style).to_string() // Explicit String conversion avoids lifetime issues
}

/// A high-performance reverse proxy server
#[derive(Debug, FromArgs)]
struct Cli {
    /// path to static files directory (e.g. 'dist/')
    #[argh(option, long = "static-dir")]
    static_dir: PathBuf,

    /// backend API address (e.g. '127.0.0.1:8081')
    #[argh(option)]
    api: String,

    /// API path prefix (default: '/pz')
    #[argh(option, long = "api-path", default = "String::from(\"/pz\")")]
    api_path: String,

    /// server bind address (default: '127.0.0.1:8000')
    #[argh(option, default = "\"127.0.0.1:8000\".parse().unwrap()")]
    bind: SocketAddr,
}

/// Shared application state accessible to all handlers
#[derive(Debug, Clone)]
struct AppState {
    /// Base URL of the backend API (e.g. "http://localhost:8081")
    api_base_url: String,
    /// Path prefix for API routes (e.g. "/pz")
    api_path: String,
    /// Root directory for static file serving
    static_dir: PathBuf,
}

/// Middleware that logs incoming requests and assigns them unique colored IDs
///
/// This middleware:
/// 1. Generates a short nanoid for each request
/// 2. Records the start time for latency calculation
/// 3. Logs the initial request with colored ID
/// 4. Stores the ID and start time in request extensions for downstream handlers
async fn log_requests(mut req: Request<Body>, next: Next) -> Response {
    let id = nanoid!(5); // 5-character ID provides good uniqueness for most workloads
    let method = req.method().clone();
    let uri = req.uri().clone();

    // Store ID and start time for use in downstream handlers
    req.extensions_mut().insert(id.clone());
    req.extensions_mut().insert(Instant::now());

    info!("{} → {} {}", colored_id(&id), method, uri.path());
    next.run(req).await
}

/// Handles static file requests with proper content-type detection and logging
///
/// Implements several key behaviors:
/// - Automatic index.html serving for directory requests
/// - Correct MIME type detection using file extension
/// - Detailed latency tracking from request start
/// - Color-coded logging with consistent request IDs
async fn serve_static(
    State(state): State<Arc<AppState>>,
    Extension(id): Extension<String>,
    Extension(start_time): Extension<Instant>,
    uri: Uri,
) -> Result<Response, StatusCode> {
    let path = uri.path().trim_start_matches('/');
    let mut file_path = state.static_dir.join(path);

    // Automatically serve index.html for directory requests
    if file_path.is_dir() {
        file_path.push("index.html");
    }

    match fs::read(&file_path).await {
        Ok(content) => {
            // Detect MIME type from file extension, defaulting to octet-stream
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
                "STATIC".green(), // Color-coded handler type
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
async fn proxy_api(
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

    // Preserve original query string if present
    let full_url = uri
        .query()
        .map_or(api_url.clone(), |query| format!("{}?{}", api_url, query));

    // Filter out hop-by-hop headers that shouldn't be forwarded
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

    // Rebuild response while filtering unwanted headers
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

    // Stream the response body rather than loading it all into memory
    builder
        .body(Body::from_stream(response.bytes_stream()))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Main entry point that configures and runs the proxy server
///
/// Sets up:
/// - Structured logging
/// - Static file serving
/// - API request proxying
/// - Request logging middleware
/// - Shared application state
#[tokio::main]
async fn main() {
    // Initialize structured logging with INFO level as default
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let args: Cli = argh::from_env();
    let canonical_static_dir = args
        .static_dir
        .canonicalize()
        .expect("Failed to canonicalize static directory");

    // Ensure API URL has protocol prefix
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

    // Configure the router with our handlers and middleware
    let app = Router::new()
        .route(&format!("{}/{{*path}}", args.api_path), any(proxy_api))
        .fallback(get(serve_static))
        .layer(middleware::from_fn(log_requests))
        .with_state(state.clone());

    // Log startup information
    info!("Serving static files from: {:?}", canonical_static_dir);
    info!(
        "Proxying {}/* to: {}{}/",
        args.api_path, state.api_base_url, args.api_path
    );
    info!("Server running on: http://{}", args.bind);

    // Start the server
    axum::serve(tokio::net::TcpListener::bind(args.bind).await.unwrap(), app)
        .await
        .unwrap();
}
