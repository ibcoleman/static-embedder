# static-embedder

A small Rust semantic-search service built around MinishLab's native
Model2Vec retrieval model (`minishlab/potion-retrieval-32M`). Text in →
512-dim vector out → stored and searched via Postgres + pgvector.

Hexagonal architecture: `EmbeddingPort` and `VectorRepository` as traits, wired
in `main()`. Adapters today: `Model2VecEmbedder` (via `model2vec-rs`) and
`PgVectorRepository` (SQLx + pgvector with an HNSW cosine index).

## Try it (GitHub Codespaces — iPad-friendly)

1. On the repo page, click **Code → Codespaces → Create codespace on main**.
2. Wait ~90 seconds for the container to build. `docker compose up -d` runs
   automatically; pgvector Postgres is listening on `localhost:5432`.
3. In the Codespace terminal: `just dev`. First run downloads the model
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
| POST   | `/embed`  | `{"text": "..."}`                    | `{"vector": [f32; 512]}`                      |
| POST   | `/index`  | `{"text": "..."}`                    | `{"id": "uuid"}` (embeds + inserts)           |
| POST   | `/search` | `{"query": "...", "k": 10}`          | `{"hits": [{"id","text","score"}, ...]}`      |

`score` is cosine similarity: 1.0 is identical direction, 0.0 is orthogonal.

## Running locally (non-Codespaces)

Prereqs: Rust (via rustup), Docker Engine with the compose plugin, and
[`just`](https://github.com/casey/just). Verify with `just doctor`. Then
pick a dev path:

```
just dev        # docker compose Postgres + cargo run (fast inner loop)
just dev-k8s    # kind + Tilt against k8s/overlays/local (production-shaped)
```

Either way, Postgres ends up on `localhost:5432` and the service on
`localhost:8080`. `just dev-k8s` additionally needs `kind`, `kubectl`,
and `tilt` on PATH (`just doctor` reports).

`just dev-k8s` is the target shape — it mirrors a real deployment with a
Deployment, Service, and StatefulSet, rendered from a kustomize base.
`just dev` is transitional and goes away in Phase 3c; see `ROADMAP.md`.

See `CLAUDE.md` for the full target list (`check`, `test-live`,
`bazel-repin`, `reset-db`, `reset-cluster`).

## Tests

Bazel owns the build + test surface (Phase 3a of the roadmap). The canonical
invocations:

```
bazel test //...                                   # offline suite (fakes)
bazel test //tests:live_db --config=live           # against docker-compose pg
```

Cargo still works for developers who prefer it — tests are declared once in
the source and both drivers can execute them:

```
cargo test                                         # offline suite
cargo test --test live_db -- --ignored             # live pg
```

CI (`.github/workflows/ci.yml`) runs `cargo fmt --check`, `cargo clippy`,
the Bazel offline suite, and the Bazel live-DB smoke test against a
pgvector service container on every PR.

## Persistent staging

There isn't one right now. An earlier attempt targeted Fly.io + Neon and
landed the deployable artifact (`Dockerfile`, `fly.toml`, `.dockerignore`),
but the project moved back to a local WSL dev loop before the first deploy
went live. See `ROADMAP.md` Phase 2 for the history and re-entry conditions.
