# Combat and Commander Rules

**Status:** Current (as of 2026-07-20)
**Module:** `crates/engine` (`src/combat.rs`, `src/apply.rs` SBA arm, `src/priority.rs` step advance, `src/state.rs` `CombatExtras`)

---

## Problem Statement

Commander is a multiplayer format where each player can independently attack any opponent. Combat spans five steps, involves keyword interactions (flying, trample, first strike, etc.), and must correctly handle elimination mid-combat, goaded creatures, pillow-fort costs, and the Commander-specific rules: command zone, cast tax, commander damage, and the 21-damage loss condition. The engine must be correct for 2–4 players simultaneously.

---

## Solution

Combat state is held in `Game::combat: CombatState` (attackers, block assignments, damage orderings) and `Game::combat_extras: CombatExtras` (goad, must-attack requirements, combat damage prevention shields). The five combat steps are distinct `Step` variants; each gates on a required declaration or advances automatically. Block legality and attacker legality are enforced at declaration time; damage assignment is a pending choice when multiple blockers exist or trample/deathtouch interact. Commander-specific rules (command zone, tax, 21-damage) are woven into the core casting, SBA, and damage paths.

---

## User Stories

1. As a **player**, I want to declare which of my creatures attacks which opponent (or opponents), so I can split my attack across multiple defending players.
2. As a **defending player**, I want to declare which of my creatures blocks which attacker, in APNAP order after attackers are declared.
3. As a **player**, I want the engine to enforce block legality (flying, protection, menace, shadow, skulk, etc.) at declaration time and reject illegal declarations.
4. As a **player**, I want my attacking creatures' and blocking creatures' combat damage dealt simultaneously in the combat damage step, with first-strike as a separate sub-step.
5. As a **player with trample attackers**, I want to be prompted to assign damage to blockers vs. the defending player (with lethal-to-each-blocker minimums).
6. As a **player**, I want to know a creature is goaded so I understand why it must attack a non-goader each turn.
7. As a **player**, I want the engine to track commander damage dealt by each opponent's commander, and lose when any one reaches 21.
8. As a **player**, I want to redirect my commander to the command zone instead of the graveyard or exile when it would go there.
9. As a **player**, I want each cast of my commander from the command zone to cost {2} more per previous command-zone cast (commander tax).
10. As a **player**, I want eliminated players to be skipped from attack targets and blocker order, with their in-combat creatures simply removed.
11. As a **player using a pillow-fort effect**, I want attackers to pay the cost or be prevented from attacking the protected player.
12. As a **player with vigilance creatures**, I want them to not tap when they attack.
13. As a **player with haste creatures**, I want them to be able to attack on the turn they entered the battlefield (not subject to summoning sickness).
14. As a **spectator or opponent**, I want to see the combat view (each attacker paired with its defender and its blockers) so I can follow combat correctly.

---

## Behavior

### Combat steps

1. **BeginCombat** — no declaration required; priority is granted. Triggered abilities for "at the beginning of combat" fire here.
2. **DeclareAttackers** — the active player submits `Intent::DeclareAttackers { attackers: Vec<(ObjectId, PlayerId)> }`. Each attacker is paired with a defending player. The engine validates the declaration (see below) and resolves attack-triggered abilities. An empty declaration (no attackers) is legal and advances normally.
3. **DeclareBlockers** — each defending player (in APNAP order relative to the active player) submits `Intent::DeclareBlockers { blocks: Vec<(ObjectId, ObjectId)> }` pairing their blockers with attackers. Each defending player's declaration is validated in turn.
4. **CombatDamage** — if any first-strike or double-strike creature is attacking or blocking, a first-strike sub-step runs first. Then regular damage is assigned. If a player must assign damage order among multiple blockers, `PendingChoice::AssignCombatDamage` is raised.
5. **EndCombat** — priority is granted; end-of-combat triggered abilities fire. The `CombatState` is cleared.

### Attacker legality

An attacking creature must:
- Be a creature the active player controls, on the battlefield.
- Not be tapped (unless it has vigilance — in which case it taps only after the declaration).
- Not have summoning sickness (unless it has haste): it must have been under the active player's control continuously since the active player's most recent turn began.
- Not have the "can't attack" keyword or continuous restriction.
- Not be phased out.

Goaded creatures (CR 701.38) **must** attack if able, and must attack a player who did not goad them if any such defending player exists. The engine enforces this as a constraint on the declaration: a goaded creature whose declaration does not satisfy these requirements causes `Reject::AttackerDeclarationInvalid`.

"Must attack this turn" requirements (`CombatExtras::must_attack`, from e.g. Furygale Flocking's tokens) are enforced similarly.

Pillow-fort costs (CR 508.1g) — "creatures attacking you cost {N}" — are checked at declaration; if a player can't or won't pay, those creatures can't attack that player (the declaration must exclude them or assign them to other defenders).

### Block legality

`Game::can_block(player, blocker, attacker)` enforces (CR 509.1):

- Blocker is a creature controlled by `player`, not tapped, not phased out.
- Blocker is not blocked by `Keyword::CantBlock`, `Keyword::Decayed`, or a continuous "can't block" grant from an attached Aura.
- If the attacker has **Flying**, the blocker must have Flying or Reach.
- If the attacker is **Unblockable**, no blocker may block it.
- If the attacker has **Skulk**, the blocker's power must not exceed the attacker's power.
- If the attacker has **Shadow**, the blocker must also have Shadow (bidirectional: Shadow creatures can only block/be blocked by Shadow).
- If the attacker has **Fear**, the blocker must be an artifact creature or a black creature.
- **Protection** (CR 702.16c): a creature with protection from a color cannot be blocked by creatures of that color.
- **Menace** is enforced at the whole-declaration level: a menace attacker must be blocked by at least two creatures, or not at all.
- `Keyword::CanBlockOnlyFlyers` (Brazen Borrower): may only block flying attackers.
- `Keyword::LesserPowerCantBlock` (Elusive Otter): blockers with less power than the attacker can't block it.
- The attacker must be attacking `player` (not another defender).

### Combat damage

- Attacking and blocking creatures deal damage simultaneously in the combat damage step (CR 510.1).
- An **unblocked attacker** deals its power in damage to the defending player (or their planeswalker if one is the target — partially implemented: commander flag is supported; planeswalker damage-target is in progress via `target = "player_or_planeswalker"`).
- A **blocked attacker** deals damage to its blockers; blocked creatures deal damage back to the attacker.
- **First strike** (CR 702.7): a first-strike or double-strike creature participates in the first-strike damage sub-step. Regular creatures deal damage in the normal sub-step. Double-strike creatures participate in both.
- **Trample** (CR 702.19): when a trample attacker is blocked, its controller may assign excess damage (beyond lethal-to-each-blocker) to the defending player. A `PendingChoice::AssignCombatDamage` is raised for the trample assignment.
- **Deathtouch** (CR 702.2): any non-zero damage from a deathtouch source is considered lethal; when trample + deathtouch interact, 1 point per blocker is lethal, and all remaining damage goes to the defending player.
- Damage is recorded as `marked_damage` on `Permanent` and triggers SBAs (lethal damage → dies) after the combat damage step.
- Combat damage to a player reduces their life total. Commander damage is also tracked separately per-source-commander.

### Commander damage

- Each player has a `commander_damage: Vec<(ObjectId, i32)>` tracking total combat damage dealt from each specific commander object.
- When a commander deals combat damage to a player and the total from that commander reaches **21 or more**, a `PlayerLost` SBA fires for the damage recipient (CR 903.10a).
- Commander identity (the commander's `ObjectId`) is tracked per attacker, not per card def, because zone changes create new object ids; the `commander: bool` flag on `Card` in the command zone is what the engine uses to identify commanders.

### Commander replacement / command zone

- When a commander would move to the graveyard, exile, hand, or library, its controller may redirect it to the **command zone** instead (CR 903.9a). This is modeled as a `PendingChoice::CommanderRedirect` raised before the zone-change event is applied.
- While in the command zone, a commander is castable from there (not from the hand).
- Each time a commander is cast from the command zone, its cost increases by `{2}` per previous command-zone cast for that player (the **command-zone cast tax**, CR 903.8). The engine tracks `Player::commander_casts_from_command_zone: u8` per seat.

### Goad

- A goaded creature (CR 701.38) must attack each combat if able, and must attack a player who did not goad it if any such player exists.
- Goad state lives in `CombatExtras::goaded: Vec<(ObjectId, PlayerId, &'static str)>` — a list of `(goaded creature, the player who goaded it, source card name for the inspect ledger)`.
- A creature may appear multiple times if goaded by multiple players.
- Goad clears at the start of the goading player's next untap step (one turn of effect, CR 701.38b).
- Continuous goad-on-attachment (the Impetus cycle, Redemption Arc) is not stored in `goaded`; it is re-evaluated live off the attachment scan whenever goad state is queried.

### Elimination during combat

- When a player is eliminated mid-combat, their attackers and blockers remain in combat but their owned objects are tombstoned. The attacking/blocking state is pruned: attackers targeting an eliminated player continue as **unblocked** (no defender to take damage), and their blockers (controlled by the eliminated player) cease to exist.
- `Game::defender_of(attacker)` returns `None` if the targeted defender has since been eliminated, and the attacker is treated as going unblocked.

### Key keywords implemented in combat

- **Flying**, **Reach**: block-legality checks in `can_block`.
- **Vigilance** (CR 702.20): attacking creature does not tap.
- **Haste** (CR 702.10): summoning sickness suppressed.
- **Trample** (CR 702.19): excess damage to player.
- **First strike / Double strike** (CR 702.7/8): two-phase damage sub-step.
- **Deathtouch** (CR 702.2): 1 damage is lethal for trample assignment.
- **Indestructible** (CR 702.12): not killed by lethal damage SBA; can still be killed by 0-toughness.
- **Lifelink** (CR 702.15): damage dealt also causes the controller to gain that much life.
- **Protection** (CR 702.16): blocks targeting sources of the protected type/color.
- **Menace** (CR 702.110): must be blocked by two or more creatures.
- **Shadow** (CR 702.28): bidirectional restriction.
- **Fear** (CR 702.36): can only be blocked by artifact or black creatures.
- **Skulk** (CR 702.72): can't be blocked by greater-power creatures.
- **Decayed** (CR 702.148): creature with this keyword can't block.
- **Unblockable** (CR 702.10b internal tag): no blocker may block.
- **Goad** (CR 701.38): must attack and must attack a non-goader.
- **CantBlock**, **LesserPowerCantBlock**, **CanBlockOnlyFlyers**: card-specific internal keyword tags.

---

## Implementation Decisions

- **`CombatState` is separate from `Permanent` fields.** `Permanent` is `Copy`; attacker/block tracking needs mutable `Vec`s, so they live in `Game::combat`.
- **Block legality and attack legality use the same predicates for listing and validation.** `Game::can_block` is shared between `Game::meaningful_actions` (listing legal blockers) and `Game::declare_blockers` (validation at submission) so they can never disagree.
- **Goad enforcement mirrors must-attack.** Both are constraint loops in `declare_attackers` that validate the declaration against requirements; a declaration failing these is rejected with `Reject::AttackerDeclarationInvalid`.
- **Commander damage uses object ids, not card def ids.** Because zone changes mint new object ids (CR 400.7), the commander is tracked as the current object id of the commander permanent in combat. This is consistent with how all other per-permanent effects are tracked.
- **Planeswalker combat damage is partially implemented.** The `target = "player_or_planeswalker"` filter exists in the DSL; direct planeswalker-takes-attacker-damage in the combat damage path follows the same route. A complete attack-a-planeswalker flow (declaring attack at a planeswalker object id rather than a player) is in progress.
- **`CombatExtras::combat_damage_prevention_shields`** models per-player, per-token combat damage prevention (Inkshield pattern). A separate `prevent_all_combat_damage_this_turn` boolean handles table-wide prevention (Moment's Peace). Both are checked at all three combat-damage chokes.

---

## Testing Decisions

- **Block legality tests** should construct a board with specific keyword combinations and assert `can_block` returns the correct boolean for each case.
- **Damage assignment tests** should trace through a trample declaration with multiple blockers and verify the resulting `marked_damage` values and player life total changes.
- **Goad tests** should arm a goad entry and verify that a declaration not attacking a non-goader is rejected.
- **Commander damage tests** should deal 20 cumulative points from one commander, then 1 more, and assert `PlayerLost` fires.
- **Commander redirect tests** should move a commander to the graveyard and verify `PendingChoice::CommanderRedirect` is raised, then accepting it moves the card to the command zone.
- **Elimination mid-combat**: eliminate a defending player after attackers are declared and verify the game continues with the attacker going unblocked.
- **Prior art**: `tests/game.rs` has multi-player combat and commander damage integration tests.

---

## Out of Scope

- **Planeswalker as attack target** (full attack-a-planeswalker declaration, CR 306.9) — partially supported via `player_or_planeswalker` damage target; the full "declare attackers at a planeswalker" declaration path is in progress.
- **Banding** (CR 702.22) — not implemented.
- **Damage prevention and redirection** (general CR 615 prevention effects beyond Inkshield/Moment's Peace) — flagged per-deck when needed (`docs/fidelity/`).
- **Ninjutsu** (CR 702.49) — not implemented.
- **Blocking multiple attackers** (a single creature blocking two attackers) — not in scope for the current pool.
- **Partner commanders** — not yet modeled; a deck has exactly one commander.

---

## Further Notes

- See `2026-07-20-engine-core-and-event-model.md` for SBA processing that follows combat damage.
- See `2026-07-20-turn-priority-and-stack.md` for how the five combat steps integrate into the turn structure and priority model.
- See `2026-07-20-choices-actions-and-resolution.md` for `PendingChoice::AssignCombatDamage` and `PendingChoice::CommanderRedirect` flows.
- `CONTEXT.md` defines **commander damage**, **goad**, **defending player**, **APNAP**, and related terms.
