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
- [x] `Justfile` covering `dev`, `check`, `test-live`, `reset-db`, `doctor`.
- [x] `cargo-mutants` nightly workflow uploading `mutants.out/` as an
  artifact. Track the baseline mutation score.
- [x] LSP plugin integration: `ENABLE_LSP_TOOL=1` in devcontainer
  `remoteEnv`; `rust-analyzer` ships on PATH via the Microsoft Rust
  devcontainer image; `just doctor` verifies both. The per-user Claude
  Code plugin install is documented in `CLAUDE.md` > LSP section.
- [x] `Model2VecEmbedder` determinism property in
  `tests/properties_live.rs`. Gated `#[ignore]` because it downloads
  weights; run via `cargo test --test properties_live -- --ignored`.

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

## Phase 2 — Persistent staging target

Goal: a URL that's always up, not dependent on a running Codespace. This is
orthogonal to the build-system migration and can land before Phase 3.

**Chosen stack: Fly.io + Neon Postgres.** Neon's free tier supports
pgvector and is fully web-UI driven (no CLI needed for the DB); Fly hosts
the app container. Fly Postgres (their native offering) was the alternative
and is still viable if we want to collapse to a single vendor later. Render
was rejected because its free Postgres tier doesn't include pgvector.

### Part 1 — Deployable artifact + first manual deploy

- [x] `Dockerfile` — multi-stage (rust builder → model downloader → distroless
  runtime with model files baked in, `gcr.io/distroless/cc-debian12:nonroot`).
- [x] `.dockerignore` — keeps the build context tight (skips `target/`, docs,
  dev tooling).
- [x] `fly.toml` — minimal template with `primary_region = "iad"`, a
  `/healthz` check, and shared-1x / 512 MB VM size.
- [x] README "Deploy to Fly.io + Neon" section — one-time setup (Neon
  provisioning + `CREATE EXTENSION vector`, `flyctl auth login`, secret,
  `flyctl deploy`) and smoke test.
- [ ] **Execute the one-time setup** (user action — CLI from Codespace).
- [ ] **Capture the live URL** in README once the first deploy is green.

**Part 1 exit**: a shareable `https://<app>.fly.dev/` URL renders the demo UI
and serves search against Neon. Redeploys are still manual `flyctl deploy`.

### Part 2 — Automate redeploys

- [ ] `.github/workflows/deploy.yml` — `superfly/flyctl-actions@master`,
  triggered on push to `main` after the `ci` job passes.
- [ ] `FLY_API_TOKEN` repo secret (generated via `flyctl tokens create deploy`).
- [ ] README update noting that merging to `main` now redeploys automatically.

**Phase 2 exit**: `main` deploys automatically; the shareable URL stays in
sync with `main` without manual intervention.

## Phase 3 — Bazel, Tilt, k8s, `just dev-sync`

Goal: align the build system with the manifesto. This is the largest piece
and should be treated as a dedicated milestone.

Sub-phases:

### 3a. Bazel for Rust only

- [ ] `WORKSPACE.bazel` / `MODULE.bazel` with `rules_rust` and
  `crate_universe`.
- [ ] Migrate `Cargo.toml` into the `crate_universe` manifest; regenerate.
- [ ] `BUILD.bazel` targets: library, binary, integration tests, property
  tests.
- [ ] CI runs `bazel test //...` in place of `cargo test`. Keep
  `cargo fmt`/`clippy` as separate steps until tooling catches up.

### 3b. k3d/kind inside the Codespace

- [ ] Extend `devcontainer.json` with a k3d (or kind) feature.
- [ ] `k8s/` directory: Deployment, Service, Postgres StatefulSet (or
  external pgvector) manifests.
- [ ] `just dev-sync` = `tilt up` against those manifests.

### 3c. Retire the old paths

- [ ] Remove `cargo run` from `README.md` and `CLAUDE.md`'s "Dev loop"
  section.
- [ ] Remove `just dev` (the `cargo run` wrapper); promote `just dev-sync`
  to the sole inner-loop entry point.
- [ ] Update `CLAUDE.md` Status table: move Phase 3 rows to **Enforced**.

**Phase 3 exit**: a single `just dev-sync` builds + deploys to local k8s;
`CLAUDE.md` Status table has zero "Phase 3" rows remaining; the repo is
ready to be extracted as the canonical Rust LLM-project template.

## Deferred / watching

- **Antithesis** as a whole-system fuzzer — watching, not adopting.
- **TypeScript codebase** — adds when we have a frontend that merits a build
  step. At that point `docs/java-manifesto-draft.md`'s TS parallels become
  load-bearing.
