# Brand edh.reilley.dev Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace player-facing product string `mtgfr` with `edh.reilley.dev` and identify Scryfall/tooling HTTP clients as `edh.reilley.dev/0.1`.

**Architecture:** String replace at existing call sites only — no shared product-name constant. Scene tests lock UI wordmarks; a small Scryfall fetch-header unit test locks the client User-Agent; tooling scripts are updated in the same pass and verified by grep.

**Tech Stack:** Foldkit Scene tests, Vitest, TypeScript, plain `.mjs` tooling scripts.

**Spec:** [brand-edh-reilley-dev-design](../specs/2026-07-24-brand-edh-reilley-dev-design.md)

## Global Constraints

- Exact wordmark / title string: `edh.reilley.dev` (lowercase, no scheme).
- Exact User-Agent: `edh.reilley.dev/0.1` (analyze-otags: `edh.reilley.dev/0.1 (otag analysis)`).
- No shared constant module for the brand string.
- Do **not** rename DBs, proto, GHCR, K8s secrets, npm/cargo names, clap CLI, Terraform hostname, localStorage keys, Faro/OTEL service names, or Style Dictionary format ids.
- Do **not** change layout/typography — text only.
- Guard-return-first; imports at top of file.
- TDD where tests apply; Angular commit messages on `cursor/brand-edh-reilley-dev-b23c`.
- Interaction/UI checklist when Scene wordmark assertions are added.

---

## File map

| File | Responsibility |
|------|----------------|
| `client/app/shell/surfaces.test.ts` | Scene: auth + lobby wordmark text |
| `client/app/view.ts` | Nav brand + `Document.title` |
| `client/app/shell/auth/view.ts` | Auth hero |
| `client/app/shell/lobby/view.ts` | Lobby hero |
| `client/index.html` | `<title>` |
| `client/lib/deck-builder/scryfall.ts` | Client Scryfall User-Agent |
| `client/lib/deck-builder/scryfall.test.ts` | Unit: fetch headers include new UA |
| `tooling/*.mjs` (listed below) | Tooling User-Agents |
| `README.md` | Repo H1 |
| `DESIGN.md` | Design system H1 |
| `docs/superpowers/specs/2026-07-24-brand-edh-reilley-dev-design.md` | Status → Implemented |

---

### Task 1: UI wordmark + Scene assertions

**Files:**
- Modify: `client/app/shell/surfaces.test.ts`
- Modify: `client/app/view.ts`
- Modify: `client/app/shell/auth/view.ts`
- Modify: `client/app/shell/lobby/view.ts`
- Modify: `client/index.html`

**Interfaces:**
- Produces: player-visible brand string `edh.reilley.dev` on auth hero, lobby hero, nav brand, document title

- [ ] **Step 1: Write the failing Scene assertions**

In `surfaces.test.ts`, inside `"renders auth login surfaces from the app view"`, add:

```ts
Scene.expect(Scene.text("edh.reilley.dev")).toExist(),
Scene.expect(Scene.text("mtgfr")).not.toExist(),
```

In the lobby surface test that already expects `Scene.text("Lobby")` (same file), add the same two wordmark expectations when the lobby entry chrome (with brand hero) is rendered.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd client && bunx vitest run app/shell/surfaces.test.ts`

Expected: FAIL — auth/lobby still render `mtgfr`, not `edh.reilley.dev`

- [ ] **Step 3: Replace UI strings**

Exact replacements:

```ts
// client/app/view.ts — nav brand link children and Document.title
["edh.reilley.dev"]
title: "edh.reilley.dev",

// client/app/shell/auth/view.ts — hero
["edh.reilley.dev"]

// client/app/shell/lobby/view.ts — hero
["edh.reilley.dev"]
```

```html
<!-- client/index.html -->
<title>edh.reilley.dev</title>
```

Do not change surrounding classes or layout.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd client && bunx vitest run app/shell/surfaces.test.ts`

Expected: PASS

Also confirm no remaining UI brand literals:

Run: `rg -n '"mtgfr"|'\''mtgfr'\''|>mtgfr<|title>mtgfr' client/app client/index.html`

Expected: no matches (DB/test URLs elsewhere may still say `mtgfr_web` — those are out of scope)

- [ ] **Step 5: Commit**

```bash
git add client/app/view.ts client/app/shell/auth/view.ts client/app/shell/lobby/view.ts \
  client/index.html client/app/shell/surfaces.test.ts
git commit -m "feat(client): brand shell wordmark as edh.reilley.dev"
```

---

### Task 2: Scryfall + tooling User-Agent

**Files:**
- Modify: `client/lib/deck-builder/scryfall.ts`
- Create: `client/lib/deck-builder/scryfall.test.ts`
- Modify:
  - `tooling/backfill-oracle.mjs`
  - `tooling/backfill-card-ids.mjs`
  - `tooling/backfill-card-meta.mjs`
  - `tooling/backfill-otags.mjs`
  - `tooling/rewrite-precon-fixtures.mjs`
  - `tooling/rewrite-grind-precon-fixtures.mjs`
  - `tooling/analyze-otags.mjs`

**Interfaces:**
- Produces: every outbound Scryfall-related User-Agent uses `edh.reilley.dev/0.1` (analyze-otags keeps the suffix)

- [ ] **Step 1: Write the failing unit test**

```ts
// client/lib/deck-builder/scryfall.test.ts
import { afterEach, describe, expect, it, vi } from "vitest";
import { searchPrints } from "./scryfall";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("searchPrints User-Agent", () => {
  it("identifies as edh.reilley.dev/0.1", async () => {
    const fetchMock = vi.fn(async () =>
      new Response(JSON.stringify({ data: [], has_more: false }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await searchPrints("00000000-0000-0000-0000-000000000000");

    expect(fetchMock).toHaveBeenCalled();
    const init = fetchMock.mock.calls[0]?.[1] as RequestInit | undefined;
    const headers = init?.headers as Record<string, string>;
    expect(headers["User-Agent"]).toBe("edh.reilley.dev/0.1");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd client && bunx vitest run lib/deck-builder/scryfall.test.ts`

Expected: FAIL — header still `mtgfr/0.1`

- [ ] **Step 3: Update client + tooling User-Agents**

In `scryfall.ts`:

```ts
"User-Agent": "edh.reilley.dev/0.1",
```

In each tooling file listed above, replace `mtgfr/0.1` with `edh.reilley.dev/0.1`.  
In `tooling/analyze-otags.mjs` only:

```js
const UA = { "User-Agent": "edh.reilley.dev/0.1 (otag analysis)", Accept: "application/json" };
```

- [ ] **Step 4: Verify tests + grep**

Run: `cd client && bunx vitest run lib/deck-builder/scryfall.test.ts`

Expected: PASS

Run: `rg -n 'mtgfr/0\.1' client/lib/deck-builder/scryfall.ts tooling/`

Expected: no matches

- [ ] **Step 5: Commit**

```bash
git add client/lib/deck-builder/scryfall.ts client/lib/deck-builder/scryfall.test.ts tooling/
git commit -m "feat(client): identify Scryfall clients as edh.reilley.dev"
```

---

### Task 3: README, DESIGN.md, design status, verify

**Files:**
- Modify: `README.md` (H1 only)
- Modify: `DESIGN.md` (H1 only)
- Modify: `docs/superpowers/specs/2026-07-24-brand-edh-reilley-dev-design.md` (status line → Implemented)

- [ ] **Step 1: Update titles and design status**

```markdown
<!-- README.md line 1 -->
# edh.reilley.dev

<!-- DESIGN.md line 1 -->
# Design System: edh.reilley.dev
```

In the design spec header:

```markdown
**Status:** Implemented
```

Leave README body prose that describes the product; do not rename the GitHub repo or rewrite historical “mtgfr codebase” mentions outside these titles.

- [ ] **Step 2: Full client verification**

Run: `cd client && bun run typecheck && bunx vitest run app/shell/surfaces.test.ts lib/deck-builder/scryfall.test.ts`

Expected: typecheck exit 0; tests PASS

Optional broader: `cd client && bunx vitest run` if time allows.

- [ ] **Step 3: Final grep safety check**

Run:

```bash
rg -n 'User-Agent.: .mtgfr|"mtgfr"|title>mtgfr|# mtgfr|# Design System: mtgfr' \
  client/app client/index.html client/lib/deck-builder/scryfall.ts \
  tooling README.md DESIGN.md
```

Expected: no matches for player brand / UA / those two H1s. (`mtgfr_web` in other files is fine.)

- [ ] **Step 4: Commit and push**

```bash
git add README.md DESIGN.md docs/superpowers/specs/2026-07-24-brand-edh-reilley-dev-design.md
git commit -m "docs: title README and design system as edh.reilley.dev"
git push -u origin cursor/brand-edh-reilley-dev-b23c
```

PR title: `feat(client): brand as edh.reilley.dev`  
Body: summary of wordmark + UA; link design spec; note internals intentionally unchanged; Interaction/UI checked for Scene wordmark assertions.

---

## Spec coverage self-check

| Spec requirement | Task |
|------------------|------|
| Wordmark on title / nav / auth / lobby | Task 1 |
| README + DESIGN.md H1 | Task 3 |
| Scryfall + tooling UA | Task 2 |
| No internals rename | (no those files) |
| Scene / UA tests | Tasks 1–2 |
| Design status Implemented | Task 3 |

## Placeholder / consistency check

- Brand string is always `edh.reilley.dev`; UA always `edh.reilley.dev/0.1`.
- No TBD steps.
