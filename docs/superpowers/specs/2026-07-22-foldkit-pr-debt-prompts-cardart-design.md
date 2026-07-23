# Foldkit PR debt — pending_choice fidelity + DOM CardArt

**Date:** 2026-07-22  
**Status:** Done  
**PR:** [#74](https://github.com/Reilley64/mtgfr/pull/74) (`cursor/foldkit-migration-design-1ef0`)  
**Context:** Cutover and merge-cleanup are on the branch. PR description **Remaining debt (non-page chrome)** lists two items: ~47 lower-traffic `pending_choice` kinds still banner+stub, and shared DOM `CardArt` / `ImageCache` (raw `<img>` works). This design closes both on the same PR.

## Decisions (locked)

| Question | Choice |
|---|---|
| Scope | **Both** debts in one design |
| Pending-choice bar | **Full coverage** — every `PendingChoiceView` kind has a faithful form; no production stub |
| CardArt bar | **Solid CardArt parity** — shared cache, loading skeleton, face sizes, one API at every DOM card face |
| Landing | **Same PR (#74)** |
| Packaging | **Formulator-first** — classify kinds into formulators; CardArt is the shared face primitive |

## Goals

1. No production path renders “Limited UI — pick an option” or uses `stubPendingChoice`.
2. Every `PendingChoiceView["kind"]` maps to a formulator that collects a valid `AnswerInput`; intents go only through existing `choiceIntent(pc, answer)`.
3. All DOM card faces (hand, stack, prompts, pile, inspect, decks list/builder) use a shared `cardArt(…)` helper backed by `sharedImageCache`, with skeleton while loading and consistent `ImageSize` / `ImageFace`.

## Non-goals

- Engine or `.proto` changes to choice shapes.
- Rewriting bitmap/canvas paint (`paintCardArt` stays).
- New pending_choice kinds.
- Non-card chrome.

## CardArt / ImageCache

### API

`client/lib/ui/card-art.ts` exports a Foldkit-facing helper:

```ts
cardArt(opts: {
  print: string;           // print id, or "" → card back
  size?: ImageSize;        // default "large"
  face?: ImageFace;        // default "front"
  alt: string;
  className: string;       // caller owns layout
  testId?: string;
}): Html
```

### Loading

Uses `sharedImageCache` (`client/lib/image-cache.ts`):

1. Resolve URL with `imageUrlByPrint` / `cardBackUrl` (unchanged contract).
2. `cache.get(url)` / `preload` so decode is shared with the bitmap layer.
3. While not ready: skeleton placeholder (`animate-skeleton`; caller’s `className` supplies width/aspect).
4. When ready: `<img src=url …>` — our `ready` set avoids skeleton flash on second mount.

### Repaint

Foldkit `Mount` (same pattern as `client/app/board/bitmap/mount.ts`): subscribe to `sharedImageCache`, re-render placeholders into images (lightweight message such as `CardArtLoaded`, or Mount-local invalidate).

### Call sites

Replace raw `h.img` + `imageUrlByPrint` for card faces:

| Surface | Default size |
|---|---|
| Hand bar, stack, prompts card picks, pile overlay, inspect | `large` |
| Deck list commander tile | `art_crop` |
| Deck builder pool / print tiles | `large` |

Bitmap `paintCardArt` remains canvas-only.

### Tests

- URL selection and skeleton→ready with fake `makeImage`.
- `sharedImageCache` marks ready once.
- Scene/smoke: hand or prompt shows `img` after load.

## Pending_choice formulators

### Invariant

`choiceIntent(pc, answer)` in `client/lib/choice.ts` already maps every `AnswerInput` → `WireIntent`. Forms only collect a valid `AnswerInput`. No per-kind intent construction in the view.

### Production surface

- **Delete** `stubPendingChoice` and the “Limited UI — pick an option” caption.
- `pendingChoicePrompt` becomes an **exhaustive** `switch (pending.kind)` with a `never` default (TypeScript).

### Formulators

Each formulator returns `Html` and uses `cardArt` for card faces:

| Formulator | Collects | Example kinds |
|---|---|---|
| `cardPick` (existing) | `card-pick` draft → `buildAnswerFromDraft` | search, scry, surveil, discard, sacrifice_*, proliferate, phase_out, may_*, exile_*, put_*_from_hand, dredge, attach_host, copy_target, item-list object picks |
| `orderTriggers` (existing) | order draft | `order_triggers` |
| `damageAssign` (existing) | damage draft | `assign_combat_damage` |
| `yesNo` | may yes/no | `may_yes_no` |
| `modeList` | mode index / trigger modes | `choose_mode`, `choose_trigger_modes` |
| `payCost` | pay / decline | `pay_cost`, `pay_or_counter`, `pay_or_controller_draws`, `pay_echo_or_sacrifice`, `pay_recover_or_exile`, `sacrifice_unless_pay`, … |
| `playerPick` | player targets | `choose_target_players`, `choose_splitting_opponent` |
| `divideTotal` | amounts sum to `total` | `divide_spell_damage`, `divide_counters` |
| `pilePick` | choose pile A/B | `opponent_chooses_pile`, `choose_pile_for_hand` |
| `partition` | split items into buckets | `partition_revealed`, `distribute_top` |
| `colorPick` / `manaColor` | color index | `choose_color`, `choose_mana_color` |
| `stringPick` | option from list | `choose_creature_type` |
| `numberPick` | count in range | `may_draw_up_to`, `trade_secrets_*`, draw-count kinds |
| `destinationPick` | enum buttons | `choose_countered_spell_destination`, `revealed_card_to_battlefield_or_hand`, `choose_top_or_bottom` |

### Pure helpers

Extend in `client/lib/choice.ts`:

- `PromptDraft` variants as needed for divide / partition / number / string.
- `buildAnswerFromDraft` / `initPromptDraft` / `cardPickReady` (and siblings) so every kind can produce an `AnswerInput` without one-offs in the view.
- Remove `FAITHFUL_PROMPT_KINDS` / `isFaithfulPromptKind` once all kinds are faithful (or repurpose as a compile-time registry of formulators).

### Exhaustiveness

A registry or test maps every `PendingChoiceView["kind"]` to a formulator. Adding a new wire kind without a formulator fails the build or a unit test.

### Acceptance

- No production source under `client/app` contains `Limited UI`.
- Every kind has at least one unit/story path that builds a `WireIntent` via `choiceIntent`.

## Packaging (commits on #74)

1. **`feat(client): shared CardArt over ImageCache`** — helper + Mount + call-site swap + tests.
2. **`feat(client): pending_choice formulators for all kinds`** — exhaustive map, extend `choice.ts`, remove stub, tests.

Order: CardArt first so formulators can use it for card faces.

## Verification

- `just client-check` green.
- CardArt + formulators tests as above.
- PR body: remove “Remaining debt” bullets; keep `BREAKING CHANGE` footer for squash-merge release.

## Out of scope reminders

Shell/board specs still have historical Solid wording in mid-Behavior (flagged in merge-cleanup Further Notes). Not part of this debt design.
