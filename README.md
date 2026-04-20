# static-embedder

A small Rust semantic-search service built around Tom Aarsen's static embedding
model (`sentence-transformers/static-retrieval-mrl-en-v1`). Text in → 384-dim
vector out → stored and searched via Postgres + pgvector.

Hexagonal architecture: `EmbeddingPort` and `VectorRepository` as traits, wired
in `main()`. Adapters today: `Model2VecEmbedder` (via `model2vec-rs`) and
`PgVectorRepository` (SQLx + pgvector with an HNSW cosine index).

## Try it (GitHub Codespaces — iPad-friendly)

1. On the repo page, click **Code → Codespaces → Create codespace on main**.
2. Wait ~90 seconds for the container to build. `docker compose up -d` runs
   automatically; pgvector Postgres is listening on `localhost:5432`.
3. In the Codespace terminal: `cargo run`. First run downloads the model
   (~8 MB) and compiles; subsequent runs are fast.
4. VS Code surfaces a "port forwarded" notification for 8080. Click
   **Open in Browser**. The first time you may also want to right-click the
   port in the **Ports** panel and set visibility to **Public** if you want
   to share the URL.
5. The browser lands on the demo UI: paste paragraphs, hit **Index**, then
   semantic-search them.

## HTTP API

| Method | Path      | Body                                 | Returns                                       |
|--------|-----------|--------------------------------------|-----------------------------------------------|
| GET    | `/`       | —                                    | Demo HTML page                                |
| GET    | `/healthz`| —                                    | `ok`                                          |
| POST   | `/embed`  | `{"text": "..."}`                    | `{"vector": [f32; 384]}`                      |
| POST   | `/index`  | `{"text": "..."}`                    | `{"id": "uuid"}` (embeds + inserts)           |
| POST   | `/search` | `{"query": "...", "k": 10}`          | `{"hits": [{"id","text","score"}, ...]}`      |

`score` is cosine similarity: 1.0 is identical direction, 0.0 is orthogonal.

## Running locally (non-Codespaces)

```
docker compose up -d
export DATABASE_URL=postgres://embedder:embedder@localhost:5432/embeddings
cargo run
```

## Tests

```
cargo test                                         # offline suite (fakes)
cargo test --test live_db -- --ignored             # against docker-compose pg
```

CI (`.github/workflows/ci.yml`) runs fmt, clippy, the offline suite, and the
live-DB smoke test against a pgvector service container on every PR.

## Staging / production

Codespaces is the dev loop; it is not persistent. A persistent deployment
target (Fly.io + managed pgvector, or Render + Neon) is a separate follow-up.
