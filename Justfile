# Command runner entry points. `just --list` shows this, too.

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
    check cargo "install via rustup: https://rustup.rs (fmt + clippy)"
    check docker "install Docker Engine or Docker Desktop"
    check bazel "install bazelisk: brew install bazelisk (Linux/macOS) or https://github.com/bazelbuild/bazelisk"
    check kind "install kind: brew install kind (or https://kind.sigs.k8s.io/docs/user/quick-start/#installation)"
    check kubectl "install kubectl: brew install kubectl (or https://kubernetes.io/docs/tasks/tools/)"
    check tilt "install tilt: brew install tilt-dev/tap/tilt (or https://docs.tilt.dev/install.html)"
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

# Creates the kind cluster if missing, then launches Tilt. Bazel builds
# the binary outside the container; a minimal Dockerfile wraps it.
# Service on localhost:8080, Postgres on localhost:5432.
# kind cluster + Tilt. The single inner-loop entry point.
dev:
    #!/usr/bin/env bash
    set -eu
    if ! kind get clusters 2>/dev/null | grep -q '^static-embedder$'; then
        echo "Creating kind cluster 'static-embedder'..."
        kind create cluster --name static-embedder --wait 120s
    fi
    exec tilt up

# Tear down the kind cluster entirely. Use when you want a clean slate.
reset-cluster:
    kind delete cluster --name static-embedder

# Note: fmt/clippy stay on Cargo until rules_rust ships equivalents we
# trust; tests run under Bazel.
# cargo fmt + clippy + `bazel test //...`. Matches CI.
check:
    cargo fmt -- --check
    cargo clippy --all-targets -- -D warnings
    bazel test //...

# Live-DB smoke test. Expects the postgres StatefulSet to be reachable
# on localhost:5432 (i.e., `just dev` running in another terminal).
test-live:
    DATABASE_URL=postgres://embedder:embedder@localhost:5432/embeddings \
        bazel test //tests:live_db --config=live

# Regenerate the crate_universe lockfile. Run after editing Cargo.toml.
bazel-repin:
    CARGO_BAZEL_REPIN=1 bazel fetch @crates//...

# Apply format changes and clippy-fixable lints.
fix:
    cargo fmt
    cargo clippy --all-targets --fix --allow-dirty --allow-staged -- -D warnings
