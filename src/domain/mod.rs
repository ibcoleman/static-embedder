use serde::Serialize;
use uuid::Uuid;

pub const EMBEDDING_DIM: usize = 512;

#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub id: Uuid,
    pub text: String,
    pub score: f32,
}
