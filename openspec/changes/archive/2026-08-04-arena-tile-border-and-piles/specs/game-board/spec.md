## MODIFIED Requirements

### Requirement: Camera, Layout, and Hit Testing

Camera SHALL be pure `{ panX, panY, zoom }` with `screen = world * zoom + pan`. Wheel and two-finger pinch SHALL emit `BoardCameraZoomed` via the camera gesture mount and set `cameraUserMoved` so later sync does not re-fit. `fitCamera` SHALL reserve live hand-bar height and re-fit on cold load, player-count change, and resize until the user moves the camera. `layout` SHALL emit world-space `RenderCard[]` with seat bands from the viewer perspective, packing, and cluster collapse. A permanent at rest SHALL occupy a square footprint; a card in motion — drag ghost or flight — SHALL keep the taller card-shaped footprint, so a played card is card-shaped until it settles. Zone-column piles — library, graveyard, exile, commander — SHALL keep the printed card's proportions, because a pile is a stack of cards rather than a permanent. Hits SHALL resolve against logical layout (topmost wins), not flight poses. DPR-aware canvas backing stores SHALL match the CSS viewport.

#### Scenario: User zoom persists across sync
- **WHEN** the player has panned or zoomed and a game delta arrives
- **THEN** the camera is not re-fitted

#### Scenario: Cluster engagement split
- **WHEN** a permanent is committed as attacker, blocker, blocked attacker, stack target, or staged/drafted target
- **THEN** it takes its own layout slot and the cluster face becomes the next free copy

#### Scenario: Square at rest, card-shaped in flight
- **WHEN** a card is played and its flight settles onto the battlefield
- **THEN** the flight paints at card proportions and the resting permanent paints square

#### Scenario: Four seats stay readable
- **WHEN** the camera fits a four-player board at 1440×900 with the live hand bar
- **THEN** a resting permanent is at least 70 screen pixels on each side

#### Scenario: Piles stay card-shaped
- **WHEN** a seat's zone column lays out its library, graveyard, exile, and commander slots
- **THEN** each pile is taller than it is wide, at the printed card's proportions

### Requirement: Battlefield Paint and Chrome

Battlefield paint order SHALL be felt → seats → resting cards → avatars → arrows → flights. A face-up resting permanent SHALL paint as a rendered card face — the card's art and its name drawn into a real card frame chosen from the card's colours and type — not as a crop of the printed card image. The rendered face's frame SHALL border the tile on all four edges. The rendered face SHALL omit the printed mana cost, because the hand bar's pip tray owns cost, and SHALL omit the printed power/toughness plate, because the live P/T badge already paints over the tile. A token SHALL draw no name and an arched top; a legendary permanent SHALL draw the legend crown. Counters, status badges, the live P/T badge, playable borders, and commander gold SHALL paint over the rendered face. A face-down permanent SHALL paint the card back. Until a face has been rendered the printed card image SHALL paint in its place, and the board SHALL repaint when the face lands. Only the battlefield SHALL take the rendered face: a zone-column pile SHALL paint the printed card image. Playability SHALL use playable borders, not unplayable darkening. Mana-only actions and free-tap lands SHALL NOT receive playable borders but remain selectable. Avatars SHALL paint Gravatar or monogram faces with life, hand count, and clock chips (max commander damage, poison, rad). After every attacked defender has declared blockers, blocked attackers SHALL point at living blockers (attack-red); block-green arrows SHALL be suppressed; blocked attackers with no living blocker SHALL paint no combat arrow. Stack→target arrows SHALL paint on the Mount layer above resting art. Shift on a combat drop SHALL commit every copy in the dragged cluster.

#### Scenario: Mana-only outline skip
- **WHEN** a permanent’s only current action is flagged `mana_only`
- **THEN** it has no playable border but remains selectable for the activation menu

#### Scenario: Post-declare blocked retarget
- **WHEN** blockers are declared and an attacker still has living blockers
- **THEN** the attack arrow points at those blockers, not the defending avatar

#### Scenario: Rendered face replaces the printed image
- **WHEN** a face-up permanent's rendered face is available
- **THEN** the board paints that face and does not paint the card's printed image

#### Scenario: Printed image covers the gap
- **WHEN** a face-up permanent's face has not been rendered yet
- **THEN** the board paints the printed image, and repaints the tile once the face is rendered

#### Scenario: Face-down permanent is a card back
- **WHEN** a permanent is face down
- **THEN** no card face is rendered for it and the card back paints instead

#### Scenario: Border closes around the tile
- **WHEN** a face-up permanent's rendered face is painted
- **THEN** the frame borders its top, both sides, and its bottom

#### Scenario: Piles keep the printed card
- **WHEN** a face-up graveyard, exile, or commander pile paints
- **THEN** no rendered face is requested for it and the printed card image paints
