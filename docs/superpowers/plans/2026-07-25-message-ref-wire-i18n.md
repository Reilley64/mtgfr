# MessageRef Wire Keys Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hard-cut server→client player-facing prose for rejects, effect/mode/stack/action labels, and auto_actions to stable `MessageRef { key, params }` values, with English catalogs client-only.

**Architecture:** Engine owns domain `MessageRef` + closed `MessageKey` constants and `Effect::message()` (replacing `label.rs`). Proto carries the same shape on new field numbers (old string fields reserved). Schema/server project refs through. Client `formatMessage` + English catalog render UI. No dual-read; coordinated API+web deploy (same posture as prior hard wire cuts).

**Tech Stack:** Protobuf / buf / tonic / prost, Rust engine+schema+server, Effect Schema + Vitest client, `just server-codegen` / `bun run gen`.

**Spec:** [docs/superpowers/specs/2026-07-25-message-ref-wire-i18n-design.md](../specs/2026-07-25-message-ref-wire-i18n-design.md)

## Global Constraints

- Hard cut: no dual-write/dual-read of English on in-scope fields; old string field numbers **reserved**, new `MessageRef` fields added.
- No English prose in `MessageRef.key` or as a substitute for machine params. Card/object **names** may appear as `string_value` data params.
- Client-only English catalogs under `client/lib/i18n/`. Locales later = new tables only.
- Delete engine player-facing formatters (`label.rs`); Rust tests assert keys/params.
- Exhaustive `Effect::message` match (no `_` arm), same discipline as today’s `label()`.
- `MessageRef` gains `repeated MessageRef children` for compound effects (`Sequence` / join phrases) — small design refinement for composition.
- Machine tokens in `string_value` / `amount_token` (filters, destinations, keywords) are **snake_case ids**, not display English; client maps them.
- Out of scope: auth/lobby/deck-legality Status English, game-log `describe()`, oracle text, true card-name i18n, codegen’d shared catalog pipeline.
- TDD per task where a seam exists; cutover tasks may share one green window after proto+projection+client land together.
- Angular commits; branch `cursor/message-ref-wire-i18n-3fed`.
- Update living feature specs in the same change; mark design **Status: Done** when implementation merges.
- Wire note: this release is a coordinated hard cut (API+web). Do not rely on N/N−1 for in-scope fields.

---

## File map

| File | Responsibility |
|------|----------------|
| `proto/mtgfr/v1/common.proto` | `MessageRef` / `MessageParam` |
| `proto/mtgfr/v1/mtgfr.proto` | `Ack`: reserve `reason=2`; add `reject_reason` MessageRef |
| `proto/mtgfr/v1/stream.proto` | Reserve old `label`/`labels`/`auto_actions` numbers; add MessageRef fields |
| `crates/engine/src/message.rs` (new) | `MessageKey`, `MessageParam`, `MessageRef`, builders, amount/filter tokens |
| `crates/engine/src/message/effect.rs` (new) or `message.rs` | `Effect::message()` exhaustive match (replaces `label.rs`) |
| `crates/engine/src/lib.rs` | `mod message;` drop `mod label;` |
| `crates/engine/src/types/stack.rs` | `ModeInfo.label` → `MessageRef`; `Reject` stays; reject→MessageRef helper |
| `crates/engine/src/query.rs` | `modes_of` uses `.message()` |
| `crates/schema/src/dto.rs` | DTO `MessageRef`; label fields become MessageRef |
| `crates/schema/src/snapshot.rs` | Action/stack projection → MessageRef |
| `crates/schema/src/projection/choice.rs` | Pending-choice labels → MessageRef |
| `crates/schema/src/catalog.rs` | Ability summaries as MessageRefs or formatted only on client |
| `crates/schema/src/event.rs` | `auto_actions: Vec<MessageRef>` |
| `crates/server/src/session.rs` | Reject + `forced_action_label` → MessageRef |
| `crates/server/src/game_loop.rs` / `grpc/game_svc.rs` / `grpc/map/stream.rs` | Ack + delta mapping |
| `client/lib/i18n/message.ts` (new) | Types + `formatMessage` |
| `client/lib/i18n/catalog/en.ts` (new) | English catalog (effects, rejects, autos, actions, tokens) |
| `client/lib/i18n/formatMessage.test.ts` (new) | Formatter + missing-key behavior |
| `client/lib/i18n/catalogCoverage.test.ts` (new) | Every Rust-emitted key has catalog entry |
| `client/lib/reject.ts` | Thin wrapper over `formatMessage` or delete in favor of i18n |
| `client/lib/wire/types.ts` + `protoMap` | Ack / views / auto_actions MessageRef |
| Prompt/stack/hand/fold call sites | `formatMessage(...)` |
| Feature specs listed in design | Document shipped MessageRef behavior |
| Design spec | Status → Done |

---

### Task 1: Engine `MessageRef` + `Effect::message` (delete `label.rs`)

**Files:**
- Create: `crates/engine/src/message.rs` (split modules if file grows past ~1k lines: `message/mod.rs`, `message/effect.rs`, `message/tokens.rs`)
- Modify: `crates/engine/src/lib.rs` — `mod message;` remove `mod label;`
- Delete: `crates/engine/src/label.rs`
- Modify: `crates/engine/src/types/stack.rs` — `ModeInfo { pub label: MessageRef, ... }`
- Modify: `crates/engine/src/query.rs` — `.message()` instead of `.label()`
- Modify: any `crates/engine/tests/**` that call `.label()`

**Interfaces:**
- Consumes: `Effect`, `Amount`, filters, `Reject`
- Produces:
  - `MessageKey` — closed set of `&'static str` constants via macro, plus `pub fn as_str(self) -> &'static str` and `pub fn all() -> &'static [MessageKey]` for catalog sync
  - `MessageParam { name: &'static str, value: MessageParamValue }`
  - `MessageParamValue` — `Str(&'static str)` | `OwnedStr(String)` for card names | `Int(i64)` | `Bool(bool)` | `AmountToken(&'static str)`
  - `MessageRef { key: MessageKey, params: Vec<MessageParam>, children: Vec<MessageRef> }`
  - `Effect::message(self) -> MessageRef`
  - `fn reject_message(Reject) -> MessageRef` → keys `reject.not_castable`, … (snake of variant)
  - `fn amount_param(name, Amount) -> MessageParam` — `Fixed(n)` → `Int(n)`; else `AmountToken` snake id (`x`, `half_x`, `per_creature_you_control`, …)
  - Filter/dest/keyword helpers emit **machine tokens** in `Str`/`OwnedStr`, never display English

- [ ] **Step 1: Write the failing test**

In `crates/engine/src/message.rs` (or `message/effect.rs` `#[cfg(test)]`):

```rust
#[test]
fn message_refs_are_stable() {
    let draw = Effect::Draw(DrawEffect::Cards {
        count: Amount::Fixed(2),
    })
    .message();
    assert_eq!(draw.key.as_str(), "effect.draw_cards");
    assert_eq!(draw.params[0].name, "count");
    assert!(matches!(draw.params[0].value, MessageParamValue::Int(2)));

    let life = Effect::Life(LifeEffect::Gain {
        amount: Amount::Fixed(1),
    })
    .message();
    assert_eq!(life.key.as_str(), "effect.life_gain");
    assert!(matches!(life.params[0].value, MessageParamValue::Int(1)));

    let scry = Effect::Dig(DigEffect::Scry {
        count: Amount::Fixed(3),
    })
    .message();
    assert_eq!(scry.key.as_str(), "effect.scry");

    let seq = Effect::Sequence {
        steps: &[
            Effect::Draw(DrawEffect::Cards {
                count: Amount::Fixed(2),
            }),
            Effect::Choice(ChoiceEffect::Discard {
                count: 2,
                target_player: false,
                or_one_matching: None,
            }),
        ],
    }
    .message();
    assert_eq!(seq.key.as_str(), "effect.sequence");
    assert_eq!(seq.children.len(), 2);
    assert_eq!(seq.children[0].key.as_str(), "effect.draw_cards");
}

#[test]
fn reject_messages_use_reject_namespace() {
    assert_eq!(
        reject_message(Reject::IllegalTarget).key.as_str(),
        "reject.illegal_target"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run --profile ci -p engine message_refs_are_stable reject_messages_use_reject_namespace`  
Expected: FAIL (missing `message` module / `message()`)

- [ ] **Step 3: Implement MessageRef + migrate every `label()` arm**

1. Add `message_keys! { ... }` listing every key you emit (`effect.*`, `reject.*`, `auto.*`, `action.*` used from engine). Start from today’s `label.rs` arms: one key per leaf (prefer `effect.<family>_<mode>` matching nested effect vocab, e.g. `effect.damage_target`, `effect.draw_cards`).
2. Implement `Effect::message` by mechanical translation of each `label.rs` arm:
   - `format!("Deal {} damage", amount_label(amount))` → key `effect.damage_target` + `amount_param("amount", amount)`
   - Long prose with filters → key + token params (`filter`, `dest`, …) using snake ids
   - `Sequence` → `effect.sequence` with `children: steps.map(Effect::message)`
   - Joiner phrases that today use `", then "` live in the **client** catalog for `effect.sequence`
3. Delete `label.rs` and all `.label()` call sites inside engine (use `.message()`).
4. Update `ModeInfo` to hold `MessageRef`.
5. Keep matches exhaustive.

Key naming rule (lock this): `effect.<serde_type>_<serde_mode>` for family leaves when unambiguous; structural `effect.sequence`, `effect.conditional` (if labeled), `effect.choose_one`. Rejects: `reject.<snake_variant>`.

- [ ] **Step 4: Run engine tests**

Run: `cargo nextest run --profile ci -p engine`  
Expected: PASS (fix any engine tests still expecting English `.label()` strings)

- [ ] **Step 5: Commit**

```bash
git add crates/engine
git commit -m "feat(engine): replace Effect::label with MessageRef keys"
```

---

### Task 2: Proto `MessageRef` + reserve old string fields

**Files:**
- Modify: `proto/mtgfr/v1/common.proto`
- Modify: `proto/mtgfr/v1/mtgfr.proto`
- Modify: `proto/mtgfr/v1/stream.proto`
- Run: `just server-codegen`

**Interfaces:**
- Consumes: none
- Produces: generated Rust/TS types for MessageRef; new fields on Ack / views / delta

- [ ] **Step 1: Add messages to `common.proto`**

```protobuf
// Stable i18n key + params. No display English in `key`.
message MessageRef {
  string key = 1;
  repeated MessageParam params = 2;
  repeated MessageRef children = 3;
}

message MessageParam {
  string name = 1;
  oneof value {
    string string_value = 2;
    int64 int_value = 3;
    bool bool_value = 4;
    string amount_token = 5;
  }
}
```

- [ ] **Step 2: Hard-cut field numbers on Ack + stream views**

For each in-scope string field:

1. Rename the old field to a reserved placeholder comment and **remove the field** from the message (or comment with `reserved N;` / `reserved "reason";`).
2. Add the MessageRef field on a **new** number.

Concrete Ack:

```protobuf
message Ack {
  bool accepted = 1;
  reserved 2;
  reserved "reason";
  optional MessageRef reject_reason = 3;
}
```

Apply the same pattern in `stream.proto` for:

- `StackObjectView.label` (4)
- `ModeView.label` (1)
- `ActionView.label` (6)
- Pending choice `label` / `labels` fields listed in the design file map
- `DeltaEnvelope.auto_actions` (4)

Use new field names that decode clearly in TS (`label`, `labels`, `auto_actions`, `reject_reason`) on the new numbers. Import `common.proto` where needed.

**Do not** change `ChoiceItem.label` (card/seat names).

- [ ] **Step 3: Codegen**

Run: `just server-codegen`  
Expected: buf generate succeeds; `cargo check -p server` may fail until Tasks 3–4 map new fields — that is expected for the hard-cut window.

- [ ] **Step 4: Commit**

```bash
git add proto justfile client/package.json
# include any buf lock changes; do not commit gitignored generated/ if ignored
git commit -m "feat(wire): add MessageRef; reserve English label fields"
```

---

### Task 3: Schema DTOs + projection hard cut

**Files:**
- Modify: `crates/schema/src/dto.rs`
- Modify: `crates/schema/src/snapshot.rs`
- Modify: `crates/schema/src/projection/choice.rs`
- Modify: `crates/schema/src/catalog.rs`
- Modify: `crates/schema/src/event.rs`
- Modify: schema tests / serde fixtures that embed English labels
- Add: small `crates/schema/src/message.rs` mapper `engine::MessageRef` → schema DTO `MessageRef` if types differ

**Interfaces:**
- Consumes: `engine::MessageRef`, `Effect::message`, `reject_message`
- Produces: DTO fields as `MessageRef` / `Vec<MessageRef>` instead of `String` for in-scope labels; `ChoiceItem.label` stays `String`

- [ ] **Step 1: Write failing projection test**

In `crates/schema/src/projection/choice.rs` tests (or snapshot tests), replace English asserts:

```rust
assert_eq!(view_label.key, "effect.draw_cards");
// not: assert_eq!(label, draw_effect().label());
```

Pick one existing mode/choose-target test and flip it to keys first (red).

- [ ] **Step 2: Run to verify fail**

Run: `cargo nextest run --profile ci -p schema <test_name>`  
Expected: FAIL on type/field mismatch or old assert

- [ ] **Step 3: Implement DTO + projection**

1. Add schema-facing `MessageRef` / `MessageParam` (serde-friendly owned `String`s).
2. Map engine refs → DTO in one helper (`to_wire_message` / `dto::MessageRef::from`).
3. Replace every in-scope `String` label with `MessageRef`:
   - Effect prose → `effect.message()`
   - Action chrome (`Keep hand`, …) → `action.keep_hand` etc. (construct MessageRef in snapshot)
   - Prefixed actions (`Cycle: {name}`) → `action.cycle` + param `name`
   - Pure card-name actions → `action.card_name` + param `name`
4. `auto_actions` on envelopes: `Vec<MessageRef>`.
5. Catalog: stop joining English; expose `Vec<MessageRef>` (or drop summary prose and let client format). Update catalog tests accordingly.
6. Fix serde fixtures in `dto.rs` tests to use MessageRef JSON shape.

- [ ] **Step 4: Run schema tests**

Run: `cargo nextest run --profile ci -p schema`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/schema
git commit -m "feat(schema): project MessageRef labels and auto_actions"
```

---

### Task 4: Server Ack + forced auto_actions as MessageRef

**Files:**
- Modify: `crates/server/src/session.rs` — `ApplyResult.reason`, `forced_action_label`, broadcast `auto_actions`
- Modify: `crates/server/src/game_loop.rs` — Ack reason tags
- Modify: `crates/server/src/grpc/game_svc.rs` — `ack_msg`
- Modify: `crates/server/src/grpc/map/stream.rs` — map MessageRef / auto_actions
- Modify: `crates/server/src/stream.rs` — types
- Modify: server tests asserting English auto_actions or string reasons

**Interfaces:**
- Consumes: `engine::reject_message`, schema/engine MessageRef
- Produces: `Ack.reject_reason: Option<MessageRef>`; delta `auto_actions: Vec<MessageRef>`

- [ ] **Step 1: Write failing test**

Update a session test that today checks auto_action English, e.g. forced target, to assert key:

```rust
assert_eq!(broadcast.auto_actions[0].key, "auto.only_one_legal_target");
```

And a reject path:

```rust
assert_eq!(ack.reject_reason.as_ref().unwrap().key, "reject.illegal_target");
```

(Adjust to actual test harness field names after Task 2.)

- [ ] **Step 2: Run to verify fail**

Run: `cargo nextest run --profile ci -p server <test_name>`  
Expected: FAIL

- [ ] **Step 3: Implement server mapping**

1. Replace `reject(&format!("{rejected:?}"))` with MessageRef from `reject_message(rejected)`.
2. Server-only tags → keys: `reject.unknown_table`, `reject.game_not_started`, `reject.not_seated`, `reject.engine_error`, `reject.stack_yield_one_shot`, `reject.not_helpless` (add to `MessageKey::all()` via a server helper or shared keys module in engine/schema).
3. Replace `forced_action_label` with `forced_action_message` returning MessageRef:
   - `auto.discarded_to_hand_size`
   - `auto.discarded`
   - `auto.only_one_legal_target`
   - `auto.trigger_order_forced`
   - `auto.sacrificed_forced` + `{ name }`
   - `auto.automatic` fallback
4. Map through gRPC (`ack_msg`, stream map) including params/children.

- [ ] **Step 4: Run server tests**

Run: `cargo nextest run --profile ci -p server`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/server
git commit -m "feat(server): Ack reject_reason and auto_actions as MessageRef"
```

---

### Task 5: Client `formatMessage` + English catalog + wire types

**Files:**
- Create: `client/lib/i18n/message.ts`
- Create: `client/lib/i18n/catalog/en.ts`
- Create: `client/lib/i18n/formatMessage.test.ts`
- Modify: `client/lib/wire/types.ts` — Ack, views, auto_actions
- Modify: `client/lib/wire/protoMap.ts` (+ tests) — decode MessageRef
- Modify: `client/lib/reject.ts` — delegate to formatMessage

**Interfaces:**
- Consumes: wire `MessageRef`
- Produces: `formatMessage(ref: MessageRef): string`

- [ ] **Step 1: Write failing tests**

`client/lib/i18n/formatMessage.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { formatMessage } from "./message";

describe("formatMessage", () => {
  it("formats effect.draw_cards", () => {
    expect(
      formatMessage({
        key: "effect.draw_cards",
        params: [{ name: "count", int_value: 2 }],
        children: [],
      }),
    ).toBe("Draw 2");
  });

  it("joins sequence children with then", () => {
    expect(
      formatMessage({
        key: "effect.sequence",
        params: [],
        children: [
          {
            key: "effect.draw_cards",
            params: [{ name: "count", int_value: 2 }],
            children: [],
          },
          {
            key: "effect.discard",
            params: [{ name: "count", int_value: 2 }],
            children: [],
          },
        ],
      }),
    ).toBe("Draw 2, then Discard 2");
  });

  it("returns raw key when missing", () => {
    expect(formatMessage({ key: "effect.unknown_zz", params: [], children: [] })).toBe(
      "effect.unknown_zz",
    );
  });

  it("formats reject.illegal_target", () => {
    expect(
      formatMessage({ key: "reject.illegal_target", params: [], children: [] }),
    ).toBe("Pick a legal target.");
  });
});
```

- [ ] **Step 2: Run to verify fail**

Run: `cd client && bun run test lib/i18n/formatMessage.test.ts`  
Expected: FAIL (module missing)

- [ ] **Step 3: Implement formatter + catalog**

1. Define TS `MessageRef` / `MessageParam` matching wire (snake_case).
2. `formatMessage`:
   - look up `enCatalog[key]`
   - resolve params: ints as decimal; `amount_token` / filter tokens via token maps; `string_value` for names as-is; recurse `children`
   - missing → return `key`
3. Port today’s English from `label.rs` (via the keys introduced in Task 1), `humanReason`, and `forced_action_label` into `catalog/en.ts`. Prefer functions `(p) => string` for parameterized keys.
4. Update `wire/types.ts` Ack: `reject_reason?: MessageRef | null` (remove string `reason`).
5. Update protoMap for MessageRef fields; fix `protoMap.test.ts`.
6. `humanReason` — if any leftover string tags remain in board code, either delete or map through a deprecated path; prefer `formatMessage` only.

- [ ] **Step 4: Run client unit tests for i18n + wire**

Run: `cd client && bun run test lib/i18n lib/wire/protoMap.test.ts lib/reject.ts`  
Expected: PASS (adjust globs to existing reject tests)

- [ ] **Step 5: Commit**

```bash
git add client/lib/i18n client/lib/wire client/lib/reject.ts
git commit -m "feat(client): formatMessage English catalog for MessageRef"
```

---

### Task 6: Client UI call sites (prompts, stack, actions, fold, intents)

**Files:**
- Modify: `client/app/game/intents.ts` — `ack.reject_reason` → `formatMessage`
- Modify: `client/app/board/submodel.ts` — reject reasons
- Modify: `client/app/board/html/prompts.ts` — titles/labels via `formatMessage`
- Modify: `client/app/board/html/stack.ts`
- Modify: `client/app/game/fold.ts` — auto_actions
- Modify: hand / radial / activation-menu / targeting sites using `action.label`
- Modify: Scene/unit fixtures that put English in wire `label` fields — store MessageRef + expect formatted English in the UI

**Interfaces:**
- Consumes: `formatMessage`, wire MessageRef fields
- Produces: unchanged player-visible English (parity with pre-cut copy)

- [ ] **Step 1: Write / update one failing Scene or unit test**

Example pattern in `prompts.test.ts` or `surfaces.test.ts`: fixture pending choice with `label: { key: "effect.life_gain", params: [{ name: "amount", int_value: 1 }], children: [] }` and assert visible text `"Gain 1 life"`.

- [ ] **Step 2: Run to verify fail**

Run: `cd client && bun run test app/board/html/prompts.test.ts` (or the file you edited)  
Expected: FAIL until call sites format

- [ ] **Step 3: Wire formatMessage through UI**

Replace every read of wire English label/auto_action/reason with `formatMessage`. Keep client-only chrome (`"Choose"`, `"Fail to find"`, waiting templates) as local strings for v1.

- [ ] **Step 4: Run focused client board tests**

Run: `cd client && bun run test app/board app/game/intents app/reject`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app client/lib
git commit -m "feat(client): render MessageRef labels in board chrome"
```

---

### Task 7: Catalog coverage sync + feature specs

**Files:**
- Create: `client/lib/i18n/catalogCoverage.test.ts`
- Create: `crates/engine/src/message/keys_list.rs` or export `MessageKey::all()` already from Task 1
- Optional: `scripts/export_message_keys.rs` / test that writes `client/lib/i18n/generatedKeys.json` — prefer **simpler**: a Rust test that prints/keys fixture checked into `client/lib/i18n/rustKeys.json` updated in the same PR when keys change; client test imports that JSON
- Modify: `docs/superpowers/specs/2026-07-20-wire-protocol-and-visibility.md`
- Modify: `docs/superpowers/specs/2026-07-20-prompts-and-pending-choices.md`
- Modify: `docs/superpowers/specs/2026-07-20-stack.md` (if it documents English labels)
- Modify: related choices/actions specs if needed
- Modify: `docs/superpowers/specs/2026-07-25-message-ref-wire-i18n-design.md` — Status: Done
- Modify: `docs/superpowers/specs/README.md` — index the design if not already

**Interfaces:**
- Consumes: `MessageKey::all()`
- Produces: failing CI if catalog missing a key

- [ ] **Step 1: Write failing coverage test**

```ts
// client/lib/i18n/catalogCoverage.test.ts
import { describe, expect, it } from "vitest";
import { enCatalog } from "./catalog/en";
import rustKeys from "./rustKeys.json";

describe("catalog coverage", () => {
  it("includes every key the engine/server can emit", () => {
    const missing = (rustKeys as string[]).filter((k) => !(k in enCatalog));
    expect(missing).toEqual([]);
  });
});
```

Add a Rust test (engine) that fails if `MessageKey::all()` drifts from committed `client/lib/i18n/rustKeys.json` **or** regenerate the JSON in the test with an env flag — simplest durable approach: Rust test asserts `include_str!("../../../client/lib/i18n/rustKeys.json")` deserializes to the same set as `MessageKey::all()`.

- [ ] **Step 2: Run to verify fail**

Run: `cd client && bun run test lib/i18n/catalogCoverage.test.ts`  
Expected: FAIL until `rustKeys.json` + catalog complete

- [ ] **Step 3: Fill catalog gaps + update specs**

1. Ensure every key in `MessageKey::all()` exists in `enCatalog` with copy matching pre-cut English.
2. Update feature specs for MessageRef behavior (current-behavior only; no migration narrative).
3. Mark design status Done.

- [ ] **Step 4: Full verify**

Run:

```bash
just server-codegen
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo nextest run --profile ci -p engine -p schema -p server
cd client && bun run check   # or just client-check
```

Expected: all green

- [ ] **Step 5: Commit**

```bash
git add client/lib/i18n docs/superpowers/specs
git commit -m "docs: MessageRef feature specs + catalog coverage guard"
```

---

## Self-review (plan vs spec)

| Spec requirement | Task |
|------------------|------|
| MessageRef key+params on wire | Task 2 |
| children for Sequence | Task 1 + 2 (design refinement) |
| Hard cut / reserve old fields | Task 2 |
| Delete engine English formatters | Task 1 |
| Reject mapper | Task 1 + 4 |
| Schema projection | Task 3 |
| Auto_actions MessageRef | Task 3–4 |
| Client catalog + formatMessage | Task 5 |
| UI call sites | Task 6 |
| Catalog sync test | Task 7 |
| Feature spec updates | Task 7 |
| Non-goals excluded | Global constraints |

No intentional placeholders remain; effect-key enumeration is specified by naming rule + exhaustive match rather than a 239-row table in this plan (source of truth = `MessageKey` + old `label.rs` arms during migration).
