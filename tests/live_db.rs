//! Live-DB smoke test. Gated with `#[ignore]` — run via:
//!
//!     docker compose up -d
//!     DATABASE_URL=postgres://embedder:embedder@localhost:5432/embeddings \
//!         cargo test --test live_db -- --ignored
//!
//! Exercises the real `PgVectorRepository` against pgvector: migration,
//! VECTOR round-trip, HNSW-indexed cosine search. Uses `FakeEmbedder`
//! so no HuggingFace download is required.

mod support;

use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use static_embedder::adapters::PgVectorRepository;
use static_embedder::ports::{EmbeddingPort, VectorRepository};
use support::FakeEmbedder;

async fn connect_or_skip() -> Option<PgVectorRepository> {
    let url = match std::env::var("DATABASE_URL") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("DATABASE_URL not set; skipping live-DB test");
            return None;
        }
    };
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("connect to postgres");
    let repo = PgVectorRepository::new(pool.clone());
    repo.migrate().await.expect("run migrations");
    sqlx::query("TRUNCATE embeddings")
        .execute(&pool)
        .await
        .expect("truncate");
    Some(repo)
}

#[tokio::test]
#[ignore]
async fn live_db_index_and_search() {
    let Some(repo) = connect_or_skip().await else {
        return;
    };
    let embedder = FakeEmbedder;

    let docs = [
        "Rust makes concurrency safe via ownership",
        "Bananas are a tropical fruit high in potassium",
        "Tokio is an async runtime for Rust",
    ];
    for text in docs {
        let vec = embedder.embed(text).await.expect("embed");
        repo.insert(Uuid::new_v4(), text, &vec)
            .await
            .expect("insert");
    }

    let query = embedder
        .embed("Rust async runtime")
        .await
        .expect("embed query");
    let hits = repo.nearest(&query, 3).await.expect("search");

    assert_eq!(hits.len(), 3);
    assert_eq!(hits[0].text, "Tokio is an async runtime for Rust");
    assert_eq!(
        hits[2].text,
        "Bananas are a tropical fruit high in potassium"
    );

    for h in &hits {
        assert!(h.score.is_finite(), "score must be finite");
    }
}
