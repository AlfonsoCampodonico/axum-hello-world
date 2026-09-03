//! A hello-world HTTP API on axum, sized to demonstrate deploying Rust on
//! Laravel Cloud. The router lives here rather than in `main.rs` so tests can
//! drive it in-process without binding a socket.

mod error;
mod routes;

use axum::{
    Router,
    routing::{get, post},
};

/// Builds the application router. Called once by `main`, and once per test.
pub fn app() -> Router {
    Router::new()
        .route("/", get(routes::hello))
        .route("/hello", get(routes::hello))
        .route("/hello/{name}", get(routes::hello_name))
        .route("/health", get(routes::health))
        .route("/version", get(routes::version))
        .route("/echo", post(routes::echo))
        .route("/openapi.json", get(routes::openapi))
        .route("/docs", get(routes::docs))
        .fallback(routes::not_found)
        .method_not_allowed_fallback(routes::method_not_allowed)
}
