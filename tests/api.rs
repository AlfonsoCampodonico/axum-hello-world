//! Integration tests driving the router in-process via `tower::ServiceExt::oneshot`.
//! No socket is bound, so the suite is fast and never races on a port.

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use axum_hello_world::app;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

struct Response {
    status: StatusCode,
    content_type: String,
    body: Vec<u8>,
}

impl Response {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("response body is not valid JSON")
    }

    fn text(&self) -> &str {
        std::str::from_utf8(&self.body).expect("response body is not valid UTF-8")
    }
}

async fn send(request: Request<Body>) -> Response {
    let response = app()
        .oneshot(request)
        .await
        .expect("router is infallible, so this cannot fail");

    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body collects")
        .to_bytes()
        .to_vec();

    Response {
        status,
        content_type,
        body,
    }
}

async fn get(uri: &str) -> Response {
    send(
        Request::builder()
            .uri(uri)
            .body(Body::empty())
            .expect("valid request"),
    )
    .await
}

async fn post_json(uri: &str, body: &str) -> Response {
    send(
        Request::builder()
            .method("POST")
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_owned()))
            .expect("valid request"),
    )
    .await
}

#[tokio::test]
async fn root_greets_the_world() {
    let response = get("/").await;

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.content_type.starts_with("application/json"));
    assert_eq!(response.json(), json!({ "message": "Hello, World!" }));
}

#[tokio::test]
async fn hello_is_an_alias_for_root() {
    assert_eq!(get("/hello").await.json(), get("/").await.json());
}

#[tokio::test]
async fn hello_greets_a_named_visitor() {
    let response = get("/hello/Ferris").await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json(), json!({ "message": "Hello, Ferris!" }));
}

#[tokio::test]
async fn hello_decodes_percent_encoded_names() {
    assert_eq!(
        get("/hello/Ada%20Lovelace").await.json(),
        json!({ "message": "Hello, Ada Lovelace!" })
    );
}

#[tokio::test]
async fn hello_trims_surrounding_whitespace() {
    assert_eq!(
        get("/hello/%20%20Ada%20%20").await.json(),
        json!({ "message": "Hello, Ada!" })
    );
}

#[tokio::test]
async fn hello_rejects_a_blank_name() {
    let response = get("/hello/%20").await;

    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    assert_eq!(response.json()["error"]["status"], 400);
}

#[tokio::test]
async fn hello_rejects_an_overlong_name() {
    let response = get(&format!("/hello/{}", "a".repeat(65))).await;

    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    assert_eq!(response.json()["error"]["status"], 400);
}

#[tokio::test]
async fn hello_accepts_a_name_at_the_length_limit() {
    let response = get(&format!("/hello/{}", "a".repeat(64))).await;

    assert_eq!(response.status, StatusCode::OK);
}

#[tokio::test]
async fn health_reports_ok() {
    let response = get("/health").await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json()["status"], "ok");
}

#[tokio::test]
async fn version_reports_crate_metadata() {
    let response = get("/version").await;
    let body = response.json();

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(body["name"], env!("CARGO_PKG_NAME"));
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
    assert!(
        body["framework"]
            .as_str()
            .is_some_and(|s| s.contains("axum")),
        "expected the framework field to name axum, got {:?}",
        body["framework"]
    );
}

#[tokio::test]
async fn echo_returns_the_posted_json() {
    let response = post_json("/echo", r#"{"hello":["world",42]}"#).await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        response.json(),
        json!({ "echo": { "hello": ["world", 42] } })
    );
}

#[tokio::test]
async fn echo_rejects_malformed_json() {
    let response = post_json("/echo", "{not json").await;

    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    assert_eq!(response.json()["error"]["status"], 400);
}

#[tokio::test]
async fn echo_rejects_a_non_json_content_type() {
    let response = send(
        Request::builder()
            .method("POST")
            .uri("/echo")
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Body::from("hello"))
            .expect("valid request"),
    )
    .await;

    assert_eq!(response.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(response.json()["error"]["status"], 415);
}

#[tokio::test]
async fn openapi_spec_is_served_as_json() {
    let response = get("/openapi.json").await;
    let spec = response.json();

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.content_type.starts_with("application/json"));
    assert!(
        spec["openapi"]
            .as_str()
            .is_some_and(|v| v.starts_with("3.1")),
        "expected an OpenAPI 3.1 spec, got {:?}",
        spec["openapi"]
    );
}

#[tokio::test]
async fn openapi_spec_documents_every_route() {
    let spec = get("/openapi.json").await.json();
    let paths = spec["paths"]
        .as_object()
        .expect("spec has a paths object")
        .keys()
        .cloned()
        .collect::<Vec<_>>();

    for route in [
        "/",
        "/hello",
        "/hello/{name}",
        "/health",
        "/version",
        "/echo",
    ] {
        assert!(
            paths.iter().any(|p| p == route),
            "{route} is missing from the OpenAPI spec (documented: {paths:?})"
        );
    }
}

#[tokio::test]
async fn docs_serve_an_html_page_referencing_the_spec() {
    let response = get("/docs").await;

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.content_type.starts_with("text/html"));
    assert!(response.text().contains("/openapi.json"));
}

#[tokio::test]
async fn unknown_paths_return_a_structured_404() {
    let response = get("/nope").await;

    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_eq!(response.json()["error"]["status"], 404);
    assert!(response.json()["error"]["message"].is_string());
}

#[tokio::test]
async fn wrong_method_returns_405() {
    let response = post_json("/health", "{}").await;

    assert_eq!(response.status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn the_router_can_be_built_more_than_once() {
    // Guards against `app()` relying on process-global state.
    let _ = app();
    let _: Router = app();
}
