# -*- mode: python -*-
# Tilt entry point for the local k8s dev loop.
#
# Run with:  just dev       (or `tilt up` if the kind cluster is ready)
# Stop with: Ctrl+C in the Tilt terminal, then `tilt down`.

# Safety rail: Tilt refuses to touch anything outside these contexts.
# Prevents accidental pushes to a real cluster if somebody's kubeconfig
# is in a weird state.
allow_k8s_contexts('kind-static-embedder')

# Build the binary via Bazel, then wrap it in the runtime Dockerfile.
#
# This is the "Bazel is the engine" path: cargo compile happens via
# `bazel build //:static-embedder` outside the container, the binary
# lands in `bazel-bin/static-embedder`, we stage it next to the
# Dockerfile, and docker build produces the final image. Incremental
# rebuilds hit Bazel's cache — if src/ didn't change, the binary
# rebuild is ~1s and only the tail end of the Docker build runs.
#
# The $EXPECTED_REF env var is set by Tilt; docker build tags the
# image with it and Tilt handles loading into the kind cluster.
custom_build(
    ref='static-embedder',
    command='''
        set -eu
        bazel build //:static-embedder
        STAGE=$(mktemp -d)
        trap "rm -rf $STAGE" EXIT
        cp -L bazel-bin/static-embedder "$STAGE/static-embedder"
        cp Dockerfile "$STAGE/Dockerfile"
        docker build -t "$EXPECTED_REF" "$STAGE"
    ''',
    deps=[
        './src',
        './migrations',
        './Cargo.toml',
        './Cargo.lock',
        './BUILD.bazel',
        './MODULE.bazel',
        './MODULE.bazel.lock',
        './Dockerfile',
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
