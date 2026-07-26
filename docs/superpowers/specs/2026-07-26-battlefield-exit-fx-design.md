# Battlefield exit FX (destroy / exile) — design

**Status:** Approved for planning (2026-07-26)  
**Living surface specs to update at implement time:**
[`2026-07-20-flights.md`](2026-07-20-flights.md),
[`2026-07-20-table-audio.md`](2026-07-20-table-audio.md),
[`../../client-canvas-map.md`](../../client-canvas-map.md) (flight layer also paints exit FX)

## Problem

When a permanent leaves the battlefield for the graveyard or exile, the client
only glides the card to the zone pile. Destroy and exile share that glide;
nothing evokes a Duelist of the Roses–style burn on death, or a distinct exile
departure. GY vs exile are differentiated only after settle (pile outline colors).

## Goal

Battlefield permanents that are destroyed or exiled play a short **in-place**
exit effect at their last battlefield pose, then the GY/exile pile face updates.
Destroy and exile use **different motions** (not only different colors). Effects
stay on Canvas 2D with a small particle budget — readable table juice, not a
cutscene.

## Non-goals

- Exit FX for mill, discard, hand/library → GY/exile, or stack-only resolutions
  that never sat as a permanent
- Commander → command zone, bounce to hand/library, battlefield → battlefield
- Keeping the zone-move **glide** for these BF→GY / BF→exile exits
- WebGL, OffscreenCanvas, workers, Pixi, or DotR pixel-perfect ash
- Screenshot golden tests for v1 paint

## Approach

**Dedicated `ExitFx` orbit** (rejected: zero-distance `CardFlight` overload;
rejected: dying state painted only on resting bitmap cards).

### Triggers

Keyed off **zone transition**, not a separate “was destroyed” rules flag:

- **Destroy FX:** permanent’s prior zone is battlefield and it enters graveyard
  (`moved_to_graveyard` from BF, including sacrifice / die / destroy).
- **Exile FX:** permanent’s prior zone is battlefield and it enters exile
  (`moved_to_exile` from BF).
- **Not tagged:** any exit whose prior zone was not the battlefield.

Provenance gains an explicit map, e.g.
`battlefieldExits: Map<cardId, "graveyard" | "exile">`, filled in
`extractProvenance` from the engine delta. Sync uses that map; generic
`zoneMoves` must **not** also spawn a glide for the same card.

### Choreography

| Kind | Motion | Particles |
|------|--------|-----------|
| Destroy | Edge-in char → ember rim → ash collapse (~450–650ms) | Warm orange/ember sprites drifting up |
| Exile | Shatter into shards → teal void pinch at center → suck inward and vanish (~450–650ms) | Cool teal/cyan shard sprites |

- FX runs **in place** at the last battlefield layout pose (or current flight
  pose if the card was mid-glide — see Edge cases).
- **No glide** to the pile for these exits.
- Card art stays readable for the first beat; motion takes over afterward.
- Board wipes: all eligible `ExitFx` start **in parallel**; a **global particle
  cap** keeps large wipes light.
- Prefer deferring the GY/exile pile face “attention flash” until FX completes
  so focus stays on the battlefield pose; resting BF paint omits the card for
  the FX duration.

### Model

`ExitFx` (board submodel, parallel to `CardFlight`):

- `id`, `print`, `name`
- `kind: "destroy" | "exile"`
- world `x` / `y` / `scale` (pose)
- progress `0→1` (and/or `t0`)
- particle seed (deterministic per FX)

Pure step/particle math lives under `board/motion/`. Paint lives under
`board/bitmap/` on the existing flight canvas layer. Sync wiring:
`event-fold` provenance → `syncFlightsWithGame` (or a sibling sync) → spawn
`ExitFx` and suppress zone-move flight.

### Edge cases

- **`prefers-reduced-motion`:** skip `ExitFx`; pile/resting state updates
  immediately.
- **Id rebind in the same fold:** attach FX using the pre-exit battlefield pose
  from the prior layout snapshot (same spirit as flight `fromCardId` / rebind).
- **Mid-flight death:** cancel or finish the glide and start `ExitFx` at the
  **current flight pose**, not the stale resting slot.
- **Tokens / face-down:** same FX; paint whatever face the Mount cache has.

### Audio

Light synthesized one-shots when FX starts:

- `tableFeel.destroy`
- `tableFeel.exile`

Skipped with reduced motion or existing audio-off paths. Document in the
table-audio surface spec at implement time.

## Testing

- Provenance: BF→GY / BF→exile tagged; non-BF exits not tagged.
- Sync: tagged cards spawn `ExitFx` and **no** zone-move flight; wipe spawns N
  FX together; particle budget enforced.
- Step: progress reaches completion and FX is removed; reduced-motion → empty
  FX list immediately.
- Paint: unit asserts on kind → particle palette / draw path (no screenshot
  goldens for v1).
- Audio: destroy/exile cue flags set when FX starts.
- Prefer lowest-layer pure tests; add Scene coverage only if a user-visible
  HTML hook appears.

## Spec updates at implement time

- **Flights:** document `ExitFx` orbit, glide suppression for BF→GY/exile,
  choreography summary, reduced-motion bypass, wipe/particle budget.
- **Table audio:** document `destroy` / `exile` cues and trigger conditions.
- **Client canvas map:** note that the flight layer also paints exit FX.

## Out of scope / follow-ups

- Richer DotR-like ash / dissolve if Canvas 2D + particles prove too thin
- Distinct FX for mill, discard, or commander zone moves
- Damage / combat-death juice beyond zone exit
