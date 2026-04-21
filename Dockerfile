# syntax=docker/dockerfile:1.7
#
# Multi-stage build:
#   1. `builder`          — compile the release binary with the full Rust toolchain.
#   2. `model-downloader` — pull the Model2Vec weights once at build time so the
#                           runtime container has no network dependency on boot.
#   3. final              — distroless/cc-debian12:nonroot with just the binary
#                           and the model files. No shell, no package manager.
#
# Result: ~30-40 MB image with a predictable cold start.

FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
# `include_str!("../../static/index.html")` and `sqlx::migrate!("./migrations")`
# resolve these directories at compile time, so they must be in the build
# context. Tests are not — the release build ignores `tests/`.
COPY migrations ./migrations
COPY static ./static
RUN cargo build --release --locked --bin static-embedder \
    && strip target/release/static-embedder

FROM debian:bookworm-slim AS model-downloader
ARG MODEL_REPO=minishlab/potion-retrieval-32M
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN mkdir -p /opt/model \
    && curl -fsSL -o /opt/model/tokenizer.json    "https://huggingface.co/${MODEL_REPO}/resolve/main/tokenizer.json" \
    && curl -fsSL -o /opt/model/model.safetensors "https://huggingface.co/${MODEL_REPO}/resolve/main/model.safetensors" \
    && curl -fsSL -o /opt/model/config.json       "https://huggingface.co/${MODEL_REPO}/resolve/main/config.json"

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder          /app/target/release/static-embedder /usr/local/bin/static-embedder
COPY --from=model-downloader /opt/model                           /opt/model
ENV MODEL_ID=/opt/model \
    BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=info
EXPOSE 8080
USER nonroot
ENTRYPOINT ["/usr/local/bin/static-embedder"]
