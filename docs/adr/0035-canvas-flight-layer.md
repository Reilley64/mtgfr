# 0035 — Canvas flight layer for continuous play motion

Status: **Accepted** (supersedes [0033](0033-segmented-card-play-motion.md))

## Context

Hand and stack rest as DOM overlays; the battlefield is canvas. Segmented legs (DOM ghost dies → blank gap → canvas entrance / CSS stack-in) made plays feel disjointed: three card sizes, no scale lerp, and a handoff that waited on the server delta before any destination art appeared.

## Decision

- Keep **resting** hand and stack as DOM.
- Own in-flight cards on a **canvas flight layer**: one actor per play carries **screen-space position + scale** from commit until settle (~150–200ms exponential ease, same τ as board tweens).
- On hand/command drop, spawn the flight **immediately** (hide DOM ghost / dim slot); retarget when the delta binds the permanent or stack id — never spawn a second `ENTER_RISE` card for that play.
- Stack resolve / leave-stack: flight from `stackAimOrigin` at stack scale to BF/GY layout; suppress duplicate canvas seeds while the flight owns the id.
- Cast → stack: flight hand → stack rest pose, then promote to the DOM stack face (no CSS `scale(0.85)` pop as the primary entrance).
- `prefers-reduced-motion`: snap flights to target.

## Consequences

- Per-card play origins remain useful for binding deltas to flights; continuous flight replaces segmented DOM/canvas legs.
- Opponent plays still enter from avatar (or flight seeded there) when no hand ghost exists.
- Drawing flights in `boardDraw` above board cards; destination faces stay hidden until settle.
