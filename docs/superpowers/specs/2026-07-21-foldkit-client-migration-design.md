# Foldkit client migration (full rewrite)

**Date:** 2026-07-21  
**Status:** Approved for planning  
**Context:** The client is SolidStart 1.3 + Effect atoms + a dual-surface board (canvas battlefield + DOM hand/stack/chrome). We want a full Foldkit rewrite with an event-reactor shape (server frames → client reactions), Foldkit Canvas for vector battlefield layers, a Mount bitmap adapter for card art parity, and Nitro (SolidStart beta 2-style adapter) hosting the existing BFF routes.

## Decisions (locked)

| Question | Choice |
|---|---|
| Migration boundary | **Full client rewrite** — Foldkit replaces SolidStart for all routes |
| Board fidelity | **Full parity** — card art, badges, flights, clusters, inspect, combat chrome |
| BFF hosting | **Nitro adapter** (SolidStart beta 2 pattern) — Foldkit SPA + existing `/api/*` routes |
| Cutover | **Big-bang** — one branch, switch when verified; no incremental Solid/Foldkit coexistence |
| Board surface split | **Dual-surface** — Foldkit Canvas (battlefield) + Foldkit HTML (hand, stack, chrome, prompts) |
| Card art on canvas | **Mount escape hatch** — vector via `Canvas.view`, bitmaps via local Mount adapter |
| Acceptance bar | **Parity** — all playable paths work; `just client-check` + live `verify` pass; SolidStart removed |

## Goals

1. **Event-reactor client** — server stream frames and intent acks become Foldkit Messages; state transitions flow through a single `update` (with submodel delegation).
2. **Full visual/interaction parity** — no regression on board UX, shell flows, reconnect, or observability.
3. **Preserve BFF + wire invariants** — `.proto` sole contract; per-viewer redaction server-side; same-origin `/api/rpc` gateway.
4. **Clear module boundaries** — pure geometry/fold helpers testable without Foldkit; board vector vs bitmap layers explicit.

## Non-goals (v1 cutover)

- Upstream Foldkit `Image` / sprite shape (Mount adapter is v1; upstream contribution is follow-up).
- SSR or marketing/SEO changes.
- Incremental route-by-route Solid/Foldkit coexistence.
- Rewriting engine, server, or wire proto.
- Pixi/Konva/WebGL or unified DOM+canvas scene graph (dual-surface invariant stays).

## Architecture

### Topology

```text
Browser
  └─ Foldkit SPA (all client routes)
       ├─ view: HTML + Canvas.view + Mount hooks
       ├─ update: AppModel / Messages (+ submodels)
       └─ fetch → same-origin /api/*

Nitro (SolidStart beta 2-style adapter)
  ├─ GET/POST /api/rpc/**     → dispatchRpc + gRPC (logic unchanged)
  ├─ GET/POST /api/**         → lobby/table routes (logic unchanged)
  ├─ POST    /api/faro/collect
  └─ static   Foldkit build + SPA fallback
```

### What changes vs today

| Layer | Today | After |
|---|---|---|
| UI | SolidStart 1.3 + Solid components | Foldkit Model / Message / update / view |
| Routing | `@solidjs/router` file routes | Foldkit typed routing |
| Async/wire | Effect atoms + `@effect/atom-solid` | Foldkit Commands, Subscriptions, Resources |
| Game fold | Solid store (`store.ts`) | Pure fold in `update` + `GameSlice` fields |
| BFF | Vinxi route handlers | Same handlers under `client/server/` + Nitro config |
| Build | `vinxi dev/build` | Vite (Foldkit) + Nitro |

**Removed dependencies after cutover:** `@solidjs/start`, `@solidjs/router`, `solid-js`, `@effect/atom-solid`, `vinxi`.

**Retained:** `effect`, `@effect/opentelemetry`, `@effect/sql-pg` (BFF), wire/grpc client, Biome, Tailwind v4, Vitest.

## Application model & event reactor

### AppModel (top-level)

```ts
AppModel = {
  route: Route
  session: SessionSlice      // me, auth status
  decks: DecksSlice          // list, builder draft
  lobby: LobbySlice | null   // active on /play/:table before started
  game: GameSlice | null     // active on /play/:table after started
}
```

### GameSlice (replaces `store.ts` + stream atoms)

```ts
GameSlice = {
  tableId: string
  seq: number
  state: VisibleState | null
  log: LogLine[]
  reject: string | null
  connected: boolean
  provenance: FoldProvenance
  tableFeel: TableFeelBatch
}
```

Presentation-local state (camera, pointer, flights, staging, inspect pin) lives in **Board submodel**, not the global app Model.

### Message sources

| Source | Examples |
|---|---|
| Routing / browser | `Navigated`, `SubmittedLogin`, `SavedDeck` |
| Stream Subscription | `ReceivedSnapshot`, `ReceivedDelta`, `StreamDisconnected`, `StreamReconnected` |
| Commands | `IntentAcked`, `IntentRejected`, `DeckSaved422` |
| Board OutMessages | `SubmitIntent`, `SetYield`, `SetTurnYield`, `SetStackDwell` |
| RAF Subscription | `TickedFrame` (flights, arrow anims) |

### Core loop

```text
SSE frame → ReceivedSnapshot | ReceivedDelta
  → update (pure fold: applySnapshot / applyDelta logic)
  → new GameSlice
  → view re-renders shell + board

User gesture → Board Messages → OutMessage → parent update
  → SubmitIntent Command → IntentAcked | IntentRejected
  → server delta via stream → fold
```

### Subscriptions

- **`gameStream(tableId)`** — while route is `/play/:table` and game active; ports `effect/stream.ts` reconnect/backoff/stale-timeout behavior.
- **`lobbyPoll(tableId)`** — while in lobby, not started.
- **`animationFrame`** — while Board submodel mounted and flights/arrow animations active.

### Resources (app-lifetime)

- **`RpcClient`** — hand-written Effect HTTP client (today `effect/client.ts`).
- **`ImageCache`** — shared by board Mount and deck builder art.

### Pure helpers retained (called from `update`, no globals)

- `applySnapshot` / `applyDelta` fold logic (from `store.ts`, made pure)
- `extractProvenance`, `describe` (`eventFold.ts`)
- `buildIntentEnvelope`, `streamDeltas`
- Wire types, `protoMap`, `grpcClient` (server edge unchanged)

## Board submodel

Board is a Foldkit **Submodel** when `route === /play/:table` and lobby `started`.

### BoardModel

```ts
BoardModel = {
  // from parent GameSlice (read-only in child)
  seq, state, provenance, connected, reject

  // presentation-local
  camera: Camera
  pointer: PointerPhase
  flights: Map<number, CardFlight>
  selectedId: number | null
  inspectPin: InspectPin | null
  staging: StagedAction | null
}
```

Child surfaces **OutMessages** to parent for wire intents.

### Dual-surface view

```text
┌─ BoardSubmodel.view ─────────────────────────────────────┐
│  TurnBanner (HTML)                                       │
│  ┌─ battlefield container (position: relative) ────────┐ │
│  │  Canvas.view        → felt, seats, avatars,       │ │
│  │                       arrows, vector card chrome     │ │
│  │  Mount.bitmapLayer  → drawImage faces, backs,       │ │
│  │                       flights (screen-space)         │ │
│  │  HTML overlays      → life-orbs, mana tray anchors  │ │
│  └─────────────────────────────────────────────────────┘ │
│  Hand (HTML)          StackOverlay (HTML)                │
│  PriorityContextBar   PromptHost   InspectDock           │
└──────────────────────────────────────────────────────────┘
```

**Camera** is single source of truth in `BoardModel.camera`; Canvas shapes and Mount bitmap paint both use `worldToScreen` / layout helpers.

### Layer responsibilities

| Layer | Renders | Input |
|---|---|---|
| **Foldkit Canvas** | Felt, seat bands, vector card frames, badges, avatars (circles + text), targeting arrows (Path), dim veil rects | `onPointerDown/Move/Up` → hit-test via logical layout |
| **Mount bitmap adapter** | Card art (`drawImage`), face-down backs, flight faces; clips from `boardCardPaint` | Repaint on Model patch + `ArtLoaded`; no pointer handling |
| **Foldkit HTML** | Hand fan, stack, priority chrome, prompts, inspect, activation radial, life-orbs | DOM events → Board Messages |

### Mount bitmap adapter (v1 card art path)

- `Mount.define` acquires a stacked `<canvas>` above the Foldkit Canvas layer (vector below, bitmap above; same viewport size).
- On parent Model patch and `TickedFrame`, repaint using ported `drawCard` / `drawFlightCard` from `boardCardPaint.ts`.
- `ImageCache` subscribe → `ArtLoaded` Message → repaint.
- `prefers-reduced-motion`: flights snap in `update`; Mount draws settled positions.

No dependency on upstream Foldkit `Image` shape for v1.

### Invariants (unchanged — see `docs/client-canvas-map.md`)

1. Hits use **logical layout**, never tweened paint positions.
2. Paint order: felt → seats → resting cards → avatars → arrows → flights on top.
3. Flight ownership suppresses duplicate stack entrances and hides resting faces.
4. Hand/stack rest as DOM; battlefield + zone piles + flights are canvas.
5. Canvas hex colors stay exempt from Tailwind tokens; DESIGN.md legend stays in sync.

## File layout, renames & moves

Big-bang PR replaces `client/src/` Solid tree with:

```text
client/
  app/
    main.ts                      # Runtime.bootstrap
    model.ts, messages.ts, update.ts, view.ts, routes.ts, init.ts
    subscriptions.ts, resources.ts

    shell/
      auth/                      # was organisms/auth.tsx
      decks/                     # was decks.tsx + deck-builder.tsx
      lobby/                     # was lobby.tsx
      portrait-gate/             # was portrait-gate.tsx

    game/
      fold.ts                    # was store.ts (pure, no globals)
      fold.test.ts               # was store.test.ts
      stream-subscription.ts     # was effect/stream.ts + net gameStreamFamily
      intents.ts                 # was intentAtoms.ts + reject.ts
      messages.ts

    board/
      submodel.ts, messages.ts, view.ts
      geometry/                  # layout, camera, hit-test, density, interaction, combat-staging
      canvas/                    # scene, felt, avatars, arrows (Shape builders)
      bitmap/                    # mount.ts, paint-cards.ts, paint-flights.ts
      motion/                    # flights, entrances, tween
      action/                    # execution, session, chrome
      html/                      # hand, stack, turn-chrome, priority-bar, overlays, inspect, prompts, …

  lib/
    wire/                        # moved from src/wire/
    rpc-client.ts                # was effect/client.ts
    event-fold.ts                # was eventFold.ts
    image-cache.ts
    scryfall.ts, deck-builder/, lobby/, otel/, faro/, design/

  server/                        # Nitro routes from src/routes/api/*
    routes/api/rpc/[...path].ts
    routes/api/[...path].ts
    routes/api/faro/collect.ts
    plugins/otel.server.ts
    db/

  public/
  styles/global.css
  nitro.config.ts
  vite.config.ts
  index.html
```

### Rename map

| Old | New | Rationale |
|---|---|---|
| `store.ts` | `app/game/fold.ts` | Pure fold, not a reactive store |
| `net.ts` | split: `stream-subscription.ts` + `routes.ts` | Separate stream lifecycle from routing helpers |
| `controllers/*` | `app/board/action/*`, `geometry/*` | Drop Solid-specific "controller" naming |
| `lib/board*.ts` | `app/board/canvas/*`, `bitmap/*` | Explicit vector vs bitmap split |
| `components/organisms/board.tsx` | `app/board/submodel.ts` + `view.ts` | Foldkit submodel, not Solid component |
| `effect/client.ts` | `lib/rpc-client.ts` | Shared Resource, not atom-specific |
| `atoms.ts` | deleted | Replaced by Model + Subscriptions |

### Big-bang deletion (same PR)

- All of `client/src/` Solid entry, components, controllers, atoms, guard, Vinxi `app.config.ts`
- Dependencies: `@solidjs/start`, `@solidjs/router`, `solid-js`, `@effect/atom-solid`, `vinxi`

## Error handling

| Failure | Behavior |
|---|---|
| Stream 401 | Reject line + redirect `/login?next=` |
| Stream 4xx (other) | Terminal stop; reconnect banner; reject line |
| Stream drop / stale timeout | Backoff reconnect (ported from `effect/stream.ts`) |
| Intent rejected | `IntentRejected` → reject banner; clear staging |
| Deck 422 | Tagged error → builder problems list |
| Image load fail | Placeholder rect + name (existing fallback) |

## Testing

| Layer | Tool | Coverage |
|---|---|---|
| Pure fold / geometry | Vitest | `fold.test.ts`, `event-fold.test.ts`, geometry tests |
| Board update | Foldkit `Story` | delta → provenance + flights; intent ack paths |
| Board HTML | Foldkit `Scene` | hand threshold, stack modes, priority chrome |
| Bitmap Mount | Vitest + mock canvas | drawImage/clip calls at correct transforms |
| BFF | Existing rpcServer tests | under `server/`, unchanged logic |
| Cutover gate | `just client-check` + verify skill | two-player live game |

## Cutover checklist

1. Routes: `/`, `/login`, `/decks/new`, `/decks/:id`, `/play`, `/play/:table`
2. BFF integration tests green
3. Verify skill: two-player game end-to-end
4. Board invariants preserved (hits ≠ paint, flight ownership, dual-surface)
5. OTEL/Faro trace propagation unchanged
6. SolidStart/Vinxi removed; `just dev` runs Nitro + Foldkit

## Risks & mitigations

| Risk | Mitigation |
|---|---|
| Foldkit Canvas lacks Image shape | Mount bitmap layer (locked decision); upstream contribution is follow-up |
| Camera sync across Canvas + Mount + HTML | Single `BoardModel.camera`; shared `worldToScreen` |
| Big-bang scope | Strict parity checklist; port pure helpers first; Story/Scene tests per submodel |
| Nitro adapter differences vs Vinxi | Port routes verbatim; keep existing BFF tests as gate |
| Performance with declarative Canvas + Mount repaint | Benchmark early on crowded board; keep shape tree shallow; bitmap layer only for art |

## Follow-ups (post-cutover)

- Contribute `Image` shape to Foldkit upstream; simplify Mount adapter if accepted.
- Foldkit DevTools MCP for agent-assisted board debugging.
- Optional: fold more presentation reactions into centralized Board `update` for clearer reactor graph.
