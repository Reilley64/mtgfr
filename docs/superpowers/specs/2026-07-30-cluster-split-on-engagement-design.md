# Cluster split on engagement — design

**Status:** Approved for planning (2026-07-30)
**Living surface specs to update at implement time:**
[`2026-07-20-battlefield.md`](2026-07-20-battlefield.md),
[`2026-07-25-activation-menu.md`](2026-07-25-activation-menu.md),
[`2026-07-20-combat-and-commander-rules.md`](2026-07-20-combat-and-commander-rules.md)

## Problem

On a crowded battlefield row, `rowSlots` (`client/app/board/geometry/layout.ts`) collapses
indistinguishable permanents into one **cluster face** carrying `cluster` (a count badge) and
`clusterMembers`. Every consumer downstream of `layout()` sees only that face — the member with
the lowest object id — so the other copies are unreachable:

- **Attacking.** `attackDrop` (`geometry/interaction.ts:111`) replaces any existing entry with the
  same `attacker` id. Dragging the face twice restages the same creature, so a cluster of six
  identical creatures can declare exactly **one** attacker.
- **Defending.** `blockDrop` dedupes by `blocker` id the same way — one blocker per cluster.
- **Targeting.** A prompt `card-pick` draft toggles `picked` by object id, so a second click on the
  cluster face *deselects* the first copy instead of choosing a second one. Multi-target spells
  cannot take two members of one cluster.
- **Abilities.** The activation radial reads `board.selectedId`, which is the face, so only the
  face's abilities are reachable. Once its once-per-turn ability is spent the engine stops listing
  the action and the remaining copies' abilities are unreachable.
- **Combat arrows.** `combatArrowEndpoints` resolves attackers and blockers through the layout id
  map and `continue`s when a card is absent, so a declared attacker sitting inside a cluster draws
  **no arrow at all**.

`clusterKey` already splits on tapped, marked damage, counters, keywords, and modifiers — which is
why tapping one copy pulls it out of the pile today. It has no notion of *commitments*: combat
declarations and target choices live in `VisibleState.combat` / `VisibleState.stack` and in board
model staging, not on `ObjectView`.

`geometry/density.ts` (cluster fan + hover raise, ~130 lines) is dead — nothing outside
`density.test.ts` imports it — although `battlefield.md` documents fanning as shipped behavior.

## Goal

1. A permanent that is **committed** to something leaves its cluster, takes its own row slot, and
   becomes hit-testable, ring-able, and arrow-able like any other permanent.
2. Because the cluster face is always the lowest-id member, the pile then hands out the next free
   copy: repeated attack drops, block drops, and target picks each reach a distinct creature.
3. Activated abilities are **counted across the pile** instead of splitting it: an ability stays on
   offer while any member can still activate it, and activation routes to a member that can.
4. Holding **Shift** on a combat drop commits the whole pile to that defender or attacker.
5. Nothing splits mid-drag. The reflow happens when the target is picked.

## Non-goals

- Any browse/expand/fan gesture for clusters. Members are identical by construction, so "the next
  free copy" is always as good as a chosen one. `density.ts` is deleted rather than wired up.
- Changing which permanents cluster in the first place (`clusterKey`'s field list, packing, seat
  bands, the count badge).
- Shift-to-select-all outside combat declarations (no shift-target, no shift-activate).
- Engine, proto, schema, or server changes. The object ids were always real and legal; only the
  client failed to hand them over.

## Design

### Engagement is a derived set, keyed by id

`rowSlots` already has the escape hatch this needs. Hosts with attachments never merge because
`keyOf` returns `id:${o.id}` for them instead of `clusterKey(o)` (`layout.ts:470`). Engagement
joins that set:

```ts
const keyOf = (o: ObjectView) =>
  hostsWithAttachments.has(o.id) || engaged.has(o.id) ? `id:${o.id}` : clusterKey(o);
```

Keying by **id**, not by the engagement itself, is load-bearing: if the split key were
`atk:seat2`, two copies attacking the same defender would re-merge into a cluster of two and draw
one arrow for two attackers.

New `client/app/board/engagement.ts` holds one pure function and no state. It lives in `board/`
rather than `board/geometry/` because it reads board staging types; `layout()` only ever receives
the finished set.

```ts
export function engagedIds(
  state: VisibleState,
  local: { combatAttackers: readonly WireAttack[]; combatBlocks: readonly WireBlock[]; promptDraft: PromptDraft | null },
): ReadonlySet<number>;
```

An object id is engaged when it is:

- an attacker — wire `state.combat.attackers` ∪ local `combatAttackers`
- a blocker — wire `state.combat.blocks` ∪ local `combatBlocks`
- in `state.combat.blocked_attackers`
- the target of a stack entry — `stackEntryTargets` over `state.stack`
- a locally drafted target — `promptDraft` `card-pick.picked`, `target.id`, `targets.ids`

`StagedAction` carries no chosen target — `completeStagedTarget` submits on release — so a staged
cast contributes nothing until its object reaches the stack.

The set is re-derived every frame, so re-merge needs no bookkeeping: an attacker rejoins the pile
when combat clears and it untaps; a targeted creature rejoins when the spell resolves.

Local staging only ever gains an entry on a completed drop, never on drag start, so no cluster
reflows under a moving pointer.

### Dispensing falls out of the ordering

`rowSlots` sorts members by id and `toSlotCard` takes the first as the face. When an engaged copy
leaves, the next free copy *is* the new face. Sequential attack drops, block drops, and target
picks therefore reach distinct creatures with no dispenser code and no cluster awareness in
`interaction.ts`, `combat-staging.ts`, or the prompt draft reducers.

### Abilities count across the pile

`selectedRadialOptions` (`board/html/activation-menu.ts`) currently derives options from the face
card's flags. It instead unions the members' entries in `state.actions` (matched on
`ActionView.object` across `[face.id, ...face.clusterMembers]`), offers each distinct ability while
at least one member still has it, and carries the `ActionView.id` belonging to a member that does.
The radial already dispatches by action id, so the activation routes to an unspent copy with no
new plumbing, and costs stay entirely the engine's call.

An ability row shows `×k` when fewer than all members can still activate it. When the last copy's
ability is spent the row disappears. Nothing splits out of the cluster for an activation — visible
consequences (tapped, counters, modifiers) split it through `clusterKey` as they do today.

### Shift commits the whole pile

`shiftDown: boolean` joins `BoardModel` beside `altDown`, set and cleared by
`board/html/keyboard-mount.ts` on `Shift` keydown/keyup and cleared on blur — the same shape as
Alt-inspect, so no modifier state has to be threaded through pointer messages.

On a combat drop with shift held and a cluster face under the drag, `handleCombatDrop` applies the
drop across every member id. The legality guards in `attackDrop` (tapped, summoning sick without
haste) read per-card facts that `clusterKey` forces to be equal across members, so checking the
face checks them all; `attackDrop` and `blockDrop` are unchanged and simply called per id.

Shift-dropping a pile of five on a seat declares five attackers, splits all five out, and draws
five arrows. Shift-dropping onto an attacker declares five blocks the same way.

The discoverability hint gains `Shift: whole pile` while a declaration is live.

### Plumbing

`layout(state, viewer, engaged?)` takes an optional third argument defaulting to an empty set.
Call sites: `submodel.cardsFor` and `submodel.syncFlightsWithGame`, `view.ts:99`,
`canvas/scene.ts:222`, and both `activation-menu.ts` calls (so the radial anchors to the split
card). `cardsFor(fold)` gains the board model, which its callers already hold. The paint path and
the hit path must derive the set identically, or a card paints where it cannot be clicked.

### Deletions

`geometry/density.ts` and `geometry/density.test.ts` are removed. `battlefield.md` loses its
cluster-fan behavior line and its density test line, and gains the split-on-engagement rule.

## Error handling

Every failure mode here is "the split didn't happen", which degrades to today's behavior rather
than to an illegal intent:

- A stale local staging id (creature died between drop and frame) engages an id that no longer
  exists. `engagedIds` is only ever membership-tested against live objects, so a missing id is
  inert.
- Shift with a non-cluster card under the drag expands to a one-element member list — the ordinary
  single-attacker path.
- Shift on a pile whose face is illegal to attack with returns `null` from `attackDrop` for every
  member, so the drop is rejected as one unit rather than partially staged.
- The engine remains the authority on legality. A radial option built from a member's action id
  can still be rejected server-side (mana spent since the snapshot); that surfaces through the
  existing `reject` path.

## Testing

Regressions first, each at the lowest layer that catches the failure.

- `board/cluster-dispense.test.ts` — sequential pointer drops and picks on one cluster reach distinct
  copies, driven through `cardAt` → `cardsFor` → `layout()` rather than through literal ids.
- `canvas/combatArrowEndpoints.test.ts` — an attacker inside a cluster draws an attack arrow (today
  it draws none). The fixture must engage a member that is not the cluster's natural face, or it
  passes either way.
- `board/engagement.test.ts` — wire attackers, local staged attackers, blockers,
  `blocked_attackers`, stack targets, and prompt drafts each land in the set; an idle board yields
  an empty one.
- `geometry/layout.test.ts` — an engaged member takes its own slot, the residual cluster's count
  drops and its face becomes the next free id, two engaged members do not re-merge with each other,
  and engaged ids on a row that never clustered change nothing.
- `board/cluster-dispense.test.ts` — two pointer picks on one cluster select two distinct ids
  instead of toggling the first off.
- `board/html/activation-menu` tests — a cluster where one of three copies has spent its
  once-per-turn ability still offers the ability, labelled `×2`, and dispatches the unspent copy's
  action id; when all three are spent the row is gone.
- `geometry/combat-staging.test.ts` — a shift drop of a five-member cluster on a seat yields five
  attackers against that defender; on an attacker, five blocks; shift with an illegal face yields
  none.
- `board/html/chrome.test.ts` — the combat coach strip carries the shift copy while a declaration
  is open.
