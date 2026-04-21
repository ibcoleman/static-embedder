# CLAUDE.md — Java flavour (draft)

> **Status: draft.** This document is the Java-flavoured twin of the Rust
> `CLAUDE.md` in the repo root. It is intended as a template for future
> Java projects where you control the language/platform and want comparable
> guardrails. Item-for-item commentary on what transfers cleanly and what
> gets rougher is included inline.

Project conventions and guardrails for AI coding agents working in Java
repos built to this template. Read this before writing code or running
commands.

## Philosophy

Same as the Rust flavour:

- **Types are guardrails.** Push errors and intent into the type system.
- **One dev path.** Every change flows through a single deploy command.
- **Tests that bite.** Properties to make wrong code hard. Mutation to make
  useless tests hard.

## Stack

- **Language:** Java 21+ (sealed interfaces + pattern matching are
  load-bearing below — do not back-port to 17 without rewriting the Result
  guidance).
- **Build:** Bazel with `rules_jvm_external` (= Java equivalent of Rust's
  `crate_universe`). No Maven `pom.xml`, no Gradle script as the authority.
  Dep versions live in a lockfile Bazel regenerates.
- **Local orchestration:** Tilt watching Bazel outputs, deploying into local
  k8s (`k3d` / `kind`). Identical to Rust flavour.
- **Command runner:** `just`. Identical to Rust flavour.
- **Framework:** Spring Boot 3 recommended, but kept thin. Prefer constructor
  injection + plain records + manual configuration over annotation-heavy
  auto-configuration. If Spring gets in the way of the type discipline,
  drop it (Helidon Nima or Micronaut are fine substitutes).
- **Deploy target:** Kubernetes.

## Dev loop

**Everything runs through `just dev-sync`.** This command drives a full
local k8s deploy via Bazel + Tilt and is the only supported inner loop.

- Do **not** introduce `./gradlew bootRun`, Spring DevTools hot reload,
  `mvn spring-boot:run`, JRebel, or any other hot-swap pathway. These
  present a state that does not match the real build and confuse both
  humans and agents.
- With Bazel caching, full redeploy-on-every-change is cheap in practice.
  Do not optimize for sub-minute iteration — the review loop dominates the
  clock.
- Run `just doctor` after clone or when things feel off. It verifies
  required binaries are on PATH (`bazel`, `tilt`, `jdtls` for the LSP
  plugin, JDK 21).

## Java conventions

### Error handling — the one genuinely rough spot

Java has no `?` operator and no union types. Two working approaches, pick
one per project and be consistent:

**Option A (recommended for most teams): Vavr `Either<E, T>`.**

- Return `Either<SomeError, Value>` for every fallible operation.
- Compose with `.flatMap`, `.map`, `.fold`. Never `.get()` without checking
  `.isRight()` first — treat `.get()` on an `Either` the same way Rust
  treats `.unwrap()` (forbidden in production code).
- Bridge libraries that throw: `Try.of(() -> libCall()).toEither()` at the
  seam, then stay in `Either`-land.

**Option B: sealed `Result` interface + records + pattern matching (Java
21+ native).**

```java
public sealed interface Result<E, T> permits Ok, Err {
    record Ok<E, T>(T value) implements Result<E, T> {}
    record Err<E, T>(E error) implements Result<E, T> {}
}
```

Consumers use `switch` with pattern matching:
```java
return switch (parse(input)) {
    case Ok<ParseError, Doc>(Doc d) -> persist(d);
    case Err<ParseError, Doc>(ParseError e) -> Result.err(new AppError.BadInput(e));
};
```

Slightly more ceremony than Vavr (no `.flatMap`/`.map` out of the box
unless you write them), but avoids the dep and leans on the language.

**Rule regardless of which you pick:** `throw` is reserved for bridging to
libraries that throw. Wrap at the seam. Do not throw inside application
code. This is the Java analogue of the Rust "no `unwrap()`" rule and of
TypeScript's "no throw" rule enforced by `eslint-plugin-neverthrow`.

### Newtypes for domain primitives

```java
public record UserId(UUID value) {}
public record OrderId(UUID value) {}
public record Cents(long value) {
    public Cents {
        if (value < 0) throw new IllegalArgumentException("negative cents");
    }
}
```

- `UserId` is not assignable to `OrderId` even though both wrap `UUID`.
  Free at the compiler level — records are value classes.
- Add a compact constructor for validation invariants as above.
- Never pass raw `String`, `long`, or `UUID` across an API boundary for
  values that have a domain name.

### Sealed interfaces for domain ADTs

```java
public sealed interface Command permits Command.Index, Command.Search {
    record Index(String text) implements Command {}
    record Search(String query, int k) implements Command {}
}
```

Combine with pattern-matching switch for exhaustive handling. The compiler
enforces exhaustiveness, same role as Rust's `match` on a `#[non_exhaustive]`
enum.

### Nullability

- `@NullMarked` package-level (JSpecify).
- **NullAway** runs as a compile-time error gate. Treat every NullAway
  violation as a compile failure, not a warning.
- Do **not** use `Optional` for fields or for method parameters —
  `Optional` is for return types and for fluent-API results only. Use
  sealed interfaces (`sealed Either<Absent, Present>`-style) for richer
  absent reasoning.

### Immutability

- Domain classes are `record`s or `final class`es with only `final` fields.
- Collections crossing module boundaries are `List.copyOf(...)` /
  `Map.copyOf(...)` or Guava `ImmutableList` / `ImmutableMap`. Never pass a
  mutable `ArrayList` out of a method.
- For "`ReadonlyDeep`" semantics, records + immutable collections are
  structurally enough. No runtime freeze required.

## Build and dependencies

- Third-party libs are managed through `rules_jvm_external` (= the Java
  `crate_universe`). Dep coordinates go in `maven_install(...)` in
  `MODULE.bazel`; Bazel generates the lock.
- Do **not** hand-write `BUILD.bazel` targets for external jars. Do not
  reintroduce a parallel `pom.xml` or Gradle script for the build.
- A `Justfile` target (`just deps-pin`) regenerates the lock.

## Testing

### Property-based tests — required for non-trivial logic

**jqwik** (= Rust `proptest` / TS `fast-check`).

```java
@Property
void embeddingsAreDeterministic(@ForAll("words") String text) {
    Embedding a = embedder.embed(text).get();
    Embedding b = embedder.embed(text).get();
    assertThat(a).isEqualTo(b);
}

@Provide
Arbitrary<String> words() {
    return Arbitraries.strings().alpha().ofMinLength(1).ofMaxLength(64);
}
```

jqwik shrinks failures to minimal repros the same way `proptest` does.

### Mutation testing — CI nightly / pre-merge

**PIT (`pitest.org`)** (= Rust `cargo-mutants` / TS `Stryker`).

- Mutation score is a first-class quality metric.
- **If a change drops the mutation score, it is a regression** — even if
  every example-based test still passes.
- Slow; schedule nightly or pre-merge, not on every commit.
- PIT integrates with Bazel via `rules_pitest` or a thin shell wrapper
  invoking `pitest.jar` against the compiled test classes.

### Example-based tests

JUnit 5 + AssertJ. Keep these as a sanity floor, not the primary quality
gate. Properties and mutation testing do the heavy lifting.

## Static analysis — the `tsconfig strict` equivalent

Java has no single "strict" flag, but a stack of compile-time tools does
the equivalent job:

| Purpose | Tool |
|---|---|
| Null-safety gate (no raw `NullPointerException` escapes) | **NullAway** (+ JSpecify `@NullMarked`) |
| Style + suspicious patterns | **Error Prone** |
| Deeper formal properties (side-effect control, type refinement) | **Checker Framework** — opt in per package, expensive to maintain broadly |
| Bytecode-level no-`throw` enforcement in app code | Custom Error Prone check or **ArchUnit** rule |

Run all of them as part of the Bazel `build`, not as separate optional
lint jobs. Warnings-as-errors for NullAway and Error Prone.

### Agent-route-arounds to forbid

Same spirit as the TS list in the Rust manifesto:

- No `@SuppressWarnings("all")` or `@SuppressWarnings("NullAway")` without
  a written justification on the same line.
- No raw casts (`(Foo) obj`) where a sealed-interface switch would work.
- No `Object` parameters where a generic or a sealed type would work.
- No reflection-based config that bypasses compile-time wiring. (Spring
  auto-wiring is allowed, but prefer explicit `@Configuration` beans over
  component scanning — scanning hides the dependency graph from the
  compiler and the agent.)

## LSP / agent tooling

- `ENABLE_LSP_TOOL=1` in the Codespace environment.
- **jdtls** (Eclipse JDT Language Server) on PATH.
- Claude Code plugin: the Java code-intelligence plugin from the official
  marketplace.
- `just doctor` verifies `jdtls`, `bazel`, `tilt`, and `ENABLE_LSP_TOOL`.

## Rules for the agent

**Do:**

- Route every build/deploy through `just dev-sync`.
- Model every fallible operation as `Either<E, T>` (Vavr) or
  `Result<E, T>` (sealed) — pick one per project and stick to it.
- Use `record` newtypes for IDs, money, timestamps with meaning, and any
  value that carries a domain name.
- Use sealed interfaces + pattern matching for domain ADTs.
- Add jqwik properties for any non-trivial new logic.
- Treat NullAway + Error Prone failures as build failures.
- Run `just doctor` when the toolchain feels off.

**Don't:**

- Add hot-reload paths (`./gradlew bootRun`, Spring DevTools, JRebel).
- `throw` in application code. Bridge at seams with `Try`/`try`, return a
  `Result`/`Either`, stay in monadic-error land thereafter.
- Use `Optional.get()` or `Either.get()` without a prior `.isPresent()` /
  `.isRight()` check — this is the Java equivalent of `.unwrap()`.
- Silence NullAway, Error Prone, or the Checker Framework without a
  justification comment.
- Pass raw `String`, `long`, or `UUID` across module boundaries for domain
  values. Wrap them in a `record`.
- Hand-write `BUILD.bazel` for third-party jars. Use `rules_jvm_external`.
- Ship example-only tests. If PIT can flip a comparison and your tests
  still pass, the tests are not doing their job.

## Roughness notes (what doesn't map cleanly)

| Rust rule | Java equivalent | Roughness |
|---|---|---|
| `Result<T, E>` + `?` | `Either<E, T>` (Vavr) or sealed `Result` | **Real:** no `?` operator, so flows are `.flatMap` chains or `switch` patterns. Readable once the team internalizes it; more verbose. |
| No `unwrap()` / `expect()` | No `.get()` on `Either` / `Optional`; NullAway | Clean via NullAway + team discipline. No single lint enforces "no `.get()`" on `Either` — consider an ArchUnit rule. |
| Sealed enums, exhaustive match | Sealed interfaces + pattern matching (Java 21) | Clean. |
| Newtype wrappers | `record UserId(UUID value) {}` | Clean. Records are strictly better than Lombok `@Value` here. |
| `proptest` | **jqwik** | Clean. |
| `cargo-mutants` / Stryker | **PIT** | Clean, mature. |
| Strict TS + `neverthrow` | NullAway + Error Prone + "no-throw in app code" enforced via custom check or ArchUnit | Spirit matches; enforcement is a stack of tools instead of one flag. |
| Bazel + `crate_universe` | Bazel + `rules_jvm_external` | Identical. |
| Tilt + k8s + `just` | Same | Identical. |

**The single biggest watch-out:** the `?` operator. Java code written in
this style reads noisier than Rust. If the team resists, the failure mode
is reverting to `throw` inside application code — at which point the
guardrail is gone. Enforcement must be bright-line: no raw `throw` passes
PR review.
