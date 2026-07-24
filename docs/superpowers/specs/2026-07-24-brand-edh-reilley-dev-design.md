# Brand: edh.reilley.dev (display + public identity)

**Status:** Design (approved for planning)  
**Date:** 2026-07-24  
**Module:** Client shell wordmark / document title; Scryfall + tooling HTTP User-Agent strings; `README.md`; `DESIGN.md`  
**Approach:** String replace at call sites (no shared product-name constant module)

---

## Problem

The player-facing product string is still **`mtgfr`**, while the public site and operator mental model are **`edh.reilley.dev`**. Outbound Scryfall calls identify as `mtgfr/0.1`. Display brand and public HTTP identity should match the hostname players use.

---

## Goals

- Show **`edh.reilley.dev`** as the wordmark and document title everywhere the UI currently shows `mtgfr`.
- Identify Scryfall (and related tooling) requests as `edh.reilley.dev/0.1`.
- Update the repo README title and design-system heading to the same brand string.

## Non-goals

- Renaming DBs (`mtgfr`, `mtgfr_web`), proto (`mtgfr.v1`), GHCR images, K8s secrets/labels, npm/cargo package names, clap CLI name (`mtgfr`), Terraform example hostname (`edh.example.com`), localStorage keys (`mtgfr.*`), Faro/OTEL service names (`edh-web` / `edh-api`), Style Dictionary format ids (`mtgfr/tailwind-theme`), or a full docs/prose sweep of historical “mtgfr” mentions.
- New logos, favicons, OG/meta SEO, or marketing pages.
- Short brand variants (`EDH`) — wordmark is always the full hostname.

---

## User stories

- As a player, I see **edh.reilley.dev** in the browser tab, nav brand link, auth hero, and lobby hero.
- As an operator reading the repo or design system, the top-level product name matches that hostname.
- As Scryfall (or a human inspecting request logs), client and tooling User-Agents identify the project as `edh.reilley.dev/0.1`.

---

## Behavior

### Player-facing wordmark

Exact replacement string: **`edh.reilley.dev`** (lowercase hostname, no scheme).

| Surface | Location |
|---------|----------|
| HTML `<title>` | `client/index.html` |
| Foldkit `Document.title` | `client/app/view.ts` |
| Nav brand link text | `client/app/view.ts` |
| Auth panel hero | `client/app/shell/auth/view.ts` |
| Lobby panel hero | `client/app/shell/lobby/view.ts` |

No layout, typography, or color changes — text only.

### Docs titles

| Doc | Change |
|-----|--------|
| `README.md` | `# edh.reilley.dev` (keep body prose that refers to the codebase/repo as needed; do not rename the GitHub repo in this change) |
| `DESIGN.md` | `# Design System: edh.reilley.dev` |

### Public User-Agent

Replace `mtgfr/0.1` with **`edh.reilley.dev/0.1`** at each call site:

- `client/lib/deck-builder/scryfall.ts`
- `tooling/backfill-oracle.mjs`
- `tooling/backfill-card-ids.mjs`
- `tooling/backfill-card-meta.mjs`
- `tooling/backfill-otags.mjs`
- `tooling/rewrite-precon-fixtures.mjs`
- `tooling/rewrite-grind-precon-fixtures.mjs`
- `tooling/analyze-otags.mjs` → `edh.reilley.dev/0.1 (otag analysis)`

No shared constant module; duplicate the string at each site (YAGNI for a fixed public hostname).

---

## Testing

- Update Scene (or chrome) assertions that expect the wordmark text `mtgfr` to expect `edh.reilley.dev` (auth/lobby/nav/title as covered today).
- No new surfaces. Interaction/UI checklist applies only if Scene wordmark assertions are added/changed.
- Unit tests that assert DB names containing `mtgfr` (e.g. `mtgfr_web`) stay unchanged.

---

## Spec follow-up

If the shell spec mentions the product string `mtgfr` as UI copy, update that line in [client-shell-deck-builder-and-observability](2026-07-20-client-shell-deck-builder-and-observability.md) in the same implementation change. Do not rewrite unrelated historical docs.
