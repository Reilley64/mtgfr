# Battlefield

**Status:** Current (as of 2026-07-27)
**Module:** `client/app/board/canvas/`, `client/app/board/bitmap/`, `client/app/board/chrome.ts`, `client/app/board/geometry/layout.ts`, `client/app/board/engagement.ts`

---

## Problem Statement

The battlefield must show a crowded Commander table with stable seat furniture, readable permanents, live combat/targeting arrows, and MTGA-style chrome without requiring every permanent to be a DOM node. It must also avoid misleading presentation: unplayable cards should not look disabled; playable actions should be called out with borders.

## Solution

Battlefield paint is split across Canvas vector shapes and Mount bitmap layers. Canvas handles felt, seats, and vector helpers; Mount paints resting permanent faces, permanent chrome, and the authoritative Gravatar/monogram avatar face chrome. Flights are documented separately in [`2026-07-20-flights.md`](2026-07-20-flights.md).

The board layer stack authority is [`docs/client-canvas-map.md`](../../client-canvas-map.md). The battlefield paint order is felt → seats → resting cards → avatars → arrows → flights.

## User Stories

- As a player, I can read each permanent and see relevant battlefield chrome.
- As a player, I can identify each seat by Gravatar face or monogram, with life below the face and commander damage below the username when present.
- As a player with priority, I can tell which battlefield permanents have playable actions from their outline.
- As a player declaring combat or targeting, arrows and target highlights stay above resting cards.
- As a player on a crowded board, packing and clusters keep permanents inside their seat bands.

## Behavior

### Paint order

The battlefield paints bottom to top:

1. Felt.
2. Seat bands and zone furniture.
3. Resting battlefield cards.
4. Avatar face, life, and seat-label paint.
5. Combat, block, spell-targeting, and drag-aim arrows.
6. Card flights.

Flights are always above resting cards and are covered in [`2026-07-20-flights.md`](2026-07-20-flights.md). HTML life-orb hit targets sit above this paint so combat drops can be targeted reliably.

### Felt and seats

`canvas/felt.ts` paints the table background and speckles. Seat geometry comes from `layout.ts`: seat bands reserve space for battlefield rows, zone columns, avatars, and mana. Packing must not cover avatar paint or move cards outside their seat band.

### Resting permanents

`bitmap/mount.ts` paints resting battlefield permanents through `paintCard`. Card faces use `sharedImageCache` with fallback art/name paint when images are not decoded yet. The resting layer skips ids in `hideCardIds` so a flying card is not double-drawn.

Resting permanent chrome includes:

- Base resting outline.
- Commander gold outline.
- Playable border when the object has a current battlefield action.
- Target highlight for staged object targets.
- Combat-damage assign, on-board divide-spell, and divide-counters draft amounts paint a crimson badge on targets (`assignAmounts` / `paintCardAssignAmount`).
- Auto-tap preview glyphs.
- Summoning-sick, keyword, goaded, prepared, owner-strip, P/T, loyalty, counter, and marked-damage badges where the wire exposes those values.

Unplayable permanents are not darkened. Castability and activation availability are represented by playable borders and action affordances.

### Selection

- Permanents that have activatable abilities (including ones that are presently illegal) are selectable so the activation menu can list them with disabled rows.
- Permanents with no activates are not selectable.
- Tap-only mana lands (tap-for-mana only) are selectable so the tap-for-mana row can open.
- Always-on seat/controller borders on every permanent are not used.

### Playable outlines

`chrome.ts` defines battlefield outline colors:

- `CARD_RESTING_OUTLINE = "#1a1a1a"`
- `PLAYABLE_BORDER = "#EAFFF0"`
- `COMMANDER_GOLD = "#E9B84A"`
- `GRAVEYARD_OUTLINE = "#7B5CFF"`
- `EXILE_OUTLINE = "#3DDC97"`

Battlefield playable borders are derived from current `ActionView` data. Tap-only mana lands remain selectable for their tap wedge but do not get a playable border unless they have another action. Commander gold can coexist with a playable border as an outer halo.

### Avatars

Avatars are painted on the Mount bitmap layer (`bitmap/mount.ts` `paintAvatars`) using the same camera transform as cards; `canvas/avatars.ts` keeps a matching vector helper for the Foldkit Canvas pass beneath the Mount layer. The circle is always a face: `PlayerView.gravatar_hash` resolves to `https://www.gravatar.com/avatar/{hash}?s=128&d=404` through `sharedImageCache`, and missing/empty/undecoded images fall back to a seat-color circle with a monogram (`monogramLetter(username, player)`). The priority player uses a gold stroke. Lost players render with muted image overlay or muted fallback fill. Targetable player highlights use Island Blue. This behavior follows the accepted [Gravatar Seat Faces Design](2026-07-25-gravatar-seat-faces-design.md).

Avatar label offsets are locked in `geometry/layout.ts`: `Hand N` paints toward the battlefield (`pos.y - 29 * zoom` for upright seats, mirrored to `pos.y + 29 * zoom` for flipped seats). Life, username, and the clock chips paint on the outer side of the circle (`+48/+66/+80` for upright seats, mirrored to `-48/-66/-80` for flipped seats). HTML hit targets (`life-orb-{seat}`) remain on the circle so combat drops and player-targeting keep the same target.

Alternate lose-the-game and attrition clocks stack below the username as chips (`clockChips(player)`, rows at `pos.y + (offsets.commander + row * 14) * zoom`), in order: `Cmd N` (fill `#db8664`), where `N` is `maxCommanderDamage(player)` — the highest `amount` from any single entry in `PlayerView.commander_damage` (the 21-damage kill clock is per commander source); `Poison N` (fill `#8fd14f`, switching to `#e0574f` at 8 or more, since ten counters eliminate a player per CR 704.5c); and `Rad N` (fill `#e8a33d`). Each chip is omitted when its total is 0 or the field is absent. Lost seats still show their chips. Targetable player highlights use Island Blue.

`restingPaintSnapshot` / `playerPaintKey` includes `gravatar_hash`, `commander_damage`, `poison`, and `rad` so Mount resting repaint runs when only avatar identity or a clock changes (life/hand/username unchanged).

### Arrows and target highlights

`canvas/combatArrowEndpoints.ts` is the shared source of truth for committed and staged combat arrow endpoints. `canvas/arrows.ts` turns those endpoints into Foldkit shapes, and the Mount bitmap layer (`paintBitmapLayer`) paints the same endpoints above resting permanents and avatars. Before every attacked defender has declared blockers, attack arrows stay attacker → defending player avatar or attacked planeswalker card, and block arrows stay blocker → attacker. After every attacked defender has declared blockers, blocked attackers point at their living blockers with attack-red arrows, block-green arrows are suppressed, unblocked attackers still point at their defender, and a blocked attacker with no living blocker paints no combat arrow. Declare-attackers drag aim uses the same arrow layer as committed arrows. Stack target arrows must not rely on the Foldkit Canvas vector pass alone: that canvas sits under the Mount resting-art layer, so arrows painted only there disappear under permanent faces.

### Canvas hex colors

Canvas and bitmap paint use explicit hex and rgba literals rather than Tailwind classes. Important values include:

- Felt base `#0B1310`; felt speckles `#1a2a22`.
- Priority gold `#ffd76a`.
- Attack red `#ff6b6b`; block green `#66ff99`; target blue `#77CCFF`.
- Face-up fallback `#e8e4d8`; face-down fallback `#2a3742`.
- Badge examples: summoning sick `#e8b24a`, goaded `#7a3b13`, prepared `#55cc99`, counters `#2f7d46`, marked damage `#8f2f2f`.
- Avatar commander-damage label `#db8664`.

When badge or outline meaning changes, update [`DESIGN.md`](../../DESIGN.md) and the board legend together.

### Packing and clusters

- Row packing compresses crowded rows inside the seat band.
- Clusters replace indistinguishable groups with one face and a count. The face is the
  lowest-id member.
- A permanent committed to something — declared or staged attacker, declared or staged
  blocker, a blocked attacker, the target of a stack object, or a staged/drafted target —
  never joins a cluster (`board/engagement.ts`). It takes its own slot, so its combat arrow,
  target ring, and hit box are its own, and the cluster face becomes the next free copy.
- Holding Shift on a combat drop commits every copy in the dragged cluster to that defender
  or attacker.

These are visual/layout rules only; they do not collapse engine objects.

## Implementation Decisions

- Keep battlefield cards on the Mount bitmap layer; do not turn every permanent into HTML.
- Paint playable availability with outlines, not unplayable darkening.
- Keep arrows above resting cards so combat and targeting remain legible.
- Keep canvas colors as code literals and sync user-facing meaning through `DESIGN.md`.
- Keep avatar paint below HTML life-orb hit targets.
- Mount `paintAvatars` is the authoritative visible avatar face/life chrome; keep `avatarShapes` in sync for fallback circles and label positions.
- Load Gravatar faces through `sharedImageCache` and use `gravatar_hash` as part of the resting paint key; do not expose email to board paint.
- Show only the max per-commander damage total on the orb (no per-source chip list).

## Testing Decisions

- Canvas scene tests assert felt, seat, avatar, and arrow ordering.
- Avatar unit tests assert Gravatar image paint, monogram fallback, mirrored flipped/upright label offsets, and `Cmd N` paint from `commander_damage` (max source only; omitted at 0) on both Mount `paintAvatars` and the vector `avatarShapes` helper, plus `Poison N` / `Rad N` chips: omitted at 0, stacked on distinct rows, and the poison fill flipping to red inside lethal range.
- Resting-snapshot tests assert `gravatar_hash`-only and `commander_damage`-only player changes invalidate Mount resting paint.
- Bitmap paint tests assert playable, commander, target, auto-tap, P/T, loyalty, counter, and damage chrome on the resting layer.
- Bitmap paint tests assert stack→target arrows paint after resting card art and avatars, and blocked attackers with no living blocker paint no combat arrow after blockers are declared.
- Combat arrow endpoint and Canvas shape tests assert pre-declare arrows, post-declare retargeting, and no-arrow blocked attackers.
- Scene tests assert arrows and interactive life-orb hit targets remain layered correctly.
- Layout tests assert packing, cluster collapse, and that committed permanents split out.

## Out of Scope

- WebGL or worker-based paint.
- Under-card resting name labels.
- New counter kinds not exposed by the wire.
- Changing combat or targeting legality; this spec covers presentation only.
- HTML overlays for commander-damage chips on the orb (breakdown lives in card inspect).

## Further Notes

- Sibling specs: [`2026-07-20-board-composition.md`](2026-07-20-board-composition.md), [`2026-07-20-board-camera-and-layout.md`](2026-07-20-board-camera-and-layout.md), [`2026-07-20-flights.md`](2026-07-20-flights.md), [`2026-07-20-card-inspect.md`](2026-07-20-card-inspect.md) (Alt life-orb commander-damage breakdown).
