# Commander damage on life-orb avatars

**Date:** 2026-07-23  
**Status:** Approved (autonomous improvement loop)  
**Context:** Engine tracks and enforces 21 commander damage; wire projects `PlayerView.commander_damage`. Avatar paint only shows life / name / hand count — players cannot see the format’s kill clock. Commander tax is already shown on the hand/command bar.

## Goals

1. Surface each seat’s highest per-commander damage total on the life orb when > 0.
2. Paint on the **visible** avatar path (Mount bitmap `paintAvatars`, which sits above the Foldkit Canvas vector pass); keep `avatarShapes` in sync for the vector helper. No wire/engine changes.
3. Keep orbs readable at 2–4 seats; avoid per-source chip clutter.

## Non-goals

- Per-commander breakdown UI (inspect dock / hover card later).
- Changing the 21-damage SBA or projection.
- HTML overlays for damage chips.

## Approaches

| Approach | Trade-off |
|----------|-----------|
| **A. Max-only compact `Cmd N` label** | Matches “any one commander to 21”; tiny paint change. **Chosen.** |
| B. List every `(from, amount)` under the orb | Crowded at 4 seats / multi-commander tables. |
| C. HTML chip overlays | Fights canvas avatar ownership; hit-target layering risk. |

## Design

### Aggregation

```ts
maxCommanderDamage(player: PlayerView): number
// max of player.commander_damage[].amount; 0 if absent/empty
```

Loss condition is per single commander source — showing the max is the relevant kill clock.

### Paint

In `paintAvatars` (`bitmap/mount.ts`) and `avatarShapes` (`canvas/avatars.ts`), when `max > 0`, add text below the username:

- Content: `Cmd ${max}`
- Fill: `#db8664` (damage-adjacent; document in battlefield canvas hex list)
- Font ~12px × zoom, centered
- Position: `pos.y + 42 * zoom` (name stays at `+27`; hand / life unchanged)

When `max === 0`, omit the label (no `Cmd 0`).

### Spec updates

- `docs/superpowers/specs/2026-07-20-battlefield.md` — Avatars behavior + hex list + testing.
- Optional legend row if the legend already lists avatar chrome meanings; skip if legend is outlines-only.

## Testing

1. Unit: `maxCommanderDamage` empty / single / multi-source (max wins).
2. `avatarShapes` / scene shape walk: with damage, text `Cmd 14` present; without, absent.
3. No Scene HTML change required (canvas vector); unit shape assertions are the seam.

## Edge cases

| Case | Behavior |
|------|----------|
| Missing `commander_damage` | Treat as empty → no label |
| Multiple sources | Show max only |
| Amount ≥ 21 but player not yet SBA-lost | Still show (snapshot lag / same frame) |
| Lost player | Still show muted orb + Cmd if present |
