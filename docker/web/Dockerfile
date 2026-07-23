# syntax=docker/dockerfile:1
# mtgfr-web — Foldkit SPA + Nitro BFF on distroless Node (see production-topology spec).

FROM oven/bun:1 AS deps
WORKDIR /client
COPY client/package.json client/bun.lock* ./
RUN bun install --frozen-lockfile

FROM oven/bun:1 AS build
WORKDIR /src
COPY --from=deps /client/node_modules ./client/node_modules
COPY client ./client
COPY proto ./proto
# Token codegen (`bun run gen:tokens`) reads repo-root DTCG JSON — must be present at build.
COPY design.tokens.json ./design.tokens.json
WORKDIR /src/client
# Empty `VITE_CARD_CDN` → Scryfall `named` fallback in the client.
ARG VITE_CARD_CDN=
ENV VITE_CARD_CDN=$VITE_CARD_CDN
ARG APP_VERSION=dev
ARG GIT_COMMIT=unknown
# Faro / Vite need these at *build* time; BFF OTEL reads APP_VERSION / GIT_COMMIT at runtime.
ENV VITE_APP_VERSION=$APP_VERSION
ENV VITE_GIT_COMMIT=$GIT_COMMIT
ENV APP_VERSION=$APP_VERSION
ENV GIT_COMMIT=$GIT_COMMIT
# `bun run build` regenerates gitignored Effect-gRPC clients, typechecks, then writes Nitro/Vite output.
RUN bun run build

FROM gcr.io/distroless/nodejs22-debian12:nonroot
WORKDIR /app
# Nitro writes `.output/public` and `.output/server`; keep both under the runtime root.
COPY --from=build /src/client/.output ./.output
ARG APP_VERSION=dev
ARG GIT_COMMIT=unknown
ENV HOST=0.0.0.0
ENV PORT=8080
ENV APP_VERSION=$APP_VERSION
ENV GIT_COMMIT=$GIT_COMMIT
# Runtime: web.tf sets `API_UPSTREAM` / `GRPC_UPSTREAM` + `WEB_DATABASE_URL` (mtgfr_web).
# Game traffic is routed by BFF `table_routes` → pod DNS (ADR 0030).
EXPOSE 8080
USER nonroot:nonroot
CMD [".output/server/index.mjs"]
