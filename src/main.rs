use std::env;
use std::sync::Arc;

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

use static_embedder::adapters::{Model2VecEmbedder, PgVectorRepository};
use static_embedder::http::{router, AppState};
use static_embedder::ports::{EmbeddingPort, VectorRepository};

const DEFAULT_MODEL: &str = "minishlab/potion-retrieval-32M";
const DEFAULT_BIND: &str = "0.0.0.0:8080";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND.to_owned());
    let model_id = env::var("MODEL_ID").unwrap_or_else(|_| DEFAULT_MODEL.to_owned());

    tracing::info!(%model_id, "loading embedding model");
    let embedder =
        Model2VecEmbedder::from_pretrained(&model_id).context("failed to load embedding model")?;

    tracing::info!("connecting to postgres");
    let pool = PgPoolOptions::new()
        .max_connections(16)
        .connect(&database_url)
        .await
        .context("failed to connect to postgres")?;

    let repo = PgVectorRepository::new(pool);
    repo.migrate().await.context("failed to run migrations")?;

    let state = AppState {
        embedder: Arc::new(embedder) as Arc<dyn EmbeddingPort>,
        repo: Arc::new(repo) as Arc<dyn VectorRepository>,
    };

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind {bind_addr}"))?;
    tracing::info!(%bind_addr, "server listening");

    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut sig) = signal(SignalKind::terminate()) {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
