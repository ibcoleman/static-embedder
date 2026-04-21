use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const EMBEDDING_DIM: usize = 512;

/// Document identity. Newtype around `Uuid` so it cannot be
/// accidentally mixed with other UUID-shaped values at call sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocId(pub Uuid);

impl DocId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for DocId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub id: DocId,
    pub text: String,
    pub score: f32,
}
