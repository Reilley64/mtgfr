# Foldkit Merge Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make PR #74 merge-ready: present-tense Foldkit docs, lobby `start_error` fix, Biome warning-clean geometry, Nitro not shipping tests, comment hygiene, and PR title/body for squash + semantic-release.

**Architecture:** One cleanup commit on `cursor/foldkit-migration-design-1ef0`. Behavior fix is pure projection in `toLobbyView` / `startError` / start route; Nitro fix is moving the server-colocated Vitest file out of `serverDir` scan; docs are present-tense path/stack edits only.

**Tech Stack:** Foldkit SPA, Nitro BFF, Vitest, Biome, commitlint (`@commitlint/config-angular`), GitHub PR squash-merge.

## Global Constraints

- Spec: Superseded by current client specs and companion docs: `docs/superpowers/specs/README.md`, `docs/superpowers/specs/2026-07-20-client-shell-deck-builder-and-observability.md`, `docs/superpowers/specs/2026-07-20-board-composition.md`
- **Single final commit** for all implementation tasks (no intermediate commits). Plan steps still use TDD locally.
- Commit subject must be lower-case after `type:`, header ≤72 chars, types from `[build, ci, docs, feat, fix, perf, refactor, revert, style, test]` only — no `chore:`, no `feat!:`.
- Do not disable `lint/style/noNonNullAssertion`.
- Do not delete migration design/plan files; mark them historical.
- Do not add features or board parity work.

## File map

| File | Role |
|---|---|
| `client/lib/lobby-store.ts` | `toLobbyView` / `startError` projection |
| `client/lib/lobby-store.test.ts` | **Create** — unit tests for started-lobby projection |
| `client/server/routes/api/[...path].ts` | Idempotent start when already started |
| `client/app/board/geometry/layout.ts` | Drop non-null assertions |
| `client/app/board/geometry/layout.test.ts` | Drop non-null assertions |
| `client/app/board/geometry/density.test.ts` | Drop non-null assertions |
| `client/server/routes/api/rpc/method-gate.test.ts` | **Delete** (move) |
| `client/lib/wire/rpc-method-gate.test.ts` | **Create** — Vitest method gate (outside Nitro `serverDir`) |
| `client/nitro.config.ts` | Optional comment only if move is sufficient |
| Comment-touched files | `client/app/faro.ts`, `client/lib/faro/collect.ts`, `client/lib/wire/rpcServer.ts`, `client/server/routes/api/[...path].ts`, `client/lib/outcome.ts`, `client/lib/inspect.ts`, `client/app/board/geometry/stackLayout.ts`, `client/lib/ui/buttonClass.ts`, `client/lib/effect/client.ts` |
| Docs | README, DESIGN, shell/board specs, migration design/plan, wire/lobby/production specs, WIRE_COMPAT |

---

### Task 1: Lobby `start_error` is null when started

**Files:**
- Create: `client/lib/lobby-store.test.ts`
- Modify: `client/lib/lobby-store.ts` (`startError`, `toLobbyView`)
- Modify: `client/server/routes/api/[...path].ts` (start handler early return)

**Interfaces:**
- Consumes: `LobbySnapshot`, `LobbySeatRow`, `toLobbyView`, `startError` from `client/lib/lobby-store.ts`
- Produces: `toLobbyView(startedSnap, hostId)` → `{ started: true, start_error: null, error: null }`; `startError` never returns `"AlreadyStarted"`

- [ ] **Step 1: Write the failing tests**

Create `client/lib/lobby-store.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { startError, toLobbyView, type LobbySnapshot } from "./lobby-store";

function snap(overrides: Partial<LobbySnapshot> = {}): LobbySnapshot {
  return {
    tableId: "ABC123",
    hostUserId: 1,
    startedAt: null,
    seats: [
      {
        seat: 0,
        userId: 1,
        username: "alice",
        deckId: -1,
        deckName: "Silverquill Influence",
        ready: true,
      },
      {
        seat: 1,
        userId: 2,
        username: "bob",
        deckId: -2,
        deckName: "Prismari Artistry",
        ready: true,
      },
    ],
    ...overrides,
  };
}

describe("toLobbyView", () => {
  it("projects a started lobby with start_error null", () => {
    const view = toLobbyView(snap({ startedAt: new Date("2026-07-22T00:00:00Z") }), 1);
    expect(view.started).toBe(true);
    expect(view.start_error).toBeNull();
    expect(view.error).toBeNull();
  });

  it("still reports pre-start gates when not started", () => {
    const notReady = snap({
      seats: [
        {
          seat: 0,
          userId: 1,
          username: "alice",
          deckId: -1,
          deckName: "Silverquill Influence",
          ready: true,
        },
        {
          seat: 1,
          userId: 2,
          username: "bob",
          deckId: -2,
          deckName: "Prismari Artistry",
          ready: false,
        },
      ],
    });
    expect(toLobbyView(notReady, 1).start_error).toBe("NotAllReady");
  });
});

describe("startError", () => {
  it("does not treat started as a start_error code", () => {
    expect(startError(snap({ startedAt: new Date("2026-07-22T00:00:00Z") }), 1)).toBeNull();
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd client && bunx vitest run lib/lobby-store.test.ts`

Expected: FAIL — `start_error` is `"AlreadyStarted"` and/or `startError` returns `"AlreadyStarted"`.

- [ ] **Step 3: Implement projection fix**

In `client/lib/lobby-store.ts`, change `startError` to drop the `AlreadyStarted` branch:

```ts
export function startError(snap: LobbySnapshot, userId: number): string | null {
  if (snap.hostUserId !== userId) return "NotHost";
  if (!snap.seats.some((s) => s.userId === userId)) return "NotSeated";
  if (snap.seats.length < 2) return "NeedTwoPlayers";
  if (!snap.seats.every((s) => s.ready)) return "NotAllReady";
  return null;
}
```

Change `toLobbyView` `start_error` assignment:

```ts
  start_error:
    snap.startedAt != null
      ? null
      : userId == null || you == null
        ? "NotSeated"
        : startError(snap, userId),
```

In `client/server/routes/api/[...path].ts` `isStart` handler, before calling `startError`, treat already-started as idempotent success:

```ts
  if (isStart) {
    const tableId = String(body.table_id ?? "");
    const snap = await loadLobby(db, tableId);
    if (!snap) {
      return json(toLobbyView(unknownLobby(tableId), me.id, "UnknownTable"), 404);
    }
    if (snap.startedAt) {
      return json(toLobbyView(snap, me.id));
    }
    const err = startError(snap, me.id);
    if (err) return json(toLobbyView(snap, me.id, err));
    // ... existing seed + commitStart path unchanged ...
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd client && bunx vitest run lib/lobby-store.test.ts`

Expected: PASS (all tests in file).

- [ ] **Step 5: Do not commit yet** (single final commit in Task 6)

---

### Task 2: Biome `noNonNullAssertion` cleanup

**Files:**
- Modify: `client/app/board/geometry/layout.ts`
- Modify: `client/app/board/geometry/layout.test.ts`
- Modify: `client/app/board/geometry/density.test.ts`

**Interfaces:**
- Consumes: existing `layout`, `withClusterFan`, `RenderCard` APIs (no signature changes)
- Produces: same behavior; lint warning-clean

- [ ] **Step 1: Fix `layout.ts` production assertions**

Replace the `rowSlots` return map body:

```ts
  return order.map((k) => {
    const group = groups.get(k);
    if (!group) return { members: [] as ObjectView[] };
    const members = [...group].sort((a, b) => a.id - b.id);
    return { members };
  });
```

Replace attachment host id uses with a local guard helper (same file, near `stacksOnHost`):

```ts
  const attachedHostId = (o: ObjectView): number | null =>
    o.attached_to == null ? null : o.attached_to;
  const stacksOnHost = (o: ObjectView) => {
    const hostId = attachedHostId(o);
    return isAttached(o) && hostId != null && freeHostIds.has(hostId);
  };
  const hostsWithAttachments = new Set(
    state.objects
      .filter((o) => o.zone === ZONE.Battlefield && stacksOnHost(o))
      .map((o) => attachedHostId(o))
      .filter((id): id is number => id != null),
  );
```

In the attachments loop:

```ts
  for (const a of attachments) {
    const hostId = attachedHostId(a);
    if (hostId == null) continue;
    const list = byHost.get(hostId) ?? [];
    list.push(a);
    byHost.set(hostId, list);
  }
```

- [ ] **Step 2: Fix tests — prefer `expect` + early fail**

In `density.test.ts`:

```ts
    expect(fan[0].fanAngle).toBeDefined();
    expect(fan[2].fanAngle).toBeDefined();
    expect(fan[0].fanAngle).toBeLessThan(0);
    expect(fan[2].fanAngle).toBeGreaterThan(0);
```

```ts
    const outer = withClusterFan([cluster], 10).find((c) => c.id === 12);
    expect(outer).toBeDefined();
    expect(outer!.x + outer!.w / 2).not.toBeCloseTo(cluster.x + cluster.w / 2);
```

Wait — `outer!` still trips the rule. Use:

```ts
    const outer = withClusterFan([cluster], 10).find((c) => c.id === 12);
    expect(outer).toBeDefined();
    if (!outer) return;
    expect(outer.x + outer.w / 2).not.toBeCloseTo(cluster.x + cluster.w / 2);
```

Same pattern in `layout.test.ts` for every `bf.find(...)!` and `byId.get(...)!`:

```ts
    const cluster = bf.find((c) => c.cluster > 1);
    expect(cluster).toBeDefined();
    if (!cluster) return;
    expect(cluster).toMatchObject({ id: 10, cluster: 4, name: "Saproling" });
```

Apply to all flagged sites (lines ~375, ~420, ~494–496, ~533–534 and any remaining `!` in that file that Biome reports).

- [ ] **Step 3: Run lint and geometry tests**

Run:

```bash
cd client && bun run lint
cd client && bunx vitest run app/board/geometry/layout.test.ts app/board/geometry/density.test.ts
```

Expected: lint prints `Found 0 warnings` (or no warnings line); both test files PASS.

- [ ] **Step 4: Do not commit yet**

---

### Task 3: Keep Vitest method-gate out of Nitro output

**Files:**
- Delete: `client/server/routes/api/rpc/method-gate.test.ts`
- Create: `client/lib/wire/rpc-method-gate.test.ts`

**Interfaces:**
- Consumes: default export handler from `client/server/routes/api/rpc/[...path].ts`
- Produces: same Vitest coverage; no `*.test.*` under `client/.output` after build

- [ ] **Step 1: Move the test**

Create `client/lib/wire/rpc-method-gate.test.ts` with the same body as the old file, but import:

```ts
import rpcHandler from "../../server/routes/api/rpc/[...path]";
```

Delete `client/server/routes/api/rpc/method-gate.test.ts`.

- [ ] **Step 2: Run the moved test**

Run: `cd client && bunx vitest run lib/wire/rpc-method-gate.test.ts`

Expected: PASS (same assertions as before).

- [ ] **Step 3: Production build has no test artifacts**

Run:

```bash
cd client && bun run build
find client/.output -name '*.test.*' -o -name '*method_gate*' 2>/dev/null | head
```

Expected: build succeeds; `find` prints nothing matching test/method_gate artifacts.

- [ ] **Step 4: Do not commit yet**

---

### Task 4: Neutralize migration-era code comments

**Files:**
- Modify comments only in:
  - `client/app/faro.ts`
  - `client/lib/faro/collect.ts`
  - `client/lib/wire/rpcServer.ts`
  - `client/server/routes/api/[...path].ts`
  - `client/lib/outcome.ts`
  - `client/lib/inspect.ts`
  - `client/app/board/geometry/stackLayout.ts`
  - `client/lib/ui/buttonClass.ts`
  - `client/lib/effect/client.ts`

**Interfaces:** none (comments only)

- [ ] **Step 1: Rewrite headers to present tense**

Examples (apply the same spirit to each file):

| File | New header intent |
|---|---|
| `faro.ts` | Faro RUM boot for the Foldkit entry (no-op without upstream). |
| `faro/collect.ts` | Faro collect helpers for the Nitro `/api/faro/collect` route. |
| `rpcServer.ts` | `/api/rpc` dispatcher — unit-testable without a Nitro route. |
| `api/[...path].ts` | ALLOWED_METHODS comment: Nitro handler methods for lobby/meta. |
| `outcome.ts` / `inspect.ts` / `stackLayout.ts` / `buttonClass.ts` | Drop “Ported from Solid…”; keep behavior description if useful. |
| `effect/client.ts` | Effect client surface under `client/lib`. |

Do not narrate the migration.

- [ ] **Step 2: Grep for leftover active wording**

Run: `rg -n 'SolidStart|Vinxi|ported from Solid|client/src/' client --glob '!**/node_modules/**' --glob '!**/.output/**'`

Expected: no matches in comments/code under `client/` (docs are Task 5). If a match is an import path or intentional historical note in a test name, fix or justify; prefer zero matches.

- [ ] **Step 3: Do not commit yet**

---

### Task 5: Present-tense docs

**Files:**
- Modify: `README.md`
- Modify: `DESIGN.md`
- Modify: `docs/superpowers/specs/2026-07-20-client-shell-deck-builder-and-observability.md`
- Modify: `docs/superpowers/specs/2026-07-20-board-composition.md`
- Modify: `docs/superpowers/specs/2026-07-20-client-shell-deck-builder-and-observability.md`
- Modify: `docs/superpowers/plans/2026-07-21-foldkit-client-migration.md`
- Modify: `docs/superpowers/specs/2026-07-20-wire-protocol-and-visibility.md`
- Modify: `docs/superpowers/specs/2026-07-20-production-topology-and-operations.md`
- Modify: `docs/superpowers/specs/2026-07-20-lobby-table-routing-and-live-game.md`
- Modify: `docs/WIRE_COMPAT.md`
- Modify: current companion docs/specs (status/current behavior notes when work lands)

**Interfaces:** none

- [ ] **Step 1: `README.md` stack + dev lines**

Replace stack rows and `just dev` line:

```markdown
| Wire | `.proto` → tonic gRPC (API) + Effect RPC (browser → Nitro BFF) |
| BFF / client | Foldkit SPA on Nitro (Vite); lobby + `table_routes` on Postgres `mtgfr_web` (Drizzle); canvas + Mount bitmap board + thin HTML overlays |
```

```bash
just dev                      # tmux: bacon server (:8080) + Foldkit/Vite client (default :3000)
```

- [ ] **Step 2: `DESIGN.md` implementation line**

```markdown
**Implementation:** Token values live in the YAML frontmatter above; Tailwind/theme wiring in `client/styles/global.css` (client-shell-deck-builder-and-observability spec). Canvas paint hexes in `client/app/board/` (`layout.ts` and paint modules) are exempt from DOM tokens; keep the legend list in sync with draw.
```

Also update the Components line if it still says `~/components/atoms` Solid wrappers — point at Foldkit `client/lib/ui/` helpers (`buttonClass`, surfaces) instead.

- [ ] **Step 3: Shell spec — Module + Solution + test inventory**

Status line: `Current (as of 2026-07-22)`.

**Module** (single line, present paths):

```markdown
**Module:** `client/app/` (entry, routes, update/view), `client/app/shell/**` (auth, decks, lobby), `client/lib/**` (rpc-client, wire, lobby-store, faro, ui helpers), `client/server/**` (Nitro BFF routes + Drizzle), `client/styles/global.css`
```

**Solution** paragraph — replace SolidStart/Vinxi/atoms wording with:

```markdown
The client is a **Foldkit** SPA on **Nitro** (Vite). A single event-reactor owns all routes (`client/app/`: `Model` / `Message` / `update` / `view` with shell submodels). Async/wire work uses Effect at runtime boundaries (`client/lib/rpc-client.ts`, streams, BFF); Foldkit owns UI state. The wire contract is a hand-written Effect HTTP client over the same-origin `/api/rpc` BFF, which dials tonic gRPC. Design tokens live in `DESIGN.md` YAML and Tailwind v4 `@theme` in `client/styles/global.css`. Biome handles format/lint. Observability: Grafana Faro (browser) + `@effect/opentelemetry` (BFF) + OTLP/tonic (API), no-op locally unless OTLP is set.
```

Rewrite **App shell and routing** behavior intro to Foldkit routes (`client/app/routes.ts`, `client/app/view.ts`) — paths stay the same table; drop SolidStart `<Router>` / `<FileRoutes />` / `RequireAuth` component names in favor of Foldkit route guards / auth submodel.

Update **Tests** inventory paths from `client/src/...` to current locations (`client/app/shell/**/*.test.ts`, `client/lib/**/*.test.ts`, `client/lib/wire/*.test.ts`, etc.). Prefer listing directories + key files rather than inventing missing test files.

Keep **Further Notes** pointer to the migration design.

- [ ] **Step 4: Board spec — Module + Solution**

Status: `Current (as of 2026-07-22)`.

**Module:**

```markdown
**Module:** `client/app/board/` — `canvas/` (vector felt/avatars/arrows/scene), `bitmap/` (Mount card art + flights), `html/` (hand, stack, prompts, chrome), `geometry/` (camera, hit-test, layout, density, interaction), `action/` (session, targeting, execution), `motion/flights.ts`, plus `submodel.ts` / `view.ts` / `messages.ts`
```

Solution stays dual-surface; ensure wording is Foldkit Canvas + Mount + HTML overlays (not Solid `board.tsx`). Living map remains `docs/client-canvas-map.md`. Keep Further Notes.

- [ ] **Step 5: Migration design + plan archive banners**

In the current client spec headers:

```markdown
**Status:** Done (cutover on branch; pending merge)  
**Note:** Historical design. Live architecture is `AGENTS.md` plus the shell and board specs.
```

In `2026-07-21-foldkit-client-migration.md` top (after title):

```markdown
> **Historical plan — executed.** Do not use as a task list. Live architecture: `AGENTS.md`, shell/board specs, and `docs/client-canvas-map.md`.
```

- [ ] **Step 6: Wire, lobby, production, WIRE_COMPAT present-tense fixes**

Replace SolidStart/Vinxi/`client/src/wire` claims:

| Doc | Present tense |
|---|---|
| wire-protocol | Module: `client/lib/wire/`; BFF is Nitro; generated clients under `client/lib/wire/generated/` |
| lobby-table-routing | BFF `client/server/` (Nitro lobby + `table_routes`); pre-game lobby on Nitro BFF |
| production-topology | Player story: Foldkit SPA; diagram/Deployment `edh-web` = Nitro BFF; Faro boot `client/app/faro.ts`; build `bun run build` → `.output/` |
| WIRE_COMPAT | “Foldkit SPA may roll with newest…” |

Do **not** rewrite unrelated historical plan files (e.g. activation-radial plans) unless they are linked as current architecture.

- [ ] **Step 7: Grep docs for present-tense drift**

Run:

```bash
rg -n 'SolidStart|Vinxi|client/src/' README.md DESIGN.md AGENTS.md docs/superpowers/specs docs/WIRE_COMPAT.md docs/client-canvas-map.md docs/agent-navigation.md
```

Expected: matches only inside historical migration design/plan bodies (acceptable) or this merge-cleanup design’s context sentences. No “current module” paths still under `client/src/`.

- [ ] **Step 8: Do not commit yet**

---

### Task 6: Verify, commit once, update PR metadata

**Files:**
- All files from Tasks 1–5
- PR title/body via GitHub (ManagePullRequest / `gh pr edit`)

**Interfaces:** none

- [ ] **Step 1: Full client check**

Run: `just client-check`

Expected: format ok, lint **0 warnings**, typecheck ok, all tests pass (including new `lobby-store` + moved method-gate tests).

- [ ] **Step 2: Build output check**

Run:

```bash
cd client && bun run build
find .output -name '*.test.*' | tee /tmp/nitro-test-artifacts.txt
test ! -s /tmp/nitro-test-artifacts.txt
```

Expected: build ok; empty artifact list.

- [ ] **Step 3: Stage everything and commit once**

```bash
git add -A
git status
git commit -m "$(cat <<'EOF'
docs: foldkit merge cleanup for cutover PR

Present-tense docs, lobby start_error fix, biome non-nulls,
Nitro test exclusion, and neutral client comments.
EOF
)"
```

If commitlint rejects `docs:` alone while body includes a fix, prefer:

```text
fix: foldkit merge cleanup for cutover PR
```

(subject ≤72, lower-case). Include both docs and fix in body.

- [ ] **Step 4: Push**

```bash
git push -u origin cursor/foldkit-migration-design-1ef0
```

- [ ] **Step 5: Update PR title and body**

Title (exact):

```text
feat: replace solidstart client with foldkit and nitro
```

Body must include summary, test plan, and:

```text
BREAKING CHANGE: client is Foldkit SPA on Nitro; SolidStart/Vinxi removed.
```

No `feat!:` in the title (squash message is the title; Angular `subject-exclamation-mark` is for commits, but semantic-release accepts `BREAKING CHANGE` footer — put the footer in the PR body so the squash commit message includes it if the merge strategy preserves body; if the host only uses title, also ensure release notes process uses body — per AGENTS.md: “title PRs with feat: / fix: (or a BREAKING CHANGE: footer)”. Put BREAKING CHANGE in the PR body; if squash only uses title, use title `feat: replace solidstart client with foldkit and nitro` and ensure the footer is in the commit that lands — **this PR’s squash message is the title only**, so for a major release the title must convey breaking change. AGENTS says: “title PRs with `feat:` / `fix:` (or a `BREAKING CHANGE:` footer) when the merge should cut a release”. Footer in PR body may not become the squash subject. Options that work:

1. PR title stays `feat: replace solidstart client with foldkit and nitro` and PR body has `BREAKING CHANGE: ...` — **only works if squash includes body**; GitHub squash-merge can include commit message body from PR if configured.
2. Rely on existing branch commit’s `BREAKING CHANGE` footer on the cutover commit — but squash replaces all commits with the title line only.

Per AGENTS.md: “The squash commit message on `main` is the **PR title** (plus `(#N)`), not the branch’s individual commits. semantic-release analyzes that squash line only.”

So the **title alone** must trigger a major for semantic-release. Angular/conventional-changelog typically needs `feat!:` or `BREAKING CHANGE` in the commit message. If only the title is used, use:

```text
feat!: replace solidstart client with foldkit and nitro
```

Conflict: commitlint on PR commits forbids `!`, but **PR title is not commitlint-checked** (only commits in range). Therefore:

- Branch commits: no `!` (already true on tip after cleanup commit).
- **PR title: `feat!: replace solidstart client with foldkit and nitro`** so squash+semantic-release cuts a major.
- PR body: also include `BREAKING CHANGE: client is Foldkit SPA on Nitro; SolidStart/Vinxi removed.` for human readers.

Update the merge-cleanup design’s PR metadata section if it said “no `!`” — implementers follow this plan’s release rule: **title may use `feat!:` because commitlint does not lint PR titles; commits must not use `!`.**

- [ ] **Step 6: Confirm CI**

Run: `gh pr checks 74`

Expected: Commitlint, Verify (client), Verify (server), Terraform all pass (or pending→pass).

- [ ] **Step 7: Mark merge-cleanup design status Done** (if not already in the same commit)

In the current companion docs/specs:

```markdown
**Status:** Done
```

If this status edit was missed in the single commit, amend only if the commit has not been pushed; otherwise a tiny follow-up is acceptable only if CI already green and user wants zero follow-ups — prefer including status in the single commit before push.

---

## Spec coverage checklist

| Spec requirement | Task |
|---|---|
| README / DESIGN present-tense | Task 5 |
| Shell + board specs Module/architecture/tests | Task 5 |
| Migration design/plan archive | Task 5 |
| Other specs claiming SolidStart | Task 5 (wire, lobby, production, WIRE_COMPAT) |
| Comment hygiene | Task 4 |
| `start_error` null when started + test | Task 1 |
| Idempotent start when already started | Task 1 |
| Biome non-null warnings | Task 2 |
| Nitro no test artifacts | Task 3 |
| Single commit | Task 6 |
| PR title/body + verify | Task 6 |

## Release-title note (resolves design vs AGENTS)

Design draft said PR title without `!`. AGENTS.md states squash uses **title only** and semantic-release analyzes that line. For a major release, the PR title must be `feat!: ...` (or include breaking change in that single line). Commitlint does not apply to PR titles. **Plan overrides design on this point only:** PR title uses `feat!:`; branch commits never use `!`.
