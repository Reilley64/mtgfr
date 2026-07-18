# syntax=docker/dockerfile:1
# mtgfr-server — distroless API (DEPLOYMENT.md). Same image runs `server serve` and `server migration …`.

FROM rust:1-bookworm AS build
WORKDIR /src

# tonic-build needs `protoc` for ADR 0032 stubs (OUT_DIR; never committed).
RUN apt-get update \
  && apt-get install -y --no-install-recommends protobuf-compiler \
  && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY proto ./proto
COPY config ./config
COPY Toasty.toml ./
COPY toasty ./toasty

# `--no-default-features` drops the test-only `test-sqlite` feature (rusqlite).
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build -p server --release --no-default-features \
    && cp target/release/server /server

FROM gcr.io/distroless/cc-debian12:nonroot
WORKDIR /
COPY --from=build /server /server
COPY --from=build /src/config/mtgfr.toml /config/mtgfr.toml
COPY --from=build /src/Toasty.toml /Toasty.toml
COPY --from=build /src/toasty /toasty
# cards::load_from_data_dir bakes CARGO_MANIFEST_DIR (/src/crates/cards) into the binary —
# the TOML pool must live at that path in the runtime image too.
COPY --from=build /src/crates/cards/data /src/crates/cards/data
ARG APP_VERSION=dev
ARG GIT_COMMIT=unknown
ENV HOST=0.0.0.0
ENV PORT=8080
ENV APP_VERSION=$APP_VERSION
ENV GIT_COMMIT=$GIT_COMMIT
EXPOSE 8080
USER nonroot:nonroot
CMD ["/server", "serve"]
