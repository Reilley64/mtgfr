# Wire catch-up hardening — design

**Status:** Implementing (2026-07-25)
**Delivery:** Sequential PR 2 of 5 (after stack multi-target #194)
**Living specs:** [`prompts-and-pending-choices`](2026-07-20-prompts-and-pending-choices.md),
[`wire-protocol-and-visibility`](2026-07-20-wire-protocol-and-visibility.md)

## Problem

Hand-written `PendingChoiceView` / `VisibleEvent` unions can drift from codegen after engine
waves. `protoMap` flattens oneofs and casts, so TypeScript exhaustiveness on the hand union does
not catch a missing kind until someone updates `types.ts`. Fidelity-grind Phase 5 still points at
pre-Foldkit paths.

## Goal

Fail `just client-check` when generated proto oneof cases diverge from hand registries; refresh
Phase 5 docs to Foldkit paths and `just client-check`.

## Approach

Type-only generated↔hand guard (Vitest): compare `PendingChoiceViewSchema.field` /
`VisibleEventSchema.field` (camel→snake) to `FORMULATOR_FOR_KIND` keys and
`VISIBLE_EVENT_KIND_PRESENCE` (`satisfies Record<VisibleEvent["kind"], true>`).

## Non-goals

- New prompt UI forms or fold narration
- Proto shape changes
- Wins 3 / 5 / 6
