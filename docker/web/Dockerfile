# syntax=docker/dockerfile:1
# mtgfr-web — SolidStart (Vinxi/Nitro) BFF on distroless Node (DEPLOYMENT.md).

FROM oven/bun:1 AS deps
WORKDIR /client
COPY client/package.json client/bun.lock* ./
RUN bun install --frozen-lockfile

FROM node:22-bookworm AS build
WORKDIR /src
COPY --from=deps /client/node_modules ./client/node_modules
COPY client ./client
COPY proto ./proto
WORKDIR /src/client
# ADR 0032: Effect-gRPC + protobuf-es clients are gitignored; regenerate from proto here.
RUN sh scripts/gen.sh
# Empty `VITE_CARD_CDN` → Scryfall `named` fallback in the client.
ARG VITE_CARD_CDN=
ENV VITE_CARD_CDN=$VITE_CARD_CDN
# Vinxi needs a real Node for the production Nitro build.
RUN npx tsc --noEmit && npx vinxi build

FROM gcr.io/distroless/nodejs22-debian12:nonroot
WORKDIR /app
COPY --from=build /src/client/.output ./.output
ENV HOST=0.0.0.0
ENV PORT=8080
# Runtime: web.tf sets `API_UPSTREAM` / `GRPC_UPSTREAM` + `WEB_DATABASE_URL` (mtgfr_web).
# Game traffic is routed by BFF `table_routes` → pod DNS (ADR 0030).
EXPOSE 8080
USER nonroot:nonroot
CMD [".output/server/index.mjs"]
