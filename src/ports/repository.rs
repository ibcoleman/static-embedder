use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use crate::domain::Hit;

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("storage backend failure: {0}")]
    Backend(String),
}

#[async_trait]
pub trait VectorRepository: Send + Sync {
    async fn insert(&self, id: Uuid, text: &str, vec: &[f32]) -> Result<(), RepoError>;
    async fn nearest(&self, vec: &[f32], k: usize) -> Result<Vec<Hit>, RepoError>;
}
