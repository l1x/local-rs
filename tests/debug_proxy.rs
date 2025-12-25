//! Simple test to debug proxy behavior

use axum::{
    Router,
    body::Body,
    http::{StatusCode, header},
    middleware as axum_middleware,
    response::Response,
    routing::{any, get},
};
use local_rs::{handlers::proxy_api, state::AppState};
use std::{path::PathBuf, sync::Arc};
use tokio::time::{Duration, sleep};

#[tokio::test]
async fn test_simple_proxy() {
    println!("Starting simple proxy test");

    // Create a very simple backend
    let backend_app = Router::new().route(
        "/api/test",
        get(|| async {
            println!("Backend received request");
            let mut response = Response::new(Body::from("Backend response"));
            response.headers_mut().insert(
                "content-type",
                header::HeaderValue::from_static("application/json"),
            );
            response
        }),
    );

    let backend_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let backend_addr = backend_listener.local_addr().unwrap();
    println!("Backend server starting on: {}", backend_addr);

    tokio::spawn(async move {
        axum::serve(backend_listener, backend_app).await.unwrap();
    });

    // Create static directory
    let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_static");
    tokio::fs::create_dir_all(&static_dir).await.unwrap();

    let api_path = "/api".to_string();
    let state = Arc::new(AppState {
        api_base_url: format!("http://{}", backend_addr),
        api_path: api_path.trim_end_matches('/').to_string(),
        static_dir: static_dir.clone(),
    });

    println!("Creating proxy app");
    let proxy_app = Router::new()
        .route("/api/{*path}", any(proxy_api)) // Use wildcard syntax
        .layer(axum_middleware::from_fn(
            |mut req: axum::http::Request<Body>, next: axum::middleware::Next| async {
                // Simple mock of the middleware for testing
                use std::time::Instant;
                req.extensions_mut().insert("test-id".to_string());
                req.extensions_mut().insert(Instant::now());
                next.run(req).await
            },
        ))
        .with_state(state);

    let proxy_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();
    println!("Proxy server starting on: {}", proxy_addr);

    tokio::spawn(async move {
        axum::serve(proxy_listener, proxy_app).await.unwrap();
    });

    // Give servers a moment to start
    sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let url = format!("http://{}/api/test", proxy_addr);
    println!("Making request to: {}", url);

    let response = client.get(&url).send().await.unwrap();

    let status = response.status();
    let headers = response.headers().clone();

    println!("Response status: {}", status);
    println!("Response headers: {:?}", headers);

    let body = response.text().await.unwrap();
    println!("Response body: {}", body);

    // This might be failing but let's see what we get
    if status == StatusCode::OK {
        println!("✅ Test passed");
    } else {
        println!("❌ Test failed, got status: {}", status);
        panic!("Expected OK, got {}", status);
    }
}
