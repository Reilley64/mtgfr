# Arena-style BO1 hand smoothing — design

**Status:** Draft for review (2026-07-25)  
**Living surface specs to update at implement time:**
[`2026-07-20-engine-core-and-event-model.md`](2026-07-20-engine-core-and-event-model.md),
[`2026-07-20-lobby-table-routing-and-live-game.md`](2026-07-20-lobby-table-routing-and-live-game.md)

## Problem

Real games deal opening hands with a single shuffle and draw seven. Extreme
land counts (mana screw / flood) are common in Commander and produce non-games,
especially with a single decisive game and no sideboard. MTG Arena’s Best-of-One
formats use a well-known **hand-smoothing** rule on the opening deal to bias
toward land ratios near the deck’s average. This project has no equivalent.

## Goal

Always-on, Arena-style **2-sample land smoothing** (closest land count to deck
expectation) for every pre-game hand deal: the initial opening hand **and every
mulligan redraw**. Mulligan *sizes* and the existing friendly first mulligan stay
as they are today. No player-facing UI; behavior is a silent product rule.

## Non-goals

- Opt-in / per-table toggle or Bo3 “paper random” mode.
- Changing friendly mulligan hand sizes, London bottoming, or Vancouver scry.
- UI disclosure, badges, or rules copy about smoothing.
- Smoothing mid-game draws, searches, or other library access.
- Matching Arena’s reported choice to leave **mulligan** hands unsmoothed
  (we deliberately smooth those too).
- Hypergeometric / probability-weighted selection among candidates (closest
  land count only).
- Sample counts other than two.

## Approach

**Engine deal helper (rejected: server-only re-shuffle in `seed_game`; rejected:
configurable N-sample / feature flag).**

### Algorithm

For a deal of hand size `N` to a seat whose library is fully stacked (commander
already designated / not in library):

1. Let `L` = number of cards in that library that match the engine’s existing
   land predicate (`CardKind::Land` / `CardFilter::Land` — one source of truth).
2. Let `expected = N * (L as f64) / (library_len as f64)`.
3. **Sample A:** run one normal library shuffle (one derive-per-op RNG), score
   land count in the top `N` cards, remember the resulting library order.
4. Restore the pre-sample library order.
5. **Sample B:** run a second normal library shuffle (next derive-per-op RNG),
   score land count in the top `N`, remember order.
6. Pick the sample with smaller `|lands_in_top_N - expected|`. Ties → Sample A.
7. Write the winning library order (do **not** shuffle a third time).
8. Draw `N` cards as today.

If `library_len < 2`, each sample follows today’s `shuffle` early-return (no
Fisher–Yates, **no** RNG op). Both samples are identical; draw
`min(N, library_len)`.

### RNG contract

Each candidate sample is one Fisher–Yates pass via the existing
`with_op_rng` / `derive_op_key(master_seed, seat, op_iteration++)` path — the
same “new bit on the RNG hash” used for ordinary shuffles. When `library_len >= 2`, a smoothed deal consumes **exactly two** ops for that
seat, then assigns the winning order. Same `master_seed` ⇒ same hands and same
subsequent op stream.

Ordinary mid-game shuffles continue to burn **one** op each.

### Where it runs

- **Opening:** `seed_game` (and any real-game setup path) uses the helper per
  seat instead of `shuffle` + seven `draw_card`s, then `begin_mulligans()`.
- **Mulligan:** after returning the hand to the library, `take_mulligan` runs
  the same helper for the next hand size (`hand_size_after_mulligans`), then
  draws that many. Auto-keep at hand size 1 unchanged.
- Direct engine tests that manually `shuffle` + `draw` stay unsmoothed unless
  they call the helper.

### Events / wire / client

No new wire fields or client chrome. Observers still see a single public
“library shuffled” fact per committed deal (not the discarded sample). Prefer
implementing the two-op smooth inside the engine apply/intent path so
intent-replay determinism stays intact; exact event arm (`LibraryShuffled` vs
dedicated apply arm) is an implementation detail as long as the RNG contract
above holds. Seed setup may keep today’s silent mutations.

## Testing

- Fixed `master_seed` + stacked library where A and B differ in land count →
  kept hand is the closer-to-expected sample; `op_iteration` advances by 2.
- Equal distance tie → Sample A.
- Mulligan redraw also burns 2 ops and applies the same selection rule for the
  new `N`.
- Mid-game / search shuffles still advance `op_iteration` by 1.
- Existing `seed_game` / mulligan phase tests remain green (opening dealt,
  mulliganing, first turn waits on keeps).

## Spec updates (implement PR)

- [`engine-core-and-event-model`](2026-07-20-engine-core-and-event-model.md):
  setup and mulligan deals use 2-sample land smoothing; document RNG ops and
  the deliberate Arena mulligan divergence.
- [`lobby-table-routing-and-live-game`](2026-07-20-lobby-table-routing-and-live-game.md):
  `seed_game` uses the smoothed opening deal.

## Error / empty

- Empty / singleton library: no RNG ops (same as `shuffle`); draw what remains;
  later empty draws still arm the usual lose-on-draw SBA.
- Zero-land or all-land libraries (`library_len >= 2`): both samples score the
  same; Sample A wins by tie-break (still two RNG ops).
