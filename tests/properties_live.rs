//! Property tests that exercise real adapters. Gated with `#[ignore]`
//! because the embedder downloads weights from HuggingFace on first run
//! and the live-DB tests need a running Postgres.
//!
//! Run with:
//!     cargo test --test properties_live -- --ignored --nocapture
//!
//! Set `MODEL_ID` to override the default model, or point it at a
//! pre-cached local directory.

use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};
use tokio::runtime::Runtime;

use static_embedder::adapters::Model2VecEmbedder;
use static_embedder::ports::EmbeddingPort;

fn default_model_id() -> String {
    std::env::var("MODEL_ID").unwrap_or_else(|_| "minishlab/potion-retrieval-32M".to_owned())
}

/// `Model2VecEmbedder` is a static lookup table; embedding the same text
/// twice must produce bit-identical vectors. Guards against e.g. a
/// future caching layer drifting, or a refactor introducing
/// nondeterministic tokenization options.
#[test]
#[ignore]
fn model2vec_embed_is_deterministic() {
    let embedder = Model2VecEmbedder::from_pretrained(&default_model_id())
        .expect("load model — set MODEL_ID or check network");
    let rt = Runtime::new().expect("build runtime");

    let config = Config {
        cases: 32,
        ..Config::default()
    };
    let mut runner = TestRunner::new(config);
    let strategy =
        "[a-zA-Z0-9 .,!?'-]{1,128}".prop_filter("non-empty after trim", |s| !s.trim().is_empty());

    runner
        .run(&strategy, |text| {
            let a = rt
                .block_on(embedder.embed(&text))
                .map_err(|e| TestCaseError::fail(format!("first embed: {e}")))?;
            let b = rt
                .block_on(embedder.embed(&text))
                .map_err(|e| TestCaseError::fail(format!("second embed: {e}")))?;
            prop_assert_eq!(a, b);
            Ok(())
        })
        .expect("determinism property failed");
}
