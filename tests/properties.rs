//! Property-based tests. Use the `FakeEmbedder` + `InMemoryRepository`
//! from `support/` so the properties exercise the hexagonal wiring,
//! not a single adapter.
//!
//! These guard against classes of regressions that example-based tests
//! miss: dimension drift, phantom hits, and unsorted results. They are
//! intentionally small — the real payoff compounds as we add more.

mod support;

use proptest::prelude::*;
use tokio::runtime::Runtime;

use static_embedder::domain::{DocId, EMBEDDING_DIM};
use static_embedder::ports::{EmbeddingPort, VectorRepository};
use support::{FakeEmbedder, InMemoryRepository};

fn runtime() -> Runtime {
    Runtime::new().expect("build current-thread runtime")
}

proptest! {
    /// For any non-empty text, the embedder must return a vector whose
    /// length matches the hardcoded dimension. Guards against a future
    /// embedder silently returning the wrong shape.
    #[test]
    fn embedder_always_returns_expected_dim(text in r"[ -~]{1,128}") {
        prop_assume!(!text.trim().is_empty());
        let v = runtime()
            .block_on(FakeEmbedder.embed(&text))
            .expect("non-empty text should embed");
        prop_assert_eq!(v.len(), EMBEDDING_DIM);
    }

    /// Every hit returned by `nearest` must correspond to a document
    /// that was actually inserted, and hits must be ordered by
    /// descending score.
    #[test]
    fn nearest_returns_only_indexed_docs_sorted_by_score(
        docs in prop::collection::vec("[a-z]{3,8}( [a-z]{3,8}){0,4}", 1..6),
        query in "[a-z]{3,8}( [a-z]{3,8}){0,4}",
    ) {
        let hits = runtime().block_on(async {
            let repo = InMemoryRepository::new();
            let embedder = FakeEmbedder;
            for text in &docs {
                let v = embedder.embed(text).await.expect("embed doc");
                repo.insert(DocId::new(), text, &v).await.expect("insert");
            }
            let qv = embedder.embed(&query).await.expect("embed query");
            repo.nearest(&qv, docs.len()).await.expect("search")
        });

        for h in &hits {
            prop_assert!(
                docs.contains(&h.text),
                "phantom hit: {:?} not in indexed docs",
                h.text
            );
        }
        for w in hits.windows(2) {
            prop_assert!(
                w[0].score >= w[1].score,
                "unsorted hits: {} before {}",
                w[0].score,
                w[1].score
            );
        }
    }
}
