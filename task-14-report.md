## Task 14

- Fixed `MayPutCounterOnCreature` prompt UX by adding an expand-only `put_counter_on_creature` discriminator on `choose_copy_target`, projecting it from schema/server, and swapping the Foldkit title/submit copy away from "Copy".
- Added `effect.choice_may_put_counter_on_creature` to `enCatalog`.
- Added regression coverage for the prompt wording, proto mapping, and schema projection.
- Updated the living specs and DSL docs to describe the reused `ChooseCopyTarget` shape plus its counter-placement discriminator.

### Verification

- `bun run test app/board/html/prompts.test.ts lib/wire/protoMap.test.ts lib/i18n/catalogCoverage.test.ts`
- `cargo test -p schema may_put_counter_on_creature`
- `cargo build -p server`
- `just client-check`
