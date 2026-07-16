# 0015 — Card imagery via self-hosted CDN

Status: **Accepted**; identity map superseded by [0031](0031-card-id-and-printing-art-preference.md).

## Decision

- Optional art CDN (bake `VITE_CARD_CDN` at web image build) serving large webp by Scryfall **Printing** UUID.
- `imageUrlByPrint()` used by builder, hand, board. Missing CDN art is a broken image (no Scryfall image-host fallback).

## Consequences

- Print UUIDs live on deck lines and ObjectViews (ADR 0031). Pool cards bake `default_print`; precons stamp explicit SoC prints.
