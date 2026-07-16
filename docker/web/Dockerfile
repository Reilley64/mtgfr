# syntax=docker/dockerfile:1
# mtgfr-web — SolidStart (Vinxi/Nitro) BFF on distroless Node (DEPLOYMENT.md).

FROM oven/bun:1 AS deps
WORKDIR /client
COPY client/package.json client/bun.lock* ./
RUN bun install --frozen-lockfile

FROM node:22-bookworm AS build
WORKDIR /client
COPY --from=deps /client/node_modules ./node_modules
COPY client ./
# `src/api/generated.ts` is produced on the host/CI via `just server-codegen` before
# `docker build` (gitignored — not committed).
# Empty `VITE_CARD_CDN` → Scryfall `named` fallback in the client.
ARG VITE_CARD_CDN=
ENV VITE_CARD_CDN=$VITE_CARD_CDN
# Vinxi needs a real Node for the production Nitro build.
RUN npx tsc --noEmit && npx vinxi build

FROM gcr.io/distroless/nodejs22-debian12:nonroot
WORKDIR /app
COPY --from=build /client/.output ./.output
ENV HOST=0.0.0.0
ENV PORT=8080
# In-cluster Axum behind nginx sticky (overridden in web.tf).
ENV API_UPSTREAM=http://edh-api-proxy.edh.svc.cluster.local:8080
EXPOSE 8080
USER nonroot:nonroot
CMD [".output/server/index.mjs"]
