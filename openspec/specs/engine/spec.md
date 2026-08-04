# engine Specification

## Purpose

Authoritative, pure, deterministic rules engine for 2–4 player free-for-all Commander. The engine owns one match as a sequential stack-and-priority state machine: validated player intents produce an event stream that mutates board facts; state-based actions and triggered abilities run automatically after each intent; legal actions and pending choices are exposed so clients need not re-implement rules. Live games are in-memory only; the event log is an audit trail, not a replay harness.

## Requirements

### Requirement: Pure Deterministic Engine
The engine MUST be pure: no I/O, no wall-clock time, no external randomness, and no async. Given the same master seed and the same sequence of accepted intents, two runs MUST produce identical board outcomes and event streams. Randomness MUST come solely from an injected 32-byte master seed. Each logical random operation MUST derive an isolated stream from `BLAKE3(master_seed || player || op_iteration)` (splitmix64), with unbiased index selection. Controller-scoped card effects MUST attribute random ops to that controller. The starting-player roll MUST be a game-level op on seat 0's counter and MUST precede library shuffles and opening hands when used on the production seeding path.

#### Scenario: Same seed and intents reproduce
- **WHEN** two games are constructed with the same master seed and receive the same accepted intents in order
- **THEN** both runs produce the same events and board facts

#### Scenario: Injected seed only
- **WHEN** a game needs randomness (shuffle, random pick, starting player)
- **THEN** the engine consumes only the injected master seed and per-seat op counters — never wall-clock or OS entropy

### Requirement: Intent Submit Path
All mutation of an authoritative match MUST flow through a single submit entry point that validates an intent, applies resulting events, runs the fixed post-intent pipeline, and returns either the produced events or a typed reject. Invalid intents (wrong player, wrong timing, unknown object, wrong choice answer, illegal declaration) MUST be rejected without mutating board facts. Out-of-range object ids MUST be rejected at the submit gate.

#### Scenario: Valid intent yields events and pipeline
- **WHEN** a living player submits a legal intent
- **THEN** the engine returns the events produced and has run the post-intent pipeline before returning

#### Scenario: Invalid intent is typed reject
- **WHEN** a submit references an unknown object, answers the wrong choice, or is otherwise illegal
- **THEN** the engine returns a typed reject and board facts are unchanged

### Requirement: Game Construction and Seats
The engine MUST support 2–4 seats. Each seat MUST start at 40 life with empty zones. Construction with a 32-byte master seed is the production path; a u64 compatibility constructor MAY expand into the first eight bytes of a master seed for tests. Raw constructors MAY park active player and priority at seat 0; production seeding MUST call starting-player selection before stacking libraries or dealing hands. Test helpers MAY fund mana and spawn objects without a network.

#### Scenario: Commander starting life
- **WHEN** a game is constructed with n seats (2–4)
- **THEN** each seat has 40 life and empty zones

#### Scenario: Starting player on production seed path
- **WHEN** production seeding constructs a game and selects the starting player
- **THEN** active player and priority are set to the rolled seat before any library or hand deal, and that roll is the game's first random operation

### Requirement: Zones and Object Identity
The engine MUST represent the seven MTG zones and a flat object arena addressed by object id. Every card, spell, and permanent MUST be an object typed by zone (card / spell / permanent). An object MUST receive a new object id on each zone change; old slots MUST become followable tombstones. Objects that have left the game (eliminated owner's cards) MUST become removed sentinels that are illegal to access. Live objects MUST store printed identity as an interned card-id handle rather than embedding a full printed definition.

#### Scenario: Zone change mints new id
- **WHEN** an object moves between zones
- **THEN** it receives a new object id and the old id resolves to the current id via tombstone chaining

#### Scenario: Removed objects are illegal inputs
- **WHEN** an intent or rules path references a removed object
- **THEN** that reference is treated as illegal (not a normal board read)

### Requirement: Event-Sourced Board Facts vs Orchestration State
Board facts (life, zones, counters, tap, damage marks, mana pools, stack contents) MUST mutate only via events applied by direct pattern-matched handlers. Priority holder, consecutive passes, pending choice, deferred resume frames, keyword obligations, resolution-finish policy, and similar orchestration MUST live as plain fields on the game and MUST NOT be reconstituted from the event log alone. Library order MUST NOT be fully event-sourced (shuffles/draws mutate the library directly) so other players never observe order through events. The event log MUST be treated as audit-only; intent replay of a full match from events alone is out of scope.

#### Scenario: Events mutate board facts
- **WHEN** an event such as life change, zone move, or mana spend is applied
- **THEN** the corresponding board fact changes and no non-event path mutates that fact

#### Scenario: Priority is not in the event log
- **WHEN** priority passes or a pending choice is raised
- **THEN** those fields update on the game directly without requiring an event to store them

### Requirement: Pre-Game Mulligans and Opening Hands
Real setup MUST stack each library, deal opening hands with 2-sample BO1 land smoothing (closest land count to deck expectation; ties keep the first sample), then enter a simultaneous mulligan phase. During mulligans, undecided living seats MAY keep or mulligan; ordinary game actions MUST be blocked until every living seat has kept. Friendly mulligan: first mulligan redraws to 7; later mulligans draw to 6, 5, …, 1. There is no London bottoming or Vancouver scry. Mulligan redraws MUST also be land-smoothed. A seat at hand size 1 MUST auto-keep after redraw. When all living seats have kept, the engine MUST clear the mulligan phase and begin the first turn. First-turn beginning steps (Untap → Upkeep → Draw) MUST run through the post-intent pipeline. In two-player games the starting player MUST skip their first draw; in 3–4 player games no player skips.

#### Scenario: Simultaneous mulligan gate
- **WHEN** the game is in the mulligan phase
- **THEN** only keep/mulligan answers from undecided living seats are accepted, and the first turn does not begin until every living seat has kept

#### Scenario: Friendly mulligan sizes
- **WHEN** a player takes their first mulligan
- **THEN** they redraw to seven cards; subsequent mulligans redraw to one fewer card each time down to one

#### Scenario: Two-player first-draw skip
- **WHEN** a two-player game begins the first turn
- **THEN** the starting player skips the first draw step

### Requirement: Post-Intent Pipeline
After every successful submit (and after beginning the first turn), the engine MUST run a fixed ordered pipeline: state-based actions to fixpoint; priority handoff if the priority holder was eliminated; enqueue triggered abilities from just-produced events; fire delayed / next-cast / combat-damage watch triggers; place pending triggers on the stack in APNAP order (then drain keyword obligations Echo, then Recover, then Cumulative upkeep); refresh every living seat's legal-action list. Pipeline phase order MUST remain stable and rules-ordered.

#### Scenario: Pipeline order after submit
- **WHEN** an intent is accepted
- **THEN** SBAs run to fixpoint before triggers are enqueued, and triggers are placed before legal actions are refreshed

#### Scenario: Keyword obligations after ordinary triggers
- **WHEN** ordinary pending trigger groups are exhausted and obligations remain
- **THEN** Echo obligations are raised before Recover before Cumulative upkeep, FIFO within each kind

### Requirement: State-Based Actions
The engine MUST check state-based actions to a fixpoint after intents, including at least: lethal damage / deathtouch death (unless indestructible); toughness ≤ 0 death (indestructible does not save); regeneration shields replacing destroy SBAs (not 0-toughness); planeswalker loyalty ≤ 0; Aura falls off illegal/missing host (token Auras cease); Equipment detaches from illegal host; +1/+1 and −1/−1 counter annihilation before death checks in the same scan; legend rule with a keep-one choice (one conflict group per sweep, lowest seat then name); life ≤ 0 loss; empty-library draw loss; ten or more poison counters loss; lethal commander damage (21 from one commander source) loss. A regeneration shield MUST NOT save a creature from 0-toughness.

#### Scenario: Lethal damage kills unless indestructible
- **WHEN** a non-indestructible creature has marked damage ≥ toughness or is deathtouched
- **THEN** it dies as an SBA

#### Scenario: Zero toughness ignores indestructible
- **WHEN** a creature's toughness is ≤ 0
- **THEN** it dies even if indestructible

#### Scenario: Legend rule pauses for choice
- **WHEN** a living controller has two or more legendary permanents with the same printed name after event-producing SBAs settle
- **THEN** the engine raises a keep-one legendary choice for that conflict group before continuing further legend groups

#### Scenario: Counter annihilation
- **WHEN** a permanent has both +1/+1 and −1/−1 counters
- **THEN** the engine removes min(counts) pairs as an SBA before death checks in that scan

### Requirement: Continuous Effects and Characteristics
Effective power, toughness, types, colors, and keywords MUST be computed on demand from a continuous-effect pipeline fed by a single duration-scoped modifier registry (plus attachments and statics), ordered with CR 613.7 timestamps. Durations MUST include until end of turn (cleanup sweep), until end of combat (end-of-combat sweep), and durationless effects that lapse with the object. Layer reads MUST cover base P/T sets, additive P/T (counters, pumps, anthems, attach bonuses), type/subtype sets and unions, ability loss before printed abilities, color set vs add, and keyword grants/strips. Results MAY be memoized and MUST invalidate on relevant events. Copy effects MUST swap the printed-definition handle; "except it has …" riders MUST be copiable characteristics that clear when a new wholesale copy replaces them.

A counter on a permanent MAY itself be the carrier of a grant: where a durationless ability is handed to a permanent by a source that leaves nothing else behind, the grant MUST be read live off that counter, so it survives its granter leaving the battlefield (CR 400.7) and ends when the counter is removed. Such a grant MUST be appended after the permanent's own abilities so existing activation indices are unchanged.

#### Scenario: A counter-carried ability outlives its granter
- **WHEN** the permanent that granted a durationless ability alongside a counter leaves the battlefield
- **THEN** the ability is still activatable off the counter, and paying it by removing that counter ends the grant

#### Scenario: Until-EOT modifiers end at cleanup
- **WHEN** cleanup removes until-end-of-turn effects
- **THEN** durationed modifiers for those hosts are dropped and characteristics no longer include them

#### Scenario: Stacked base P/T uses timestamps
- **WHEN** two base P/T set effects apply to the same permanent
- **THEN** same-layer ordering uses timestamps so the later set can override the earlier

### Requirement: Replacement Effects
The engine MUST maintain a live replacement registry covering combat and noncombat damage prevention/shields, counter placement modifiers, token creation modifiers, life-gain modifiers, and additional ETB counters (including non-spell battlefield-entry paths). Spell-only as-enters choices such as devour and enter-as-copy MAY remain on the spell-resolution path. Full CR 614/616 ordering across arbitrary overlapping replacements is NOT required beyond pool-backed registry coverage.

#### Scenario: Shared registry on damage and ETB
- **WHEN** combat damage is dealt or a permanent enters via a non-spell path with enters-with-counters / extra-counter statics
- **THEN** those modifications are read through the same replacement registry path

Damage moved by a redirection shield (CR 615.10) MUST be dealt for real at its new recipient: it MUST NOT be moved a second time (CR 616.1), and when the new recipient is a player it MUST fire damage-to-a-player triggers and count toward the turn's damage history exactly as an ordinary hit does.

#### Scenario: Redirected damage lands as damage
- **WHEN** a prevention shield redirects a hit onto a player
- **THEN** that player takes the damage, damage-to-a-player triggers on the dealing source fire, and the redirect is not applied again

A shield whose source gate asks about the spell or ability that **caused** the damage MUST be answered from the stack item currently resolving, not from the damage's source object: it MUST match whenever that item targets the shielded permanent, whatever source the item points the damage at. Damage minted outside any resolution — combat damage — MUST NOT match such a gate.

#### Scenario: An ability that targets the shielded creature is caught
- **WHEN** an activated ability targets a creature under a cause-gated shield and makes its own source deal damage to that creature
- **THEN** that damage is prevented, while the same ability aimed at an unshielded creature still deals its damage

### Requirement: Elimination and Winner
A player who reaches ≤ 0 life, must draw from an empty library, accumulates lethal poison, takes 21 combat damage from one commander source, or concedes MUST lose. On loss, every object that player owns MUST leave the game; control effects involving that player MUST end; the seat MUST be skipped in turn order and priority. Attackers targeting an eliminated defender MUST continue as unblocked. Eliminated seats remain in the player list as lost and MUST still be able to observe the match stream at the host layer. When exactly one living player remains, that player MUST be the winner.

#### Scenario: Loser's objects leave the game
- **WHEN** a player loses
- **THEN** objects they own are removed and they are skipped for priority and turn order

#### Scenario: Sole survivor wins
- **WHEN** only one living player remains
- **THEN** the engine reports that player as the winner

### Requirement: Turn Structure and Turn-Based Actions
A turn MUST progress through: Untap → Upkeep → Draw → Main1 → BeginCombat → DeclareAttackers → DeclareBlockers → CombatDamage → EndCombat → Main2 → End → Cleanup. Untap MUST NOT grant priority. Cleanup MUST NOT grant priority unless a triggered ability fired or discard-to-hand-size requires a choice (then a mini priority round may occur before another cleanup pass). All other steps MUST grant priority to the active player on entry. Turn-based actions at step start MUST include: untap controlled permanents and clear turn-scoped tallies — which MUST include a ledger of damage dealt this turn, recording the dealing source, the recipient, and the amount actually dealt — / goad for the previous active player / advance suspend time counters (Untap); draw one card subject to first-draw skip (Draw); discard to hand size 7, remove marked damage, end until-EOT effects and expiring permissions (Cleanup). Mana pools MUST empty at each step/phase boundary except `persist` mana, which carries until used or end of turn.

A permanent MUST stay tapped through its controller's untap step while any continuous effect says it does not untap, and while it carries a mark for a skipped untap step. A skipped-untap mark MUST be consumed one per untap step, so "the next two untap steps" holds a permanent down through exactly two of them and no more. Marks MUST be independent of counters on the permanent: an effect that reads counters MUST re-read them each untap step, so removing the last counter frees the permanent at the next one.

#### Scenario: Untap has no priority
- **WHEN** the active player's untap step begins
- **THEN** permanents untap and turn tallies clear without granting priority

#### Scenario: Two skipped untap steps release on the third
- **WHEN** a permanent is marked to skip its controller's next two untap steps
- **THEN** it stays tapped through the next two and untaps normally at the third

#### Scenario: Cleanup discard pause
- **WHEN** a player has more than seven cards in hand at cleanup
- **THEN** the engine raises discard-to-hand-size before continuing

#### Scenario: Mana empties between steps
- **WHEN** a step ends and mana in the pool is not marked persist
- **THEN** that mana does not carry into the next step

### Requirement: Priority and Stack
Priority MUST begin with the active player on steps that grant it. After a player acts (cast, activate, play land) or a stack item resolves, priority MUST return to the active player. When consecutive passes equal the number of living players: if the stack is non-empty, resolve the top item, reset passes, and return priority to the active player; if the stack is empty, advance to the next step. Combat declaration steps MUST remain until a valid declaration is made (empty declarations legal when not forced by goad/must-attack). Mana abilities that produce mana and have no target MUST resolve immediately without using the stack and without changing priority or the pass counter.

An activated ability on the stack MUST be targetable and counterable in its own right, independently of the permanent that produced it: countering it MUST remove it from the stack without touching its source, and a targeting restriction naming its source's card type MUST be enforced when targets are chosen. A counter-unless-pays form MUST offer the payment to the *ability's* controller, and MUST counter the ability only when that player declines.

#### Scenario: An activated ability is countered on the stack
- **WHEN** a spell that counters a target activated ability from an artifact source resolves against an artifact's ability
- **THEN** that ability leaves the stack without resolving, the artifact itself is untouched, and an ability from a nonartifact source was never a legal target

#### Scenario: All-pass resolves stack
- **WHEN** all living players pass with a non-empty stack
- **THEN** the top stack item resolves and priority returns to the active player with passes reset

#### Scenario: All-pass advances step
- **WHEN** all living players pass with an empty stack outside a waiting declaration gate
- **THEN** the turn advances to the next step

#### Scenario: Mana ability is instant and stackless
- **WHEN** a player taps for mana or activates a mana ability
- **THEN** mana is produced without using the stack and priority is unchanged

### Requirement: Meaningful Actions for Auto-Pass
The engine MUST expose whether a seat has any meaningful action. Meaningful actions MUST include: available land drops at sorcery speed on an empty stack; castable spells (timing, zone, affordability via the same auto-tap planner used for payment, legal targets/modes) — on an empty stack outside main/declare windows only instant-speed casts count for auto-pass, except after attackers are declared defenders' declare-attackers priority also counts empty-stack instants; activatable non-mana abilities; awaited combat declarations. Bare mana production MUST NEVER count as meaningful. A separate empty-stack-instant predicate MUST exist for host End Turn chrome and MUST be broader than the auto-pass meaningful set where specified. Yield flags, stack-hold timers, and helpless dwell MUST remain host-layer chrome; the engine MUST remain intent-only and receive only pass/act intents.

#### Scenario: Helpless seat has no meaningful action
- **WHEN** a living seat has no land drop, castable non-mana play, non-mana activate, or awaited declaration under the meaningful-action rules
- **THEN** `has_meaningful_action` is false even if they could tap for mana

#### Scenario: Engine does not know yields
- **WHEN** a host arms stack yield or turn yield for a seat
- **THEN** the engine state is unchanged except insofar as the host submits PassPriority on that seat's behalf

### Requirement: Pending Choices
While a pending choice is set, legal actions MUST be empty and only the matching answer intent from the awaited player MUST be accepted. Choice kinds MUST cover at least: target selection (including multi-clause and up-to-N); order simultaneous triggers for one controller; may yes/no; may draw up to N; pay optional cost; discard to hand size; sacrifice edicts / choose sacrifice; arrange top (scry/surveil); search library/graveyard (fail-to-find always legal); choose mode (placement vs mid-resolution); commander redirect; assign combat damage; choose attach host; legend keep; pay echo or sacrifice; pay recover or exile; pay cumulative upkeep or sacrifice; triggered discard; may exile discarded nonlands to play. "May" choices, arrange-top, search fail-to-find, commander redirect, and keep-one edicts MUST NOT be auto-forced. Target choice with min ≥ 1 and exactly one legal target MAY be forced; min == 0 with one legal target MUST NOT be forced. An ability whose effect carries an independent second target clause MUST have that clause's full complement of legal targets available at announcement, before any cost is paid, or the activation MUST be rejected; a clause whose legal set exactly equals its required count MUST be filled without raising a choice.

#### Scenario: Second target clause short of targets refuses the activation
- **WHEN** an ability with a mandatory two-target second clause is activated while only one legal target for that clause exists
- **THEN** the activation is rejected before costs are paid rather than resolving with fewer targets

#### Scenario: Choice gates other actions
- **WHEN** a pending choice is active
- **THEN** legal actions are empty and only the awaited player's correct answer is accepted

#### Scenario: May choices are never forced
- **WHEN** a may-yes/no or may-draw-up-to choice is pending
- **THEN** declining (or choosing zero) remains legal and the engine does not auto-answer

### Requirement: Forced Actions
When a pending choice has exactly one unambiguous legal answer under the conservative forced-action rules, the engine MUST expose that single auto-submittable intent so the host can submit it automatically. Real decisions MUST never be auto-submitted.

#### Scenario: Single mandatory target is forced
- **WHEN** a target choice requires at least one target and exactly one legal target exists
- **THEN** forced action returns that answer

#### Scenario: Arrange top is never forced
- **WHEN** a one-card scry/surveil arrange-top is pending
- **THEN** forced action returns none because top vs bottom is a real decision

### Requirement: Legal Actions and Stable Ids
After state changes, the engine MUST recompute a per-seat legal-action list from meaningful actions (plus paid mana activates flagged as mana-only for menus without stopping auto-pass). Each legal action MUST carry a stable monotonic id, acting player, and kind. An action whose (player, kind) identity survives a refresh MUST keep its id; dead ids MUST never be recycled. Clients MUST be able to execute via take-action-by-id (optional chosen inputs) identically to the equivalent concrete intent. Meaningful kinds include keep/mulligan, play land, cast (including split half / face-down / prepared), activate, cycle, hand ability, suspend, encore, turn face up, standing prevention payment, and combat declarations.

#### Scenario: Stable id across non-removing state change
- **WHEN** a cast action remains legal after an unrelated tap that does not remove it
- **THEN** the cast action retains the same id across refresh

#### Scenario: Mana-only actions do not halt auto-pass
- **WHEN** a permanent's only listed activate is paid mana production
- **THEN** that action is marked mana-only and does not make `has_meaningful_action` true

### Requirement: Payment and Auto-Tap
Casts, activations, cycling, and pay-cost choices MUST settle payment in-engine: verify affordability from pool plus free-tap sources; auto-tap free sources (lands before non-lands, non-pain before pain, broader color preferred); plan paid nested mana abilities feed-first without recursive mint loops; deduct mana and emit tap/spend events in the same delta. Clients MUST NOT be required to pre-sequence taps for a cast. Manual tap-for-mana remains available. Paid mana abilities with generic costs MUST appear as activate actions and MUST NOT be auto-tapped by the planner. Net-zero converters MUST be excluded from the planner.

#### Scenario: One-click cast auto-taps
- **WHEN** a player casts an affordable spell with untapped free mana sources and an empty or insufficient pool
- **THEN** the engine taps sources and spends mana as part of the cast without a prior manual tap intent

#### Scenario: Pain lands preferred later
- **WHEN** both pain and non-pain free sources can pay a cost
- **THEN** the planner taps non-pain sources before pain sources

### Requirement: Effect Resolution and Resume
Mid-resolution player input MUST raise a pending choice and park continuation in non-event-sourced resume state so a single effect sequence may pause multiple times without callbacks. Named resolution-frame tallies MUST let a later step read values produced earlier in the same resolution ("this way" amounts). When an instant or sorcery finishes without pausing, a finish policy MUST send it to its final destination (graveyard by default, or one-shot overrides such as tuck bottom / exile / exile with time counters). New card behavior MUST be expressed as data-driven effect vocabulary dispatched by the engine rather than hard-coded per-card branches in the core submit path.

#### Scenario: Multi-pause effect resumes in order
- **WHEN** a resolving sequence raises two successive choices
- **THEN** answering each in order drains the deferred continuation and completes the effect

#### Scenario: This-way tally binds producer to consumer
- **WHEN** an earlier resolution step records counters-removed-this-way and a later step draws that many
- **THEN** the later step reads the named tally from the current resolution frame

### Requirement: Casting and Spell Resolution
Casting MUST validate timing, zone, affordability, modes, and targets; may raise target choice when needed; settle payment; move the card to the stack as a spell; and enqueue cast triggers for the next priority window. On resolution, permanents enter the battlefield; instants/sorceries run effects then finish to their destination. Commander casts from the command zone MUST apply command-zone cast tax ({2} more per prior command-zone cast for that seat).

#### Scenario: Spell goes to stack then resolves
- **WHEN** a cast is accepted and all players eventually pass
- **THEN** the spell resolves from the stack according to its kind (permanent entry or effect run + finish)

#### Scenario: Commander tax increases
- **WHEN** a seat casts its commander from the command zone after prior command-zone casts
- **THEN** the mana cost is increased by {2} per previous such cast for that seat

### Requirement: Combat Structure
Combat MUST use five steps: BeginCombat (priority, beginning-of-combat triggers); DeclareAttackers (active or overridden declarer submits attacker→defender pairs; empty legal when not forced); DeclareBlockers (each defending seat in APNAP order, or overridden block declarer for those seats); CombatDamage (first-strike/double-strike sub-step when needed, then regular; damage assignment choice when required); EndCombat (priority, then clear combat state). Blocked status for an attacker MUST remain durable for the rest of combat even if all blockers later leave, so non-trample deals no defender damage while trample may still assign to the defender.

#### Scenario: Empty attack declaration advances
- **WHEN** the attack declarer submits no attackers and none are required
- **THEN** the declaration is legal and combat proceeds without attackers

#### Scenario: Blocked with no living blockers
- **WHEN** an attacker was blocked and all blockers have left combat
- **THEN** a non-trample attacker deals no damage to the defending player, while a trample attacker may still assign its power to that defender

Triggers that fire on a declaration MUST go on the stack in that declaration step, before the
combat damage step's turn-based action (CR 509.4), whether the declaration arrived as an intent or
was sealed empty by an all-pass round. A "whenever this creature attacks and isn't blocked" trigger
MUST be queued only once every attacked seat's block declaration is final (CR 509.1h), over the
attackers nobody blocked.

#### Scenario: All-pass declaration still resolves its triggers first
- **WHEN** a declaration is sealed because every player passed, and that declaration queued triggers
- **THEN** the step stays open for one more priority round so those triggers are put on the stack and resolve before combat damage

### Requirement: Attack and Block Legality
Attackers MUST be controlled creatures that are untapped (vigilance taps only as needed by rules), without summoning sickness unless haste, without can't-attack restrictions, and able to pay any pillow-fort attack tax toward the chosen defender. Goaded and must-attack requirements MUST be enforced at declaration; goad's "if able" MUST intersect attack-tax affordability (force only toward affordable defenders; prefer affordable non-goader when one exists). Block legality MUST enforce flying/reach, unblockable, skulk, shadow (bidirectional), fear, protection, menace (whole declaration), can't-block / decayed / continuous can't-block, can-block-only-flyers, lesser-power-can't-block, and that the attacker is attacking that defender. Listing and validation MUST share the same predicates. Declaration overrides MAY move who chooses attacks/blocks for the turn without changing what may be chosen; overridden declarers fall back if that seat has lost.

#### Scenario: Illegal block rejected
- **WHEN** a declare-blockers intent pairs a ground creature with a flying attacker and the blocker lacks flying/reach
- **THEN** the engine rejects the declaration

#### Scenario: Goad prefers affordable non-goader
- **WHEN** a goaded creature can afford to attack a non-goader and also a goader
- **THEN** a declaration that attacks only a goader is rejected

#### Scenario: Declaration override moves chooser only
- **WHEN** an attack-declarer override is set
- **THEN** that seat submits the declare-attackers intent but the creatures declared remain the active player's

Attack bans MUST be answered by the same predicate that answers "is this creature able to attack",
so a restriction beats a requirement (CR 509.1a): board-wide filter bans (Moat's "creatures without
flying can't attack") apply to every player's creatures regardless of who controls the source; a
creature that attacked during its controller's *own* last turn, or that was blocked by a creature
whose ability bans it for its controller's next turn, MUST be barred for that turn; and a player who
neither cast a spell nor put a nontoken permanent onto the battlefield during their own last turn
MUST NOT be attackable while such a static is on the battlefield, though their planeswalkers still
may be. Those "during your last turn" facts MUST be rolled once per turn at the active player's
cleanup step, since the per-turn tallies are cleared at every untap and intervening seats' turns
would erase them. A cap on how many creatures may attack or block each combat MUST be enforced over
the whole declaration — across all players, counted in creatures rather than in blocks — rather than
by banning any individual creature.

#### Scenario: Board-wide attack ban reaches every seat
- **WHEN** a static bans creatures matching a filter from attacking
- **THEN** creatures matching it are unable to attack no matter whose battlefield they or the source are on

#### Scenario: Attacked-last-own-turn creature must rest
- **WHEN** a creature with "can't attack if it attacked during your last turn" attacked on its controller's previous turn
- **THEN** it is not a legal attacker this turn, and becomes legal again the turn after

#### Scenario: Attack cap beats a requirement to attack
- **WHEN** a cap allows two attackers and three goaded creatures could otherwise be forced to attack
- **THEN** a declaration of two is legal and a declaration of three is rejected

### Requirement: Combat Damage and Combat Keywords
Unblocked attackers MUST deal power to the defending player (planeswalker-as-attack-target is only partially supported). Blocked attackers and blockers MUST deal damage simultaneously within each damage sub-step. First strike and double strike MUST participate in the first-strike sub-step; double strike also in the regular sub-step. Trample MUST raise damage assignment with lethal-to-each-blocker minimums; deathtouch MUST treat any non-zero damage as lethal for those minimums. Damage MUST mark permanents and/or reduce life, then SBAs apply. Implemented combat-relevant keywords include flying, reach, vigilance, haste, trample, first/double strike, deathtouch, indestructible, lifelink, protection, menace, shadow, fear, skulk, decayed, unblockable, goad, and related can't-block restrictions. Per-player and table-wide combat damage prevention shields MUST be consulted at combat-damage application.

#### Scenario: First strike before regular damage
- **WHEN** a first-strike creature and a non-first-strike creature are in combat
- **THEN** first-strike damage is assigned and SBAs may remove creatures before regular damage

#### Scenario: Trample assignment choice
- **WHEN** a trample attacker is blocked by one or more creatures
- **THEN** its controller receives an assign-combat-damage choice respecting lethal minimums (1 per blocker with deathtouch)

#### Scenario: Every division in a sub-step is answered before any damage lands
- **WHEN** a damage sub-step begins and two or more attackers in it are each blocked by multiple creatures
- **THEN** every such attacker with positive power raises `PendingChoice::AssignCombatDamage` first, and no damage in that sub-step is dealt until all divisions are answered

Each damage sub-step MUST be one turn-based action (CR 510.1): the divisions for the whole batch
are collected before any of the batch's damage is dealt. The division MUST be read in the damage
step rather than at declare blockers, so it totals the power the attacker has at that moment (CR
510.1a) — a rampage bonus or a pump spell cast in response to the blocks is assignable. Assignment
order MUST be the blocker declaration order (CR 509.2). When a banding blocker is involved, the
**defending** player answers the division instead of the attacking player (CR 702.22j).

A creature told it "assigns no combat damage this turn" (CR 510.1a) MUST be excluded from the
assignment itself rather than have its damage prevented: it neither raises a division question nor
deals damage in either sub-step, for the rest of the turn.

#### Scenario: Assigns no combat damage
- **WHEN** an unblocked attacker is told it assigns no combat damage this turn
- **THEN** the defending player's life is unchanged when the combat damage step passes

### Requirement: Commander Identity Rules
Each deck has exactly one commander. When a commander would move to graveyard, exile, hand, or library, its controller MUST be offered redirection to the command zone (not forced). Commander combat damage MUST be tracked per commander source identity as implemented (by source commander's owner seat for lethal aggregation) and MUST eliminate a player at 21 or more from one source. Partner commanders are out of scope.

#### Scenario: Commander redirect offered
- **WHEN** a commander would leave for graveyard or exile
- **THEN** a commander-redirect choice is raised and the player may allow the normal zone or send it to the command zone

#### Scenario: Twenty-one commander damage
- **WHEN** cumulative combat damage from one commander source to a player reaches 21
- **THEN** that player loses as an SBA

### Requirement: Data-Driven Card Behavior Boundary
Printed definitions MUST be interned behind stable card ids shared across clones of a game. Effect and event payloads that need printed identity MUST carry handles, not full embedded definitions. New rules vocabulary grows from real card demand: new effect variants plus dispatch/apply coverage and card scripts — callers MUST NOT bypass the effect runner to apply card logic ad hoc.

#### Scenario: Game clone shares printed defs
- **WHEN** a game is cloned for projection or look-ahead
- **THEN** mutable board state is independent while printed definitions remain shared via the intern table
