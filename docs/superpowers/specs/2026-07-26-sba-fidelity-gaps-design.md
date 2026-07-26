# SBA fidelity gaps (design)

**Status:** Approved design input (autonomous bridge of CR 704 gaps, 2026-07-26).
**Surfaces:** `engine-core-and-event-model`, `choices-actions-and-resolution`, `prompts-and-pending-choices`, `combat-and-commander-rules` (legend interaction with commanders), wire/proto as needed.

---

## Problem Statement

The SBA *machinery* (fixpoint sweep before triggers) is sound, but coverage has pool-driven holes called out in review:

1. **No legend rule (CR 704.5j)** — two legendary permanents with the same name under one controller coexist.
2. **No +1/+1 ↔ −1/−1 annihilation (CR 704.5r)** — both counter kinds can sit on one permanent; fidelity backlog (deathdancer / wickerbough) deferred this.
3. **Token death skips the graveyard** — `TokenCeasedToExist` never visits GY (CR 111.7 modeling); dies triggers already fire off that event for the pool (e.g. Pest).
4. **Ascend checked only at SBA sweeps** — continuous rule approximated; pool-equivalent.

## Locked scope for this change

| Gap | Action |
|-----|--------|
| Legend rule (704.5j) | **Implement** with player choice |
| Counter annihilation (704.5r) | **Implement** in SBA sweep |
| Token → GY then cease | **Defer** — Pest/dies already use `TokenCeasedToExist`; no pool card needs GY inspection of a dead token |
| Ascend continuous | **Defer** — behaviorally identical for the pool |
| World / Battle / Saga SBAs | **Out of scope** — flag-don't-force until a card needs them |

## Approaches (legend)

1. **PendingChoice keep-one (recommended)** — detect conflicting name groups in `check_state_based_actions`; raise `ChooseLegendaryKeep`; SBA sweep pauses while choice is pending (existing guard); answer puts the others through `graveyard_or_command` / token cease.
2. Auto-pick newest/oldest — not rules-faithful.
3. Forced only when N=2 with heuristics — still wrong for N>2.

**Recommendation:** (1). One conflicting name-group per sweep (first found); next sweep handles further groups. Controllers choose independently if multiple players conflict (APNAP / seat order: raise for the lowest seat index with a conflict first).

## Design

### Legend rule

- Group battlefield permanents by `(controller, lowercase? printed name)` among those with `def.legendary` and not bestowed-as-Aura-only if that would make them noncreature Auras… **Rule:** apply to every permanent whose *current* CardDef is legendary (including copies). Bestowed attached permanents are Auras, not creatures — if the CardDef is still legendary while bestowed, CR still applies to the legendary permanent.
- If a group has size ≥ 2 for a living controller → do **not** mint death events in the scan; instead `pending::raise(ChooseLegendaryKeep { player, name, options })` from the sweep path when choice is empty.
- Answer intent: `ChooseLegendaryKeep { player, keep }` — `keep` must be in `options`; every other option leaves via existing death events (`graveyard_or_command` / `TokenCeasedToExist`).
- Commanders among the losers still auto-divert to the command zone (existing always-yes CR 903.9).
- Wire: new `PendingChoiceView` / proto arm + client prompt (pick one permanent to keep). Expand-only.

### Counter annihilation (704.5r)

- In `check_state_based_actions` (or a sibling check folded into the same sweep), for each permanent with both `plus_counters > 0` and `kind_counters[MinusOneMinusOne] > 0`, mint an event that removes `min(plus, minus)` of each kind (provenance-aware via existing counter remove/apply paths).
- Prefer a dedicated `Event::CountersAnnihilated { object, pairs }` or reuse existing counter-removal events so characteristics cache invalidates correctly.
- After annihilation, P/T layers see the reduced counts; 0-toughness SBA can then kill on a later fixpoint iteration if needed.

### Testing

- Two legendary permanents same name same controller → choice → keep one, other leaves.
- Different names → no choice.
- Different controllers → no choice.
- Commander loser → command zone.
- Permanent with 3 +1/+1 and 2 −1/−1 → ends with 1 +1/+1 and 0 −1/−1.
- Existing Pest dies / Wickerbough tests stay green.

### Specs

Update `engine-core` SBA list; `choices-actions` + `prompts` for the new choice; wire spec for the new view arm.

## Out of Scope

Token GY visit, continuous ascend, World/Battle/Saga uniqueness, full CR 704 completeness.
