# static-embedder

A small Rust semantic-search service built around MinishLab's native
Model2Vec retrieval model (`minishlab/potion-retrieval-32M`). Text in →
512-dim vector out → stored and searched via Postgres + pgvector.

Hexagonal architecture: `EmbeddingPort` and `VectorRepository` as traits, wired
in `main()`. Adapters today: `Model2VecEmbedder` (via `model2vec-rs`) and
`PgVectorRepository` (SQLx + pgvector with an HNSW cosine index).

## Try it (GitHub Codespaces — iPad-friendly)

1. On the repo page, click **Code → Codespaces → Create codespace on main**.
2. Wait ~2 minutes for the container to build. The devcontainer
   postCreate step installs bazelisk, kind, kubectl, and tilt.
3. In the Codespace terminal: `just dev`. First run creates a kind
   cluster (~30s), builds the binary via Bazel (~3-5 min cold), wraps
   it in a Dockerfile, loads it into kind, and Tilt rolls it out.
4. VS Code surfaces a "port forwarded" notification for 8080. Click
   **Open in Browser**. The first time you may also want to right-click
   the port in the **Ports** panel and set visibility to **Public** if
   you want to share the URL.
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

Prereqs (all on PATH; `just doctor` verifies): Rust (via rustup),
Docker, [`just`](https://github.com/casey/just), `bazelisk`, `kind`,
`kubectl`, `tilt`. On macOS/Linux, `brew install bazelisk kind
kubernetes-cli tilt-dev/tap/tilt` covers the k8s + Bazel tools.

Then:

```
just dev
```

That creates a kind cluster (if missing), `bazel build`s the binary,
wraps it in a minimal Dockerfile, loads it into kind, and brings up
the stack via Tilt. Service on `localhost:8080`; in-cluster Postgres
on `localhost:5432`. Ctrl+C leaves the cluster running for the next
`tilt up`; `just reset-cluster` nukes it.

See `CLAUDE.md` for the full target list (`check`, `test-live`,
`bazel-repin`, `reset-cluster`).

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
