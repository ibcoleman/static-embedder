#![allow(dead_code)] // Shared across test binaries; not every binary uses everything.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

use async_trait::async_trait;
use uuid::Uuid;

use static_embedder::domain::{Hit, EMBEDDING_DIM};
use static_embedder::ports::{EmbedError, EmbeddingPort, RepoError, VectorRepository};

/// Deterministic bag-of-words fake embedder.
///
/// Each word is hashed to a bucket in `[0, EMBEDDING_DIM)` and its bit set to
/// 1.0; the resulting vector is L2-normalized. Texts sharing words get high
/// cosine similarity, texts with no overlap get ~0. Good enough to assert that
/// relevance-ordered results come back in the expected order.
pub struct FakeEmbedder;

impl FakeEmbedder {
    fn bucket(word: &str) -> usize {
        let mut h = DefaultHasher::new();
        word.hash(&mut h);
        (h.finish() as usize) % EMBEDDING_DIM
    }
}

#[async_trait]
impl EmbeddingPort for FakeEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        if text.trim().is_empty() {
            return Err(EmbedError::EmptyInput);
        }
        let mut v = vec![0.0_f32; EMBEDDING_DIM];
        for word in text.split_whitespace() {
            let lc = word.to_lowercase();
            v[Self::bucket(&lc)] = 1.0;
        }
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut v {
                *x /= norm;
            }
        }
        Ok(v)
    }
}

struct Row {
    id: Uuid,
    text: String,
    vec: Vec<f32>,
}

pub struct InMemoryRepository {
    rows: Mutex<Vec<Row>>,
}

impl InMemoryRepository {
    pub fn new() -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
        }
    }
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0_f32;
    let mut na = 0.0_f32;
    let mut nb = 0.0_f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

#[async_trait]
impl VectorRepository for InMemoryRepository {
    async fn insert(&self, id: Uuid, text: &str, vec: &[f32]) -> Result<(), RepoError> {
        let mut rows = self
            .rows
            .lock()
            .map_err(|e| RepoError::Backend(format!("lock poisoned: {e}")))?;
        rows.push(Row {
            id,
            text: text.to_owned(),
            vec: vec.to_vec(),
        });
        Ok(())
    }

    async fn nearest(&self, vec: &[f32], k: usize) -> Result<Vec<Hit>, RepoError> {
        let rows = self
            .rows
            .lock()
            .map_err(|e| RepoError::Backend(format!("lock poisoned: {e}")))?;
        let mut scored: Vec<Hit> = rows
            .iter()
            .map(|r| Hit {
                id: r.id,
                text: r.text.clone(),
                score: cosine(vec, &r.vec),
            })
            .collect();
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(k);
        Ok(scored)
    }
}
