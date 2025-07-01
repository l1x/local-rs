use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Extension, Path, State},
    http::{HeaderMap, HeaderValue, Method, Request, StatusCode, Uri, header},
    middleware::{self, Next},
    response::Response,
    routing::{any, get},
};
use clap::Parser;
use nanoid::nanoid;
use owo_colors::{AnsiColors, DynColors, OwoColorize, Style};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Instant};
use tokio::fs;
use tracing::{Level, info};

// 32 distinct ANSI colors for ID coloring
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

/// Deterministically maps an ID to one of our 32 colors
fn get_color_for_id(id: &str) -> AnsiColors {
    let hash = id.chars().fold(0u32, |acc, c| acc.wrapping_mul(31).wrapping_add(c as u32));
    COLORS[(hash % 32) as usize]
}

/// Formats ID with consistent color
pub fn colored_id(id: &str) -> String {
    let color = get_color_for_id(id);
    let style = Style::new().color(DynColors::Ansi(color));
    format!("[{}]", id).style(style).to_string()
}

#[derive(Debug, Parser)]
#[command(name = "proxy-server", version, about)]
struct Cli {
    #[arg(long, value_name = "DIR")]
    static_dir: PathBuf,
    #[arg(long, value_name = "ADDR")]
    api: String,
    #[arg(long, default_value = "/pz", value_name = "PATH")]
    api_path: String,
    #[arg(long, default_value = "127.0.0.1:8000", value_name = "ADDR")]
    bind: SocketAddr,
}

#[derive(Debug, Clone)]
struct AppState {
    api_base_url: String,
    api_path: String,
    static_dir: PathBuf,
}

async fn log_requests(mut req: Request<Body>, next: Next) -> Response {
    let id = nanoid!(5);
    let method = req.method().clone();
    let uri = req.uri().clone();

    req.extensions_mut().insert(id.clone());
    req.extensions_mut().insert(Instant::now());

    info!("{} → {} {}", colored_id(&id), method, uri.path());
    next.run(req).await
}

async fn serve_static(
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
    let api_url = format!("{}{}/{}", state.api_base_url, state.api_path, path.trim_start_matches('/'));

    let full_url = uri.query().map_or(api_url.clone(), |query| format!("{}?{}", api_url, query));

    let mut filtered_headers = HeaderMap::new();
    for (key, value) in headers.iter() {
        if !matches!(key.as_str(), "host" | "accept-encoding" | "connection" | "keep-alive") {
            filtered_headers.insert(key.clone(), value.clone());
        }
    }

    info!("{} → {} {}", colored_id(&id), "API".yellow(), full_url);
    let proxy_start_time = Instant::now();

    let response = client.request(method.clone(), &full_url)
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
        if !matches!(key.as_str(), "transfer-encoding" | "content-encoding" | "connection" | "keep-alive") {
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

    builder.body(Body::from_stream(response.bytes_stream()))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let args = Cli::parse();
    let canonical_static_dir = args.static_dir.canonicalize()
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
        .layer(middleware::from_fn(log_requests))
        .with_state(state.clone());

    info!("Serving static files from: {:?}", canonical_static_dir);
    info!("Proxying {}/* to: {}{}/", args.api_path, state.api_base_url, args.api_path);
    info!("Server running on: http://{}", args.bind);

    axum::serve(
        tokio::net::TcpListener::bind(args.bind).await.unwrap(),
        app
    ).await.unwrap();
}