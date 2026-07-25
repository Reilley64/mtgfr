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
