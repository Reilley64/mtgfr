# Wave A Arc-slice report

Status: DONE_WITH_CONCERNS

## Commits

- Checkpoint A: `6fcf563` — `refactor(engine): store sequence-like Effect slices in Arc`
- Checkpoint B: `2612961` — `docs: document Arc effect slices in engine specs`

## Test commands and results

- `cargo build -p engine` — passed
- `cargo nextest run --profile ci -p engine --lib` — passed, 91 tests run, 91 passed, 0 skipped
- `cargo nextest run --profile ci -p engine` — passed, 1948 tests run, 1948 passed, 0 skipped

## Concerns

- `Effect::Sequence`, `Effect::ChooseOne`, and `Effect::Conditional` now use `Arc<[Effect]>`, and the runtime `Box::leak` sites that built effect sequences were retired.
- The broader `CardDef` slice fields (`abilities`, `keywords`, `colors`, `granted_keywords`, `halves`, `hand_ability`, and related `'static`-backed arrays) were not fully converted in this pass.
- The remaining blocker is Rust's current `Arc::from` / `Arc::new` non-`const` initialization in the existing `const`/static card-fixture pattern, which would force much larger fixture churn than the requested single-wave change.
- The living specs were updated to drop the old "Arc follow-up" wording for effect sequences and to document the remaining `CardDef` `'static` backing truthfully.
