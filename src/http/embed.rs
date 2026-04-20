use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::http::error::ApiError;
use crate::http::AppState;

#[derive(Debug, Deserialize)]
pub struct EmbedRequest {
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct EmbedResponse {
    pub vector: Vec<f32>,
}

pub async fn handler(
    State(state): State<AppState>,
    Json(req): Json<EmbedRequest>,
) -> Result<Json<EmbedResponse>, ApiError> {
    let vector = state.embedder.embed(&req.text).await?;
    Ok(Json(EmbedResponse { vector }))
}
