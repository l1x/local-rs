//! Shared application state.

use std::path::PathBuf;

/// Shared application state accessible to all handlers
#[derive(Debug, Clone)]
pub struct AppState {
    /// Base URL of the backend API (e.g. "http://localhost:8081")
    pub api_base_url: String,
    /// Path prefix for API routes (e.g. "/pz")
    pub api_path: String,
    /// Root directory for static file serving
    pub static_dir: PathBuf,
}
