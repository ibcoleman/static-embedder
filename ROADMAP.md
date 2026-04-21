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
- [ ] LSP plugin integration: add `ENABLE_LSP_TOOL=1` to the devcontainer
  `remoteEnv`, confirm `rust-analyzer` is on PATH, update `just doctor` to
  verify both.
- [ ] Extend property coverage as domain types grow. Concrete next property:
  "any text re-embedded produces the same vector" (determinism check for
  `Model2VecEmbedder`).

**Phase 1 exit**: nightly mutation baseline committed to `ROADMAP.md`; agents
can rely on `just doctor` to diagnose environment drift.

## Phase 2 — Persistent staging target

Goal: a URL that's always up, not dependent on a running Codespace. This is
orthogonal to the build-system migration and can land before Phase 3.

Candidate stacks (pick one, then execute):

- **Fly.io + managed Fly Postgres.** pgvector extension available; deploy
  flow via `superfly/flyctl-actions` on push to `main`. Free tier covers a
  1-VM demo.
- **Fly.io + Neon Postgres.** Same app host, external managed Postgres.
  Cleaner iPad-only setup — Neon is entirely web-UI and supports pgvector on
  the free tier.
- **Render + managed Postgres.** Rejected for now: pgvector isn't available
  on Render's free Postgres tier.

Work items (regardless of choice):

- [ ] `Dockerfile` (multi-stage: `rust:slim` → `distroless/cc`).
- [ ] `fly.toml` (or the chosen platform's equivalent).
- [ ] `.github/workflows/deploy.yml` triggered on push to `main` after
  `ci` passes.
- [ ] Document the one-time manual steps (provisioning, `CREATE EXTENSION
  vector`, `FLY_API_TOKEN` secret) in `README.md`.

**Phase 2 exit**: `main` deploys automatically; a shareable URL renders the
demo against a persistent Postgres.

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
