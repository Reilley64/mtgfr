# 0028 — Battlefield row packing and permanent clusters

Status: **Accepted**.

## Context

A battlefield row that exceeds the nominal ~9 tidy slots used to keep full spacing and spill toward the neighbour seat. On a 2×2 Commander table that collides boards and fights the camera. MTGA instead keeps permanents inside the seat and increases density.

## Decision

- **Row packing:** each battlefield row independently compresses horizontal spacing so cards stay inside the seat band (soft floor — compress as far as needed). Alignment rules stay (Creatures/Lands center-out; Noncreature left / planeswalkers right). No seat widening, no spill.
- **Permanent clusters:** only when a row would overflow full spacing, collapse every eligible identical group (indistinguishable on what the table shows, no attachment stacks) into one face + count; then pack if still over. Hover/hold fans members in an arc; short touch tap selects the top member. A selected fanned member stays raised (and keeps the fan open) until deselected; the activation radial centers on that member.
- **Hover raise:** packed distinct cards lift on hover for paint and hit-testing.

## Consequences

- `boardBounds` / camera fit stay on the nominal seat footprint.
- Clusters are not GY/exile **Piles** — separate render field and click path.
