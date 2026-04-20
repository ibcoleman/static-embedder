use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::domain::Hit;
use crate::http::error::ApiError;
use crate::http::AppState;

const DEFAULT_K: usize = 10;
const MAX_K: usize = 100;

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub k: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub hits: Vec<Hit>,
}

pub async fn handler(
    State(state): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, ApiError> {
    let k = req.k.unwrap_or(DEFAULT_K);
    if k == 0 || k > MAX_K {
        return Err(ApiError::BadRequest(format!(
            "k must be between 1 and {MAX_K}"
        )));
    }
    let vector = state.embedder.embed(&req.query).await?;
    let hits = state.repo.nearest(&vector, k).await?;
    Ok(Json(SearchResponse { hits }))
}
