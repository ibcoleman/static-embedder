use std::sync::Arc;

use async_trait::async_trait;
use model2vec_rs::model::StaticModel;

use crate::domain::EMBEDDING_DIM;
use crate::ports::{EmbedError, EmbeddingPort};

pub struct Model2VecEmbedder {
    model: Arc<StaticModel>,
}

impl Model2VecEmbedder {
    pub fn from_pretrained(repo_or_path: &str) -> Result<Self, EmbedError> {
        let model = StaticModel::from_pretrained(repo_or_path, None, None, None)
            .map_err(|e| EmbedError::Backend(format!("failed to load model: {e}")))?;
        Ok(Self {
            model: Arc::new(model),
        })
    }
}

#[async_trait]
impl EmbeddingPort for Model2VecEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        if text.trim().is_empty() {
            return Err(EmbedError::EmptyInput);
        }

        let model = Arc::clone(&self.model);
        let owned = text.to_owned();
        let vector = tokio::task::spawn_blocking(move || model.encode_single(&owned))
            .await
            .map_err(|e| EmbedError::Backend(format!("encode task panicked: {e}")))?;

        if vector.len() != EMBEDDING_DIM {
            return Err(EmbedError::WrongDimensions {
                expected: EMBEDDING_DIM,
                got: vector.len(),
            });
        }
        Ok(vector)
    }
}
