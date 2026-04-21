mod support;

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use static_embedder::http::{router, AppState};
use static_embedder::ports::{EmbeddingPort, VectorRepository};
use support::{FakeEmbedder, InMemoryRepository};

fn app() -> axum::Router {
    let state = AppState {
        embedder: Arc::new(FakeEmbedder) as Arc<dyn EmbeddingPort>,
        repo: Arc::new(InMemoryRepository::new()) as Arc<dyn VectorRepository>,
    };
    router(state)
}

async fn post_json(app: &axum::Router, path: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request");
    let response = app.clone().oneshot(req).await.expect("service call");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1 << 20)
        .await
        .expect("read body");
    let value: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json body")
    };
    (status, value)
}

async fn get_body(app: &axum::Router, path: &str) -> (StatusCode, Vec<u8>) {
    let req = Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .expect("build request");
    let response = app.clone().oneshot(req).await.expect("service call");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1 << 20)
        .await
        .expect("read body");
    (status, bytes.to_vec())
}

#[tokio::test]
async fn healthz_returns_ok() {
    let (status, body) = get_body(&app(), "/healthz").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"ok");
}

#[tokio::test]
async fn frontend_serves_demo_page() {
    let (status, body) = get_body(&app(), "/").await;
    assert_eq!(status, StatusCode::OK);
    let html = std::str::from_utf8(&body).expect("utf-8 html");
    assert!(html.contains("Static Embedder"), "page missing title");
    assert!(html.contains("<textarea"), "page missing textarea");
}

#[tokio::test]
async fn embed_returns_512_dim_vector() {
    let (status, body) = post_json(&app(), "/embed", json!({ "text": "hello world" })).await;
    assert_eq!(status, StatusCode::OK);
    let vector = body["vector"].as_array().expect("vector array");
    assert_eq!(vector.len(), static_embedder::domain::EMBEDDING_DIM);
}

#[tokio::test]
async fn embed_empty_text_is_400() {
    let (status, _body) = post_json(&app(), "/embed", json!({ "text": "   " })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn index_then_search_ranks_relevant_doc_first() {
    let app = app();

    for text in [
        "Rust makes concurrency safe via ownership",
        "Bananas are a tropical fruit high in potassium",
        "Tokio is an async runtime for Rust",
    ] {
        let (status, _) = post_json(&app, "/index", json!({ "text": text })).await;
        assert_eq!(status, StatusCode::OK);
    }

    let (status, body) = post_json(
        &app,
        "/search",
        json!({ "query": "Rust async runtime", "k": 3 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let hits = body["hits"].as_array().expect("hits array");
    assert_eq!(hits.len(), 3);

    let top = hits[0]["text"].as_str().expect("top text");
    assert_eq!(top, "Tokio is an async runtime for Rust");

    let last = hits[2]["text"].as_str().expect("last text");
    assert_eq!(last, "Bananas are a tropical fruit high in potassium");
}

#[tokio::test]
async fn search_rejects_k_zero() {
    let (status, _) = post_json(&app(), "/search", json!({ "query": "hi", "k": 0 })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// Boundary: MAX_K (100) is inclusive — should succeed.
/// Guards against a `>` → `>=` mutation in the upper-bound check.
#[tokio::test]
async fn search_accepts_k_equal_max_k() {
    let (status, _) = post_json(&app(), "/search", json!({ "query": "hi", "k": 100 })).await;
    assert_eq!(status, StatusCode::OK);
}

/// Boundary: MAX_K + 1 (101) must be rejected.
/// Guards against a `>` → `==` mutation (which would only reject k == MAX_K).
#[tokio::test]
async fn search_rejects_k_above_max_k() {
    let (status, _) = post_json(&app(), "/search", json!({ "query": "hi", "k": 101 })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
