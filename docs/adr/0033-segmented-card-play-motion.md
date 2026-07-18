# 0033 — Segmented card play motion

Status: **Superseded** by [0035](0035-canvas-flight-layer.md).

Hand and stack live as DOM overlays; the battlefield (and most zone piles) live on the canvas. This ADR kept **segmented legs** — play-in into the stack section on the DOM side, canvas entrance-seeds for battlefield / graveyard / exile — rather than one continuous cross-surface flight ghost. Own plays recorded a **play origin** per hand-card id; opponents played in from the **player avatar**. Tokens used a **creator origin** (stack object → source permanent → avatar). Non-play card entries used **zone-sourced entrances**.

## Considered options (historical)

- **Continuous play-flight ghost** across DOM and canvas — rejected here; later accepted in 0035 as a canvas-only flight layer with scale lerp (resting chrome stays DOM).
- **Single last-drop seed** — rejected: overlapping plays mis-seed; replaced by per-card-id **play origin** map (still used under 0035).
