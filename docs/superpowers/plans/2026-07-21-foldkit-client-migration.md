# Foldkit Client Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the SolidStart client with a Foldkit SPA (event-reactor Model/update/view) hosted on Nitro, preserving full board parity via Foldkit Canvas + Mount bitmap adapter + HTML overlays, and deleting SolidStart/Vinxi at cutover.

**Architecture:** One Foldkit application owns all client routes. Nitro (SolidStart beta 2-style Vite adapter) serves the SPA and existing BFF handlers under `client/server/`. Stream frames and intents become Foldkit Messages; pure fold/geometry helpers stay framework-agnostic; the Board submodel splits vector (`Canvas.view`), bitmap (`Mount`), and HTML overlays.

**Tech Stack:** Foldkit + Effect v4 beta (align to Foldkit’s pinned beta), Nitro + Vite, Tailwind v4, Biome, Vitest, existing Effect-gRPC BFF, Foldkit Story/Scene tests.

**Spec:** [docs/superpowers/specs/2026-07-21-foldkit-client-migration-design.md](../specs/2026-07-21-foldkit-client-migration-design.md)

## Global Constraints

- Big-bang cutover on a feature branch — do not leave Solid/Foldkit coexistence in `main`.
- Full board parity required before deleting Solid (auth, decks, lobby, board, reconnect, OTEL/Faro).
- Dual-surface board: Foldkit Canvas + Mount bitmaps + Foldkit HTML overlays (no unified scene graph).
- Mount escape hatch for card art (no upstream Foldkit `Image` dependency for v1).
- Wire/proto unchanged; BFF logic ports verbatim; per-viewer redaction stays server-side.
- Align `effect` / `@effect/*` to Foldkit’s exact peer pin (today Foldkit docs: `effect@4.0.0-beta.97` — verify at install time and keep all `@effect/*` on that same version).
- Keep Biome as the client linter/formatter (do not switch to Foldkit’s oxlint/prettier defaults).
- TDD: failing test → implement → pass → commit per task.
- Angular commit messages (`feat:`, `fix:`, `test:`, `build:`, `docs:`, …).
- Preserve `docs/client-canvas-map.md` invariants (hits ≠ paint, flight ownership, paint order).
- Acceptance: `just client-check` green + `.agents/skills/verify` live two-player pass.

## Scope note

This is one cutover plan with **phased tasks**. Each task leaves a testable artifact on the branch. SolidStart remains bootable until Task 12 removes it; until then, prefer building the new tree under `client/app`, `client/lib`, `client/server` while leaving `client/src` intact, then swap entrypoints in Task 12.

---

## File map (target)

| Path | Responsibility |
|------|----------------|
| `client/package.json` | Foldkit + Nitro deps; remove SolidStart/Vinxi at cutover |
| `client/vite.config.ts` | Foldkit HMR plugin + Nitro + Tailwind |
| `client/nitro.config.ts` | `serverDir: "./server"`, SPA fallback |
| `client/index.html` | Foldkit entry |
| `client/app/entry.ts` | `Runtime.makeApplication` + `Runtime.run` |
| `client/app/model.ts` | `AppModel` Schema |
| `client/app/messages.ts` | Top-level Message union |
| `client/app/update.ts` | App update + submodel delegation |
| `client/app/view.ts` | Route switch view |
| `client/app/routes.ts` | Typed routes |
| `client/app/resources.ts` | `RpcClient`, `ImageCache` Resources |
| `client/app/shell/**` | Auth, decks, lobby, portrait-gate submodels |
| `client/app/game/fold.ts` | Pure snapshot/delta fold |
| `client/app/game/stream-subscription.ts` | SSE reconnect subscription |
| `client/app/game/intents.ts` | SubmitIntent / yield Commands |
| `client/app/board/**` | Board submodel, geometry, canvas, bitmap, motion, action, html |
| `client/lib/wire/**` | Moved from `src/wire/` |
| `client/lib/rpc-client.ts` | Moved from `src/effect/client.ts` |
| `client/lib/event-fold.ts` | Moved from `src/lib/eventFold.ts` |
| `client/server/routes/api/**` | Ported BFF routes |
| `client/styles/global.css` | DESIGN.md tokens / Tailwind `@theme` |
| `justfile` | `client-*` recipes point at new scripts |

---

### Task 1: Scaffold Foldkit + Nitro beside Solid

**Files:**
- Create: `client/vite.config.ts`
- Create: `client/nitro.config.ts`
- Create: `client/index.html`
- Create: `client/app/entry.ts`
- Create: `client/app/model.ts`
- Create: `client/app/messages.ts`
- Create: `client/app/update.ts`
- Create: `client/app/view.ts`
- Create: `client/app/init.ts`
- Create: `client/styles/global.css` (copy from `client/src/global.css` initially)
- Modify: `client/package.json` (add Foldkit/Nitro; keep Solid scripts until Task 12)
- Test: `client/app/smoke.test.ts`

**Interfaces:**
- Produces: Foldkit app that boots with `Model = { ready: true }` and renders a placeholder body
- Produces: `bun run dev:foldkit` (or equivalent) starting Vite+Nitro without breaking `bun run dev` SolidStart

- [ ] **Step 1: Write the failing smoke test**

```ts
// client/app/smoke.test.ts
import { describe, expect, it } from "vitest";
import { init, Model, update } from "./main-exports"; // re-export from init/model/update

describe("foldkit scaffold", () => {
  it("init returns a ready model", () => {
    const [model] = init();
    expect(Model.make(model).ready).toBe(true);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd client && bunx vitest run app/smoke.test.ts`

Expected: FAIL (module not found)

- [ ] **Step 3: Install Foldkit + Nitro with Effect pin**

```bash
cd client
# Verify Foldkit's current effect peer from npm/docs, then:
bun add foldkit effect@4.0.0-beta.97 @effect/platform-browser@4.0.0-beta.97
bun add -d nitro
# Align every @effect/* already in package.json to the same beta (opentelemetry, sql-pg, atom-solid until removed)
```

If SolidStart cannot resolve the new Effect beta, keep Solid on the old lockfile until Task 12 and isolate Foldkit install in a way that still typechecks the new tree — preferred: bump Effect repo-wide now and fix Solid compile breaks in this task (small) so one Effect version exists.

- [ ] **Step 4: Minimal Foldkit program + Vite/Nitro config**

`client/app/model.ts`:

```ts
import { Schema as S } from "effect";

export const Model = S.Struct({ ready: S.Boolean });
export type Model = typeof Model.Type;
```

`client/app/messages.ts`:

```ts
import { Schema as S } from "effect";
import { m } from "foldkit/message";

export const Booted = m("Booted");
export const Message = S.Union([Booted]);
export type Message = typeof Message.Type;
```

`client/app/init.ts`:

```ts
import type { Runtime } from "foldkit";
import type { Message } from "./messages";
import type { Model } from "./model";

export const init: Runtime.ApplicationInit<Model, Message> = () => [{ ready: true }, []];
```

`client/app/update.ts`:

```ts
import type { Command } from "foldkit";
import { Match as M } from "effect";
import type { Message } from "./messages";
import type { Model } from "./model";

export const update = (
  model: Model,
  message: Message,
): readonly [Model, ReadonlyArray<Command.Command<Message>>] =>
  M.value(message).pipe(
    M.withReturnType<readonly [Model, ReadonlyArray<Command.Command<Message>>]>(),
    M.tagsExhaustive({
      Booted: () => [model, []],
    }),
  );
```

`client/app/view.ts` — return a Foldkit `Document` with title `mtgfr` and body text `Foldkit scaffold`.

`client/app/entry.ts` — `Runtime.makeApplication({ init, update, view, … })` + `Runtime.run` per Foldkit getting-started.

`client/vite.config.ts`:

```ts
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import { nitro } from "nitro/vite";
// import foldkit HMR plugin from 'foldkit/vite' (exact export per installed version)

export default defineConfig({
  plugins: [/* foldkitPlugin(), */ nitro(), tailwindcss()],
});
```

`client/nitro.config.ts`:

```ts
import { defineConfig } from "nitro";

export default defineConfig({
  serverDir: "./server",
});
```

Add `client/server/routes/api/health.get.ts` returning `{ ok: true }` so Nitro has a route.

- [ ] **Step 5: Run smoke test + hit health**

Run: `cd client && bunx vitest run app/smoke.test.ts`

Expected: PASS

Run Foldkit/Nitro dev briefly and `curl -s localhost:<port>/api/health` → `{"ok":true}` (port per Vite/Nitro default).

- [ ] **Step 6: Commit**

```bash
git add client/package.json client/bun.lock client/app client/vite.config.ts client/nitro.config.ts client/index.html client/styles client/server/routes/api/health.get.ts
git commit -m "build: scaffold Foldkit SPA beside SolidStart with Nitro"
```

---

### Task 2: Port wire + RPC client + BFF routes to `client/server` / `client/lib`

**Files:**
- Create: `client/lib/wire/**` (copy from `client/src/wire/**`, adjust imports)
- Create: `client/lib/rpc-client.ts` (from `client/src/effect/client.ts`)
- Create: `client/lib/otel/**`, `client/lib/faro/**`, `client/lib/api-upstream*.ts` as needed by routes
- Create: `client/server/routes/api/rpc/[...path].ts`
- Create: `client/server/routes/api/[...path].ts`
- Create: `client/server/routes/api/faro/collect.ts`
- Create: `client/server/db/client.ts` (from `src/db/client.ts`)
- Create: `client/server/plugins/otel.server.ts`
- Test: move/adapt `client/src/wire/rpcServer.test.ts` → `client/lib/wire/rpcServer.test.ts` (or keep path and update imports)
- Test: `client/lib/rpc-client.test.ts` (port of `effect/client.test.ts`)

**Interfaces:**
- Consumes: existing `dispatchRpc`, `grpcClient`, cookie/session helpers
- Produces: Nitro handlers with **identical** HTTP semantics to Vinxi routes (`/api/rpc/**`, `/api/**`, `/api/faro/collect`)

- [ ] **Step 1: Port `rpcServer` tests to import from `lib/wire` and fail on missing module**

Copy `rpcServer.test.ts` to `client/lib/wire/rpcServer.test.ts` and change imports to `~/lib/wire/...` (or relative). Run:

`cd client && bunx vitest run lib/wire/rpcServer.test.ts`

Expected: FAIL until files exist.

- [ ] **Step 2: Copy wire + rpc-client + route bodies**

- Move logic, do not rewrite: `dispatchRpc`, `streamResponse`, `grpcRequestEnv`, Faro collect caps.
- Adapt SolidStart `APIEvent` / `vinxi/http` cookie helpers to Nitro equivalents (`getCookie`/`setCookie` from `h3` / Nitro utilities — match whatever Nitro version exposes; keep cookie name `session` and same options).
- Keep codegen path working: `bun run gen` still writes wire generated types; point `lib/wire` at generated output (either keep `src/wire/generated` temporarily or move gen outdir in `proto/buf.gen.yaml` — prefer updating gen outdir to `client/lib/wire/generated` in this task).

- [ ] **Step 3: Run BFF unit tests**

Run:

```bash
cd client && bunx vitest run lib/wire/rpcServer.test.ts lib/rpc-client.test.ts
```

Expected: PASS

- [ ] **Step 4: Manual smoke — auth me via Nitro**

With API + Postgres up (`just migrate` if needed), start Foldkit/Nitro, `curl` `/api/rpc/auth/me` without cookie → expect 401/null semantics matching current BFF.

- [ ] **Step 5: Commit**

```bash
git add client/lib client/server client/proto-related-if-any
git commit -m "feat: port BFF routes and wire client under Nitro + lib/"
```

---

### Task 3: Pure game fold (`app/game/fold.ts`)

**Files:**
- Create: `client/app/game/fold.ts`
- Create: `client/app/game/fold.test.ts` (port from `store.test.ts`, remove Solid store assertions)
- Create: `client/lib/event-fold.ts` (move from `src/lib/eventFold.ts`)
- Test: `client/app/game/fold.test.ts`

**Interfaces:**
- Produces:

```ts
export type GameFoldState = {
  seq: number;
  state: VisibleState | null;
  log: LogLine[];
  reject: string | null;
  provenance: FoldProvenance;
  tableFeel: TableFeelBatch;
};

export function emptyGameFold(): GameFoldState;
export function applySnapshotPure(prev: GameFoldState, seq: number, state: VisibleState): GameFoldState;
export function applyDeltaPure(prev: GameFoldState, delta: DeltaEnvelope): GameFoldState;
export function setRejectPure(prev: GameFoldState, reason: string | null): GameFoldState;
```

No module-level mutable maps — provenance/tableFeel live on `GameFoldState`.

- [ ] **Step 1: Write failing tests (port critical cases)**

Port at least:

- older seq ignored
- snapshot replaces state and clears provenance
- delta appends log + builds land_played provenance
- same-seq empty events refresh `stack_hold_remaining_ms` only

Example:

```ts
it("applyDeltaPure records landPlayFrom provenance", () => {
  let g = emptyGameFold();
  g = applySnapshotPure(g, 0, mkState([]));
  g = applyDeltaPure(g, mkDelta(1, [{ kind: "land_played", from: 9, permanent: 3, player: 1 }], [forest]));
  expect(g.provenance.landPlayFrom.get(3)).toBe(9);
  expect(g.tableFeel.land).toBe(true);
});
```

- [ ] **Step 2: Run tests — expect FAIL**

`cd client && bunx vitest run app/game/fold.test.ts`

- [ ] **Step 3: Implement pure fold**

Port logic from `client/src/store.ts` into pure functions returning new `GameFoldState` (immutable updates; Maps copied per delta).

- [ ] **Step 4: Run tests — expect PASS**

- [ ] **Step 5: Commit**

```bash
git commit -m "refactor: extract pure game fold for Foldkit GameSlice"
```

---

### Task 4: App routing + session Resource + placeholder shell views

**Files:**
- Create: `client/app/routes.ts`
- Create: `client/app/resources.ts`
- Create: `client/app/subscriptions.ts`
- Create: `client/app/shell/auth/{messages,update,view,submodel}.ts`
- Modify: `client/app/model.ts`, `messages.ts`, `update.ts`, `view.ts`
- Test: `client/app/shell/auth/story.test.ts` (Foldkit Story) or Vitest on update

**Interfaces:**
- Produces routes: `/`, `/login`, `/decks/new`, `/decks/:id`, `/play`, `/play/:table`
- Produces `SessionSlice = { me: Me | null }`
- Produces Commands: `FetchMe`, `Login`, `Signup`, `Logout` using `RpcClient` Resource

- [ ] **Step 1: Failing Story — unsigned me stays null**

```ts
import { Story } from "foldkit";
import { expect, test } from "vitest";
import { update, init } from "../../…";
import { ReceivedMe } from "./messages";

test("session folds me", () => {
  const [model] = init();
  Story.story(
    update,
    Story.with(model),
    Story.message(ReceivedMe({ me: null })),
    Story.model((m) => {
      expect(m.session.me).toBeNull();
    }),
  );
});
```

(Adjust imports to actual module layout.)

- [ ] **Step 2: Run — FAIL**

- [ ] **Step 3: Implement routing + session slice + login/signup views (parity with `auth.tsx`)**

- Reuse `safeNext` rules from current auth.
- On success: Command sets cookie via existing BFF; client navigates to `next`.
- Portrait gate as HTML dialog submodel matching `portrait-gate.tsx` behavior.

- [ ] **Step 4: Run Story + manual `/login` smoke**

- [ ] **Step 5: Commit**

```bash
git commit -m "feat: Foldkit routing and auth session shell"
```

---

### Task 5: Decks list + deck builder shell

**Files:**
- Create: `client/app/shell/decks/list/**`
- Create: `client/app/shell/decks/builder/**`
- Move helpers: `client/lib/deck-builder/*` from `src/lib/deckBuilderPrint.ts`, `lookupCards.ts`, etc.
- Test: builder update Story for add/remove/save 422

**Interfaces:**
- Consumes: `RpcClient.listDecks`, `searchCards`, `createDeck`, `updateDeck`, `getDeck`
- Produces: deck list view on `/`; builder on `/decks/new` and `/decks/:id`
- Parity: singleton rules, commander picker, printing preference, infinite scroll PAGE=100

- [ ] **Step 1: Failing Story — save 422 surfaces problems**

```ts
test("CreateDeck422 folds into problems list", () => {
  // message DeckSaveFailed({ problems: ["Too many cards"] })
  // expect model.decks.builder.problems to equal that list
});
```

- [ ] **Step 2: Run — FAIL**

- [ ] **Step 3: Port deck list + builder UI to Foldkit HTML**

Keep server-side search/paging semantics from `deck-builder.tsx`. Prefer porting pure helpers first (`reconcileEntries`, `commanderPrintForRow`).

- [ ] **Step 4: Run Stories + `bunx vitest run lib/deck-builder`

- [ ] **Step 5: Commit**

```bash
git commit -m "feat: Foldkit decks list and deck builder"
```

---

### Task 6: Lobby + play route handoff

**Files:**
- Create: `client/app/shell/lobby/**`
- Move: `client/lib/lobby/*` from `src/lib/lobby*.ts`
- Modify: `client/app/view.ts` for `/play` and `/play/:table`
- Test: lobby poll Story / Vitest on `lobbyPoll` stop-when-started

**Interfaces:**
- Produces: `LobbySlice` with seats, ready, host start
- Produces: Subscription `lobbyPoll(tableId)` while not started
- On `started`: parent sets `game` slice active and mounts Board submodel (Board can be placeholder until Task 8+)

- [ ] **Step 1: Failing test — poll stops when started**

Port assertion from `lobbyPoll` / `lobbyStore` tests.

- [ ] **Step 2: Implement lobby UI + subscription**

Match `lobby.tsx`: share link copy, Ready, Start (≥2 ready), `unlockTableAudio` on Ready.

- [ ] **Step 3: Tests PASS + manual lobby smoke**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: Foldkit lobby and play route handoff"
```

---

### Task 7: Game stream Subscription + intent Commands

**Files:**
- Create: `client/app/game/stream-subscription.ts`
- Create: `client/app/game/intents.ts`
- Create: `client/app/game/messages.ts`
- Modify: `client/app/model.ts` (`game: GameSlice | null`)
- Modify: `client/app/update.ts` to fold stream messages via `apply*Pure`
- Test: `client/app/game/stream-subscription.test.ts` (port `effect/stream.test.ts`)
- Test: Story for `ReceivedDelta` updating `game.seq`

**Interfaces:**
- Produces Messages: `ReceivedSnapshot`, `ReceivedDelta`, `StreamStatus`, `StreamTerminalError`
- Produces Commands: `SubmitIntent`, `SetYield`, `SetTurnYield`, `SetStackDwell`
- Subscription active iff `route` is play table and lobby started / `game != null`

- [ ] **Step 1: Port stream reconnect tests to call `streamDeltas` from new module path — FAIL if missing**

- [ ] **Step 2: Implement subscription wrapping `streamDeltas`**

Preserve: backoff, jitter, stale timeout 15s, heartbeat filter, 4xx terminal.

- [ ] **Step 3: Wire update**

```ts
ReceivedDelta: (msg) => {
  if (!model.game) return [model, []];
  return [evo(model, { game: (g) => applyDeltaPure(g, msg) }), []];
}
```

- [ ] **Step 4: Tests PASS**

- [ ] **Step 5: Commit**

```bash
git commit -m "feat: Foldkit game stream subscription and intent commands"
```

---

### Task 8: Board geometry port (pure)

**Files:**
- Create: `client/app/board/geometry/layout.ts` (from `src/layout.ts`)
- Create: `client/app/board/geometry/camera.ts`
- Create: `client/app/board/geometry/hit-test.ts`
- Create: `client/app/board/geometry/density.ts`
- Create: `client/app/board/geometry/interaction.ts`
- Create: `client/app/board/geometry/combat-staging.ts`
- Move tests alongside (rename files to match)
- Test: existing geometry tests must PASS unchanged in behavior

**Interfaces:**
- Produces: same pure APIs as today (`layout`, `worldToScreen`, `hitTest`, `pointerDown`, `fitCamera`, …)

- [ ] **Step 1: Copy tests to new paths; run — FAIL (missing modules)**

- [ ] **Step 2: Move implementations; fix imports only**

- [ ] **Step 3: `bunx vitest run app/board/geometry` — PASS

- [ ] **Step 4: Commit**

```bash
git commit -m "refactor: move board geometry into app/board/geometry"
```

---

### Task 9: Board submodel + Foldkit Canvas vector layer

**Files:**
- Create: `client/app/board/submodel.ts`
- Create: `client/app/board/messages.ts`
- Create: `client/app/board/view.ts`
- Create: `client/app/board/canvas/scene.ts`
- Create: `client/app/board/canvas/felt.ts`
- Create: `client/app/board/canvas/avatars.ts`
- Create: `client/app/board/canvas/arrows.ts`
- Test: `client/app/board/canvas/scene.test.ts` — given `VisibleState` fixture, `sceneShapes` contains expected Rect/Circle counts
- Test: Story — pointer down on empty felt enters pan phase

**Interfaces:**
- Consumes: `GameFoldState` fields from parent
- Produces: `BoardModel` with `camera`, `pointer`, …
- Produces: `Canvas.view({ width, height, shapes, onPointerDown/Move/Up })`
- Pointer handlers: convert canvas point → `hitTest` on logical layout → Board Messages

- [ ] **Step 1: Failing scene test**

```ts
it("builds a felt background rect", () => {
  const shapes = sceneShapes(boardFixture);
  expect(shapes.some((s) => s._tag === "Rect")).toBe(true);
});
```

- [ ] **Step 2: Implement Board submodel shell + vector scene from `boardFelt` / avatar / arrow geometry**

Port vector-capable paint to `Shape`s. Card **frames** can be Rect/Path; **art** deferred to Task 10.

- [ ] **Step 3: Tests PASS; visual smoke on `/play/:table` shows felt + seats**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: Foldkit Board submodel with Canvas vector layer"
```

---

### Task 10: Mount bitmap adapter (card art + flights)

**Files:**
- Create: `client/app/board/bitmap/mount.ts`
- Create: `client/app/board/bitmap/paint-cards.ts`
- Create: `client/app/board/bitmap/paint-flights.ts`
- Create: `client/app/board/motion/flights.ts` (from `playMotion.ts` + `cardFlight.ts`)
- Move: `client/lib/image-cache.ts`
- Test: `paint-cards.test.ts` with mock `CanvasRenderingContext2D` asserting `drawImage` called
- Test: flight Story — `TickedFrame` advances flight position

**Interfaces:**
- Produces: stacked bitmap canvas via `Mount.define` above Foldkit Canvas
- Consumes: `BoardModel.camera`, layout cards, `ImageCache`, flights
- Message: `ArtLoaded` on cache settle

- [ ] **Step 1: Failing test — paintCard calls drawImage when cache hit**

```ts
it("drawImage when print cached", () => {
  const ctx = mockCtx();
  const cache = { get: () => fakeImg };
  paintCard(ctx, cam, card, cache);
  expect(ctx.drawImage).toHaveBeenCalled();
});
```

- [ ] **Step 2: Port `boardCardPaint` bitmap paths into `paint-cards.ts` / `paint-flights.ts`**

- [ ] **Step 3: Implement Mount lifecycle + RAF `TickedFrame` subscription for flights**

Preserve flight ownership / hide sets invariants from canvas map.

- [ ] **Step 4: Tests PASS; visual: card art on battlefield**

- [ ] **Step 5: Commit**

```bash
git commit -m "feat: Mount bitmap layer for card art and flights"
```

---

### Task 11: Board HTML overlays + action session

**Files:**
- Create: `client/app/board/html/hand.ts`
- Create: `client/app/board/html/stack.ts`
- Create: `client/app/board/html/turn-chrome.ts`
- Create: `client/app/board/html/priority-bar.ts`
- Create: `client/app/board/html/overlays.ts`
- Create: `client/app/board/html/inspect.ts`
- Create: `client/app/board/html/prompts.ts`
- Create: `client/app/board/html/mana-tray.ts`
- Create: `client/app/board/html/activation-radial.ts`
- Create: `client/app/board/html/discoverability.ts`
- Create: `client/app/board/action/{execution,session,chrome}.ts`
- Test: Foldkit Scene tests for hand drop threshold + priority Next
- Test: port `action-session.test.ts` / `combatStaging` tests to new modules

**Interfaces:**
- OutMessages: `SubmitIntent`, `SetYield`, `SetTurnYield`, `SetStackDwell`
- Staging stays in BoardModel until intent submit (per spec)
- Camera shared for mana tray projection

- [ ] **Step 1: Failing Scene — primary Next visible when empty stack + your priority**

- [ ] **Step 2: Port HTML overlays and action pipeline**

Match behaviors in client-game-board-and-interaction spec: hand threshold, stack modes, inspect Alt, radial, combat drag, prompts.

- [ ] **Step 3: Unit/Scene tests PASS**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: Foldkit board HTML overlays and action session"
```

---

### Task 12: Cutover — swap entrypoints, delete Solid, verify

**Files:**
- Modify: `client/package.json` scripts (`dev`/`build`/`start` → Vite+Nitro Foldkit)
- Modify: `justfile` client recipes if needed
- Delete: `client/src/**` Solid app (keep nothing Solid-specific)
- Delete deps: `@solidjs/start`, `@solidjs/router`, `solid-js`, `@effect/atom-solid`, `vinxi`
- Delete: `client/app.config.ts`
- Update: `docs/client-canvas-map.md` paths to `app/board/**`
- Update: shell/board specs “Implementation Decisions” only if they claim SolidStart as current (or add Further Notes pointing at Foldkit migration design)
- Test: `just client-check`
- Test: verify skill two-player live game

**Interfaces:**
- Produces: single Foldkit+Nitro client as the only client

- [ ] **Step 1: Point `dev`/`build`/`test` at Foldkit tree; remove Solid scripts**

- [ ] **Step 2: Delete Solid tree + deps; fix any broken imports**

- [ ] **Step 3: `just client-check`**

Expected: PASS

- [ ] **Step 4: Live verify**

Follow `.agents/skills/verify/SKILL.md` — two-player game: lobby → start → cast → priority → reconnect banner path if feasible.

Expected: playable parity

- [ ] **Step 5: Update canvas map + AGENTS.md client stack blurb if present**

- [ ] **Step 6: Commit**

```bash
git commit -m "feat!: replace SolidStart client with Foldkit + Nitro

BREAKING CHANGE: client is Foldkit SPA on Nitro; SolidStart/Vinxi removed."
```

- [ ] **Step 7: Push branch + open/update PR**

Title suggestion: `feat!: replace SolidStart client with Foldkit + Nitro`

---

## Spec coverage checklist

| Spec requirement | Task(s) |
|---|---|
| Full Foldkit rewrite all routes | 4–6, 12 |
| Nitro BFF host | 1–2, 12 |
| Big-bang cutover | 12 |
| Dual-surface board | 9–11 |
| Mount bitmap art | 10 |
| Event-reactor stream/intents | 3, 7 |
| File renames / layout | 2–11 |
| Parity acceptance (`client-check` + verify) | 12 |
| Effect peer alignment | 1 |
| OTEL/Faro preserved | 2, 12 |
| Canvas map invariants | 8–11 |

## Placeholder / consistency self-review

- No TBD steps; Effect pin called out as “verify at install.”
- Pure fold API names (`applySnapshotPure` / `applyDeltaPure`) consistent across Tasks 3 and 7.
- Board OutMessage names consistent in Tasks 7 and 11.
- Solid deletion deferred to Task 12 only.
