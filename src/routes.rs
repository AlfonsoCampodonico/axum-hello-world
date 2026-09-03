//! Request handlers. Each one is a plain async function: axum extracts what it
//! declares and turns what it returns into a response.

use axum::{
    Json,
    extract::{Path, rejection::JsonRejection},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use serde::Serialize;
use serde_json::{Value, json};

use crate::error::ApiError;

/// Bump alongside the `axum` requirement in Cargo.toml.
const FRAMEWORK: &str = "axum 0.8";

/// The longest name `/hello/{name}` will greet, in characters.
const MAX_NAME_LEN: usize = 64;

const OPENAPI_SPEC: &str = include_str!("../assets/openapi.json");
const DOCS_PAGE: &str = include_str!("../assets/docs.html");

#[derive(Serialize)]
pub struct Greeting {
    message: String,
}

impl Greeting {
    fn for_name(name: &str) -> Self {
        Self {
            message: format!("Hello, {name}!"),
        }
    }
}

/// `GET /` and `GET /hello`
pub async fn hello() -> Json<Greeting> {
    Json(Greeting::for_name("World"))
}

/// `GET /hello/{name}`
pub async fn hello_name(Path(name): Path<String>) -> Result<Json<Greeting>, ApiError> {
    let name = name.trim();

    if name.is_empty() {
        return Err(ApiError::bad_request("name must not be blank"));
    }

    if name.chars().count() > MAX_NAME_LEN {
        return Err(ApiError::bad_request(format!(
            "name must be at most {MAX_NAME_LEN} characters"
        )));
    }

    Ok(Json(Greeting::for_name(name)))
}

/// `GET /health` — what the platform's probe hits.
pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

/// `GET /version`
pub async fn version() -> Json<Value> {
    Json(json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "framework": FRAMEWORK,
    }))
}

/// `POST /echo` — takes the rejection by hand so a malformed body gets the same
/// error envelope as everything else instead of axum's plain-text default.
pub async fn echo(payload: Result<Json<Value>, JsonRejection>) -> Result<Json<Value>, ApiError> {
    match payload {
        Ok(Json(body)) => Ok(Json(json!({ "echo": body }))),
        Err(rejection) => Err(ApiError::new(rejection.status(), rejection.body_text())),
    }
}

/// `GET /openapi.json` — the spec is compiled into the binary, so there is no
/// file to ship next to it.
pub async fn openapi() -> Response {
    ([(header::CONTENT_TYPE, "application/json")], OPENAPI_SPEC).into_response()
}

/// `GET /docs`
pub async fn docs() -> Html<&'static str> {
    Html(DOCS_PAGE)
}

pub async fn not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "Not Found")
}

pub async fn method_not_allowed() -> ApiError {
    ApiError::new(StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed")
}
