# syntax=docker/dockerfile:1.7
#
# Thin runtime image for static-embedder.
#
# The binary is produced OUTSIDE this file by `bazel build //:static-embedder`
# and passed in as a build context file. Tilt's `custom_build` orchestrates
# the bazel invocation + staging + docker build; see Tiltfile for the glue.
#
# Stages:
#   1. `model-downloader` — pull Model2Vec weights once at build time. Cached
#                           across rebuilds unless MODEL_REPO changes.
#   2. final               — distroless/cc-debian12:nonroot + binary + model.
#
# Result: ~40-50 MB image (binary is ~36 MB; weights ~8 MB; base <10 MB).
# Incremental rebuilds: just the binary layer.

FROM debian:bookworm-slim AS model-downloader
ARG MODEL_REPO=minishlab/potion-retrieval-32M
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN mkdir -p /opt/model \
    && curl -fsSL -o /opt/model/tokenizer.json    "https://huggingface.co/${MODEL_REPO}/resolve/main/tokenizer.json" \
    && curl -fsSL -o /opt/model/model.safetensors "https://huggingface.co/${MODEL_REPO}/resolve/main/model.safetensors" \
    && curl -fsSL -o /opt/model/config.json       "https://huggingface.co/${MODEL_REPO}/resolve/main/config.json"

# Runtime base must have glibc new enough for the Bazel-built binary
# (currently GLIBC_2.39 required). Debian 13 (trixie) ships glibc 2.41.
# distroless's Debian 12 variant is too old as of mid-2025; when
# distroless publishes a Debian 13 cc image, swap to that for the
# smaller footprint. debian:trixie-slim is ~75 MB vs distroless
# cc's ~25 MB — acceptable tradeoff for glibc compatibility.
FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 65532 --user-group nonroot
# The binary is expected alongside the Dockerfile in the build context,
# staged there by Tilt's custom_build from `bazel-bin/static-embedder`.
COPY static-embedder /usr/local/bin/static-embedder
COPY --from=model-downloader /opt/model /opt/model
ENV MODEL_ID=/opt/model \
    BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=info
EXPOSE 8080
USER nonroot
ENTRYPOINT ["/usr/local/bin/static-embedder"]
