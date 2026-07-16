# 0025 — Modal pinned card inspect

Status: **Accepted**.

## Context

The board used cursor-following Alt-preview. Arena docks a large face and lets you scrub. We want Arena-readable inspect without abandoning the 4-seat table or turning Alt into hover scrub.

## Decision

- **Alt-down** over a face-up card **pins** that card into a left **inspect dock** with a full-board dim scrim (modal: board/HUD clicks blocked).
- Pointer move while Alt is held does **not** change the pinned card; release Alt or **Esc** dismisses. Space is ignored while open.
- Prepare DFCs flip in the dock; open on the **play face** (back when the permanent is `prepared`).
- Deck builder keeps cursor-follow hover preview; in-game surfaces use the dock.
- A battlefield pin carries the permanent's **`objectId`**. The dock shows a **sourced mod ledger** under the oracle text (flowing into a second column when needed): contributions grouped by **source card def name**, each name an underlined link. Continuous mods are re-derived live from the snapshot; timed/stateful mods record the source name when applied. **Not** CR 613 layers (ADR 0003) — attribution only.
- Clicking a source name **pushes** that card def onto an inspect **history stack** (catalog oracle only — no ledger on the source view). A **Back** control pops. Ledger rows appear only while the current history entry has a live battlefield `objectId`.
- Marked damage is out of scope for the ledger.

## Consequences

- `ObjectView.prepared` and `CatalogCard.back` are required for play-face default and flip text.
- `ObjectView.modifiers` (`ModifierSourceView`: `source_name` + contribution crumbs) is required for the ledger; empty off the battlefield / when unmodified.
- Flip chrome and source links need pointer events on the dock; the scrim blocks the rest of the UI until dismiss.
- Engine events (`CountersPlaced`, `TempBoost`, `Goaded`, `ControlGainedUntilEndOfTurn`) carry `source_name`; provenance batches live in `Game::modifier_provenance` so `Permanent` stays `Copy`.
