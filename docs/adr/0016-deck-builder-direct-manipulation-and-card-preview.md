# 0016 — Deck builder: direct manipulation + card preview

Status: **Accepted**

## Decision

- Pool column scrolls; deck panel fixed. Shared `CardPreview` overlay (hover in builder, Alt on board).
- Left-click pool = add one; right-click context menu (basics/commander variants). Click deck row = remove one.
- Commander set via menu only; singleton enforced in helpers.

## Consequences

- No commander `<select>` or per-row number inputs. Alt-preview battlefield-only for now.
