//! Command-line interface configuration.

use argh::FromArgs;
use std::{net::SocketAddr, path::PathBuf};

/// A high-performance reverse proxy server
#[derive(Debug, FromArgs)]
pub struct Cli {
    /// path to static files directory (e.g. 'dist/')
    #[argh(option, long = "static-dir")]
    pub static_dir: PathBuf,

    /// backend API address (e.g. '127.0.0.1:8081')
    #[argh(option)]
    pub api: String,

    /// API path prefix (default: '/pz')
    #[argh(option, long = "api-path", default = "String::from(\"/pz\")")]
    pub api_path: String,

    /// server bind address (default: '127.0.0.1:8000')
    #[argh(option, default = "\"127.0.0.1:8000\".parse().unwrap()")]
    pub bind: SocketAddr,
}
