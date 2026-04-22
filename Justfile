# Command runner entry points. `just --list` shows this, too.
#
# Phase 3 will collapse `dev` into `dev-sync` (Bazel + Tilt into local k8s)
# and retire the Cargo / docker-compose paths. Until then, these are the
# honest documented inner loop.

_default:
    @just --list

# Verify prerequisites. Run me if anything feels off.
doctor:
    #!/usr/bin/env bash
    set -eu
    missing=0
    check() {
        if ! command -v "$1" >/dev/null 2>&1; then
            echo "MISSING  $1  ($2)"
            missing=1
        else
            echo "ok       $1"
        fi
    }
    check cargo "install via rustup: https://rustup.rs"
    check docker "install Docker Engine or Docker Desktop"
    check bazel "install bazelisk: brew install bazelisk (Linux/macOS) or https://github.com/bazelbuild/bazelisk"
    if ! docker compose version >/dev/null 2>&1; then
        echo "MISSING  docker compose plugin"
        missing=1
    else
        echo "ok       docker compose"
    fi
    if ! command -v rust-analyzer >/dev/null 2>&1; then
        echo "warn     rust-analyzer  (rustup component add rust-analyzer)"
    else
        echo "ok       rust-analyzer"
    fi
    if [ "${ENABLE_LSP_TOOL:-}" != "1" ]; then
        echo "warn     ENABLE_LSP_TOOL not set to 1 (see CLAUDE.md)"
    else
        echo "ok       ENABLE_LSP_TOOL"
    fi
    if [ "$missing" -ne 0 ]; then
        echo "fail: one or more prerequisites missing"
        exit 1
    fi
    echo "ok"

# Bring Postgres up and run the service against it.
dev:
    docker compose up -d
    DATABASE_URL=postgres://embedder:embedder@localhost:5432/embeddings cargo run

# Note: fmt/clippy stay on Cargo until rules_rust ships equivalents we
# trust; tests run under Bazel.
# cargo fmt + clippy + `bazel test //...`. Matches CI.
check:
    cargo fmt -- --check
    cargo clippy --all-targets -- -D warnings
    bazel test //...

# Live-DB smoke test against docker-compose Postgres.
test-live:
    docker compose up -d
    DATABASE_URL=postgres://embedder:embedder@localhost:5432/embeddings \
        bazel test //tests:live_db --config=live

# Regenerate the crate_universe lockfile. Run after editing Cargo.toml.
bazel-repin:
    CARGO_BAZEL_REPIN=1 bazel fetch @crates//...

# Drop the pgdata volume. Use after migration changes.
reset-db:
    docker compose down -v
    docker compose up -d

# Apply format changes and clippy-fixable lints.
fix:
    cargo fmt
    cargo clippy --all-targets --fix --allow-dirty --allow-staged -- -D warnings
