//! Integration tests for proxy behavior

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
    response::Response,
    routing::{any, get},
    Router,
    middleware as axum_middleware,
};
use local_rs::handlers::{proxy_api, serve_static};
use local_rs::middleware::log_requests;
use local_rs::state::AppState;
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn test_proxy_backend_unavailable() {
    // Use a non-existent backend address
    let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_static");
    tokio::fs::create_dir_all(&static_dir).await.unwrap();
    
    let state = Arc::new(AppState {
        api_base_url: "http://127.0.0.1:99999".to_string(), // Non-existent port
        api_path: "/api".to_string(),
        static_dir: static_dir.clone(),
    });
    
    let proxy_app = Router::new()
        .route("/api/{*path}", any(proxy_api))
        .fallback(get(serve_static))
        .layer(axum_middleware::from_fn(log_requests))
        .with_state(state);

    let proxy_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        axum::serve(proxy_listener, proxy_app).await.unwrap();
    });

    // Give the server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("http://{}/api/test", proxy_addr))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test] 
async fn test_proxy_with_mock_backend() {
    // Create a simple mock backend server
    let backend_app = Router::new()
        .route("/api/test", get(|| async {
            let mut response = Response::new(Body::from("Backend response"));
            response.headers_mut().insert(
                "content-type", 
                header::HeaderValue::from_static("application/json")
            );
            response.headers_mut().insert(
                "x-backend", 
                header::HeaderValue::from_static("test-value")
            );
            response
        }))
        .route("/api/echo", axum::routing::post(|request: Request<Body>| async move {
            let body_bytes = axum::body::to_bytes(request.into_body(), usize::MAX).await.unwrap();
            let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
            
            let mut response = Response::new(Body::from(format!("Echo: {}", body_str)));
            response.headers_mut().insert(
                "content-type", 
                header::HeaderValue::from_static("text/plain")
            );
            response
        }));

    let backend_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let backend_addr = backend_listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        axum::serve(backend_listener, backend_app).await.unwrap();
    });

    // Create static directory and proxy app
    let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_static");
    tokio::fs::create_dir_all(&static_dir).await.unwrap();
    
    let state = Arc::new(AppState {
        api_base_url: format!("http://{}", backend_addr),
        api_path: "/api".to_string(),
        static_dir: static_dir.clone(),
    });
    
    let proxy_app = Router::new()
        .route("/api/{*path}", any(proxy_api))
        .fallback(get(serve_static))
        .layer(axum_middleware::from_fn(log_requests))
        .with_state(state);

    let proxy_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        axum::serve(proxy_listener, proxy_app).await.unwrap();
    });

    // Give servers a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();

    // Test GET request
    let response = client
        .get(&format!("http://{}/api/test", proxy_addr))
        .header("x-custom", "test-value")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("content-type").unwrap(), "application/json");
    assert_eq!(response.headers().get("x-backend").unwrap(), "test-value");
    assert_eq!(response.text().await.unwrap(), "Backend response");

    // Test POST request with body
    let request_body = "{\"name\": \"test\", \"value\": 123}";
    let response = client
        .post(&format!("http://{}/api/echo", proxy_addr))
        .body(request_body)
        .header("content-type", "application/json")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.text().await.unwrap(), format!("Echo: {}", request_body));
}

#[tokio::test]
async fn test_proxy_query_parameters() {
    let backend_app = Router::new()
        .route("/api/search", get(|request: Request<Body>| async move {
            let query_string = request.uri().query().unwrap_or("");
            let mut response = Response::new(Body::from(format!("Query: {}", query_string)));
            response.headers_mut().insert(
                "content-type", 
                header::HeaderValue::from_static("text/plain")
            );
            response
        }));

    let backend_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let backend_addr = backend_listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        axum::serve(backend_listener, backend_app).await.unwrap();
    });

    let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_static");
    tokio::fs::create_dir_all(&static_dir).await.unwrap();
    
    let state = Arc::new(AppState {
        api_base_url: format!("http://{}", backend_addr),
        api_path: "/api".to_string(),
        static_dir: static_dir.clone(),
    });
    
    let proxy_app = Router::new()
        .route("/api/{*path}", any(proxy_api))
        .fallback(get(serve_static))
        .layer(axum_middleware::from_fn(log_requests))
        .with_state(state);

    let proxy_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        axum::serve(proxy_listener, proxy_app).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("http://{}/api/search?q=test&page=2&limit=10", proxy_addr))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let response_text = response.text().await.unwrap();
    assert!(response_text.contains("q=test"));
    assert!(response_text.contains("page=2"));
    assert!(response_text.contains("limit=10"));
}

#[tokio::test]
async fn test_proxy_header_filtering() {
    let backend_app = Router::new()
        .route("/api/headers", get(|request: Request<Body>| async move {
            // Echo back all headers we received
            let mut response = Response::new(Body::from("Headers received"));
            for (name, value) in request.headers().iter() {
                response.headers_mut().insert(name.clone(), value.clone());
            }
            response
        }));

    let backend_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let backend_addr = backend_listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        axum::serve(backend_listener, backend_app).await.unwrap();
    });

    let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_static");
    tokio::fs::create_dir_all(&static_dir).await.unwrap();
    
    let state = Arc::new(AppState {
        api_base_url: format!("http://{}", backend_addr),
        api_path: "/api".to_string(),
        static_dir: static_dir.clone(),
    });
    
    let proxy_app = Router::new()
        .route("/api/{*path}", any(proxy_api))
        .fallback(get(serve_static))
        .layer(axum_middleware::from_fn(log_requests))
        .with_state(state);

    let proxy_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        axum::serve(proxy_listener, proxy_app).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("http://{}/api/headers", proxy_addr))
        .header("host", "should-be-filtered.com")
        .header("connection", "keep-alive")
        .header("accept-encoding", "gzip")
        .header("x-custom", "should-preserve")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    // Hop-by-hop headers should be filtered out (except those re-added by reqwest like host)
    assert!(response.headers().get("connection").is_none());
    assert!(response.headers().get("accept-encoding").is_none());
    
    // Custom headers should be preserved
    assert_eq!(response.headers().get("x-custom").unwrap(), "should-preserve");
}

#[tokio::test]
async fn test_proxy_error_propagation() {
    let backend_app = Router::new()
        .route("/api/error", get(|| async {
            let mut response = Response::new(Body::from("Backend error"));
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            response.headers_mut().insert(
                "x-error", 
                header::HeaderValue::from_static("backend-failure")
            );
            response
        }));

    let backend_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let backend_addr = backend_listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        axum::serve(backend_listener, backend_app).await.unwrap();
    });

    let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_static");
    tokio::fs::create_dir_all(&static_dir).await.unwrap();
    
    let state = Arc::new(AppState {
        api_base_url: format!("http://{}", backend_addr),
        api_path: "/api".to_string(),
        static_dir: static_dir.clone(),
    });
    
    let proxy_app = Router::new()
        .route("/api/{*path}", any(proxy_api))
        .fallback(get(serve_static))
        .layer(axum_middleware::from_fn(log_requests))
        .with_state(state);

    let proxy_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        axum::serve(proxy_listener, proxy_app).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("http://{}/api/error", proxy_addr))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(response.headers().get("x-error").unwrap(), "backend-failure");
    assert_eq!(response.text().await.unwrap(), "Backend error");
}