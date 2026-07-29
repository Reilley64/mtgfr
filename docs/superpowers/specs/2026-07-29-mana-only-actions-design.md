# Mana-only actions — design

**Status:** Approved for planning (2026-07-29)
**Living surface specs to update at implement time:**
[`2026-07-25-activation-menu.md`](2026-07-25-activation-menu.md),
[`2026-07-20-battlefield.md`](2026-07-20-battlefield.md),
[`2026-07-20-wire-protocol-and-visibility.md`](2026-07-20-wire-protocol-and-visibility.md)
(field table for `ActionView`)

## Problem

A land whose only ability is a paid mana ability — Viridescent Bog's `{1}, {T}: Add {B}{G}`,
and every filter land, karoo, and Signet like it — paints the mint playable border on the
battlefield. During an opponent's spell that border reads as "the game is waiting for you to
make this play", when in fact the seat is helpless and the pause is the ordinary 2 s
`STACK_HOLD` beat every helpless seat gets before its pass resolves the stack.

The engine already draws the right line and does not halt for these:

- `Game::meaningful_actions` (`crates/engine/src/query.rs`) skips mana abilities outright, so
  `has_meaningful_action` — the only input to the server's auto-pass skip check
  (`crates/server/src/session.rs`) — is `false` for a board of nothing but mana sources.
- `Game::paid_mana_activates` (`crates/engine/src/query.rs`) lists paid tap-for-mana modes
  separately, explicitly "not part of `meaningful_actions`, so they never stop auto-pass",
  because the activation menu still needs a row for them.

`Game::refresh_actions` (`crates/engine/src/lib.rs`) then chains both lists into one
`Vec<LegalAction>` and the distinction is lost. `ActionView` carries no field for it, so
`playableBattlefieldObjectIds` (`client/app/board/chrome.ts`) — which borders any object with a
battlefield `ActionView` — cannot tell a real play from bare mana production.

The mismatch compounds: `can_act` is `has_meaningful_action`, and `radial.ts` disables every
menu row on `!canAct`. The mint border therefore advertises a card whose only menu row is
greyed out.

## Goal

Bare mana production never paints as a play. Priority behavior is unchanged — it is already
correct.

## Non-goals

- Changing `meaningful_actions`, auto-pass, or the `STACK_HOLD` beat. The halt is not caused by
  mana abilities and is not in scope.
- Removing paid mana activates from the action list. The activation menu renders their cost chip
  and `auto_tap` preview from the `ActionView`.
- Ungating the `!canAct` row disable. When a seat has no meaningful action the mana it could make
  is unspendable, so a live row would buy nothing.
- Free-tap mana sources (`taps_for_mana` — basics, Sol Ring, mana dorks). They already have no
  `ActionView` and so already take no border.

## Approach

**Carry the engine's existing distinction to the wire** (rejected: dropping paid mana activates
from the action list and rebuilding the menu row from a per-permanent flag, which loses the cost
chip and auto-tap preview; rejected: inferring the class client-side from the action's
`MessageRef` label, which the activation-menu spec already bans for costs).

### Engine

- `LegalAction` gains `mana_only: bool`.
- `Game::refresh_actions` sets it `true` for the entries it takes from `paid_mana_activates` and
  `false` for the `meaningful_actions` half. The id-preserving lookup keys on `(player, kind)` and
  is unaffected.

### Wire

- `ActionView` gains `mana_only: bool` (`crates/schema/src/dto.rs`) and the corresponding
  `.proto` field, projected straight from `LegalAction`.
- Additive field, expand-only, WIRE-compatible under `just proto-check`. Run `just server-codegen`
  and `bun run gen` after the proto change.

### Client

- `playableBattlefieldObjectIds` (`client/app/board/chrome.ts`) skips actions with
  `mana_only === true`. Everything else about the border set is unchanged, including the existing
  summoning-sick carve-out.
- The activation menu is untouched: mana-only rows still list, still show their cost chip, still
  obey the `!canAct` disable.

### Stale doc

`ActionView`'s neighbouring `taps_for_mana` doc comment in `crates/schema/src/dto.rs` still claims
"Mana abilities never reach the action list (`meaningful_actions` skips them)". That stopped being
true when `paid_mana_activates` landed. Correct it in the same change.

## Deferred: when mana production does matter

Some cards make bare mana production worth stopping for — a static that keeps unspent mana
(Kruphix, Omnath, Upwelling), or a controller-facing "whenever you tap a land for mana" payoff.
With one of those out, *every* mana source including basic lands should stop priority and paint the
border.

**No card in `crates/cards/data/` can trigger that rule today**, which is why it is deferred rather
than built:

- Mana persistence exists only as `persist_until_end_of_turn` on a specific `add_mana` effect
  (Rousing Refrain). There is no "unspent mana doesn't empty" `StaticEffect`.
- `Trigger::PermanentBecomesTapped { for_mana: true }` exists, but every scripted user is a
  downside or opponent-facing: Manabarbs, Psychic Venom, Lifetap, Kudzu. Stopping a player's
  priority to offer "tap your land and take 1" is the opposite of the intent.
- `StaticEffect::TappedForManaBonus` (Mirari's Wake, Fertile Ground, Wild Growth, Gauntlet of
  Might, Mana Flare) only produces *more* mana, which is equally unspendable, so it must not trip
  the rule either.

The shape to build when the first payoff card is scripted, recorded as a `ponytail:` comment on
`Game::paid_mana_activates`:

- Add `Game::mana_matters(player) -> bool`: true when the player controls a mana-persist static or
  a controller-facing tapped-for-mana payoff, and has an untapped mana source.
- OR it into `has_meaningful_action` rather than pushing entries into `meaningful_actions` — that
  gets the priority stop without inventing an action kind that would duplicate activation-menu rows
  for permanents that already have one.
- Carry it as a snapshot boolean so the client borders every card with `taps_for_mana`, basics
  included, which no per-action flag can express.

## Testing

- **Engine:** a paid mana activate appears in `legal_actions` but not in `meaningful_actions`, and
  `has_meaningful_action` is `false` for a board of Viridescent Bog plus a Forest — locking the
  invariant whose loss produced the border.
- **Schema:** `mana_only` projects `true` for the Bog's activate and `false` for a non-mana
  activated ability on the same battlefield.
- **Client:** a `mana_only` battlefield `ActionView` contributes no id to
  `playableBattlefieldObjectIds`, while a non-mana one still does.
