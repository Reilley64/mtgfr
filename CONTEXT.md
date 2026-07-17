# Domain Glossary

The ubiquitous language for the game engine and protocol. Terms only — no implementation detail. Keep test names and interface vocabulary aligned to these.

## Table & participants
- **Table** — a single game instance that seats 2–4 players. One table = one game = one event stream.
- **Game** — the authoritative state of one match in progress.
- **Player** — a seat at a table, identified by a stable id, with a life total.

## Cards & objects
- **Card** — a data-driven script (identity + rules behavior). Canonical **id** is Scryfall's oracle id; **name** is the printed name for display and search. The north star is to support any card faithfully (ADR 0014); the implemented pool grows from real cards, one at a time.
- **Printing** — a specific Scryfall card object (UUID). An art preference only — which face art to show — not rules identity. Many Printings map to one Card.
- **Default print** — each Card's baked Scryfall-canonical Printing UUID; used when a deck line or object has no other print chosen.
- **Card kind** — what a card fundamentally is: creature, spell (instant/sorcery), or land.
- **Permanent** — a card that exists on the battlefield (e.g. a creature or land).
- **Marked damage** — damage recorded on a permanent this turn; compared against toughness by a state-based action. Removed during cleanup.
- **Tapped** — a permanent turned sideways (e.g. a land that produced mana); the untap step untaps a player's permanents at the start of their turn.

## Abilities & effects
- **Ability** — a unit of card behavior: an **effect** gated by a **timing**.
- **Timing** — when an ability happens: as a **spell** (an instant/sorcery's one-shot effect), **triggered** (e.g. on entering the battlefield), **activated** (by paying a cost), or **static** (continuous).
- **Effect** — a single parametrized game action (e.g. deal damage). The effect vocabulary grows only as real cards demand it — the pool drives the model, not the reverse.
- **Triggered ability** — an ability that fires on an event (e.g. enter-the-battlefield); it's put on the stack the next time a player would receive priority, in **APNAP** order (active player, then non-active).
- **Activated ability** — an ability a player activates by paying its cost (tap and/or mana), of the form "cost: effect".
- **Mana ability** — an activated ability that produces mana and takes no target; it resolves immediately without using the stack (CR 605).
- **Summoning sickness** — a creature can't attack or use tap abilities unless its controller has controlled it since their turn began; **haste** removes this restriction.
- **Land drop** — a player may play at most one land per turn, as a special action (no stack).
- **Spell speed** — when a spell may be cast: **instant** speed (anytime you hold priority) or **sorcery** speed (your main phase, empty stack, your turn). Creatures are sorcery-speed.
- **Target / target spec** — what an ability points at (e.g. a creature on the battlefield); legality is checked when the spell/ability goes on the stack.
- **Keyword** — a named, reusable ability shorthand (e.g. flying, vigilance).
- **Populate** — a keyword action (CR 701.32): create a token that's a copy of a creature token you control. Modeled as a token-copy effect over a "creature token you control" choice; with no such token, it does nothing.
- **Effective power/toughness** — a creature's P/T computed on demand and additively (ADR 0003, no CR 613 layers): base + counters + until-EOT pumps + anthems + attachment grants. A **set-base** effect (Darksteel Mutation) replaces the printed base with a fixed value, then those additive modifiers still apply on top.

## Choices
- **Choice / pending choice** — a decision the engine pauses for. Represented as data; the awaited player answers with an intent, resuming the engine. See ADR 0004. While a choice is pending, no other action is legal. Kinds include: **order triggers**, **choose target**, **may** / **pay cost** (optional abilities), **assign combat damage**, **arrange top** (scry/surveil), **search library**, **choose mode**, **discard**, **sacrifice edict**, and more — see `PendingChoice` in `crates/engine/src/types.rs`.
- **Optional ability** — a triggered ability its controller *may* decline ("you may …"), possibly by paying a cost. Modeled as a flag on the ability.
- **Legal targets** — the objects an action may legally target right now per its `TargetSpec`. The engine enumerates them; the client highlights the same set.

## Mana
- **Mana** — the resource spent to cast spells, produced in five **colors** (WUBRG).
- **Land** — a permanent that taps for one mana of its color.
- **Cost** — generic mana plus colored pips required to cast a spell or activate an ability.
- **Mana pool** — a player's available mana; it empties when a step or phase ends. Credits are typed (WUBRG, colorless, any, either, restricted), not a single total.
- **Mana tray** — per-seat display of that player's non-empty **mana pool**, anchored just outside the seat band under the zone column's battlefield-side edge (flips with top-row seats), drawn with mana-font glyphs.

## Turn structure
- **Turn** — one player's sequence of steps; the **active player** is whose turn it is.
- **Step** — a slot within a turn: untap, upkeep, draw, main 1, the five combat steps (begin combat, declare attackers, declare blockers, combat damage, end combat), main 2, end, cleanup. Every step grants priority except untap and cleanup.
- **Turn-based action** — an automatic action as a step begins (untap permanents, draw a card, cleanup discard) — not on the stack.

## Zones
- **Zone** — a place a card can be: library, hand, battlefield, graveyard, exile, command, stack.
- **Library** — a player's deck, ordered; drawn from the top. Trying to draw from an empty library loses the game (a state-based action).

## The sequential model
- **Cast** — moving a spell from hand onto the stack, choosing targets and paying costs.
- **Stack** — the zone holding spells/abilities waiting to resolve; last-in, first-out.
- **Resolve** — applying the topmost stack object's effect, then removing it from the stack.
- **Priority** — the right to act. The active player holds priority; players act or pass.
- **Priority window** — a point where players may act; when all players pass in succession, the top of the stack resolves.
- **Meaningful action** — a play worth stopping priority for: a land to play, an affordable and legal spell, a non-mana activated ability, or a combat declaration. Bare tap-for-mana doesn't count. A player with none can be safely skipped.
- **Auto-pass** — the server passing priority on a player's behalf through windows where they have no meaningful action, so priority advances without a manual Pass each time. The engine stays intent-only; the server just submits the passes (see ADR 0007). Distinct from **stack yield**.
- **State-based action (SBA)** — an automatic check applied between resolutions (e.g. a creature with `marked_damage >= toughness` is moved to the graveyard). Never uses the stack.

## Event sourcing & protocol
- **Intent** — a player's requested action (client → server input to the engine).
- **Event** — a canonical, full-information record of something that happened. Events mutate game state (objects, the stack, mana); priority/pass bookkeeping and pending choices are orchestration state and live in the submit path (see `apply.rs:106–109`), and library order is deliberately not event-sourced. The engine emits events; it is audience-unaware.
- **Delta** — a batch of events sent to one player over the wire, each carrying a **sequence number (`seq`)** (the monotonic ordering/resume watermark) *and* the viewer's full render state after those events applied. Self-sufficient: the client folds it in place — replace the board from the state, grow the log from the events — with no snapshot refetch (see ADR 0006).
- **Game log** — the running narration a client builds by folding the delta event stream (joining object ids to names). Every delta is already a loggable event, so the log is nearly free.
- **VisibleEvent** — an event after per-viewer redaction (private information removed for players who may not see it).
- **Redaction / Projection** — mapping a canonical `Event` to a `VisibleEvent` for a specific viewer. Lives outside the engine (in the schema/wire layer), never inside it.
- **Snapshot** — a full redacted view of the game for one viewer at a given `seq` (`VisibleState`): turn state, per-player facts, and the objects that viewer may see. A client renders from a snapshot, then applies deltas. Because game setup emits no events, the opening board arrives as a snapshot, not a delta replay. See ADR 0005.

## Table & networking
- **Table** — one game and its streaming plumbing; the unit clients connect to. Held in an in-process **table registry** keyed by `table_id` (multiple concurrent tables; single instance, no Redis).
- **Lobby** — a table's pre-game phase: up-to-four claimable seats, each choosing a **deck**, that the **host** (first to join) starts once ≥2 seats are claimed and all are ready.
- **Seat claim / ready-up** — a signed-in user claims the next open seat (bound to their **account**) and toggles a ready flag; the game is seeded only when the host starts.
- **Account** — a user, identified by email + password. Seat ownership and deck ownership are keyed by the account, not a browser.
- **Session** — a signed-in user's authenticated context: an HttpOnly cookie bound to a server-side session record. Every request carries it; the server resolves it to the account (replacing the old anonymous browser token).
- **Seat** — a player's position at a table (0–3); assigned when a browser claims a seat in the lobby.
- **Spectator** — an eliminated player still receiving the stream (the game continues without them).
- **Stack view** — the ordered, renderable list of what's on the stack (spells and abilities, with labels and targets). Default presentation is a right-edge physical pile; when reading room is needed it can open as an **Expanded stack view**.
- **Stack peek compression** — shrinking the vertical peek between objects in the physical **Stack view** so the pile still fits the usable screen band (same overflow idea as battlefield **row packing**, for the stack).
- **Stack expand control** — count + magnifier on the stack presentation (not in the **priority context bar**) that opens the **Expanded stack view**. Appears once stack size hits a reading threshold or once **stack peek compression** starts, whichever comes first.
- **Expanded stack view** — horizontal left→right layout of the stack that replaces the physical pile while open (left = bottom of stack, right = top / next to resolve). Dismissed explicitly, or auto-collapsed when both expand thresholds clear; always clears when the stack empties. Not a **Pile**. While open, arrow targeting from a **staged stack card** is suspended and resumes when expand closes.
- **Full stack view** — denser center-of-table layout used when even the **Expanded stack view** cannot fit the stack (MTGA’s “really big stack” mode). Same order as the strip (left = bottom, right = top), as a centered horizontal spread that compresses/overlaps to fit; wraps to another row only if a single band still cannot fit. Still the **Stack view**, not a **Pile**. Arrow targeting from a **staged stack card** stays suspended here too, and resumes when returning to the physical pile.
- **Staged stack card** — local preview of a hand card awaiting a target, shown as the visual top for the caster (pile top, or rightmost in **Expanded stack view**); the targeting arrow aims from that card in the physical pile (not yet on the engine stack).
- **Stack hold** — server pause before the auto-pass that would resolve the top of an uncontested stack, so the table can read the card.
- **Helpless dwell** — a seat with no meaningful response hovering the stack during a hold, which may postpone resolution up to a hard cap (ADR 0026).
- **Stack Pass** — one-shot `pass_priority` while you can act on a non-empty stack (Space/Enter and the stack **Pass** button).
- **Stack yield** — per-seat flag: the server auto-passes that seat for the rest of the current stack; clears when the stack empties. Armed once from the **priority context bar** (no cancel control there). Not the same as server **auto-pass** for helpless seats (ADR 0007 / 0027) or **turn yield**.
- **Turn yield** — per-seat standing toggle: the server auto-passes this seat through every priority window until that seat becomes the **active player** again (start of their turn), until this seat is **attacked** (an attacker declared at them — not at another player), or until this seat takes an intentional action (cast, activate, manual **Stack Pass** / **Next**, etc.); then it clears. Being attacked also makes empty-stack instants meaningful for that seat's Declare Attackers priority so they can respond before blockers when they have something to cast; helpless defenders auto-pass through to blockers. Independent of **stack yield**. Distinct from server **auto-pass** for helpless seats.
- **Instant-priority focus** — client dimming of battlefield permanents you cannot use while you hold priority in an instant-speed window (stack up, not your turn, or a non-main / non-declare step). Empty-stack main and declare attackers/blockers stay fully bright; the hand still dims cards with no cast action. Untapped mana sources stay bright (mana abilities are not on the wire action list).
- **Priority context bar** — the primary board control cluster for advancing or yielding priority: **Next** / combat confirms / **Stack Pass**, plus **stack yield**, **turn yield**, and related context items. Always docked bottom-right. In paint order it sits above the **Stack view** (even when the stack is empty / that layer is vacant); prompt forms sit above this bar. Not stack-only chrome — empty-stack **Next** lives here too.
- **Pile** — a stacked zone rendered as one card standing in for the whole zone (graveyard/exile), showing the top card and a count; clicking it expands the full contents.
- **Combat view** — each declared attacker paired with the player it's attacking, plus the blocks, surfaced on the wire so every client draws the right arrows and each defender sees who's coming.
- **Player avatar** — a life orb per seat in board world space (pans/zooms with the table): life inside the circle; priority and target rings on the rim; hand count above; username below, with commander damage when any is present. Drop target for attacking or targeting a player. Not a seat HUD — **library** count lives on the Library pile; mana lives on the **mana tray**. Labels scale with camera zoom (no screen-px floor).
- **Seat band** — one seat's outlined area on the Commander table (zone column + battlefield rows + outer avatar). Seats sit in a 2×2 quadrant (viewer bottom-left; others in turn order at front, side, then diagonal), outlined in the seat's color.
- **Battlefield row** — one horizontal lane inside a seat band. Every seat always has three, centerward → outer: **Noncreature row**, **Creatures row**, **Lands row**.
- **Noncreature row** — unattached artifacts, enchantments, and planeswalkers.
- **Creatures row** — creatures (including dual-type creatures whose wire kind is creature).
- **Lands row** — lands.
- **Row packing** — when a battlefield row has more permanents than fit at full spacing, that row alone compresses horizontal spacing so its cards stay inside the seat band (MTGA-style overlap / peek). Prefer a comfortable step, but keep compressing below it if needed — even to near-total overlap. Other rows are unaffected; the seat does not widen and cards do not spill into a neighbour. Hovering a packed card **hover-raises** it (paint + hit) so buried distinct permanents stay reachable.
- **Hover raise** — bringing the hovered battlefield card above its packed neighbours for paint and hit-testing; does not change layout positions.
- **Permanent cluster** — several identical permanents in one battlefield row rendered as a single face with a count. Used only when the row would otherwise overflow full spacing; if the row still fits, every permanent stays its own card. Once the row overflows, every eligible identical group in that row collapses (not a minimum subset); if still over capacity, **row packing** compresses what’s left. Identity is “indistinguishable on what the table shows” except object identity, and only among permanents with no **attachment stack**. Pointer hover fans members in an arc (so the collapsed face is not a click target); touch hold fans members; a short touch tap on the collapsed face selects a stable top member for **permanent selection** / **activation radial**. Clicking a fanned member selects that member — the radial centers on it and it stays raised until deselected (which also holds the fan open). Fan and **hover raise** apply on every seat, including while targeting. Not a **Pile** (zone) and not an **Attachment stack**.
- **Attachment stack** — attached Auras/Equipment rendered on their host (offset under the host card), not in a battlefield row.
- **Play threshold** — the screen height a hand card must be dragged above to play/cast it (below it snaps back to hand).
- **Card inspect** — deliberate reading of one card: Alt pins it into the inspect dock; Esc or releasing Alt leaves.
- **Inspect dock** — left-edge panel showing the inspected card face (large art, rules text, and a flip control when a back face exists).
- **Play face** — which face the inspect dock opens on: the back when the inspected permanent is prepared, otherwise the front.
- **Prepared** — battlefield status on a prepare double-faced card: its controller may cast a copy of its back-face spell; casting clears prepared.
- **Permanent selection** — the viewer's focused permanent for acting; click selects one of yours on the battlefield.
- **Activation radial** — pie of legal activates (including tap-for-mana) around the selected permanent.

## Combat & Commander
- **Attacker / blocker** — a creature declared attacking a chosen **defending player**, and a creature declared to block it; combat damage is dealt in the combat-damage step.
- **Defending player** — the player a given attacker is attacking; each attacker picks its own (you may split an attack across several opponents). Each attacked player declares their own blocks, in APNAP (turn) order.
- **APNAP** — active player, then each non-active player in turn order: the order simultaneous triggers go on the stack and the order attacked players declare blocks.
- **Elimination** — a player who has lost leaves the game: their owned objects leave all zones and they drop from turn/priority rotation; play continues until one player remains (the **winner**).
- **Commander** — a designated legendary creature that starts in the command zone.
- **Color identity** — the colors in a card's cost and rules text; a Commander deck's cards must fall within its commander's identity.
- **Commander damage** — combat damage tracked per commander source; 21 from one commander loses the game.
- **Command-zone cast tax** — casting a commander from the command zone costs an extra {2} per previous such cast.
- **Replacement effect** — a rule that replaces one event with another before it happens (e.g. a commander that would leave may go to the command zone instead).

## Accounts & decks
- **Deck** — a user-authored, persisted list: a name, a **commander** (Card id), and 99 cards as `(id, count, print)` with **print** required on every line. Owned by an account; a lobby seat plays one of the owner's decks (replacing the old fixed precon choice).
- **Legendary** — a card supertype; only a legendary creature may be a deck's commander.
- **Commander legality** — the rules a deck must satisfy to be saved or played: exactly one legendary-creature commander, 99 other cards, singleton except basic lands, and every card's **color identity** within the commander's.
- **Deck builder** — the screen for assembling a deck from the pool: browse the **card catalog**, pick a commander, add cards, choose **Printings**, save (the server validates legality and returns every problem at once).
- **Card catalog** — the pool exposed for browsing, carrying each card's engine-true stats, keywords, and a plain-English ability summary (not Scryfall oracle text, which wouldn't match a simplified card).
