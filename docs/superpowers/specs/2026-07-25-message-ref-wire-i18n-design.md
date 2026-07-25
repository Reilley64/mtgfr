# MessageRef on the wire (i18n-ready keys)

**Status:** Done  
**Date:** 2026-07-25  
**Module:** `proto/mtgfr/v1/`, `crates/engine` (effect/reject message refs; delete `label.rs`), `crates/schema` (projection), `crates/server` (Ack / auto_actions), `client/lib/` (catalog + `formatMessage`), prompt/stack/reject UI call sites  
**Related:** [wire-protocol-and-visibility](2026-07-20-wire-protocol-and-visibility.md), [prompts-and-pending-choices](2026-07-20-prompts-and-pending-choices.md), [choices-actions-and-resolution](2026-07-20-choices-actions-and-resolution.md), [stack](2026-07-20-stack.md)

---

## Goal

Replace server→client **player-facing prose** for intent rejects, effect/mode/stack/action labels, and auto-action notices with stable **`MessageRef` { key, params }** values. English copy lives in a **client-only** catalog. No locale negotiation yet — the contract is ready for catalogs later.

Hard cut: string label/reason fields that carried English leave the wire in the same change set. No dual-write / dual-read fallbacks.

---

## Non-goals (v1)

- Auth / lobby / deck-legality gRPC `Status` English messages
- Game-log narration (`client/lib/event-fold.ts` `describe()` templates)
- Oracle text translation
- True card-name localization (product names stay English **data**)
- Locale negotiation (`Accept-Language`, user locale field)
- Codegen’d shared catalog pipeline (TOML → Rust + TS)

---

## Decisions

| Topic | Choice |
|--------|--------|
| Primary goal | Stable keys on the wire; English UI now; locales later |
| Scope | Rejects + effect/mode/stack/action labels + auto_actions |
| English location | Client-only catalogs |
| Param shape | Key + typed param bag |
| Card/object names in sentences | English **data params** (e.g. `{ name: "Grizzly Bear" }`); frame is keyed |
| Rollout | Hard cut (no dual-write) |
| Engine English formatters | Delete (`label.rs` and equivalents); Rust tests assert keys/params |
| Architecture | Single `MessageRef` pattern everywhere in scope |

---

## Wire contract

Shared proto messages:

```protobuf
message MessageRef {
  string key = 1;              // e.g. "reject.illegal_target", "effect.deal_damage"
  repeated MessageParam params = 2;
  repeated MessageRef children = 3;
}

message MessageParam {
  string name = 1;             // e.g. "amount", "name"
  oneof value {
    string string_value = 2;
    int64 int_value = 3;
    bool bool_value = 4;
    string amount_token = 5;   // tokenized Amount: "3", "X", "half_x", …
  }
}
```

**Field replacements (hard cut):**

Reserve the old string field numbers (do not change a field’s type in place). Add new `MessageRef` fields; delete all reads/writes of the old string fields in the same change set (no dual-read). Suggested names:

| Today (remove / reserve) | After |
|--------|--------|
| `Ack.reason` (`optional string`) | `optional MessageRef reject_reason` |
| Stack / action / choice `string label` | `MessageRef label` on a **new** field number; old `label` reserved |
| `repeated string labels` (modes, triggers, …) | `repeated MessageRef labels` on new field numbers; old reserved |
| `repeated string auto_actions` | `repeated MessageRef auto_actions` on a new field number; old reserved |

Consumers use **MessageRef only**; reserved string fields are unused.

**Not wrapped as MessageRef:** identifying card/object **names** on `ObjectView` / `ChoiceItem` (and similar) remain plain `string` identity/display data.

**Key namespaces:** `reject.*`, `effect.*`, `auto.*`, plus small closed action keys as needed (`action.keep_hand`, `action.declare_attackers`, …). Mode and trigger-mode rows reuse the underlying effect’s `effect.*` message (same key + params as that effect would use on the stack).

**Rules:**

- No English prose in `MessageRef.key` or as a substitute for params.
- Unknown keys render as the raw `key` in the UI (dev-visible), never invented server English.
- Coordinated client+server deploy; no mixed-version dual-read of English strings.

---

## Engine & schema

### Engine

- Replace `Effect::label()` / `crates/engine/src/label.rs` with `Effect::message(&self) -> MessageRef` that returns key + params only.
- Introduce a Rust `MessageKey` enum (or equivalent closed set) at construction sites; serialize to the wire string (`effect.deal_damage`). Call sites cannot invent arbitrary key strings.
- `Amount` and similar become params (`int_value` and/or `amount_token`), not interpolated English.
- Map `Reject` (and server session tags such as `UnknownTable`, `GameNotStarted`, …) to `MessageRef` via one mapper (`reject.illegal_target`, typically no params).
- Delete player-facing English formatters on engine/server projection paths. Tests assert `key` + params.

### Schema / projection

- Snapshot, choice, action, and stack projection copy `MessageRef` through — no string assembly for those fields.
- Card/object names remain plain strings where they identify a card.
- Name-bearing auto-actions: key + `{ name: "<card name>" }` (English data param).
- Visibility/redaction unchanged: never put hidden card names into params for viewers who must not see them (same rules as today’s labels).

### Server

- Intent reject path: encode `MessageRef` on `Ack` instead of `format!("{rejected:?}")`.
- Auto-action notices: emit `MessageRef` list, not English sentences.

---

## Client catalog & rendering

- Single formatter: `formatMessage(ref: MessageRef): string` under `client/lib/i18n/`.
- English catalog: `key → (params) => string` (or template + substitution).
- Missing key / bad params → return the raw `key` (optional dev console warn). UI must not crash or show blank.
- Call sites:
  - Reject chrome: today’s `humanReason` becomes `formatMessage(ack.reason)` (thin wrapper OK).
  - Prompt titles, stack rows, mode rows, auto_actions that used wire `label` call `formatMessage`.
- Client-authored chrome that never came from the server (`"Choose"`, `"Fail to find"`, waiting-banner templates) may stay as local English for v1; can join the same catalog later without wire changes.
- Sync guard: a test that every `MessageKey` the Rust side can emit has a catalog entry (export string forms / fixture). No full codegen pipeline in v1.
- Locales later: `formatMessage(ref, locale)` selecting catalog tables — **no wire change**.

---

## Testing

- Engine: unit tests for effect message builders and reject mapper — representative families + param tokens (amounts, names).
- Schema/wire: projection tests assert `MessageRef` fields, not English phrases.
- Client: catalog coverage (every emitted key has an entry); UI/Scene tests that show reject/prompt/stack text assert formatted English via `formatMessage`.
- Hard cut verification: regenerate wire (`just server-codegen` / client gen); the change set does not compile until all consumers are updated — no dual-read shims.

---

## Living spec coverage

Shipped behavior is documented in:

- [wire-protocol-and-visibility](2026-07-20-wire-protocol-and-visibility.md) — `MessageRef` on Ack / labels / auto_actions
- [prompts-and-pending-choices](2026-07-20-prompts-and-pending-choices.md) — titles from formatted refs
- [choices-actions-and-resolution](2026-07-20-choices-actions-and-resolution.md) — action labels and auto-action notices
- [stack](2026-07-20-stack.md) — stack labels from formatted refs

---

## Architecture sketch

```
┌─────────────┐   MessageRef { key, params }    ┌──────────┐  formatMessage   ┌────────┐
│ Engine      │ ───────────────────────────────► │ Proto /  │ ───────────────► │ English│
│ Reject /    │   (no prose)                     │ Schema   │   client catalog │ UI     │
│ Effect::    │                                  └──────────┘                  └────────┘
│ message()   │
└─────────────┘

Card names / seat ids stay string *data* on views or as MessageParam.string_value.
```

---

## Risks & mitigations

| Risk | Mitigation |
|------|------------|
| Large blast radius (`label.rs` ~1600 lines + many prompt fields) | Mechanical key extraction; keep param vocabulary small; land as one coordinated PR train |
| Key catalog drift (Rust emits key client lacks) | Closed `MessageKey` + catalog coverage test |
| Redaction leak via params | Same visibility rules as today’s labels; review auto_action / choice projection |
| Hard cut breaks mixed-version clients | Single deploy train; acceptable for this product stage |

---

## Success criteria

1. No English prose on in-scope wire fields (`Ack.reason`, effect/mode/stack/action labels, `auto_actions`).
2. Client renders the same player-facing English as today for covered surfaces (modulo intentional copy cleanup).
3. Engine has no player-facing label formatters; tests lock keys/params.
4. Adding a second locale later requires only new client catalog tables + optional locale selection — not proto changes.
