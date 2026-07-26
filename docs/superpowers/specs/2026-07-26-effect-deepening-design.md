# Deepen Effect integration on the client (design)

**Status:** Approved design input (2026-07-26).
**Surfaces (update at implement time):** [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md), [lobby-entry-ui](2026-07-20-lobby-entry-ui.md), [lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md), [wire-protocol-and-visibility](2026-07-20-wire-protocol-and-visibility.md), [coverage-by-set](2026-07-26-coverage-by-set.md), [deck-list-and-builder](2026-07-20-deck-list-and-builder.md) (Wave 5 Scryfall only), production-topology notes if Drizzle/runtime wiring changes.
**Upstream patterns:** `client/app/domain/rpc-client.ts`, `client/app/resources.ts`, Foldkit Commands/Subscriptions; Drizzle Effect Postgres (`drizzle-orm/effect-postgres` + `@effect/sql-pg`).

---

## Problem Statement

The browser `/api/rpc` path already follows the house Effect style (`RpcClient` + Schema + Foldkit Commands). Lobby/meta HTTP, BFF gRPC dispatch, and `mtgfr_web` Drizzle access still return `Promise` and get wrapped in `Effect.tryPromise` / `runPromise` bridges. That erases typed errors, triples runtime hops on `/api/rpc`, and leaves `lobby-store` / `withLobbyAuth` half-Effect. The old assumption that Drizzle could not target Effect v4 is obsolete: `drizzle-orm@1.0.0-rc` ships native `effect-postgres` against `@effect/sql-pg`.

## Goal

1. Effect owns first-party async work on the Foldkit client and Nitro BFF domain path.
2. Foldkit `Model` / `update` / `view` stay pure and synchronous; side effects only via Commands, Subscriptions, and Mounts.
3. Nitro remains the HTTP host (`defineHandler` edge); no host swap to `@effect/platform-bun`, no `defineEffectHandler`.
4. Collapse Promise islands in ranked waves, updating living surface specs in the same change as each wave.

## Locked decisions

| Decision | Choice |
|---|---|
| Design scope | Full multi-wave roadmap in this doc (not Wave 1-only) |
| Approach | Parallel twin clients — `LobbyClient` mirrors `RpcClient`; do not unify rpc+lobby under one HttpApi yet |
| Lobby/meta errors | Tagged transport/decode errors (`Schema.TaggedErrorClass`); Commands map tags to UI; keep `LobbyView.error` body codes for domain outcomes |
| Browser DI | `makeClient(fetch)` + `Context.Service` `LobbyClient` + `Layer` in `resources.ts` |
| BFF gRPC | Full cutover — Effect API on `grpcClient`, `dispatchRpc` as `Effect.gen`; Nitro `/api/rpc` is the only `runTracedRequest` edge for dispatch |
| SQL / Drizzle | Own wave: bump to `drizzle-orm@rc` / `drizzle-kit@rc`, replace pg-proxy with `drizzle-orm/effect-postgres`, Effect `WebDb` Layer, Effect `lobby-store` |
| HTTP helpers | After SQL wave: `withLobbyAuth` / meta helpers take Effect bodies → `Response`; still export `defineHandler` |
| Optional cleanup | Scryfall `searchPrints` as Effect; `streamDeltas` → `Stream` (drop callback bridge) |
| Host / helpers | Keep Nitro; no `defineEffectHandler`; no `@effect/platform-bun` unless a Layer truly needs it later |
| Effect pins | Keep `effect` and direct `@effect/*` on the same exact beta (currently `4.0.0-beta.101`) |

## Approaches considered

### Program shape

1. **Parallel twin clients + staged BFF/SQL (chosen)** — Matches existing `RpcClient` house style; independent shippable waves.
2. **One shared browser HTTP / HttpApi service** — Larger blast radius; forces unify while rpc is hand-written REST-over-BFF and lobby is path-per-op REST.
3. **Effect only behind existing Promise façades** — Leaves `tryPromise` forever; conflicts with tagged errors and Service DI.

### SQL timing

1. **Dedicated Drizzle RC + Effect store wave, then HTTP helper cutover (chosen)** — Isolates RC bump risk from handler churn.
2. **Merge SQL + `withLobbyAuth` in one PR** — Fewer PRs; harder to bisect RC/schema breakages.
3. **Defer SQL indefinitely** — Rejected once Effect v4 Drizzle RC is available and pg-proxy is the main BFF bridge.

## Design

### House style (non-negotiable)

```ts
// Commands depend on services; no first-party Effect.tryPromise over our own Promise clients.
Effect.gen(function* () {
  const lobby = yield* LobbyClient
  return yield* lobby.joinTable(tableId, { deck_id: deckId }).pipe(
    Effect.map((view) => ReceivedLobbyView({ view })),
    Effect.catchTag("LobbyHttpError", () => Effect.succeed(LobbyRequestFailed({ message: "Unreachable" }))),
    // …other tags → existing lobbyErrorCopy codes
  )
})
```

- `update` never `await`s or `Effect.run*`.
- `run*` only at edges: Foldkit runtime resources, Nitro `runTracedRequest`, and (until fully layered) process SQL runtime provision for BFF handlers.

### Wave plan

| Wave | Deliverable | Living specs to update |
|---|---|---|
| **1** | Effect `LobbyClient` for `/api/tables/**` + `/api/meta/**`; tagged errors; Commands + `lobbyPoll` drop `tryPromise` | shell-routes-and-auth, lobby-entry-ui, coverage-by-set |
| **2** | Effect `grpcClient` public API; `dispatchRpc` as `Effect.gen` with typed failures; `/api/rpc` only `runTracedRequest` edge; delete Promise sandwich | wire-protocol-and-visibility, shell-routes-and-auth |
| **3** | `drizzle-orm@rc` + `drizzle-kit@rc`; `drizzle-orm/effect-postgres` + existing `@effect/sql-pg`; Effect `WebDb` Layer; rewrite `lobby-store` (and DB construction) as Effect; remove pg-proxy bridge in `server/db/client.ts` | lobby-table-routing-and-live-game, shell-routes-and-auth; topology only if runtime/entry notes change |
| **4** | `withLobbyAuth` / `runMetaGet` take `Effect` → `Response`; route files stay `defineHandler` | lobby-table-routing-and-live-game, shell-routes-and-auth |
| **5** | Optional: Scryfall `searchPrints` as Effect; `streamDeltas` returns `Stream` (no `Stream.callback` bridge) | deck-list-and-builder and/or game stream notes as touched |

Each wave is a separate PR (or stacked commits) that leaves `just client-check` green.

### Wave 1 — Browser `LobbyClient`

**Modules:** `client/app/domain/lobby/client.ts`, new tagged errors (colocated or `domain/lobby/errors.ts`), `client/app/resources.ts`, `shell/lobby/update.ts`, `shell/lobby/poll.ts`, `shell/coverage/update.ts`, `fetch-api-version.ts`, tests.

**API shape:** Mirror `rpc-client.ts`:

- `makeClient(fetchImpl)` for tests; app singleton with `credentials: "include"` and URL prefix `/api`.
- Methods: `createTable`, `joinTable`, `readyUp`, `startGame`, `lobbyState`, `apiMeta`, `coverageMeta` (same product paths as today).
- Success: Schema-decode JSON bodies (`LobbyView`, created table, meta/coverage DTOs).
- Failure: fail with tagged errors (below), not `null`.

**Transport tagged errors** (distinct from `LobbyView.error` domain codes such as `UnknownTable` / `NotHost`):

| Tag | When |
|---|---|
| `LobbyUnauthorized` | HTTP 401 |
| `LobbyNotFound` | HTTP 404 when the body is not a usable success DTO |
| `LobbyBadRequest` | HTTP 400 when relevant |
| `LobbyHttpError` | Other non-2xx or network failure (include status when known) |
| `LobbyDecodeError` | Body present but Schema decode fails |

**Command mapping:** `catchTag` (and related) map to existing `LobbyRequestFailed` / UI codes in `lobbyErrorCopy`. Prefer keeping user-visible strings stable; use a more specific code than `Unreachable` when a tag clearly matches (e.g. unauthorized → sign-in path if product already handles it; 404 without `LobbyView` → stale-link / `UnknownTable` copy where that is already the UX).

**DI:**

```ts
export class LobbyClient extends Context.Service<LobbyClient, Client>()("LobbyClient") {}
// resources.ts: Layer.merge(Layer.succeed(RpcClient, client), Layer.succeed(LobbyClient, lobbyClient))
```

**Poll:** `lobbyPoll` uses `LobbyClient.lobbyState` (or injected Effect) and no longer types success as `LobbyView | null` from a Promise helper. Soft-fail policy for poll (skip emit vs surface error) stays product-consistent with today’s “filter nulls” behavior, implemented via Effect recovery rather than `null` returns from the client module.

### Wave 2 — BFF gRPC Effect dispatch

**Modules:** `client/app/domain/wire/grpcClient.ts`, `rpcServer.ts`, `server/routes/api/rpc/[...path].ts`, related tests.

**Target flow:**

1. Route: method gate, cookies, body parse (Nitro/`Response` world).
2. `runTracedRequest(traceparent, spanName, dispatchRpc(...))` once.
3. `dispatchRpc` is `Effect.gen` that calls Effect gRPC helpers and maps outcomes.
4. Route maps outcome → JSON / SSE `Response` and cookie set/clear (cookie side effects may remain imperative after the Effect completes, as today).

**Delete:** Promise façade used only to re-enter Effect (`runPromise` inside `grpcClient` for dispatch callers, `Effect.tryPromise` around `dispatchRpc` in the route). Keep explicit `outboundTraceparent` passing — separate ManagedRuntimes for OTEL vs gRPC remain until a later unification is justified; do not reintroduce Node ALS.

**Errors:** Prefer typed failures (`GrpcCallError` / Schema) inside the Effect; HTTP status mapping stays at the route/`outcomeToResponse` boundary.

### Wave 3 — Drizzle RC + Effect store

**Modules:** `client/package.json` pins, `client/server/db/client.ts`, `client/db/schema.ts` (only as required by RC), `client/drizzle.config.ts`, `client/app/domain/lobby-store.ts`, migrate scripts/tests.

**Target:**

- Depend on `drizzle-orm@rc` and `drizzle-kit@rc` aligned with docs for Effect Postgres.
- Replace `drizzle-orm/pg-proxy` + `runtime.runPromise` callback with `drizzle-orm/effect-postgres` (`PgDrizzle.make` / `makeWithDefaults`) provided alongside `PgClient.layer` from `@effect/sql-pg`.
- Expose a `WebDb` (name flexible) as an Effect `Context` service / Layer used by BFF domain code.
- Rewrite `lobby-store` operations as `Effect.fn` / `Effect.gen` that `yield*` queries.
- `just client-migrate` and existing migration files remain the schema source of truth; fix only what the RC kit requires.

**Risk control:** This wave does not need to rewrite every Nitro route helper — callers may still `runPromise` an Effect store op at the HTTP edge until Wave 4. The win is removing the pg-proxy throw bridge and making store logic Effect-native.

### Wave 4 — Effect HTTP helpers

**Modules:** `client/server/lobby-http.ts`, table/meta route files (thin), tests.

**Target:**

```ts
withLobbyAuth(event, spanName, Effect.fn(function* (ctx) { … return Response }))
```

- Auth, `grpcRequestEnv`, sweep, and store calls stay inside the Effect when Wave 3 enables it.
- Route default export remains `defineHandler(async (event) => …)` that awaits the helper.
- No project-specific `defineEffectHandler` wrapper.

### Wave 5 — Optional cleanups

- **Scryfall:** `searchPrints` returns Effect; builder Command drops `tryPromise`. Add a Service only if a second consumer appears.
- **Game stream:** `streamDeltas` (or equivalent) exposes `Stream` directly; `stream-subscription.ts` drops the callback → `Stream.callback` bridge where it is only adapting our own API.

### Error handling summary

| Layer | Mechanism |
|---|---|
| Browser lobby/meta transport | Tagged errors on `LobbyClient` |
| Lobby domain outcomes | `LobbyView.error` string codes (unchanged product vocabulary) |
| `/api/rpc` | Effect failures → existing HTTP/SSE mapping at route |
| SQL | Effect-typed Sql/Drizzle errors; LobbyDb 500 JSON shaping stays at HTTP helper |

### Data flow (target)

```text
Foldkit Command
  → yield* LobbyClient | RpcClient
  → same-origin HTTP (/api/… or /api/rpc)
Nitro defineHandler
  → runTracedRequest(Effect program)
  → yield* WebDb / Effect grpcClient
  → Postgres mtgfr_web | tonic gRPC
```

## Testing Decisions

| Wave | Proof |
|---|---|
| 1 | `makeClient(stubFetch)` tests (mirror `rpc-client.test.ts`); Command/poll unit tests for tag → UI code; Scene only if copy/testid changes |
| 2 | `rpcServer` / method-gate tests execute Effect dispatch (or `runPromise` in test); no Promise dispatch façade |
| 3 | `lobby-store` tests run Effect programs against `mtgfr_web`; migrate path still applies; pg-proxy gone |
| 4 | `lobby-http.test.ts` supplies Effect callbacks; HTTP route tests keep status/body assertions |
| 5 | Builder/stream unit tests as touched |

Per wave: `just client-check` (or equivalent client format/lint/typecheck/test) green before merge.

## Out of Scope

- Replacing Nitro with `@effect/platform-bun` / `BunHttpServer` as the SPA+BFF host
- Introducing `defineEffectHandler`
- Unifying `/api/rpc` and lobby/meta under a single Effect `HttpApi` contract
- Merging OTEL and gRPC `ManagedRuntime`s (document-only unless a wave hits a concrete bug)
- Server Rust/engine Effect work (N/A)
- Offline PWA / precache (unchanged)

## Further Notes

- The stale comment in `client/server/db/client.ts` (`@effect/sql-drizzle` still Effect 3) is superseded by Drizzle’s first-party `effect-postgres` on the 1.0 RC line; prefer that path over waiting on `@effect/sql-drizzle`.
- Wave 3 may need schema/relations adjustments required by RC `PgDrizzle.make({ relations })`; keep SQL tables and Drizzle migrate history intact unless the kit forces a mechanical change.
- Clipboard and other browser APIs may remain thin `Effect.tryPromise` at Command edges — low leverage.
- Cross-link from this design when implementing; do not leave living specs describing Promise lobby clients or pg-proxy after the corresponding wave merges.
