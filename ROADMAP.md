# Roadmap

The architecture target is described in `CLAUDE.md`. This file sequences the
work to get there from where we are today. Each phase ends at a state where
the repo is self-consistent — `CLAUDE.md`, `README.md`, and the code agree
with each other.

## Phase 1 — Harden the current stack (in progress)

Goal: make the Cargo + docker-compose + Codespaces dev loop a faithful
representative of the manifesto within the constraints of the current build
system.

- [x] `DocId` newtype wrapping `Uuid` at the domain boundary.
- [x] `proptest` dev-dep + `tests/properties.rs` with two seed properties
  (dimension invariance, hit-belongs-to-corpus + sort order).
- [x] `Justfile` covering the core inner-loop commands (`dev`, `check`,
  `test`, `doctor`, etc.; full set evolves through Phase 3).
- [x] `cargo-mutants` nightly workflow uploading `mutants.out/` as an
  artifact. Track the baseline mutation score.
- [x] LSP plugin integration: `ENABLE_LSP_TOOL=1` in devcontainer
  `remoteEnv`; `rust-analyzer` ships on PATH via the Microsoft Rust
  devcontainer image; `just doctor` verifies both. The per-user Claude
  Code plugin install is documented in `CLAUDE.md` > LSP section.
- [x] `Model2VecEmbedder` determinism property in
  `tests/integration_embedder.rs` (originally `properties_live.rs`,
  renamed in 3c's tail for clearer taxonomy). Gated `#[ignore]` because
  it downloads weights; run via `cargo test --test integration_embedder
  -- --ignored`.

**Phase 1 closeout (one open item)**

The first scheduled `cargo-mutants` run produced the baseline below.
Phase 1 is now closed.

```
Mutation baseline
  Run date:        2026-04-21
  cargo-mutants:   27.0.0
  Pre-fix run:     5 caught, 16 missed  (23.8%)
                   — exposed 6 real gaps (body-content assertions on
                     healthz/frontend, upper-bound k check in search)
                     and 10 out-of-scope mutants (live-only adapters,
                     main/shutdown wiring).
  Post-fix run:    28 total mutants generated
                   11 caught   (all viable mutants killed)
                    0 missed
                    0 timeout
                   17 unviable (did not compile; e.g. Default::default()
                                substitutions where the return type has
                                no Default impl)
                   Mutation score: 11 / (11 + 0 + 0) = 100% viable.
  In-scope files:  src/domain/, src/ports/, src/http/, src/adapters/*
                   except model2vec_embedder.rs and pg_vector_repository.rs
  Out of scope:    src/main.rs (wiring), the two live-only adapters
                   (coverage lives behind #[ignore]). Rationale and
                   re-enable conditions live in .github/workflows/mutants.yml.
  Policy:          Target ≥80% on in-scope code (CLAUDE.md). Alert on
                   any drop >5 points in a single commit. Chase deltas,
                   not the absolute number; treat equivalent mutants as
                   exclusions with a comment, not tests to bolt on.
```

The policy in the last block matters: 100% today is a side effect of a
small surface area, not a target to defend. As domain logic grows we
expect the raw score to drift; what we care about is that *new code
arrives with tests that kill its mutants*, which will show up as
near-stable score even as the code grows.

## Phase 2 — Persistent staging target (deferred)

Goal: a URL that's always up, not dependent on a running dev machine. This
was orthogonal to the build-system migration and could have landed before
Phase 3.

**Status (2026-04-21):** deferred. The project moved back to a local WSL
dev loop (`just dev` + docker-compose Postgres) and the need for a
shareable demo URL went away. The deployable artifact from the first
attempt is still in the tree:

- `Dockerfile` — multi-stage (rust builder → model downloader → distroless
  runtime with model files baked in). Still useful; Phase 3 will reuse it
  for the k3d/kind image.
- `.dockerignore` — keeps the build context tight.
- `fly.toml` — Fly-specific. Harmless until somebody picks this phase back
  up; delete if we commit to a different host.

The original plan picked **Fly.io + Neon Postgres** (Neon for pgvector on
the free tier, Fly for the app container). Fly Postgres was a viable
single-vendor alternative. Render was rejected because its free Postgres
tier doesn't include pgvector. If staging comes back on the table, that
analysis is the starting point — re-validate pricing and pgvector support
on each option before committing.

**Re-entry conditions:** someone needs a persistent demo URL, or Phase 3
k8s work produces manifests that want a real cloud target.

## Phase 3 — Bazel, Tilt, k8s (one `just dev`)

Goal: align the build system with the manifesto. This is the largest piece
and should be treated as a dedicated milestone.

Sub-phases:

### 3a. Bazel for Rust only (done — 2026-04-21)

- [x] `MODULE.bazel` with `rules_rust` 0.69.0 and `crate_universe` in
  `from_cargo` mode. Bazel 9.1.0 pinned via `.bazelversion`. WORKSPACE is
  intentionally absent — Bazel 9 removes it.
- [x] Cargo.toml stays the dep source of truth; `crate_universe` reads it
  directly. Repin via `CARGO_BAZEL_REPIN=1 bazel fetch @crates//...`
  (or `just bazel-repin`). Both `Cargo.lock` and `MODULE.bazel.lock` are
  committed.
- [x] `BUILD.bazel` targets: `rust_library` + `rust_binary` at root;
  `rust_test` per integration/property file under `tests/BUILD.bazel`.
  `integration_db` and `integration_embedder` carry `tags = ["manual", "external"]`
  and retain their `#[ignore]` attrs so both `cargo test` and
  `bazel test //...` skip them by default; opt in with
  `bazel test //tests:integration_db --config=live`.
- [x] CI runs `bazel test //...` (offline suite) and
  `bazel test //tests:integration_db --config=live` (live smoke). `cargo fmt` /
  `cargo clippy` stay as separate Cargo steps per the roadmap note.

**Exit evidence:** `bazel test //...` and `bazel test //tests:integration_db
--config=live` both green locally against docker-compose Postgres.
`just check` and `just test-integration` now wrap those Bazel invocations.

### 3b. kind + Tilt dev loop

- [x] Extend `devcontainer.json` with kind + kubectl + tilt (installed
  via the Homebrew devcontainer feature for parity with local WSL).
- [x] `k8s/` directory: kustomize base (Deployment + Service + Postgres
  StatefulSet + headless Service) plus a `local` overlay establishing
  the pattern for future staging/prod overlays.
- [x] `Tiltfile` at root: `docker_build` of the existing Dockerfile,
  `k8s_yaml(kustomize('./k8s/overlays/local'))`, port forwards for 8080
  (app) and 5432 (Postgres). Context-locked to `kind-static-embedder`.
- [x] `just dev-k8s` as a *transitional* target — creates the kind
  cluster if missing, then runs `tilt up`. Coexists with the existing
  `just dev` (cargo + docker-compose) so the inner loop stays live
  during the cutover. `just reset-cluster` deletes the kind cluster.

**3b exit evidence:** `just dev-k8s` brings up kind + the stack,
surfaces `localhost:8080/healthz` → `ok`, and `/embed` returns a
512-dim vector. Docker-compose path (`just dev`) still works
identically for anyone who prefers it.

### 3c. Retire the old paths + unify on Bazel (done — 2026-04-22)

- [x] **Image build moved to Bazel.** Tiltfile now uses `custom_build`
  invoking `bazel build //:static-embedder`, then stages the binary
  next to a minimal `Dockerfile` that wraps it onto distroless/cc.
  No cargo inside the container. Measured cold rebuild (src change):
  ~12 seconds (bazel 6s + docker wrap 6s), down from ~5 min.
- [x] `just dev-k8s` → `just dev`. The cargo-based `just dev` is gone.
- [x] `docker-compose.yml` deleted. `.devcontainer/devcontainer.json`
  `postCreateCommand` no longer runs `docker compose up -d` (it now
  installs the Bazel + k8s toolchain and warms the cargo cache).
- [x] `cargo run` removed from README and CLAUDE.md's Dev loop section.

**3c exit evidence:** `just dev` on a clean kind cluster produces a
healthy stack in <2 min cold (Bazel compile + image wrap + pod start);
src/ edits roll out in <15s. `bazel test //...` still green;
`cargo test` still green (source unchanged).
- [ ] Update `CLAUDE.md` Status table: move Phase 3 rows to **Enforced**.

**Phase 3 exit (reached 2026-04-22):** `just dev` builds + deploys to
local k8s via Tilt. `CLAUDE.md` Status table has zero "Phase 3" rows
remaining. The repo is ready to be extracted as the canonical Rust
LLM-project template.

## Deferred / watching

- **Antithesis** as a whole-system fuzzer — watching, not adopting.
- **TypeScript codebase** — adds when we have a frontend that merits a build
  step. At that point `docs/java-manifesto-draft.md`'s TS parallels become
  load-bearing.
