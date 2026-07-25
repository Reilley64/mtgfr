# Wave A review-fix report

Status: DONE

## Scope

- Stable `CardId` interning now dedupes non-empty Scryfall oracle ids while preserving fresh ids for empty-id test stubs.
- Flipped permanents, prepared back faces, adventure restores, and split-card restores now reuse stable face ids instead of minting fresh `CardId`s on read or restore.
- `PlayPermissions::adventure_fronts` and `split_halves_on_stack` now store `CardId` restore handles, not cloned `CardDef`s.
- Living engine specs now describe the shipped `CardId` behavior honestly and explicitly leave the remaining Arc-slice migration as a Wave A follow-up.

## Verification

- `cargo nextest run --profile ci -p engine --lib defs::tests` — passed after the red-to-green regression cycle for oracle-id dedupe plus flip/adventure/split stability.
- `cargo nextest run --profile ci -p engine flip_source_swaps_to_back_face` — passed.
- `cargo nextest run --profile ci -p engine adventure_creature_cast_directly_from_hand_still_works` — passed.
- `cargo nextest run --profile ci -p engine --lib` — passed, 91 tests run, 91 passed, 0 skipped.
- `cargo nextest run --profile ci -p engine` — passed, 1948 tests run, 1948 passed, 0 skipped.

## Remaining follow-up

- Arc-slice migration for leaked `&'static` ability/effect slices is not part of this fix. The specs now call that out as a remaining Wave A follow-up rather than claiming it is already complete.
