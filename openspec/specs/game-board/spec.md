# game-board Specification

## Purpose

The in-game board is the Foldkit Canvas + Mount + HTML Commander table: lobby entry into a seated game, camera/layout, battlefield paint, hand/stack/prompts/priority chrome, local action sessions, flights, overlays, audio, and the event log — composed so four seats stay readable without a single DOM battlefield or a client rules engine.
## Requirements
### Requirement: Lobby Entry and Seated Pregame

Play routes SHALL use path-param deck and table ids (`/play/:deckId`, `/play/:deckId/:table`, `/play/:table`). Entry SHALL be Layout C: selected deck card left; Host primary; soft-inline Join code + ghost Join; ghost Back — no deck `<select>`, Bringing strip, or choose→join mode switch. Seated lobby SHALL poll until `started`, show seat-color dots and Gravatar/monogram faces (public `gravatar_hash` only), Ready/Start, table-code copy with clipboard fallback, and watcher copy for unsigned seats. Ready SHALL unlock table audio. Pregame play-route entry with a deck SHALL fire-and-forget `WarmDeckArt` at `fetchPriority: "low"`. Host create→redirect SHALL NOT flash claim-seat chrome on the entry route. On start, seated pregame URLs SHALL replace with `/play/:table` preserving the table id.

#### Scenario: Entry without selected deck
- **WHEN** the player has no selected deck on entry
- **THEN** amber empty copy points them back to Your decks

#### Scenario: Stale table link
- **WHEN** lobby returns `UnknownTable`
- **THEN** the UI shows stale-link copy asking for a fresh host code

#### Scenario: Table-only clears deck
- **WHEN** the player opens `/play/:table`
- **THEN** prior `selectedDeckId` is cleared so claim-seat chrome cannot reuse a stale deck

### Requirement: Board Composition and Viewers

The live board SHALL compose Foldkit Canvas (vector furniture/arrows), Mount bitmap (resting cards, avatar face/life, flights), and HTML overlays (hand, stack, prompts, priority, mana, log, inspect, system). Layer order authority SHALL be `docs/CLIENT_CANVAS_MAP.md`. The root SHALL be `board-mount` (`select-none`, overflow-hidden). Spectators and eliminated players SHALL see a read-only board without hand or action controls. An `aria-live` status summary SHALL describe board state. Reconnect banner SHALL appear when the stream is disconnected. Quiet HUD close controls SHALL use enlarged `.hit-quiet` hit targets for coarse pointers. Portrait phones SHALL use app-root CSS landscape rotate; the board SHALL NOT vertical-reflow.

#### Scenario: Connecting state
- **WHEN** no `VisibleState` is available yet
- **THEN** `board-connecting` shows inside `board-mount`

#### Scenario: Spectator suppression
- **WHEN** the viewer is a spectator
- **THEN** hand bar and action affordances are absent

### Requirement: Camera, Layout, and Hit Testing

Camera SHALL be pure `{ panX, panY, zoom }` with `screen = world * zoom + pan`. Wheel and two-finger pinch SHALL emit `BoardCameraZoomed` via the camera gesture mount and set `cameraUserMoved` so later sync does not re-fit. `fitCamera` SHALL reserve live hand-bar height and re-fit on cold load, player-count change, and resize until the user moves the camera. `layout` SHALL emit world-space `RenderCard[]` with seat bands from the viewer perspective, packing, and cluster collapse. A permanent at rest SHALL occupy a square footprint; a card in motion — drag ghost or flight — SHALL keep the taller card-shaped footprint, so a played card is card-shaped until it settles. Hits SHALL resolve against logical layout (topmost wins), not flight poses. DPR-aware canvas backing stores SHALL match the CSS viewport.

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

### Requirement: Battlefield Paint and Chrome

Battlefield paint order SHALL be felt → seats → resting cards → avatars → arrows → flights. A face-up resting permanent SHALL paint as a rendered card face — the card's art and its name drawn into a real card frame chosen from the card's colours and type — not as a crop of the printed card image. The rendered face SHALL omit the printed mana cost, because the hand bar's pip tray owns cost, and SHALL omit the printed power/toughness plate, because the live P/T badge already paints over the tile. A token SHALL draw no name and an arched top; a legendary permanent SHALL draw the legend crown. Counters, status badges, the live P/T badge, playable borders, and commander gold SHALL paint over the rendered face. A face-down permanent SHALL paint the card back. Until a face has been rendered the printed card image SHALL paint in its place, and the board SHALL repaint when the face lands. Playability SHALL use playable borders, not unplayable darkening. Mana-only actions and free-tap lands SHALL NOT receive playable borders but remain selectable. Avatars SHALL paint Gravatar or monogram faces with life, hand count, and clock chips (max commander damage, poison, rad). After every attacked defender has declared blockers, blocked attackers SHALL point at living blockers (attack-red); block-green arrows SHALL be suppressed; blocked attackers with no living blocker SHALL paint no combat arrow. Stack→target arrows SHALL paint on the Mount layer above resting art. Shift on a combat drop SHALL commit every copy in the dragged cluster.

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

### Requirement: Hand and Zone Bar

Active seated players SHALL see a bottom DOM bar in Arena order: command, hand, graveyard, exile. The bar SHALL scale with `handMetrics` / `handUiScale` (clamped). Playable hand/command tiles SHALL get playable borders; unplayable tiles SHALL stay full brightness. Drag-to-play above the play threshold SHALL commit; below SHALL snap back. Multi-legal-mode activation SHALL park in `playModePick` with docked coach and primary-bar mode buttons; single mode SHALL run immediately. Drag ghost SHALL paint on the Mount screen-motion layer (not HTML). Spectators and eliminated players SHALL NOT see the hand bar. Pick chrome SHALL use `data-selected` / `data-selectable` group variants.

#### Scenario: Multi-mode hand play
- **WHEN** a hand tile has two or more legal modes
- **THEN** activation opens `play-mode-aim` and does not submit until a mode is chosen or Cancel restores the card

#### Scenario: Drag commit hides source
- **WHEN** a playable hand card is drag-played past the threshold
- **THEN** the hand tile hides while the flight owns the card

### Requirement: Stack Overlay

The stack SHALL be a right-edge DOM overlay with pile / expanded strip / full-grid presentations. Labels SHALL format wire `MessageRef`s. Declared targets SHALL paint one Island Blue arrow per resolvable destination on the Mount layer. Legal aim faces SHALL set `data-legal-target` and submit on click/keyboard. Priority holders hovering a non-empty stack SHALL emit `SetStackDwell`. Resting stack faces SHALL hide only for `kind: "stack"` flights. Pending board-aim without a stack entry for the source SHALL show a source-art ghost; spell sources already on the stack SHALL NOT duplicate.

#### Scenario: Multi-target caption
- **WHEN** a stack object has multiple resolved targets
- **THEN** the caption lists all labels joined with `, ` after ` → `

#### Scenario: Ability face during source flight
- **WHEN** a battlefield flight owns an ability’s source permanent
- **THEN** the ability’s stack face remains visible

### Requirement: Screen Motion

Drag ghosts, `CardFlight`s, and battlefield `ExitFx` SHALL share one Mount flight-layer paint pass. Flights SHALL spawn from authorized local seeds or sync provenance, retarget to authoritative poses, hold local hand seeds until provenance, and settle without duplicate resting faces (`hideCardIds`, `handHidden`, owned ids). Battlefield→graveyard/exile SHALL use in-place ExitFx (destroy/exile), not a zone glide. Rejected intents and Cancel SHALL drop held seeds. Reduced motion SHALL snap flights and complete ExitFx immediately. Pose-only ticks SHALL repaint only the flight layer. Lift shadow on drag ghosts and flights SHALL match the shared lift-shadow tokens.

#### Scenario: Continuous drag-to-flight
- **WHEN** a hand drag releases into a seeded flight
- **THEN** the canvas ghost and flight share pose, scale, and lift shadow without an HTML ghost jump

#### Scenario: Battlefield destroy exit
- **WHEN** provenance marks a permanent leaving the battlefield for the graveyard
- **THEN** ExitFx destroy choreography runs at the last battlefield pose and suppresses the generic glide

### Requirement: Mana Tray

In-play floating mana SHALL render as a world-projected, pointer-events-none DOM tray under resting permanents. Spell/payment mana chrome SHALL stay with prompt/hand layers, not the in-play tray.

#### Scenario: Empty pool omits tray
- **WHEN** no seat has floating mana chips to project
- **THEN** the mana tray is omitted

### Requirement: Activation Menu

Selecting a battlefield permanent with options SHALL open a card-anchored DOM activation menu (scrim, arm-on-press / commit-on-release, keyboard commit). Options SHALL include tap-for-mana plus battlefield `ActionView`s; cluster faces SHALL aggregate member actions into one row per label with `×k` when multiple copies remain. `mana_only` rows SHALL list normally. Payment SHALL remain engine-side with `auto_tap` preview only. Empty option lists SHALL render nothing.

#### Scenario: Arm then slide off
- **WHEN** the player presses a row then releases on a different row or the scrim
- **THEN** that press does not commit the original row

#### Scenario: Cluster availability chip
- **WHEN** three identical copies can still activate the same ability
- **THEN** the row shows `×3` and commits against a copy that still has the action

### Requirement: Card Inspect

Alt/Option SHALL pin a face-up hand, stack, or battlefield card into the topmost dock inspect overlay (backdrop blocks board clicks); Alt over a life orb SHALL pin a text-only player dock with life and per-commander damage breakdown. Dismissal SHALL be Alt release, Escape, or backdrop click. Battlefield pins SHALL show modifier ledger and marked damage when present. Space SHALL be blocked while the dock is open.

#### Scenario: Hand aux preferred
- **WHEN** Alt is held over a hand tile that also overlaps battlefield geometry
- **THEN** the hand/stack aux hover wins the pin

#### Scenario: Life-orb commander breakdown
- **WHEN** Alt pins a seat with commander-damage rows
- **THEN** the dock lists each source as amount / 21 while orb paint stays max-only `Cmd N`

### Requirement: Prompts and Primary-Bar Takeover

`promptPresentation` SHALL classify local sessions then the viewer’s `pending_choice` as `none`, `simple`, or `modal`. Simple prompts SHALL put answer buttons in `priority-context-bar` with informational bottom coaches; modal prompts SHALL use centered shells and hide idle priority controls. Engine prompts SHALL render only for the awaited active seated player; others SHALL see `pending-choice-waiting`. Submissions SHALL go through `choiceIntent` / formulators. Board-aim, hand/GY/exile pile aim, steppers, and lane arrange UIs SHALL cover the shipped choice kinds; uncategorized kinds SHALL fall back to modal. Pending-choice prompt frames SHALL NOT use dismissible `Dialog`.

#### Scenario: Non-decider waiting
- **WHEN** another seat has `pending_choice`
- **THEN** non-deciders and spectators see `Waiting for {name}…` without interactive prompt buttons

#### Scenario: Simple yes/no takeover
- **WHEN** the awaited player faces `may_yes_no`
- **THEN** Yes/No live in `priority-context-bar` and the coach stays button-free

#### Scenario: On-board damage assign
- **WHEN** every `assign_combat_damage` blocker is on the battlefield
- **THEN** clicks move 1 damage onto blockers and Assign confirms from the primary bar

### Requirement: Turn and Priority Chrome

`PriorityContextBar` SHALL show Next / Resolve card / Resolve stack / combat confirm as appropriate; End Turn and Until my turn SHALL be rockers keyed by `aria-checked`. Any classified prompt SHALL suppress idle priority actions. Mulligan overlay SHALL lock the opening hand until Keep/Mulligan; after local keep, waiting copy SHALL name undecided seats. On first mulliganing fold, a one-shot `first-player-reveal` spotlight SHALL name the CR 103.1 starter (sessionStorage per table; reduced motion skips hop). Turn banner phases SHALL use `data-phase-state` / `data-your-turn`. Discoverability SHALL include auto-hiding hint strip, legend, and combat coach during declare windows. Sound toggle SHALL sit top-left for all viewers.

#### Scenario: Prompt hides idle Next
- **WHEN** a simple or modal prompt is classified
- **THEN** idle Next/Resolve/End Turn/Until my turn controls are suppressed

#### Scenario: First-player reveal one-shot
- **WHEN** mulligans begin on a table already marked seen in sessionStorage
- **THEN** the spotlight does not replay and mulligan controls remain reachable

### Requirement: Action Session and Targeting

Local action sessions SHALL stage X, modes, cost picks, targets, and combat declarations via pure planners; the engine SHALL remain authoritative for payment and legality. Targeting SHALL use engine-projected legal targets (arrows on-board; pickers off-board). Combat staging SHALL drop onto life orbs / planeswalkers / attackers; required attacks SHALL merge before confirm; Shift SHALL stage whole clusters. `CancelActionClicked` / Escape SHALL clear local sessions without answering engine `pending_choice`. Auto-tap preview SHALL prefer in-flight session actions over hover.

#### Scenario: Local cancel vs engine choice
- **WHEN** the player presses Escape during a local staged cast
- **THEN** local staging clears and no engine pending choice is answered

#### Scenario: Shift-drop cluster
- **WHEN** the player Shift-drops a five-copy cluster onto a defender
- **THEN** all five copies stage as attackers against that defender

### Requirement: System Overlays

Result and concede confirmations SHALL use shared `modalDialog` / `confirmDialog`. Result SHALL raise once per outcome and stay down after dismiss. Concede SHALL submit only after confirm. `PileOverlay` SHALL expand non-battlefield piles with optional selectable thumbs. Looked-at opponent hands SHALL offer `seen-hand-*` chips into the pile overlay. Reconnect banner SHALL distinguish transient loss from terminal 401/404. Inspect SHALL sit above HUD overlays; open native dialogs may layer above inspect.

#### Scenario: Result stays dismissed
- **WHEN** the viewer dismisses the result overlay
- **THEN** later folds do not raise it again

#### Scenario: Seen hand chip
- **WHEN** the viewer’s snapshot itemized an opponent’s looked-at hand
- **THEN** a `seen-hand-<seat>` chip opens that hand in `PileOverlay`

### Requirement: Table Audio

Table audio SHALL be synthesized Web Audio cues unlocked on lobby Ready (happy path) and recoverable via Sound-on. Mute preference SHALL persist in `localStorage` (`mtgfr.sound`). Attention and table-feel cues SHALL fire from board `data-*` attributes; destroy/exile feels SHALL skip under reduced motion. Muted or suspended contexts SHALL no-op silently.

#### Scenario: Ready unlocks audio
- **WHEN** the player presses Ready in the lobby
- **THEN** `unlockTableAudio` runs synchronously on that gesture path

### Requirement: Board Log Panel

When the fold log is non-empty, a DOM log panel above the hand bar SHALL show the last 30 lines collapsed (expandable to the in-memory cap of 200), support copy of the full buffer, and mark `auto: true` lines with an AUTO chip. Snapshots SHALL NOT clear the log; only deltas append. Spectators SHALL see the same public narration.

#### Scenario: Collapsed window
- **WHEN** the log has more than 30 retained lines and is collapsed
- **THEN** only the newest 30 paint, with expand revealing older retained lines

