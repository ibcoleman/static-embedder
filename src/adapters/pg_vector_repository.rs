use async_trait::async_trait;
use pgvector::Vector;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{DocId, Hit};
use crate::ports::{RepoError, VectorRepository};

pub struct PgVectorRepository {
    pool: PgPool,
}

impl PgVectorRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn migrate(&self) -> Result<(), RepoError> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| RepoError::Backend(format!("migration failed: {e}")))
    }
}

#[async_trait]
impl VectorRepository for PgVectorRepository {
    async fn insert(&self, id: DocId, text: &str, vec: &[f32]) -> Result<(), RepoError> {
        let embedding = Vector::from(vec.to_vec());
        sqlx::query("INSERT INTO embeddings (id, text, embedding) VALUES ($1, $2, $3)")
            .bind(id.0)
            .bind(text)
            .bind(embedding)
            .execute(&self.pool)
            .await
            .map_err(|e| RepoError::Backend(format!("insert failed: {e}")))?;
        Ok(())
    }

    async fn nearest(&self, vec: &[f32], k: usize) -> Result<Vec<Hit>, RepoError> {
        let query = Vector::from(vec.to_vec());
        let limit = i64::try_from(k)
            .map_err(|_| RepoError::Backend(format!("k={k} does not fit in i64")))?;

        let rows: Vec<(Uuid, String, f64)> = sqlx::query_as(
            "SELECT id, text, 1 - (embedding <=> $1) AS score \
             FROM embeddings \
             ORDER BY embedding <=> $1 \
             LIMIT $2",
        )
        .bind(&query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepoError::Backend(format!("search failed: {e}")))?;

        Ok(rows
            .into_iter()
            .map(|(id, text, score)| Hit {
                id: DocId(id),
                text,
                score: score as f32,
            })
            .collect())
    }
}
