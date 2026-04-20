pub mod embed;
pub mod error;
pub mod index;
pub mod search;

use std::sync::Arc;

use axum::response::Html;
use axum::routing::{get, post};
use axum::Router;

use crate::ports::{EmbeddingPort, VectorRepository};

const FRONTEND_HTML: &str = include_str!("../../static/index.html");

#[derive(Clone)]
pub struct AppState {
    pub embedder: Arc<dyn EmbeddingPort>,
    pub repo: Arc<dyn VectorRepository>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(frontend))
        .route("/healthz", get(healthz))
        .route("/embed", post(embed::handler))
        .route("/index", post(index::handler))
        .route("/search", post(search::handler))
        .with_state(state)
}

async fn frontend() -> Html<&'static str> {
    Html(FRONTEND_HTML)
}

async fn healthz() -> &'static str {
    "ok"
}
