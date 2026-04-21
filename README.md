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
| POST   | `/embed`  | `{"text": "..."}`                    | `{"vector": [f32; 512]}`                      |
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

## Deploy to Fly.io + Neon

Codespaces is the dev loop; it is not persistent. The staging target is
**Fly.io** (app host) + **Neon** (managed Postgres with pgvector). Both have
free tiers and are fully web-UI driven for the infra pieces. One-time setup
below; subsequent deploys are `flyctl deploy` until Part 2 wires up CI-driven
redeploys (see `ROADMAP.md`).

### One-time setup

1. **Create a Neon project** at <https://neon.tech>. Any region is fine; the
   free tier supports pgvector out of the box. Note the `postgresql://...`
   connection string from the dashboard.
2. **Enable pgvector** in Neon's web SQL editor:
   ```sql
   CREATE EXTENSION IF NOT EXISTS vector;
   ```
3. **Install `flyctl`** in the Codespace terminal:
   ```bash
   curl -L https://fly.io/install.sh | sh
   export PATH="$HOME/.fly/bin:$PATH"
   ```
4. **Log in to Fly** (opens a browser tab to authenticate):
   ```bash
   flyctl auth login
   ```
5. **Provision the app** — either accept `fly.toml` as-committed and pick a
   unique app name:
   ```bash
   flyctl apps create static-embedder-<your-suffix>
   # then edit `app = "..."` in fly.toml to match
   ```
   …or regenerate `fly.toml` entirely via `flyctl launch --dockerfile --no-deploy`.
6. **Set the database secret** (paste the Neon connection string):
   ```bash
   flyctl secrets set DATABASE_URL="postgresql://USER:PASS@HOST/DB?sslmode=require"
   ```
7. **Deploy**:
   ```bash
   flyctl deploy
   ```
   First deploy pulls the model weights from HuggingFace during the build
   step (~8 MB) and bakes them into the image. Subsequent deploys reuse the
   cached layer.
8. **Smoke test**:
   ```bash
   flyctl status                           # grabs the hostname
   curl https://<app>.fly.dev/healthz      # -> ok
   open https://<app>.fly.dev/             # demo UI
   ```

### Redeploying

Manual until Part 2 lands:

```bash
flyctl deploy
```

Part 2 adds a GitHub Actions workflow that runs `flyctl deploy` on every
push to `main` after CI passes, gated on a `FLY_API_TOKEN` repo secret.
