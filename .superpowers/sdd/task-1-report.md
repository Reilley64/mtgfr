# Task 1 Report: Shell font tokens + @fontsource wiring

## Summary

Implemented Wave 1 Task 1 exactly per the task brief: added `@fontsource/manrope` and `@fontsource/space-grotesk` packages, extended `design.tokens.json` with `font.shell` and `font.display` tokens (keeping `font.sans` for board/legacy), wired font-face CSS imports in `global.css`, and regenerated Style Dictionary outputs.

## What was implemented

### Step 1 — Font packages

Installed pinned versions via Bun:

- `@fontsource/manrope@5.2.5`
- `@fontsource/space-grotesk@5.2.6`

Both resolved exactly as specified in the brief (no registry fallback needed).

### Step 2 — `design.tokens.json`

Replaced the `font` block with three entries:

| Token | Value | Description |
|-------|-------|-------------|
| `font.sans` | `system-ui, sans-serif` | Board HUD / canvas chrome |
| `font.shell` | `"Manrope", system-ui, sans-serif` | Shell UI body/controls |
| `font.display` | `"Space Grotesk", system-ui, sans-serif` | Shell titles/brand/table code |

### Step 3 — `client/styles/global.css`

Added five `@fontsource` imports immediately after the file header comment and before `@import "tailwindcss";`:

- Manrope 400, 600, 700
- Space Grotesk 500, 700

### Step 4 — Token regeneration

Ran `bun run gen:tokens` in `client/`. Updated:

- `client/styles/tokens.generated.css` — now includes `--font-shell` and `--font-display` in `@theme`
- `client/app/domain/design-tokens.generated.ts` — regenerated (gitignored; not committed per brief)

Tailwind v4 `@theme` `--font-*` entries produce utilities `font-sans`, `font-shell`, and `font-display`.

### Step 5 — Sanity check

```bash
rg -n "font-shell|font-display|Manrope|Space Grotesk" \
  /workspace/client/styles/tokens.generated.css /workspace/design.tokens.json
```

Matches confirmed in both files.

## Verification

| Check | Result |
|-------|--------|
| `bun run gen:tokens` | Exit 0; wrote `tokens.generated.css` |
| `rg` sanity grep | Matches in JSON + generated CSS |
| `bun test app/domain/design-tokens.test.ts app/domain/ui/surfaces.test.ts` | 8 pass, 0 fail |

No new test harness added — brief specified grep-only verification when `gen-tokens.test.mjs` does not exist (it does not).

## Files changed

| File | Change |
|------|--------|
| `client/package.json` | Added `@fontsource/manrope`, `@fontsource/space-grotesk` |
| `client/bun.lock` | Lockfile updated |
| `design.tokens.json` | Extended `font` section with `shell` + `display` |
| `client/styles/global.css` | Fontsource CSS imports |
| `client/styles/tokens.generated.css` | Regenerated `--font-shell`, `--font-display` |

## Commit

```
ce24e23 feat(client): add Manrope and Space Grotesk shell font tokens
```

## Self-review

**Scope compliance:** Task 1 only — no `shellFrame`, atmosphere classes, or portrait rotate work. Correct.

**Token semantics:** `font.sans` unchanged for board/legacy (`feltClass` still uses `font-sans`). Shell fonts are available but not yet applied to shell surfaces; that is expected for later tasks.

**Import order:** Fontsource imports precede Tailwind as required so `@font-face` rules are available when theme utilities resolve.

**Naming collision note:** `text.display` (22px size token) and `font.display` (Space Grotesk family) are distinct namespaces (`--text-display` vs `--font-display`). No conflict observed.

**Optional utility notes:** Brief mentioned "optional utility notes" in global.css; none were specified in the brief body, so none added.

**Tests:** Existing `design-tokens.test.ts` still passes (`--font-sans` assertion). `surfaces.test.ts` does not assert `font-sans` on `feltClass` but `feltClass()` implementation still includes it — verified by reading `surfaces.ts`.

## Concerns

None blocking. Follow-up tasks will need to apply `font-shell` / `font-display` to shell chrome; fonts are loaded and tokenized but visually unchanged until then.
