# -*- mode: python -*-
# Tilt entry point for the local k8s dev loop (Phase 3b).
#
# Run with:  just dev-k8s     (or `tilt up` if the kind cluster is ready)
# Stop with: Ctrl+C in the Tilt terminal, then `tilt down`.

# Safety rail: Tilt refuses to touch anything outside these contexts.
# Prevents accidental pushes to a real cluster if somebody's kubeconfig
# is in a weird state.
allow_k8s_contexts('kind-static-embedder')

# Build the app image from the committed Dockerfile. Docker's layer cache
# keeps rebuilds reasonable: Cargo.toml/lock changes rarely, so the deps
# layer is reused; the model-downloader stage is cached via its build
# args; only src/ edits force a full rebuild of the builder stage.
#
# Note: this path uses `cargo build --release` inside the container, not
# Bazel. Aligning the container build to Bazel (via rules_oci + a thin
# runtime Dockerfile) is a 3c optimization once the baseline works.
docker_build(
    'static-embedder',
    '.',
    dockerfile='Dockerfile',
    # Only files in these paths invalidate the image build. Everything
    # else (tests/, k8s/, docs) is ignored.
    only=[
        'Cargo.toml',
        'Cargo.lock',
        'src',
        'migrations',
        'static',
    ],
)

# Apply the local overlay. kustomize is built into kubectl 1.14+, and
# Tilt's `kustomize()` helper shells out to the local binary.
k8s_yaml(kustomize('./k8s/overlays/local'))

# Postgres resource: expose 5432 on the host for ad-hoc psql/DB tools.
k8s_resource(
    'postgres',
    port_forwards='5432:5432',
    labels=['infrastructure'],
)

# App resource: expose 8080 and gate on postgres being ready so the
# app's migration step doesn't race the DB.
k8s_resource(
    'static-embedder',
    port_forwards='8080:8080',
    resource_deps=['postgres'],
    labels=['app'],
)
