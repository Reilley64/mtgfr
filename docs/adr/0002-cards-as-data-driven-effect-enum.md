# 0002 — Cards as a data-driven effect enum

Status: **Accepted**

## Decision

- `CardDef { kind, cost, abilities: &'static [Ability] }` where `Ability { timing, effect }`.
- `Timing`: Spell | EtbTriggered | Activated(Cost) | Static. `Effect` enum grows only as real cards demand it.
- Targets validated at stack entry; abilities are `&'static` so `CardDef` stays `Copy`.

## Consequences

- New behavior = new `Effect` variant + `execute_effect` arm + TOML authoring.
- Continuous effects use additive recompute (0003).
