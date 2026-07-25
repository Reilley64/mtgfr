# Task 5 Report: Client formatMessage + English catalog + wire types

Status: DONE_WITH_CONCERNS

## Changes

- Added `client/lib/i18n/message.ts` with `formatMessage(ref: MessageRef): string`.
- Added `client/lib/i18n/catalog/en.ts` with every current Rust `MessageKey`, plus schema action/card/keyword chrome keys.
- Ported explicit English for reject keys, forced auto-action keys, action chrome, and common effect labels.
- Updated client wire types for `Ack.reject_reason`, stable view labels/labels, and `auto_actions` as `MessageRef`; `ChoiceItem.label` remains `string`.
- Updated `protoMap` to decode protobuf `MessageParam.value` oneofs into snake_case `string_value` / `int_value` / `bool_value` / `amount_token`.
- Mapped unary gRPC acks through `ackFromProto` before returning them from the BFF client.
- Routed rejected acks and auto-action log lines through `formatMessage`.

## TDD evidence

- RED: `cd client && bun run test lib/i18n/formatMessage.test.ts lib/wire/protoMap.test.ts`
  - Failed on missing `./message` and undecoded message-param oneofs.
- GREEN: same command
  - 2 files passed, 8 tests passed.

## Verification

- `cd client && bun run gen:tokens:check && bun run lint && bun run typecheck && bun run test lib/i18n lib/wire/protoMap.test.ts app/reject.test.ts`
  - Passed.
  - Lint emitted only the existing Biome schema-version info.
- `just client-check`
  - Not used as final evidence: its `format` step rewrites generated token CSS casing/wrapping, causing the next `client-tokens-check` to report stale generated tokens.

## Concerns / handoff

- The catalog has full key presence for current Rust `MessageKey`s, but many less-common `effect.*` entries use generated English from the key name plus params. Reject, auto, action, and common effect copy are explicit.

## Review fix: MessageRef hard cut

- Removed the `MessageRef = string | {...}` dual-read: `MessageRef` is now object-only with a recursive Effect `Schema.Codec`; `formatMessage` throws for bare strings and only returns the raw key for missing catalog entries.
- Removed `DEFAULT_KEYS`, `defaultLabel`, and runtime title-casing fallback from `client/lib/i18n/catalog/en.ts`; every catalog key is now an explicit formatter/literal entry, with historical English ported for effect/reject/auto/action surfaces.
- Added regression coverage for schema rejection of string refs, `formatMessage` rejection of string refs, explicit historical `effect.control_tap_target`, and unknown-key raw-key behavior.
- Updated test fixtures that were still using English strings in `MessageRef` fields to object-shaped refs via a test helper; `ChoiceItem.label` remains a plain string.

## Review fix verification

- RED: `cd client && bun run test lib/i18n lib/wire/protoMap.test.ts`
  - Failed on string passthrough, generated `effect.control_tap_target` fallback (`Control tap target`), and `Schema.Any` accepting string refs.
- GREEN: `cd client && bun run test lib/i18n lib/wire/protoMap.test.ts`
  - Passed: 2 files, 11 tests.
- GREEN: `cd client && bun run typecheck`
  - Passed after converting remaining `MessageRef` fixtures to object refs.
