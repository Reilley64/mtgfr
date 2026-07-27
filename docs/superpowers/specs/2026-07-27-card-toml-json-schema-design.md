# Card TOML JSON Schema + generated DSL reference (design)

**Status:** Approved design input (2026-07-27; revised same day — full generation from
CardDef TOML surface + generated `DSL_REFERENCE`).
**Surfaces:** `card-dsl-and-card-pool` (`crates/cards/data/`, `crates/engine` `de` /
`CardDef` / `Effect`), `.agents/skills/card-dsl/`.

Related: [card-dsl-and-card-pool](2026-07-20-card-dsl-and-card-pool.md),
`.agents/skills/card-dsl/SKILL.md`, `.agents/skills/card-dsl/DSL_REFERENCE.md`.

This document is **design input only**. Implement-time work must update the living surface
spec and skill docs listed below — a design sidecar does not replace those updates
([AGENTS.md](../../../AGENTS.md) Feature specs).

---

## Problem Statement

Card authors and agents edit ~780 deckable TOMLs (plus ~43 token profiles) that only become
validated when Rust `toml::from_str` → `CardDef` runs (pool load / tests). Typos fail late
with serde-oriented messages. There is no editor-associated schema and no non-Rust single-file
validator.

Separately, `.agents/skills/card-dsl/DSL_REFERENCE.md` (~2k lines) hand-documents the same
surface. It drifts from `types` / `de.rs` whenever the DSL grows; agents are told “code wins,”
but the reference is still the human/agent field guide.

Both artifacts should be **projections of one Rust TOML surface**, not parallel hand lists.

---

## Goal

1. **JSON Schema** for card/token TOML — fully generated from the Rust types that define the
   CardDef TOML deserialize surface (not a hand-maintained skeleton).
2. **`DSL_REFERENCE.md`** — generated from the same surface (rustdoc / structured field
   metadata), with `--check` drift detection.
3. Keep **runtime deserialize into `CardDef`** as the engine authority; schema and markdown are
   projections for tools and people.

---

## Locked decisions

| Decision | Choice |
|---|---|
| Authority | Rust CardDef TOML deserialize surface wins; schema + DSL reference are generated projections |
| Schema strategy | **Fully generated** from TOML-facing Rust types (schemars or equivalent) — **no hand skeleton** |
| Why not `JsonSchema` on today’s `CardDef` as-is | Memory shape ≠ TOML (`'static` / `Arc` / `deserialize_with`, `[kind] type = "instant"` vs `CardKind::Spell`, flat `[cost]`, flat activated `[[abilities]]`). Generation targets the **TOML document types** that serde already conceptually implements |
| How “from CardDef” is achieved | Extract / introduce an explicit **TOML surface** (`CardToml` / existing raw structs in `de.rs` promoted) that **is** what `toml::from_str` parses, then `Into` / existing fold into `CardDef`. Schema + docs derive from that surface |
| DSL reference | **Generated** `DSL_REFERENCE.md` (or `.generated.md` checked in) from the same types’ docs + enum tags; process/header discipline (§0 comments) stays in `SKILL.md` (not schemaable) |
| Depth | Full structural coverage of the TOML surface as types gain `JsonSchema` + docs — ship incrementally by migrating `de.rs` special cases onto named surface types, but **direction is full gen**, not permanent hybrid |
| Card vs token | Shared generated `$defs`; thin token root requires non-empty `id` / `default_print` |
| Validator | Rust in `crates/cards` (TOML → JSON → schema); committed schema for editors |
| Comments | Oracle / `# ponytail:` remain in source files; stripped before validate |
| Not in schema / generated reference | Fidelity judgment, Scryfall liveness, deck legality, live-game rules behavior |

---

## Approaches considered

1. **Hybrid gen vocabularies + hand skeleton** (prior revision) — Smaller first PR; permanent
   dual maintenance of the skeleton. **Superseded** by product preference for full generation.
2. **`schemars` directly on today’s `CardDef` / `Effect` without a TOML surface** — Would emit
   the wrong document shape. Rejected.
3. **Explicit TOML surface types → `CardDef` + generate schema and DSL reference (chosen)** —
   One Rust authoring model for what appears in TOML; schemars (or sibling) emits JSON Schema;
   a doc renderer emits `DSL_REFERENCE.md` from rustdoc / attributes on those same types.
   Custom `de.rs` visitors become named types instead of opaque functions over time.

---

## Design

### Architecture

```text
  crates/engine (card-dsl)
       │
       ├─ TOML surface types  (CardToml, KindToml, AbilityToml, Effect, …)
       │     serde Deserialize  ──►  fold / Into  ──►  CardDef (runtime)
       │     JsonSchema + rustdoc
       │
       └─ gen (just cards-schema / cards-dsl-ref)
              ├─► crates/cards/schema/card.schema.json
              ├─► crates/cards/schema/token.schema.json
              └─► .agents/skills/card-dsl/DSL_REFERENCE.md
```

Authors still write TOML. Load path stays `toml::from_str::<CardToml>` (or today’s entry that
becomes that) then convert to `CardDef`. Engine tests that build `CardDef` literals unchanged.

### Making the surface generatable

Today many TOML spellings live only inside `de.rs` `deserialize_with` / manual `Deserialize`
impls. Implement work:

1. **Inventory** opaque deserializers (`CardKind`, `Cost`, flat `Ability` / `Timing`, filter
   shorthands, etc.).
2. **Promote** each to a public (or `pub(crate)`) TOML struct/enum with ordinary serde attrs
   (`tag`, `rename_all`, `flatten` where honest) that schemars can see.
3. **Fold** those types into existing `CardDef` / `Timing` / … runtime values (keep current
   runtime types stable).
4. Derive or implement **`JsonSchema`** on the TOML surface. Map `&'static str` / `Arc<[T]>`
   load-only fields to `String` / `Vec<T>` on the surface types.
5. Prefer **doc comments on surface fields** as the single prose source for both schema
   `description` and generated markdown tables.

Do not invent a second hand-written JSON Schema. Do not generate Rust from JSON Schema.

### JSON Schema outputs

- Dialect: JSON Schema 2020-12.
- Committed: `crates/cards/schema/card.schema.json`, `token.schema.json`.
- `just cards-schema` regenerates; `just cards-schema-check` fails on drift.
- Root instance = one card/token after TOML→JSON (comments stripped).

### Generated DSL reference

- Output: `.agents/skills/card-dsl/DSL_REFERENCE.md` (committed; agents already read this path).
- `just cards-dsl-ref` / `just cards-dsl-ref-check` — same regenerate/check posture.
- Content generated from surface types:
  - Top-level fields table (name, JSON-schema-ish type, default, rustdoc notes).
  - `[cost]`, `[kind]`, keywords, timings, effect `type` / `mode` enums with variant docs.
  - Nested tables (`[[abilities]]`, effect families) as sections mirroring type nesting.
- **Not generated from types** (remain in `SKILL.md` or a tiny hand prologue include):
  - §0 oracle-comment / `# ponytail:` / `approximates` discipline (file comments are outside
    the schema instance).
  - “Flag, don’t force-script” and fidelity process pointers.
- Optional: generated file starts with a banner `<!-- generated by just cards-dsl-ref; do not
  edit -->` and a short pointer to `SKILL.md` for process rules.
- **Migration:** first implement PR may generate a structural reference that is thinner than
  today’s hand prose; **move long CR / example / ponytail notes from the hand file into rustdoc
  on the surface types** as those types are promoted, until the generated doc replaces the hand
  file. Do not maintain two full references long-term — delete or shrink the hand body once gen
  coverage matches.

### Validator CLI

- `just cards-toml-validate [paths…]` — TOML → JSON → schema; path + JSON Pointer + message
  (+ enum hints when available).
- Wire schema/ref `--check` into the card-pool verify path used by `just check` / server verify.
- Full-pool validate once the generated schema accepts the current pool; fix surface/schema if
  not — do not loosen Rust deserialize to match a bad schema.

### Editor / agent integration

- `card-dsl` skill: validate via `just cards-toml-validate`; treat generated
  `DSL_REFERENCE.md` as the field guide; on disagreement, Rust surface types win (regenerate).
- Optional Taplo / editor schema association for `crates/cards/data/**/*.toml`.
- `fidelity-grind`: optional validate on touched TOMLs.

### Waves (implementation sequencing, still one design)

| Wave | Deliverable |
|---|---|
| **A** | TOML surface entry (`CardToml` + kind/cost/ability enough for a large fraction of pool) + schemars → `card.schema.json` + validate CLI + check recipes |
| **B** | Generate `DSL_REFERENCE.md` from the same surface docs; check recipe; move field prose from hand reference into rustdoc as types land |
| **C** | Finish promoting remaining `de.rs` special cases onto surface types until schema/reference cover the full authoring surface; remove obsolete hand reference body |

Waves may ship as separate PRs; each must keep pool load green and `--check` honest.

---

## At implement time — update these living docs

| Doc | What to add |
|---|---|
| [card-dsl-and-card-pool](2026-07-20-card-dsl-and-card-pool.md) | TOML surface → CardDef; schema + DSL ref generation; recipes; testing seams |
| `.agents/skills/card-dsl/SKILL.md` | Validate/gen commands; generated reference; process §0 stays here; rustdoc is field prose SoT |
| `.agents/skills/card-dsl/DSL_REFERENCE.md` | Becomes generated (banner + content); hand edits redirected to Rust docs |
| `justfile` | `cards-schema`, `cards-schema-check`, `cards-dsl-ref`, `cards-dsl-ref-check`, `cards-toml-validate` |
| CI / [ci-and-release](2026-07-20-ci-and-release.md) | If check recipes are added to verify beyond existing `just check` wiring |
| `.agents/skills/fidelity-grind/SKILL.md` | Only if grind gains an explicit validate / ref-check step |

Index this design under Process/policy in [specs README](README.md).

---

## Verification plan (implementation PR)

1. `just cards-schema` / `cards-dsl-ref` produce stable committed artifacts; `--check` variants
   pass on a clean tree.
2. Known-good card TOML validates; typo’d effect `type` fails with enum hint + file path.
3. Stale schema or stale `DSL_REFERENCE.md` fails the matching `--check`.
4. Token profile validates; empty token `id` fails.
5. Changing a surface field’s rustdoc and regenerating updates the reference section for that
   field.
6. Full pool still loads via Rust (`cards` tests / server build).
7. Engine `CardDef { … }` literal tests still compile without going through TOML.

---

## Out of scope

- Generating Rust / `CardDef` from JSON Schema (wrong direction)
- Encoding fidelity, Scryfall API freshness, or deck legality in schema
- Putting exporters or schema gen inside pure engine gameplay paths (gen is a build/dev binary
  or `card-dsl`-featured tool crate path)
- Auto-generating SKILL.md process rules (§0 comment discipline)
- Requiring schema validate before Rust-literal engine unit tests

---

## Further notes

- “Fully generated from CardDef” means **from the CardDef TOML deserialize surface**, not from
  the interned runtime struct layout. If those diverge, fix the surface types — do not paper
  over with a hand schema.
- Prior hybrid design is obsolete; do not implement a permanent hand-maintained skeleton.
- DTCG analogy: one authored source → many projections. Here the authored source is Rust TOML
  surface types (+ rustdoc); projections are schema, DSL reference, and runtime `CardDef`.
