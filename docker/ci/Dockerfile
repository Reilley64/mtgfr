# syntax=docker/dockerfile:1
# mtgfr-ci — thin toolchain for server verify (Rust + protoc + just + nextest).
# Not the Cloud agent image; not the production server/web images.

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive \
    CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:/usr/local/bin:${PATH}

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    git \
    pkg-config \
    protobuf-compiler \
    python3 \
    && rm -rf /var/lib/apt/lists/*

# Non-root user for GHA container: jobs (matches ubuntu runner convention).
RUN id -u ubuntu >/dev/null 2>&1 || useradd -m -s /bin/bash ubuntu \
    && mkdir -p /usr/local/cargo /usr/local/rustup \
    && chown -R ubuntu:ubuntu /usr/local/cargo /usr/local/rustup

USER ubuntu
WORKDIR /home/ubuntu

# Rust stable + rustfmt/clippy (same components as verify-jobs.yml).
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile minimal -c rustfmt -c clippy

# Pin tool versions known-good with current CI (bump when rebuilding the image).
RUN cargo install just --version 1.56.0 --locked \
    && cargo install cargo-nextest --version 0.9.140 --locked

# Smoke at build time so a broken image fails the publish job.
RUN rustc --version \
    && cargo --version \
    && rustfmt --version \
    && cargo clippy --version \
    && protoc --version \
    && just --version \
    && cargo nextest --version
