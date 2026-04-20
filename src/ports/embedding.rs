use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmbedError {
    #[error("empty input text")]
    EmptyInput,
    #[error("model returned {got}-dim vector, expected {expected}")]
    WrongDimensions { expected: usize, got: usize },
    #[error("embedding backend failure: {0}")]
    Backend(String),
}

#[async_trait]
pub trait EmbeddingPort: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError>;
}
