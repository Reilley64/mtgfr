# syntax=docker/dockerfile:1
# mtgfr-server — distroless API (DEPLOYMENT.md). Same image runs `server serve` and `server migration …`.

FROM rust:1-bookworm AS build
WORKDIR /src
RUN apt-get update && apt-get install -y --no-install-recommends protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY config ./config
COPY Toasty.toml ./
COPY toasty ./toasty

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build -p server --release \
    && cp target/release/server /server

FROM gcr.io/distroless/cc-debian12:nonroot
WORKDIR /
COPY --from=build /server /server
COPY --from=build /src/config/mtgfr.toml /config/mtgfr.toml
COPY --from=build /src/Toasty.toml /Toasty.toml
COPY --from=build /src/toasty /toasty
ENV HOST=0.0.0.0
ENV PORT=8080
EXPOSE 8080
USER nonroot:nonroot
CMD ["/server", "serve"]
