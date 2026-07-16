# syntax=docker/dockerfile:1
# mtgfr-web — Vite SPA + distroless `server static` with gzip (DEPLOYMENT.md).

FROM oven/bun:1 AS assets
WORKDIR /client
COPY client/package.json client/bun.lock* ./
RUN bun install --frozen-lockfile
COPY client ./
# `src/api/generated.ts` is produced on the host/CI via `just server-codegen` before
# `docker build` (gitignored — not committed). `tsc`/`vite` need it in the build context.
# Build-args (not just ENV) so `docker build --build-arg …` can target a different API/CDN
# without editing this file. Empty `VITE_CARD_CDN` → Scryfall `named` fallback in the client.
ARG VITE_API_ORIGIN=https://api.edh.example.com
ARG VITE_CARD_CDN=
ENV VITE_API_ORIGIN=$VITE_API_ORIGIN
ENV VITE_CARD_CDN=$VITE_CARD_CDN
RUN bunx tsc --noEmit && bunx vite build

FROM rust:1-bookworm AS server-build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build -p server --release \
    && cp target/release/server /server

FROM gcr.io/distroless/cc-debian12:nonroot
WORKDIR /
COPY --from=server-build /server /server
COPY --from=assets /client/dist /dist
ENV HOST=0.0.0.0
ENV PORT=8080
ENV STATIC_ROOT=/dist
EXPOSE 8080
USER nonroot:nonroot
CMD ["/server", "static"]
