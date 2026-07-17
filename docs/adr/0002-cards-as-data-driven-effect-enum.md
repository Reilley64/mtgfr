# 0002 — Cards as a data-driven effect enum

Status: **Accepted** (consequences clarified 2026-07 for `Game::run` locality)

## Decision

- `CardDef { kind, cost, abilities: &'static [Ability] }` where `Ability { timing, effect }`.
- `Timing`: Spell | EtbTriggered | Activated(Cost) | Static. `Effect` enum grows only as real cards demand it.
- Targets validated at stack entry; abilities are `&'static` so `CardDef` stays `Copy`.

## Consequences

- New behavior = new `Effect` variant + resolution behind [`Game::run`] (pause via
  `pending::raise` / `ChoiceRequest`, or dig-prep helpers that emit events then raise; pure mint
  via private `execute_effect` / family helpers in `resolution/`) + Event `apply` + TOML
  authoring. Callers never bypass `run` for Effect→board.
- Continuous effects: see [0003](0003-additive-continuous-effects-no-layers.md) (`PtLayer` for P/T).
