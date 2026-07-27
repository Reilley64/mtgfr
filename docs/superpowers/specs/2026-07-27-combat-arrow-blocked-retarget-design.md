# Combat arrow blocked retarget — design

**Status:** Approved for planning (2026-07-27)  
**Living surface specs to update at implement time:**
[`2026-07-20-combat-and-commander-rules.md`](2026-07-20-combat-and-commander-rules.md),
[`2026-07-20-battlefield.md`](2026-07-20-battlefield.md),
[`2026-07-20-wire-protocol-and-visibility.md`](2026-07-20-wire-protocol-and-visibility.md)

## Problem

Committed combat paint always draws a red attacker→defender arrow (today always the
player avatar) plus green blocker→attacker arrows. After blockers are declared that
presentation still points at the player/planeswalker, which reads as “this will hit
the seat” even when the attacker is blocked. If a blocking creature leaves combat
(sacrifice, bounce, destroy), the green arrow disappears and the red player arrow
remains — worse, the engine currently drops the `(blocker, attacker)` pair from
`CombatState.blocks` in `remove_from_combat`, so damage assignment treats an empty
blocker list as **unblocked** and can deal full power to the player. That violates
CR 509.1h (a blocked creature remains blocked for that combat) and, for trample,
fails to apply CR 702.19b (blocked trample with no living blockers assigns all
damage to the player or planeswalker it’s attacking).

## Goal

1. **Board:** After every attacked seat has finished declaring blockers, show combat
   as attacker→living blockers (red only). Unblocked attackers keep red→player or
   red→planeswalker card. Blocked attackers with no living blockers show **no**
   combat arrow (including trample).
2. **Rules:** Track that an attacker became blocked for the rest of combat. Damage
   uses living blockers when present; when none remain, non-trample deals nothing to
   the player/PW, trample deals full power to the player/PW.
3. **Pre-declaration:** Until all attacked seats have declared, keep today’s staging
   language — red→defender (player avatar or planeswalker card) and green
   blocker→attacker for staged and committed blocks.

## Non-goals

- Changing declare-attackers / declare-blockers drag staging UX beyond arrow
  endpoints (confirm, click-to-cancel, coach copy stay as today).
- Showing a special “blocked, no arrow” badge or chrome; absence of the arrow is
  the signal.
- Trample overflow visualization while living blockers remain (no dual
  blocker+player arrows).
- Fixing unrelated combat fidelity gaps (menace edge cases, planeswalker damage
  mid-implementation elsewhere) beyond remains-blocked + trample-with-zero-blockers.

## Approach

**Engine durable blocked set + shared client arrow endpoints** (rejected: client-only
paint retarget — cannot show “no arrow” after blockers leave because the wire drops
the block pair; rejected: wire display flag without damage fix — board would lie
about where damage goes for non-trample).

### Engine

- Add a durable collection on `CombatState` of attackers that became blocked this
  combat (e.g. `blocked_attackers: Vec<ObjectId>` or equivalent set semantics).
- Populate when blockers are declared (`Event::BlockerDeclared` / declare-blockers
  batch): each attacker named in at least one block becomes blocked.
- Do **not** clear an attacker from that set when a blocker (or the last blocker)
  leaves via `remove_from_combat`. Still remove the `(blocker, attacker)` pair from
  `blocks` and remove an attacker entirely if the attacker itself leaves combat.
- Clear with the rest of combat state at end of combat.
- Combat damage for an attacker still in combat:
  - Living blockers from `blocks` → assign among blockers (existing path, including
    trample overflow to the defender when applicable).
  - No living blockers **and** attacker is in the durable blocked set:
    - **Trample** → full power to the defending player/planeswalker (CR 702.19b).
    - **Otherwise** → no damage to the defending player/planeswalker (CR 509.1h).
  - No living blockers **and** not in the blocked set → unblocked path (full power
    to defender), unchanged.

### Wire / schema

- Project the durable blocked set on `CombatView` (e.g. `blocked_attackers:
  ObjectId[]`). Expand-only proto field per `docs/WIRE_COMPAT.md`.
- Existing `attackers`, `blocks`, `attackers_declared`, `blockers_declared` stay.
- Client regenerates wire types after proto/schema change.

### Client arrows

- One pure endpoint helper shared by Foldkit `canvas/arrows.ts` and Mount
  `paintCombatArrows` so the two paint paths cannot drift.
- Inputs: committed (+ staged, while relevant) attackers/blocks, `blockers_declared`,
  `blocked_attackers`, battlefield cards (for living endpoints), avatar positions.
- **Gate for post-declaration mode:** every distinct defending **player** among
  current attackers appears in `blockers_declared`. Until then, pre-declaration mode.
- **Pre-declaration mode:**
  - Red: attacker → planeswalker card center when `defender_planeswalker` is set and
    that permanent is still on the battlefield; else → defending player avatar.
  - Green: blocker → attacker (staged and committed), unchanged.
- **Post-declaration mode:**
  - No green block arrows.
  - If attacker is in `blocked_attackers` and has one or more living blockers on the
    battlefield: one red arrow per living blocker (attacker → blocker).
  - If attacker is in `blocked_attackers` and has zero living blockers: **no** combat
    arrow for that attacker (trample included — intentional; damage may still go to
    the seat for trample).
  - If attacker is not blocked: red → player avatar or planeswalker card as above.
- Declare-drag aim arrows stay on the existing aim/drag path; only committed (and
  staged list) endpoints change via the helper.

## Testing

- **Engine:** declare block → remove/sacrifice blocker → assert non-trample deals no
  player/PW damage; repeat with trample → full power to defender. Multi-blocker:
  remove one, remaining still receive assignment. Unblocked attacker unchanged.
- **Schema projection:** `blocked_attackers` present after declare; still listed after
  blocker leaves; cleared after combat ends.
- **Client unit (endpoint helper):** pre-all-declared keeps red→defender + green;
  post-all-declared retargets red→blockers and drops green; blocked with empty living
  blockers ⇒ no red; PW card endpoint when `defender_planeswalker` set; unblocked
  still→defender; partial `blockers_declared` stays in pre mode.
- **Mount/bitmap:** extend existing combat-arrow layering tests for retarget and
  no-arrow cases (outcomes, not paint-parity folklore).

## Spec updates at implement time

- **combat-and-commander-rules:** document durable blocked status, damage when all
  blockers have left (non-trample vs trample), and that `remove_from_combat` does not
  un-block an attacker.
- **battlefield:** document arrow modes (pre vs post all declarations), PW card
  endpoints, red→blockers after declare, no arrow when blocked with no living
  blockers.
- **wire-protocol-and-visibility:** document `CombatView.blocked_attackers` (or final
  field name) and expand-only wire note.

## Out of scope / follow-ups

- Optional later: a subtle blocked marker when the arrow is absent so trample’s
  silent full-power hit is less surprising — explicitly rejected for v1 (arrow
  absence only).
- Dual arrows for trample-with-living-blockers (overflow to player) — rejected.
