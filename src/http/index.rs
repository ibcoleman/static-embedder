use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::http::error::ApiError;
use crate::http::AppState;

#[derive(Debug, Deserialize)]
pub struct IndexRequest {
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct IndexResponse {
    pub id: Uuid,
}

pub async fn handler(
    State(state): State<AppState>,
    Json(req): Json<IndexRequest>,
) -> Result<Json<IndexResponse>, ApiError> {
    let vector = state.embedder.embed(&req.text).await?;
    let id = Uuid::new_v4();
    state.repo.insert(id, &req.text, &vector).await?;
    Ok(Json(IndexResponse { id }))
}
