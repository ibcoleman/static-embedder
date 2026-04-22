# CLAUDE.md

Project conventions and guardrails for AI coding agents working in this repo.
Read this before writing code or running commands.

## Status

This project is in transition. The manifesto below is the target state; not
every rule is enforced today. Apply rules according to this table:

| Rule | Today | Notes |
|---|---|---|
| `Result` + `?` for every fallible op | **Enforced** | Domain + adapters both follow this |
| No `unwrap()` / `expect()` in non-test code | **Enforced** | Clippy gate in CI would catch backsliding; add explicit lint next |
| Sealed `enum` domain errors via `thiserror` | **Enforced** | `EmbedError`, `RepoError`, `ApiError` |
| Newtype-wrap primitives with domain meaning | **Enforced (partial)** | `DocId(Uuid)` in place; extend as new types appear |
| Property-based tests for non-trivial logic | **Enforced (seed)** | `tests/properties.rs`. Add a property alongside any new domain logic |
| Mutation testing in CI | **Enforced** | `.github/workflows/mutants.yml` runs `cargo-mutants` nightly. Baseline score recorded in `ROADMAP.md` after first run. |
| Build via Bazel (`rules_rust` + `crate_universe`) | **Enforced** | Binary, tests, and container image builds all flow through Bazel. CI runs `bazel test //...`; Tilt's `custom_build` invokes `bazel build //:static-embedder` and wraps it in a thin Dockerfile. `fmt` / `clippy` stay on Cargo. |
| Local orchestration via Tilt + local k8s | **Enforced** | `just dev` = kind cluster + Tilt against kustomize manifests. No docker-compose in the tree. |
| Single `just dev` path, no hot reload | **Enforced** | `just dev` = kind + Tilt against `k8s/overlays/local`. Bazel builds the binary; a minimal Dockerfile wraps it. No other inner-loop entry points exist. Do not add hot-reload flavours. |
| No `cargo run` in docs | **Enforced** | Inner loop is `just dev` (kind + Tilt); `cargo run` is not in README/CLAUDE.md. Cargo still handles fmt + clippy. |
| Pedantic TypeScript (strict, `neverthrow`, `type-fest`) | **Not applicable yet** | Frontend is vanilla JS embedded in the binary. Applies when a real TS codebase appears |
| LSP plugin integration | **Enforced (env)** | Devcontainer sets `ENABLE_LSP_TOOL=1` and ships `rust-analyzer` on PATH. `just doctor` verifies both. The Claude Code plugin install is still a per-user action — see "LSP / agent tooling" below. |

Agents: when a request would require a rule marked **Phase 3**, do not
improvise — surface the tension and ask.

## Philosophy

- **Types are guardrails.** Push errors and intent into the type system so the
  compiler — not human review — catches agent mistakes. Make wrong states
  unrepresentable.
- **One dev path.** Every change flows through a single build. No hot reloads,
  no parallel "fast paths" that can diverge from the real build and mislead
  the model. `just dev` boots a kind cluster, Bazel builds the binary,
  Tilt wraps it in a minimal Dockerfile and deploys. That's the only
  inner loop.
- **Tests that bite.** Property-based tests make it hard to write wrong code
  that passes. Mutation tests make it hard to write useless tests that pass.
  We run both (mutation is on the adopt-next list).

## Stack

- **Language today:** Rust (Cargo). TypeScript pending an actual frontend that
  needs a build step.
- **Build:** Bazel (`rules_rust` + `crate_universe`) for the binary,
  tests, and container image. Cargo's `Cargo.toml` is still the dep
  source of truth — `crate_universe` reads it in `from_cargo` mode.
  `fmt` / `clippy` stay on Cargo. (TypeScript via `rules_js` when a
  real TS codebase lands.)
- **Local orchestration:** `just dev` boots a `kind` cluster (if
  missing) and runs `tilt up` against `k8s/overlays/local`. Bazel
  builds the binary; `Dockerfile` wraps it onto distroless/cc. Tilt
  handles image loading, port-forwards (8080 app, 5432 postgres), and
  incremental rebuilds.
- **Command runner:** `just`.
- **Deploy target (staging):** none today. An earlier Fly.io + Neon attempt
  is deferred; see `ROADMAP.md` Phase 2.

## Dev loop

From the repo root on your dev machine (WSL or Codespace):

```
just dev               # kind cluster + tilt up (the only inner loop)
just test              # offline suite: api + properties via fakes
just test-integration  # real DB + real embedder (needs `just dev` up)
just check             # cargo fmt + clippy + `just test` (matches CI)
just mutants           # cargo-mutants locally (mirrors nightly CI)
just bazel-repin       # regenerate crate_universe pins after editing Cargo.toml
just reset-cluster     # delete the kind cluster entirely
just doctor            # verify prerequisites are on PATH
```

One engine end-to-end: Bazel builds the binary, Tilt's `custom_build`
stages it with a minimal Dockerfile, kind runs the resulting image.
No cargo in the container, no docker-compose on the side, no
hot-reload flavours.

Do **not** add `cargo watch`, `tsc --watch`, or other hot-reload pathways.
They present a state that does not match the real build and confuse both
humans and agents.

## Rust conventions

- Use `Result` + `?` for every fallible operation. **No `unwrap()` /
  `expect()` in production code** (tests and one-off test helpers are fine).
- **Newtype-wrap primitives that carry domain meaning**: `DocId(Uuid)` today.
  As new semantically-distinct values appear (other IDs, money, timestamps
  with meaning, vector dimensions as types, etc.), wrap them. Never pass raw
  `String` or `u64` across an API boundary when the value has a name.
- **Sealed `enum`s** for exhaustive domain modelling; lean on the
  exhaustiveness checker. `thiserror` for error types in library/adapter
  code.
- Cargo.toml is the source of truth for third-party crates. Bazel reads it
  via `crate_universe` in `from_cargo` mode — never hand-write `BUILD`
  files for external crates. After editing Cargo.toml, repin with:
  `CARGO_BAZEL_REPIN=1 bazel fetch @crates//...`
  (or `just bazel-repin`). Both `Cargo.lock` and `MODULE.bazel.lock` are
  committed.

## TypeScript conventions

(Not applicable until a TS codebase exists. When it does:)

### tsconfig (non-negotiable)

```json
{
  "compilerOptions": {
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true,
    "noFallthroughCasesInSwitch": true
  }
}
```

### Error handling with `neverthrow`

- Return `Result` / `ResultAsync`. **Do not `throw`** in application code.
- `eslint-plugin-neverthrow` enforces both "don't throw" and "you must
  actually handle the `Result`." Do not disable it.
- `try` / `catch` is reserved for bridging to libraries that throw. Wrap
  those boundaries into `Result` at the seam and keep the rest of the code
  in `Result`-land.

### `type-fest`

- `Opaque` / `Tagged` — nominal types. `UserId` is not assignable to `OrderId`
  even though both wrap `string`. Use for every ID-like field.
- `Except`, `SetRequired`, `SetOptional` — derive related types without
  duplication.
- `ReadonlyDeep` — TS `readonly` is shallow and structural. Use
  `ReadonlyDeep` (or `as const`) when you want Java-`final`-like guarantees.

## Testing

### Property-based tests (required for non-trivial logic)

- Rust: `proptest` — in place. See `tests/properties.rs`.
- TS: `fast-check` — when TS appears.

Anything with a non-trivial input space — parsers, state machines,
serializers, domain logic with invariants — gets properties, not just
examples. Agents find it much harder to write a passing-but-meaningless
implementation against properties than against examples, because they
cannot see which inputs will be generated.

### Mutation testing (CI, nightly / pre-merge)

- Rust: `cargo-mutants` — adding as a nightly GitHub Actions workflow.
- TS: `Stryker` — when TS appears.

Mutation score is a first-class quality metric. **If a change drops the
mutation score, it is a regression — even if every example-based test still
passes.** Mutation runs are slow by nature; schedule them nightly or
pre-merge rather than on every commit.

### Antithesis

On the radar as a deterministic-hypervisor whole-system fuzzer. Watching,
not adopting today. Do not wire it in without a discussion.

## LSP / agent tooling

Code-intelligence tools (goto-def, find-refs, type-at-cursor, rename) are
exposed to the agent via Claude Code's LSP integration.

Setup (target state — not fully wired yet):

1. `ENABLE_LSP_TOOL=1` in the dev environment (exact name matters —
   it is `ENABLE_LSP_TOOL`, not `LSP_TOOL_ENABLE`). The devcontainer sets
   this via `remoteEnv`; WSL users should export it from their shell rc.
2. Language server binaries on PATH:
   - `rust-analyzer` — `rustup component add rust-analyzer`.
   - `vtsls` — when TS arrives.
3. In Claude Code: `/plugin marketplace add
   anthropics/claude-plugins-official`, then `/plugin install` the Rust
   (and later TS) code-intelligence plugins.
4. Restart Claude Code. Confirm with `claude --debug` — look for
   `LSP server plugin:rust:rust initialized`.

`just doctor` verifies steps 1 and 2. If goto-def feels broken, run it first.

## Rules for the agent

**Do:**

- Route build/deploy through `just` targets.
- Model every fallible operation as `Result`. Propagate with `?`.
- Use newtype wrappers for IDs and other semantically-distinct values.
- Add property-based tests for any non-trivial new logic.
- Run `just doctor` if anything about the toolchain feels off.
- Trust the toolchain: Bazel for builds + tests + images, kind + Tilt
  for the dev loop, Cargo for fmt + clippy + dep management. Don't
  introduce a fourth engine without a discussion.

**Don't:**

- Add hot-reload or dev-server paths (`cargo watch`, `tsc --watch`,
  `pnpm dev`). `just dev` — kind + Tilt against `k8s/overlays/local`,
  Bazel-built binary wrapped in a thin Dockerfile — is the only
  dev-loop entry point. Don't add siblings.
- `throw` in TS application code (when TS exists). Don't `unwrap()` /
  `expect()` in Rust production code.
- Silence the type checker (`any`, non-trivial `as` casts, `// @ts-ignore`).
- Use raw `string` / number primitives for IDs — wrap them.
- Ship tests that only exercise the happy path. If `cargo-mutants` / Stryker
  can flip a comparison and your tests still pass, the tests are not doing
  their job.
- Hand-write BUILD files for third-party crates. Use `crate_universe`;
  regenerate with `just bazel-repin` after Cargo.toml edits.
