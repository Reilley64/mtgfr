# Quandrix Unlimited deck increments (2026-07-26)

Deck report: [quandrix-unlimited.md](quandrix-unlimited.md). This file is the sole
engine-capability backlog for this deck (ranked increments plus the concrete Quandrix cards they
unblock).

From `docs/decklists/quandrix_unlimited.md` (official Wizards `soc` precon; commander Zimone,
Infinite Analyst). After the documentation re-audit, only two deck cards still need engine work.
Quandrix's loud X/counter/copy core is largely already landed; the remaining gaps are narrower:
one cast-time X legality cap, and one spell whose resolution-time choices are still exposed on the
stack.

### 1. `cast-x-cannot-exceed-player-count` — 1 card, S — LANDED (2026-07-26)

Landed as `CardDef::cast_x_max = Some(CastXMax::PlayerCount)` (`cast_x_max = "player_count"` in
TOML), read by `Game::cast_x_ceiling`. `Game::validate_cast` rejects an announced X above the
living-seat count with `Reject::IllegalChoice` (before payment — a legality cap, not a mana
shortfall), and the snapshot's cast count-picker clamps its offered ceiling to the same bound.
`open_the_way.toml` opts in and drops its residual note; `Open the Way` is now faithful.

**Depends on:** none.
**Cards:** `open_the_way.toml`
**Sketch:** `Open the Way` is the first in-pool card whose printed rules directly cap announced X by
the current player count. The engine already knows the seat count and already computes cast-time
X affordability through `max_payable_x`, but it never folds a non-mana upper bound into that path.
Add a narrow cast-time X ceiling that a card can opt into — either a dedicated `cast_x_max =
"player_count"` card field or a small condition read by `validate_cast_cost_picks` /
`max_payable_x`. The result should reject `X > living_players` before payment and should clamp any
UI/count-picker affordance to the same ceiling. Regression bar:

- in a four-seat game, `Open the Way` can be cast at X = 4 but not X = 5,
- in a two-seat test fixture, X = 2 stays legal and X = 3 is rejected,
- the existing reveal-until-X-lands behavior stays unchanged once the cast is legal.

### 2. `resolution-time-parity-choice-with-optional-nontargeted-primer` — 1 card, M

**Depends on:** none.
**Cards:** `zimones_hypothesis.toml`
**Sketch:** `Zimone's Hypothesis` currently gets its printed result by front-loading two
resolution-time decisions into cast-time structures: odd/even is expressed as a modal choice, and
the optional "+1/+1 counter on a creature" primer is expressed as a targeted clause. To make the
card faithful, keep the whole sequence inside resolution:

1. optionally choose a creature and put a +1/+1 counter on it, without making that creature a spell
   target on the stack,
2. then choose odd or even during resolution,
3. then return each creature with the chosen post-counter parity to its owner's hand.

The smallest clean implementation is a mid-resolution choice primitive for parity (or one bespoke
effect for this card) plus a way for the optional primer to pick a creature during resolution
without advertising a target at cast time. Regression bar:

- the stack does not reveal odd/even before the spell resolves,
- the spell still resolves if the creature you planned to counter leaves before resolution,
- the mass bounce reads the creature powers after the optional counter lands.
