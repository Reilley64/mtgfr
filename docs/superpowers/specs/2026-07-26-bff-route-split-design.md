# BFF route split and h3 handler cutover (design)

**Status:** Implemented (2026-07-26). Shipped behavior is documented in living specs: [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md) (BFF file-per-op map, `defineHandler`), [coverage-by-set](2026-07-26-coverage-by-set.md) (`api/meta/coverage/v1.get.ts`), [lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md), [lobby-entry-ui](2026-07-20-lobby-entry-ui.md).
**Surfaces:** Nitro BFF `client/server/routes/api/**`, `client/server/lobby-http.ts`, `client/app/domain/lobby/client.ts`, `shell/lobby/update.ts`.

---

## Problem Statement

Lobby and meta HTTP live in one catch-all `api/[...path].ts` that dispatches by method + path string. Handlers use h3 v1 aliases (`defineEventHandler`, `getMethod`, `getRequestHeader`, `readRawBody`) and wrap named functions instead of exporting `defineHandler` directly. Join/ready/start put required `table_id` in the JSON body, which conflicts with the project routing rule that required identifiers belong in path params.

## Goal

- One Nitro route file per lobby/meta operation.
- Cut over touched routes to Nitro 3 / h3 v2 preferred APIs.
- Export `defineHandler` inline as the default export (no `handle`/`forward` alias).
- Move join/ready/start `table_id` into the path; drop it from bodies.
- Keep `api/rpc/[...path].ts` as a single catch-all, with the same handler-style cutover.

## Locked decisions

| Decision | Choice |
|---|---|
| Scope | Lobby + meta split; rpc stays catch-all with rename/inline export; also cut over `health.get.ts` and `faro/collect.ts` |
| Approach | Nitro file-per-op + shared `client/server/lobby-http.ts` helpers |
| Join/ready/start paths | Nested: `POST /api/tables/{table}/join|ready|start/v1` |
| Bodies | join `{ deck_id }`; ready `{ ready }`; start empty/`{}` |
| Handler style | `export default defineHandler(async (event) => { ... })` |
| Route cache | No `defineCachedHandler` (coverage keeps domain SWR caches; images stay client `sharedImageCache`) |
| Proto / gRPC | Unchanged |

## Approaches considered

1. **Nitro file-per-op + small shared helpers (chosen)** — Clear ownership, DRY auth/tracing, matches Nitro routing.
2. Fat domain handlers with one-line route re-exports — Fights direct `defineHandler` export; extra layer.
3. Split only meta; keep lobby catch-all — Smaller diff; misses one-file-per-op and path-param clarity.

## Design

### File map

Delete `client/server/routes/api/[...path].ts`. Add:

| File | Route |
|---|---|
| `api/meta/health/v1.get.ts` | `GET /api/meta/health/v1` |
| `api/meta/version/v1.get.ts` | `GET /api/meta/version/v1` |
| `api/meta/coverage/v1.get.ts` | `GET /api/meta/coverage/v1` |
| `api/tables/v1.post.ts` | `POST /api/tables/v1` |
| `api/tables/[table]/lobby/v1.get.ts` | `GET /api/tables/{table}/lobby/v1` |
| `api/tables/[table]/route/v1.delete.ts` | `DELETE /api/tables/{table}/route/v1` |
| `api/tables/[table]/join/v1.post.ts` | `POST /api/tables/{table}/join/v1` |
| `api/tables/[table]/ready/v1.post.ts` | `POST /api/tables/{table}/ready/v1` |
| `api/tables/[table]/start/v1.post.ts` | `POST /api/tables/{table}/start/v1` |

Also cut over (no split): `api/rpc/[...path].ts`, `api/health.get.ts`, `api/faro/collect.ts`.

### Shared helpers (`client/server/lobby-http.ts`)

- `json(data, status?)` → `Response`
- Session cookie → `grpcRequestEnv`
- Auth gate (`fetchMe` → 401)
- Web DB + `sweepWebDb`
- Traced request + LobbyDb 500 JSON shaping
- `tableParam(event)` from `event.context.params.table`

Domain logic stays in `lobby-store`, `coverage-meta`, `api-upstream-auth` (no logic move into helpers beyond HTTP glue).

### h3 / Nitro deprecation cutover (touched files only)

| Current | Replace with |
|---|---|
| `defineEventHandler` | `defineHandler` |
| `getMethod(event)` | `event.req.method` |
| `getRequestHeader(event, name)` | `event.req.headers.get(name)` |
| `readRawBody(event, "utf8")` | `await event.req.text()` |
| `readRawBody(event, false)` | `new Uint8Array(await event.req.arrayBuffer())` |
| Named handler + wrap | Inline `export default defineHandler(async (event) => { ... })` |

**Keep:** `getCookie` / `setCookie` / `deleteCookie`, `getRequestURL` (rpc searchParams), `H3Event` type, `definePlugin`.

Table params use `event.context.params.table` (Nitro docs). `normalizePublicApiPath` remains for non-lobby catch-all needs; lobby/meta no longer use it.

### Client wire

Update `client/app/domain/lobby/client.ts` and `shell/lobby/update.ts`:

- `joinTable(tableId, { deck_id })` → `POST tables/{table}/join/v1`
- `readyUp(tableId, { ready })` → `POST tables/{table}/ready/v1`
- `startGame(tableId)` → `POST tables/{table}/start/v1`

Preserve lobby view error codes, auth 401, LobbyDb 500 body, Faro proxy semantics, tracing.

### Living specs at implement time

Update in the same change:

- [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md) — per-op file map; drop mega-route row
- [lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md) / [lobby-entry-ui](2026-07-20-lobby-entry-ui.md) — nested join/ready/start if they document flat verbs
- [coverage-by-set](2026-07-26-coverage-by-set.md) — module path `api/meta/coverage/v1.get.ts`

## Testing Decisions

- Update `lobby/client.test.ts` for new paths and function signatures.
- Keep `rpc-method-gate.test.ts` importing the rpc default export.
- Assert join/ready/start use path `table` and bodies without `table_id`.
- Verify with `just client-typecheck` and focused Vitest (lobby client, rpc gate, faro if body path changes).
- No new Scene suite unless a shell surface changes (not expected).

## Out of Scope

- `defineCachedHandler` for coverage or images
- Splitting Effect RPC into one file per method
- Proto / tonic / gRPC contract changes
- Changing create-table response shape or meta JSON shapes
- Unrelated app-wide h3 migrations outside `client/server/routes/api/**` (+ the new helper module)

## Further Notes

- Approved in brainstorming: scope B, path-param move (nested under table), Approach 1.
- Coverage Scryfall denominators remain domain in-memory SWR caches; card art remains `sharedImageCache`.
