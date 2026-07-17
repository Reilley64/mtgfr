# 0033 — Segmented card play motion

Status: **Accepted**.

Hand and stack live as DOM overlays; the battlefield (and most zone piles) live on the canvas. We keep **segmented legs** — play-in into the stack section on the DOM side, canvas entrance-seeds for battlefield / graveyard / exile — rather than one continuous cross-surface flight ghost. Own plays record a **play origin** per hand-card id; opponents play in from the **player avatar**. Tokens use a **creator origin** (stack object → source permanent → avatar), which requires creator provenance on `token_created`. Non-play card entries use **zone-sourced entrances**.

## Considered options

- **Continuous play-flight ghost** across DOM and canvas — rejected: two surfaces, more timing bugs, and the existing entrance-seed / stack overlay model already covers land and stack→battlefield when origins are fixed.
- **Single last-drop seed** — rejected: overlapping plays mis-seed; replaced by per-card-id **play origin** map.
