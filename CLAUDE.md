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
| Build via Bazel (`rules_rust` + `crate_universe`) | **Phase 3** | See `ROADMAP.md` |
| Local orchestration via Tilt + local k8s | **Phase 3** | Today: `docker compose up -d` |
| Single `just dev-sync` path, no hot reload | **Phase 3** | Today: `just dev` wraps `cargo run` honestly; do not add hot-reload flavours |
| No `cargo run` in docs | **Phase 3** | Today: `cargo run` is the documented inner loop |
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
  the model. (Phase 3 enforces this via Bazel + Tilt; today, `just dev` is a
  thin wrapper over `cargo run` + `docker compose up`.)
- **Tests that bite.** Property-based tests make it hard to write wrong code
  that passes. Mutation tests make it hard to write useless tests that pass.
  We run both (mutation is on the adopt-next list).

## Stack

- **Language today:** Rust (Cargo). TypeScript pending an actual frontend that
  needs a build step.
- **Build today:** Cargo. **Phase 3 target:** Bazel (`rules_rust` +
  `crate_universe` for Rust; `rules_js` for TS).
- **Local orchestration today:** docker-compose in a Codespace. **Phase 3
  target:** Tilt watching Bazel outputs, deploying into local k8s
  (`k3d`/`kind` inside the Codespace).
- **Command runner:** `just`.
- **Deploy target (staging):** TBD — see `ROADMAP.md` Phase 2.

## Dev loop (current)

From the repo root inside a Codespace:

```
just dev        # docker compose up -d + cargo run
just check      # fmt + clippy + offline tests
just test-live  # live-DB smoke test against docker-compose Postgres
just reset-db   # drop the pgdata volume (use when migrations change)
just doctor     # verify prerequisites are on PATH
```

Do **not** add `cargo watch`, `tsc --watch`, or other hot-reload pathways.
They present a state that does not match the real build and confuse both
humans and agents. When Phase 3 lands, `just dev` collapses into
`just dev-sync` (Bazel + Tilt) and `cargo run` disappears from the docs.

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
- Current dep manager is Cargo. **When we move to Bazel**, third-party crates
  will be managed through `crate_universe`: add deps to `Cargo.toml`, regen,
  never hand-write `BUILD` files for external crates.

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

1. `ENABLE_LSP_TOOL=1` in the Codespace environment (exact name matters —
   it is `ENABLE_LSP_TOOL`, not `LSP_TOOL_ENABLE`).
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
- When a task would require a **Phase 3** capability (Bazel, Tilt, k8s) that
  isn't in place yet, surface it and ask rather than improvising.

**Don't:**

- Add hot-reload or dev-server paths (`cargo watch`, `tsc --watch`,
  `pnpm dev`). `just dev` is the only dev-loop entry point today; `just
  dev-sync` will be the only one in Phase 3.
- `throw` in TS application code (when TS exists). Don't `unwrap()` /
  `expect()` in Rust production code.
- Silence the type checker (`any`, non-trivial `as` casts, `// @ts-ignore`).
- Use raw `string` / number primitives for IDs — wrap them.
- Ship tests that only exercise the happy path. If `cargo-mutants` / Stryker
  can flip a comparison and your tests still pass, the tests are not doing
  their job.
- Hand-write BUILD files for third-party crates (once Bazel lands); use
  `crate_universe`.
