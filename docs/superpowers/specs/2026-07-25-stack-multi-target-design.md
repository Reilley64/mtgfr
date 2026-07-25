# Stack multi-target truth — design

**Status:** Approved for planning (2026-07-25)
**Delivery:** Sequential PR 1 of 5 (wins 2 → 1 → 3 → 5 → 6)
**Living surface specs to update at implement time:**
[`2026-07-20-stack.md`](2026-07-20-stack.md),
[`2026-07-20-wire-protocol-and-visibility.md`](2026-07-20-wire-protocol-and-visibility.md)
(field table for `StackObjectView`)

## Problem

Commander multi-target spells and dual-clause stack objects project a single
`StackObjectView.target` (schema uses `Game::spell_target` → primary only). The
board paints one Island Blue arrow and captions that primary. Players cannot
read the full declared target set from the stack surface.

## Goal

Tell the truth on the stack: every declared target is on the wire, painted as an
arrow, and listed in stack chrome captions. Expand-only wire; no hard breaks.

## Non-goals

- Modal per-mode target chrome in the stack panel.
- Removing singular `optional WireTarget target` (compat / expand-only).
- Retarget UX, stack dwell changes, canvas-painted stack cards.
- Client inference of targets from fold/log events.
- Wins 1, 3, 5, 6 (separate sequential PRs).

## Approach

**Expand-only wire + schema projection + client paint** (rejected: client-only
synthesis from log events; rejected: replace `target` with `targets` only).

### Wire & projection

- Proto `StackObjectView`: add `repeated WireTarget targets = 7`.
- Keep `optional WireTarget target = 5` as the primary (first chosen) for older
  clients and existing call sites.
- Schema snapshot for spells: concatenate `spell.targets.iter()` then
  `spell.targets_second.iter()` into `targets`; set `target = targets.first()`.
- Abilities: project the ability’s chosen target list the same way (today’s
  singular becomes 0–1 entries).
- Targetless entries: empty `targets`, unset/None `target`.
- Never invent targets on the client. After codegen, TypeScript gains
  `targets?: WireTarget[]` while `target` remains.

### Client arrows & captions

- Resolve list: use `entry.targets` when present and non-empty; else fall back to
  wrapping `entry.target` as a one-element list.
- Pure helpers (unit-tested): `stackEntryTargets(entry)` and caption formatting
  that joins target labels with `, `.
- `stackTargetArrowShapes`: one Island Blue arrow per resolved target from the
  same `stackFaceScreenOrigin` as today; skip unresolved/off-board destinations.
- Pile top caption and expanded/full faces show all target labels (not only
  primary).

### Testing

- Schema: multi-target (incl. second clause) projects full `targets` and
  `target === targets[0]`.
- Client unit: 0 / 1 / N targets; mixed player + object; missing permanent skipped.
- Scene: expanded/full stack shows multi-target caption text.
- Geometry/arrow tests extend existing `stackTargetArrowShapes` coverage.
- Codegen path stays green (`just server-codegen` / client gen recipes).

### Spec updates (implement PR)

- [`stack`](2026-07-20-stack.md): Behavior — multi-target arrows + captions;
  Testing — multi-target cases.
- [`wire-protocol-and-visibility`](2026-07-20-wire-protocol-and-visibility.md):
  `stack` row documents optional primary `target` plus `targets` list.

## Error / empty

- Targetless → no arrows, no target caption line.
- Partial remaining targets after illegality/counter → project whatever the
  engine still has on the stack object.

## Success criteria

- A multi-target spell on the stack shows an arrow to each declared target that
  has a resolvable screen destination.
- Expanded/full (and pile top) captions list every declared target label.
- Singular `target` remains populated as the first target for wire compat.
- No regression for single-target and targetless stack entries.
