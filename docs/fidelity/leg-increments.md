# Legends (`leg`) increments (2026-07-29)

Set report: [leg.md](leg.md). This file is the sole engine-capability backlog for `leg`
(ranked increments + per-card exotics). Numbering is local to this file.

This is a **set** grind, not a deck grind — intake is Scryfall `set:leg unique:cards`, not an
Archidekt link, and there is no precon to ship at the end. 310 unique cards: 7 already in the
pool, 102 authored, 198 blocked here, 3 out of scope.

Increments 1–93 come from the intake read. Increments 94–117 were raised by the Phase 3
authoring wave, which found 39 cards the intake had judged authorable but that turned out to
need engine work; those cards moved to section D and are ranked at the bottom of this file.

Ranked S-first within dependency order. Legends is a combat-rules set: where 2ed's centre of
gravity was damage prevention, Legends' is *what happens in the combat phase*, and the engine
skips most of it. Four clusters gate 82 of the 159 blocked cards:

- **Combat modifiers and restrictions** (#1, #3, #4, #5, #9, #10, #23, #29, #56, #87) — rampage,
  "bands with other", landwalk negation, keyword stripping, block restriction/requirement by
  filter, and attacker/blocker count caps. 40 cards. #3 (bands with other) is the only **XL** in
  the set and is staged below.
- **World enchantments** (#2) — CR 704.5k, the "legend rule for enchantments". 11 cards, plus it
  converts the standing Concordant Crossroads residual into real work. Gates nothing else, but
  every one of those 11 also needs its own body increment, so #2 lands first or those 11 stall.
- **Damage prevention and redirection** (#12, #34, #38, #60) — Legends takes 2ed's consumable
  shield and adds *filters* ("by attacking creatures without flying", "by creatures it's
  blocking", "by Walls") plus redirection to a creature. 14 cards.
- **Base power/toughness rewriting** (#22) — "change base power and toughness … (This effect
  lasts indefinitely.)" is a Legends idiom with no equivalent anywhere in the current pool.
  5 cards, and it is the layer-system work the engine has so far avoided.

Below those the set is a long tail: 77 of the 117 increments are single-card exotics. That is
Legends — it is the set where design was still inventing one-off rules text per card.

### Observability re-audit

Two pool-absence claims in the tree are falsified by Legends. Each is folded into the increment
that clears it (full detail in [leg.md](leg.md) § B):

| Claim | Where | Falsified by | Increment |
| --- | --- | --- | --- |
| "World supertype … the only World card in the pool, so the rule has nothing to interact with yet" | `crates/cards/data/concordant_crossroads.toml` | 11 World enchantments | #2 |
| "only 'attacking' is modeled — no `blocking` filter axis exists yet on `PermanentFilter`" | `crates/cards/data/spurnmage_advocate.toml` | the note describes a card that does not exist — Spurnmage Advocate reads "Destroy target **attacking** creature", with no "or blocking" clause. Cleared in #8; the card's real defect is now #120 | #8, #120 |

Everything else held.

---

### 1. `rampage-n` — 9 cards, M
Depends on: nothing.
Rampage N (CR 702.23) is a triggered ability that fires on *becoming blocked* and scales with the
number of blockers beyond the first. The engine has a `blocks_or_becomes_blocked_by` trigger but
nothing that reads the blocker *count* for one attacker.
*Landed:* `Keyword::Rampage(u8)` (`{ rampage = N }` in TOML, matching `Ward`/`Toxic`'s width)
carrying its N. Like Myriad and Prowess the keyword *is* the ability, so there is no printed
`[[abilities]]` block: `Game::queue_rampage_triggers` synthesizes one `BlocksOrBecomesBlocked`
trigger per attacker per instance of the keyword (CR 702.23c) from `Game::seal_blocks` and
`answer_choose_block_target`, the two places a block declaration is sealed. The pump rides a new
`Amount::BlockersBeyondFirst { per }` that counts the *living* blockers when the ability resolves,
which is CR 702.23b's "calculated only once per combat, when the triggered ability resolves" — a
blocker killed in response shrinks the bonus, one killed afterwards does not.
Rapid Fire's conditional grant is `Condition::TargetHasRampage` under `negate = true` (an exact
`target_has_keyword` can't express "any N"). Gabriel Angelfire stays blocked on #39 — its rampage
3 comes from a modal upkeep choice, which this increment does not touch.
*Residual:* the pump lands on the characteristic but never on the damage. The engine locks the
combat damage division at declare blockers against the *unpumped* power, so the bonus is assigned
to nobody — see **#119**, which all 8 cards carry an `approximates` against. Rampage is correct as
a P/T modification and inert as damage until that lands.

### 2. `world-supertype` — 11 cards, M
Depends on: nothing.
CR 704.5k: if two or more permanents have the World supertype, all but the most recently
gained one are put into their owners' graveyards as a state-based action. The intake sketch had
two wrong premises, corrected here: there was **no** World supertype anywhere — `CardDef` carried
only the `legendary` and `snow` supertype bools, and `concordant_crossroads.toml` was authored as
a plain enchantment — and the existing legend rule (CR 704.5j) tracks **no timestamp**; it is a
`PendingChoice::ChooseLegendaryKeep` raised after the SBA sweep settles.
*Landed:* a `world` bool on `CardDef` / `CardToml` (the `legendary`/`snow` pattern) and a CR
704.5k sweep in `Game::check_state_based_actions` that keeps the World permanent with the highest
object id — ids are minted in entry order — and puts the rest into their owners' graveyards, with
no choice and no grouping by controller or name. `concordant_crossroads.toml` carries
`world = true` and lost its `approximates`.
*Residual:* CR 704.5k's tie clause (simultaneous entry → all go) is unreachable, since object ids
are minted one at a time even when one effect puts several permanents onto the battlefield; it
needs a batch-scoped entry epoch, and no pool card can produce a tie yet. Each of the 11 World
cards still needs its own body increment (16, 23, 25, 35, 44, 48, 75, 80) — this one only makes
the supertype mean something.

### 3. `bands-with-other` — 7 cards, XL
Depends on: the 2ed banding increment (#14 there) — "bands with other" is banding plus a filter,
and it inherits banding's damage-assignment machinery wholesale.
"Bands with other <quality>" (CR 702.22b) is the single largest rules surface in the set: it
changes who may attack together (CR 702.22c), how a band is blocked as a group (CR 702.22h/i), and
— the expensive part — swaps who *assigns* combat damage: the defending player divides a banded
attacker's damage among its blockers (CR 702.22j, an exception to 510.1c), and the attacking player
divides that blocker's damage among the attackers it is blocking (CR 702.22k, an exception to
510.1d). **Five** Legends lands grant it by color to legendary creatures
(Adventurers' Guildhouse/green, Cathedral of Serra/white, Mountain Stronghold/red, Seafarer's
Quay/blue, Unholy Citadel/black — the `cycle-leg-banding-land` otag) and two cards (#5) strip it.
That is the 7: 5 lands + the 2 strippers. Master of the Hunt also grants it, by token name, but it
is **#123** — the name-as-quality form is blocked on data conventions this increment does not touch.
*Slice staging:*
1. **Band formation** (CR 702.22c — *not* 702.22j, which is the blocker-side damage rule) — an
   attack-declaration grouping where creatures declare as a band, legal only if some member has a
   `BandsWith` whose quality every member has. No damage changes yet; banded attackers are just
   recorded as a group. **LANDED.** `Keyword::BandsWith(BandsWithQuality::Legendary)` in
   `crates/cards/src/types/card.rs`; `CombatState::bands` + engine-only
   `Intent::DeclareAttackersInBands` in `crates/engine/src/types/stack.rs`;
   `Game::declare_attackers_in_bands` / `Game::band_is_legal` / `Game::attacking_bands` in
   `crates/engine/src/combat.rs` (CR 702.22c's plain-banding sentence is checked too, so a
   printed-banding band forms); Cathedral of Serra scripted via `keyword_anthem`. Tests:
   `crates/engine/tests/leg_bands_with_other.rs`. The wire/client surface for
   `DeclareAttackersInBands` is **increment #121**, not part of any slice here.
2. **Blocked as a group** (CR 702.22h: "if an attacking creature becomes blocked by a creature, each
   other creature in the same band as the attacking creature becomes blocked by that same blocking
   creature"). **LANDED.** `Game::blocks_extended_to_bands` in `crates/engine/src/combat.rs`,
   called on the first line of `Game::seal_blocks`, so the extra pairs become ordinary
   `Event::BlockerDeclared`s and blocked-ness, damage routing and the block-trigger scans all follow
   for free. Tests in `crates/engine/tests/leg_bands_with_other.rs`. Three findings worth carrying:
   - The extension is **not** a legality check and must run *after* `Game::block_restrictions_ok`.
     CR 702.22h's own example blocks the flier in a flier + swampwalker band and the swampwalker
     "will also become blocked" — so a member the blocker could never have blocked itself (evasion,
     menace, its one-creature ceiling) becomes blocked anyway. Only the *declared* pair is checked.
   - It lives in `seal_blocks`, not `declare_blockers`, so blocks an effect writes down with no
     declaration (Camouflage, via `pending/handlers/dig.rs`) extend along the band too. That is
     CR 702.22i, and since no card in the pool makes a creature blocked with no blocking creature at
     all, it is 702.22i's whole reachable surface.
   - CR 702.22f ("an attacking creature that's removed from combat is also removed from the band it
     was in") falls out of gating each extension on `combat.attackers`, which removal prunes.
3. **Damage assignment transfer** — two halves, both still open:
   - **CR 702.22k** (the active player divides a *blocker's* damage among the attackers it blocks)
     is missing entirely: `combat_damage_substep` deals a blocker's full power to every attacker it
     is blocking, so there is no division to reassign. That is a plain CR 510.1d gap first, and only
     then a banding one — the six banding cards' residual notes all name this.
   - **CR 702.22j** (the defending player divides a banded *attacker's* damage among its blockers)
     is half-built: `Game::damage_assigner` already hands the choice to a banding blocker's
     controller, but not to its second clause — a blocker that is "both a \[quality\] creature with
     'bands with other \[quality\]' and another \[quality\] creature".
4. **The remaining granting cards** — Adventurers' Guildhouse, Mountain Stronghold, Seafarer's
   Quay and Unholy Citadel, each a copy of Cathedral of Serra with its own `color`. **LANDED**, with
   no engine change at all: the four are pure data over slice 1's `keyword_anthem` grant, and they
   carry Cathedral's `approximates` for the slice 3 damage-division residual. Tests in
   `crates/engine/tests/leg_bands_with_other.rs`. Master of the Hunt was **split out to #123** — its
   quality is a card name rather than a supertype, and it is blocked on the token registry's
   real-Scryfall-id convention, not on the rules.
Do not start this before #1 and #4 land; it is the highest-risk work in the set and everything
else in the combat cluster is cheaper.
**Land #119 before slice 3.** Slice 3 grows a "who chooses" axis on `PendingChoice::
AssignCombatDamage`, and #119 has to *move* that same choice from declare blockers to the combat
damage step (CR 510.1a). Doing slice 3 first means building the new axis onto the wrong timing and
then reworking it; #119 first means slice 3 lands on a choice that is already raised where the
rules put it. Slices 1, 2, and 4 are unaffected.

### 4. `landwalk-negation` — 8 cards, M
Depends on: nothing.
"Creatures with <type>walk can be blocked as though they didn't have <type>walk" — eight cards,
one per basic type plus two legendary creatures carrying it as a static. The engine models
landwalk as an evasion check in block legality with no way to switch it off globally.
*Landed:* `StaticEffect::LandwalkNegated { land }` (`mode = "landwalk_negated"`, `land = "swamp"`),
one variant per basic land type. `Game::landwalk_negated` sweeps the battlefield for a live one the
way `Game::cant_block_filter` does — the static names a *keyword*, not a controller, so whose
permanent prints it never matters — and `Game::can_block`'s existing landwalk arm is the only
consultation: `!self.landwalk_negated(land) && lands_with_subtype_controlled(..) > 0`. CR 702.14b's
evasion is checked there, never removed, so the attacker keeps `Keyword::Landwalk` and Island
Sanctuary's "except by creatures with … islandwalk" attack restriction still sees it. Lord Magnus
prints the static twice and both apply: a per-land-type scan finds each on its own, with no
replacement semantics to get wrong.
Ur-Drago, Gosta Dirk and Lord Magnus also print plain first strike, so all three are whole cards
here rather than partial.

### 5. `lose-keyword-until-eot` — 6 cards, M
Depends on: #3 (Shelkin Brownie and Tolaria strip "bands with other"), #4 for the landwalk
vocabulary.
Targeted keyword *removal* until end of turn: all landwalk (Hammerheim), flying (Radjan Spirit),
banding and all "bands with other" (Tolaria), first strike or swampwalk — the controller's choice
(Urborg), and defender-on-block (Elder Land Wurm, which loses it to its own trigger).
*Sketch:* a `LoseKeywords { keywords: KeywordSelector, duration }` effect where the selector can
name a specific keyword, a keyword *family* (all landwalk, all bands-with-other), or a
player-chosen one-of-two. Removal is a continuous effect in the ability-adding/removing layer, so
it must lose to nothing and win over grants applied earlier by timestamp.

### 6. `legendary-filter-axis` — 3 cards, S
Depends on: nothing.
`PermanentFilter` has `nonlegendary` but no positive `legendary`. Karakas ("{T}: Add {W}. / {T}:
Return target legendary creature to its owner's hand."), Arena of the Ancients ("Legendary
creatures don't untap during their controllers' untap steps. / When this artifact enters, tap all
legendary creatures."), Willow Satyr ("You may choose not to untap this creature during your untap
step. / {T}: Gain control of target legendary creature for as long as you control this creature and
this creature remains tapped."), and the four bands-with-other lands (#3) all want the positive
form. *Sketch:* add `legendary: bool` alongside `nonlegendary` in
`crates/cards/src/types/filter.rs` and match it in `Game::permanent_matches`. Arena's enters
trigger also needs `ControlEffect::TapAll` to honor its filter's `controller` axis instead of
hardwiring "you control", so a card that says "all" reaches across the table; Willow Satyr is
otherwise Rubinia Soulsinger's shape (`may_choose_not_to_untap` + `gain_control_while`), which
already lands. Smallest increment in the set; unblocks four others.

### 7. `legendary-landwalk` — 1 card, S
Depends on: #6.
Livonya Silone has legendary landwalk — "can't be blocked as long as defending player controls a
legendary land". Landwalk today keys on basic land *types*; this one keys on a supertype.
*Sketch:* widen the landwalk keyword's payload from a basic land type to a land filter, with the
existing basic-type variants expressed through it. The evasion check then reuses #6's axis.

### 8. `attacking-or-blocking-filter` — 7 cards, S — **LANDED** (wave 2, the union axis only)
Depends on: nothing.
Cards: **Crimson Manticore**, **D'Avenant Archer**, **Lady Caleria**, **Tor Wauki** (all four
faithful), **Tetsuo Umezawa**, **Lesser Werewolf**, **Sentinel**.
"Target attacking or blocking creature" — the union of two axes that both already exist
(`attacking` and, since the 2ed grind, `blocking`). Landed as `PermanentFilter::attacking_or_blocking`
(`crates/cards/src/types/filter.rs`), one bool consulted in `Game::permanent_matches`; the four
archers above are scripted and faithful.
The `CombatState` enum the original sketch proposed was **not** built: `attacking`,
`attacking_you`, `blocking` and `unblocked` are read across `query.rs`, `message.rs` and
`anthem_static`, `PermanentFilter` derives `Default`, and a fifth bool costs one line — collapsing
four working axes into an enum is churn with no fidelity gain.
Two shapes remain, each folded into the increment of the card that actually needs it, because both
of those cards are blocked on further work anyway and an unused axis is dead code:
- "Target **tapped or blocking** creature" (Tetsuo Umezawa) — a second union axis; Tetsuo is also
  blocked on #15 (can't be the target of Aura spells), so it landed there as
  `PermanentFilter::tapped_or_blocking`.
- "Target creature **blocking or blocked by** this creature" (Lesser Werewolf, Sentinel) — the
  *combat-partner* relation, not a global filter: it needs the source threaded against the current
  block assignment. Lesser Werewolf is also blocked on #54 and Sentinel on #22, so it lands with
  whichever of those comes first.
Spurnmage Advocate turned out **not** to belong here at all: its real oracle is "{T}: Return two
target cards from an opponent's graveyard to their hand. Destroy target attacking creature." —
plain `attacking`, no union. The stale blocking-axis note is gone; the card's real defect — a body
that models invented text — is now #120.

### 9. `block-restriction-by-filter` — 3 cards, M
Depends on: nothing.
"Can't be blocked except by Walls and/or creatures with flying" (Elven Riders), "except by Walls"
(Evil Eye of Orms-by-Gore), "except by artifact creatures and/or white creatures" (Seeker).
The engine's evasion checks are hard-coded per keyword. *Sketch:* a
`CantBeBlockedExcept { filter: PermanentFilter }` continuous effect consulted in block legality;
"and/or" is a filter union, so the filter type needs an `any_of` list. Elder Spawn's negative form
("can't be blocked by red creatures", #29) shares this plumbing inverted.

### 10. `attack-ban-by-filter` — 2 cards, M
Depends on: nothing.
Akron Legionnaire and Evil Eye of Orms-by-Gore both ban *their own controller's* other creatures
from attacking, with an exclusion filter ("except creatures named Akron Legionnaire and artifact
creatures" / "Non-Eye creatures you control"). *Sketch:* a `CantAttack { filter }` continuous
effect checked at attack declaration. The filter is evaluated per candidate attacker, so the
name/subtype exclusions ride the existing filter axes; the only new part is the declaration-time
hook, which the attack-requirement work (#56) also wants.

### 11. `mana-battery-counters` — 5 cards, M
Depends on: nothing.
The five mana batteries share one shape: `{2}, {T}: Put a charge counter on this artifact.`, then
`{T}, Remove any number of charge counters from this artifact: Add {B}, then add an additional {B}
for each charge counter removed this way.` The intake sketch had two wrong premises, corrected
here: the mana added is the battery's **own color** (`{W}`/`{U}`/`{B}`/`{R}`/`{G}`, one per card),
not `{C}`; and neither "new thing" was new — the engine has expressed both since the 2ed grind's
storage lands.
*Landed:* nothing. `ActivationCost::remove_counters_x` + `remove_counters_kind` is exactly
"remove any number of counters of `kind` as a cost" (Fungal Reaches' "Remove X storage counters
from this land"), bounds-checked against the source's actual count in `Game::activate_ability`
(CR 602.2b; `X = 0` stays legal, CR 107.3c), and `Effect::Mana(ManaEffect::Add)`'s `repeat = "x"`
is the mana scaled by what that cost paid. The base `{B}` and the per-counter `{B}` are two
`[[abilities.effects]]` blocks — one per oracle clause — and a `Sequence` of mana adds is still a
mana ability (`Effect::is_mana_ability`), so the whole thing resolves without the stack (CR
605.3a). No `Cost::RemoveAnyNumberOfCounters`, no `Amount::CountersRemovedThisWay`, and **no
pending choice**: the removal count rides on `Intent::ActivateAbility`'s `x`, so it is recorded in
the intent log and replay-deterministic by construction rather than by a re-derivation the submit
path would have to pause for. Five cards, all faithful, pure card authoring.

### 12. `filtered-damage-prevention` — 9 cards, L
Depends on: the 2ed prevention-shield increment (#4 there).
2ed's shields prevent the next N damage from a source or all damage to a permanent. Legends adds
a **filter on the damage's source** and, in two cases, on the *relationship* between source and
recipient: by attacking creatures without flying (Al-abara's Carpet), by spells that target it
(Bronze Horse), by enchanted creatures (Enchanted Being, Wall of Putrid Flesh), by Walls (Marble
Priest), by creatures it's blocking (Wall of Shadows, Wall of Vapor), by a black or red source of
your choice (Greater Realm of Preservation), and by anything a targeting spell or ability causes
to damage the creature (Silhouette).
*Sketch:* extend the existing shield with `source_filter: Option<PermanentFilter>` plus a small
`SourceRelation` enum for the two relational cases (`BlockedByThis`, `EnchantedByAnything`).
Silhouette is the odd one — it filters on *why* the damage happened (a targeting spell/ability),
which means the damage event needs to carry its cause. Land that last, or approximate it and say
so.

### 13. `board-state-conditional-anthem` — 4 cards, M
Depends on: nothing.
Static pumps gated on a board-state predicate: "as long as you control no nonartifact, nonwhite
creatures" (Angelic Voices), "as long as an opponent controls a nontoken white permanent" (Beasts
of Bogardan, Ivory Guardians), "as long as it's not attacking" applied per-creature (Arcades
Sabboth). *Sketch:* the existing anthem effect grows a `condition: Option<Condition>` evaluated
continuously; the per-creature form (Arcades Sabboth) needs the condition evaluated against each
affected permanent rather than globally, so the condition takes the candidate as its subject.
`nontoken` is a new filter axis.

### 14. `becomes-blocked-color-change` — 1 card, M
Depends on: nothing.
Aisling Leprechaun: "Whenever this creature blocks or becomes blocked by a creature, that creature
becomes green. (This effect lasts indefinitely.)" *Sketch:* a `SetColor` effect with no duration —
an indefinite continuous effect in the color-changing layer, applied to the triggering combat
partner. "Indefinite" (not "until end of turn") means it survives combat and turn cleanup, which
the current duration enum cannot express; add a `Duration::Indefinite` that the cleanup step
skips.

### 15. `cant-be-targeted-by` — 3 cards, S — **LANDED**
Depends on: nothing.
Cards: **Bartel Runeaxe**, **Tetsuo Umezawa**, **Anti-Magic Aura** (all three faithful).
"Can't be the target of Aura spells" (Bartel Runeaxe, Tetsuo Umezawa) and "can't be the target of
spells and can't be enchanted by other Auras" (Anti-Magic Aura). Landed as
`StaticEffect::CantBeTargetedBy { spells: SpellFilter, attached: bool }`
(`crates/cards/src/types/effect/static.rs`), enforced by `Game::cant_be_targeted_by_spell` in the
same `retain` as shroud/hexproof/protection, so an Aura or spell aimed at a shielded permanent is
rejected with `Reject::IllegalTarget`. The sketch's single `filter` field grew the `attached` scope
flag so one variant serves both "*this creature* can't be the target" (self-shield) and "*enchanted
creature* can't be the target" (Anti-Magic Aura), keeping enforcement to one battlefield scan; a
`cant_be_targeted_by` field on `GrantToAttached` was rejected because the two self-shielding legends
would still have needed a standalone variant. Deliberately narrower than `Keyword::Shroud`: the
scan only fires when the source isn't a battlefield permanent, so an activated ability still targets
through it (CR 111.1).
"Can't be enchanted by other Auras" needed **no new engine work** — `GrantToAttached`'s existing
`cant_be_enchanted` flag already feeds both `Game::host_cant_be_enchanted_by` (the cast-time refusal,
CR 303.4a) and `Game::attachment_host_legal` (the continuous CR 704.5n sweep that puts an Aura which
*became* illegally attached into its owner's graveyard). Anti-Magic Aura scripts its two halves as
two separate `[[abilities]]` static blocks, since two effects in one block fold into
`Effect::Sequence` and become invisible to both static scanners.
**Also owned Tetsuo Umezawa's targeting**, deferred here by #8: "{U}{B}{B}{R}, {T}: Destroy target
tapped or blocking creature." shipped as `PermanentFilter::tapped_or_blocking`, the twin of #8's
`attacking_or_blocking` — one bool read against `Game::is_tapped` and `CombatState::blocks`. A union
rather than two axes because blocking never taps (CR 509.1), so intersecting them would match
nothing.

### 16. `arboria-attack-restriction` — 1 card, M
Depends on: #2.
Arboria: "Creatures can't attack a player unless that player cast a spell or put a nontoken
permanent onto the battlefield during their last turn." Needs a per-player, per-turn memory of two
distinct events surviving into the *following* turn. *Sketch:* two per-player turn flags
(`cast_a_spell`, `put_nontoken_permanent_onto_battlefield`) set by the existing cast and ETB
paths, snapshotted at end of turn as "last turn's" values, and read by the attack-declaration
legality check. Their turn is "last turn" relative to the attack, which for a 4-player game means
the most recent turn *that player* took, not the previous turn in sequence.

### 17. `dont-untap-by-filter` — 1 card, S — absorbed into #6
Depends on: #6.
Arena of the Ancients: "Legendary creatures don't untap during their controllers' untap steps",
plus an ETB tap-all. **Premise was wrong:** the engine already has the filtered global —
`StaticEffect::DoesntUntap { filter }`, which Meekstone's "creatures with power 3 or greater don't
untap during their controllers' untap steps" has used since the 2ed grind — so no new continuous
effect was needed. Arena needed only #6's positive `legendary` axis plus `ControlEffect::TapAll`
honoring its filter's `controller` axis, and landed with #6.

### 18. `counter-spell-targeting-your-permanent` — 2 cards, S — **LANDED** (wave 4)
Depends on: nothing.
Avoid Fate and Ring of Immortals both counter "target instant or Aura spell that targets a
permanent you control".
*Landed:* `SpellFilter::InstantOrAuraTargetsPermanentYouControl`
(`filter = "instant_or_aura_targets_permanent_you_control"`), matched **inline** in
`Game::legal_targets_for`'s `SpellOnStack` enumeration rather than in
`Game::spell_matches_filter` — that enumeration is the only choke that holds the *counterer* apart
from the candidate spell's own controller, and "you" here is the counterer. `Color` and
`ManaValueEqualsX` special-case in the same place for the same reason; both filter functions carry a
comment saying so. Because it re-evaluates live, the CR 608.2b resolution re-check (the same
function, via `target_still_legal`) sees a target permanent change control in response and the
counterspell fizzles. Any one of the candidate's targets qualifying is enough (CR 608.2b: a spell may
have more than one target). Tests: `crates/engine/tests/leg_counter_spell_targeting_yours.rs`.
No general And-combinator over `SpellFilter` exists — the instant-or-Aura half is matched in the
same arm; see #105 if a second card wants the pieces separately.

### 19. `damage-dealt-this-turn-ledger` — 3 cards, L
Depends on: nothing.
Three cards read a *history* of damage the engine does not keep: "half the damage dealt by one of
those sorcery spells this turn" (Backdraft), "damage dealt to this creature this turn by other
sources named Blazing Effigy" (Blazing Effigy), and "all damage that would be dealt this turn by
target sorcery spell is dealt to that spell's controller instead" (Reverberation).
*Sketch:* a turn-scoped damage ledger — `(source_id, source_name, target, amount)` rows appended
by the damage path, cleared at cleanup — with `Amount` variants that query it. Reverberation is
not a ledger read but a *redirection registered against a spell object*, so it also needs the
damage path to consult per-source redirections (shared with #60). Sizeable because the ledger
touches every damage site; do it once, carefully.

### 20. `damage-triggered-aura-payback` — 2 cards, M
Depends on: nothing.
Backfire ("whenever enchanted creature deals damage to you, this Aura deals that much damage to
that creature's controller") and Relic Bind ("whenever enchanted artifact becomes tapped, choose
one — …"). *Sketch:* two new trigger tags — `enchanted_deals_damage_to_you` carrying the amount
as a trigger-scoped `Amount`, and `enchanted_becomes_tapped`. Relic Bind's modal half reuses the
existing modal-effect surface.

### 21. `blood-lust-conditional-pump` — 1 card, S — **LANDED** (wave 4)
Depends on: nothing.
Blood Lust: "+4/-4 if toughness 5 or greater, otherwise +4/-X where X is its toughness minus 1"
— i.e. always leaves the creature with at least 1 toughness. The intake's "collapses to arithmetic"
reading was **checked against Scryfall and is exactly right**.
*Landed:* pure data, no engine change — the existing conditional/arithmetic `Amount` surface already
expressed it, so `blood_lust.toml`'s toughness delta is a `compare` on `target_toughness` against 5
with the else branch computing `(target_toughness - 1) * -1`. X is computed once at resolution
(CR 608.2g) and locked in as a fixed `TempBoost` delta (CR 613.4), so later toughness changes stack
on top of it rather than recomputing the pump — the two locking tests in
`crates/engine/tests/leg_blood_lust.rs` cover both directions, and a -1/-1 counter afterward can
still kill a creature Blood Lust "saved".

### 22. `change-base-power-toughness` — 5 cards, M
Depends on: nothing.
"Change the base power and toughness of … to X/Y (This effect lasts indefinitely.)" — Brine Hag
(0/2 to everything that damaged it), Halfdane (copy a target's P/T until the end of your next
upkeep), Sentinel (base toughness to 1 plus a blocker's power), Wall of Tombstones (base toughness
to 1 plus creature cards in your graveyard), and Transmutation (switch P/T until end of turn).
This is layer 7b work the engine has not needed before — it must sit *below* +N/+N counters and
pump effects, not compose with them arbitrarily.
*Sketch:* a `SetBasePowerToughness { power: Option<Amount>, toughness: Option<Amount>, duration }`
continuous effect applied in a distinct sub-layer ahead of the additive modifiers, with the
`Amount` snapshotted at application (CR 613.4 — the value is locked in, not recomputed). Halfdane
needs `Duration::UntilEndOfYourNextUpkeep`; the rest need #14's `Indefinite`. Transmutation's
switch is the same layer with a swap rather than a set.
Sentinel also needs #8's deferred **combat-partner relation** for "target creature blocking or
blocked by this creature" — see #54, which wants the same thing; whichever of the two lands first
builds it.

### 23. `attacker-blocker-count-cap` — 1 card, M
Depends on: #2.
Caverns of Despair: "No more than two creatures can attack each combat. No more than two creatures
can block each combat." A global cap across *all* players, not per player.
*Sketch:* a `MaxAttackersPerCombat` / `MaxBlockersPerCombat` continuous value consulted at
declaration, with the running count scoped to the combat phase. Declaration is already validated
in one place, so this is a legality predicate rather than new orchestration — the work is the
per-combat counter and its reset.

### 24. `chains-of-mephistopheles` — 1 card, L
Depends on: nothing.
"If a player would draw a card except the first one they draw in each of their draw steps, that
player discards a card instead. If the player discards a card this way, they draw a card. If the
player doesn't discard a card this way, they mill a card." A draw *replacement* effect with a
nested conditional, and famously the hardest-templated card in the set.
*Sketch:* a draw-replacement hook (the engine has none) plus a per-player, per-draw-step draw
counter to identify the exempt first draw. The replacement's body is itself a draw, which must not
re-trigger the replacement for the same event (CR 614.5). Land the replacement hook first as its
own slice; the card body is small once the hook exists.

### 25. `any-player-may-activate` — 2 cards, M
Depends on: nothing.
Land's Edge ("Any player may activate this ability") and Clergy of the Holy Nimbus ("Only your
opponents may activate this ability"). `AbilityToml` has `only_owner_may_activate` but no way to
say *anyone* or *only opponents*. *Sketch:* replace that bool with an
`activator: Activator { Controller, Opponents, AnyPlayer }` enum, defaulting to `Controller`, and
thread it through the activation-legality check and the priority-holder's available-actions list.
Migrate the existing `only_owner_may_activate` users in the same change.

### 26. `counter-gated-untap-suppression` — 2 cards, M
Depends on: nothing.
Cocoon and Venarian Gold both read "doesn't untap during [its controller's] untap step if it has a
<kind> counter on it", with an upkeep trigger removing one. *Sketch:* a `DontUntap` continuous
effect (shared with #17) whose condition is a counter-presence check on a named counter kind, plus
the two counter kinds. Cocoon's "If you can't, sacrifice it, put a +1/+1 counter…, and that
creature gains flying" is an if-you-can't fallback on the remove-a-counter effect, which the
existing remove-counter path does not report.

### 27. `game-is-a-draw` — 1 card, S
Depends on: nothing.
Divine Intervention. The engine has win/lose outcomes but no draw. *Sketch:* a `GameOutcome::Draw`
alongside the existing outcomes and an effect that sets it, plus the "when you remove the last
intervention counter from this enchantment" trigger shape — an ordinary triggered ability on the
*removal* (CR 603.2), not a state trigger on the count reaching zero.

Landed: `CounterKind::Intervention`, `MiscEffect::GameIsADraw` → `Event::GameDrawn`,
`Trigger::YouRemoveLastCounterFromThis { kind }` (TOML timing
`you_remove_last_counter_from_this` + `counter_kind` sibling), `Game::outcome() -> Option<GameOutcome>`,
and a post-draw `submit` gate. Tests: `crates/engine/tests/leg_game_is_a_draw.rs`.

### 28. `becomes-color-of-your-choice` — 1 card, M
Depends on: #14.
Dream Coat: "Enchanted creature becomes the color or colors of your choice. Activate only once each
turn." *Sketch:* #14's `SetColor` effect taking a player-chosen color *set* (one or more) rather
than a fixed color — a pending choice at resolution. `once_each_turn` already exists on
`AbilityToml`.

### 29. `elder-spawn` — 1 card, S
Depends on: #9 (shares the block-restriction plumbing, inverted).
"At the beginning of your upkeep, unless you sacrifice an Island, sacrifice this creature and it
deals 6 damage to you" plus "can't be blocked by red creatures". *Sketch:* the block restriction
is #9's filter with the sense flipped (`CantBeBlockedBy { filter }`); the upkeep half is an
unless-you-pay trigger where the "cost" is a sacrifice rather than mana, which the existing
upkeep-tax shape (Chromium, Arcades Sabboth) can grow into by widening its cost type.

### 30. `move-attached-aura` — 1 card, M
Depends on: nothing.
Enchantment Alteration: "Attach target Aura attached to a creature or land to another permanent of
that type." *Sketch:* a `MoveAura { target_aura, new_host }` effect calling the existing attach
path, with a legality check that the new host matches the Aura's enchant restriction. The
two-target shape where the second target's legality depends on the first is new — the target
chooser must resolve them in order.

### 31. `tap-creature-for-mana-by-mana-value` — 1 card, S
Depends on: nothing.
Energy Tap: "Tap target untapped creature you control. If you do, add {C} equal to that creature's
mana value." *Sketch:* an `Amount::TargetManaValue` read at resolution, feeding the existing
add-mana effect. The if-you-do gate is the existing conditional-on-cost-paid shape.

### 32. `granted-land-ability-with-conditional-counter` — 1 card, M
Depends on: nothing.
Equinox: enchanted land gains "{T}: Counter target spell if it would destroy a land you control."
Two new things: granting an *activated* ability to the enchanted permanent, and a counter whose
effect is conditional on what the countered spell would do. *Sketch:* an aura effect that grants a
full `Ability` (the DSL can already express the ability; it cannot express granting one), plus a
`SpellFilter::would_destroy_land_you_control` — which is a prediction, so implement it as "the
spell has a destroy effect whose filter could match a land you control", and mark the
approximation.

### 33. `eureka` — 1 card, M
Depends on: nothing.
"Starting with you, each player may put a permanent card from their hand onto the battlefield.
Repeat this process until no one puts a card onto the battlefield." *Sketch:* a repeating
round-robin optional choice — a pending-choice loop in the submit path that terminates when a
full cycle passes with no player acting. Determinism matters: the loop state (whose turn to
choose, whether anyone acted this cycle) is orchestration state that must survive intent replay.

### 34. `feint` — 1 card, M
Depends on: #12.
Feint: "Tap all creatures blocking target attacking creature. Prevent all combat damage that would
be dealt this turn by that creature and each creature blocking it." *Sketch:* a combat-group
selector (an attacker plus everything blocking it) reused for both halves, and a prevention shield
scoped to *damage dealt by* a set of permanents rather than damage dealt to one — the opposite
direction from every shield built so far.

### 35. `revealed-zone-visibility` — 2 cards, M
Depends on: #2.
Field of Dreams (top card of every library revealed) and Revelation (all hands revealed). These
are the only cards in the set that change the **server-side visibility filter**, which is a hard
rule (hands and libraries are private). *Sketch:* per-player visibility overrides
(`hand_revealed_to_all`, `library_top_revealed_to_all`) derived from continuous effects and read
by the projection layer, not by the engine's own logic. The engine change is small; the projection
and client change is the real work, and it must fail *closed* — an override that fails to apply
leaks nothing.

### 36. `firestorm-phoenix` — 1 card, L
Depends on: nothing.
"If this creature would die, return it to its owner's hand instead. Until that player's next turn,
that player plays with that card revealed in their hand and can't play it." *Sketch:* a dies-
replacement (the engine has regeneration but not a general death replacement), plus a per-card
"revealed and unplayable until <player>'s next turn" marker that follows a *specific card object*
into the hand — which means the hand needs per-card annotations that survive the zone change.
Interacts with #35's visibility work.

### 37. `attacks-unblocked-trigger` — 1 card, M
Depends on: nothing.
Floral Spuzzem: "Whenever this creature attacks and isn't blocked, you may destroy target artifact
defending player controls. If you do, this creature assigns no combat damage this turn."
*Sketch:* an `attacks_and_isnt_blocked` trigger firing at the declare-blockers step once blocks are
locked in, plus an `AssignsNoCombatDamage` marker the damage step checks. The "defending player"
scoping is per-attack, so the trigger carries the defender.

### 38. `damage-reduction-replacement` — 1 card, M
Depends on: #12.
Forethought Amulet: "If an instant or sorcery source would deal 3 or more damage to you, it deals 2
damage to you instead." Not prevention — a *replacement* that rewrites the amount, gated on the
source type and a threshold. *Sketch:* a `ReplaceDamageAmount { source_filter, when: Condition,
new_amount }` hook in the damage path, ordered after prevention (CR 615.9 — the recipient chooses
the order, but with one effect it does not matter here).

### 39. `gabriel-angelfire` — 1 card, M
Depends on: #1, #5.
"At the beginning of your upkeep, choose flying, first strike, trample, or rampage 3. Gabriel
Angelfire gains that ability until your next upkeep." *Sketch:* a modal grant where the mode set is
a fixed keyword list and the duration is `UntilYourNextUpkeep` (#22 needs the same duration for
Halfdane). The grant replaces the previous one implicitly by expiring, so no explicit removal is
needed.

### 40. `exchange-control` — 2 cards, M
Depends on: nothing.
Gauntlets of Chaos (exchange two chosen permanents sharing a type, then destroy Auras attached to
them) and Juxtapose (exchange the greatest-mana-value creature you each control, then the same for
artifacts). *Sketch:* an `ExchangeControl { a, b }` effect on top of the existing control-change
path, with the "shares one of those types" legality check for Gauntlets and a
greatest-mana-value selector with a controller-chosen tiebreak for Juxtapose. Control changes are
already continuous effects, so the exchange is two of them applied together.

### 41. `delayed-chosen-landwalk` — 1 card, M
Depends on: #4 (the landwalk vocabulary), #5 (grants).
Giant Slug: "{5}: At the beginning of your next upkeep, choose a basic land type. This creature
gains landwalk of the chosen type until the end of that turn." *Sketch:* a delayed trigger whose
resolution prompts for a basic land type and applies a keyword grant with a
`UntilEndOfThatTurn` duration. The choice happens on the *delayed* resolution, not at activation.

### 42. `attacked-last-turn-restriction` — 2 cards, M
Depends on: nothing.
Giant Turtle ("can't attack if it attacked during your last turn") and Wall of Dust ("whenever this
creature blocks a creature, that creature can't attack during its controller's next turn").
*Sketch:* a per-permanent `attacked_on_turn: Option<TurnId>` stamp set at attack declaration, read
by the attack-legality check; Wall of Dust instead applies a targeted "can't attack during your
next turn" marker to another creature, which needs the marker to survive until that player's next
turn ends. Both are #10's `CantAttack` with a temporal condition.

### 43. `glyph-cycle` — 5 cards, L
Depends on: nothing.
The five Glyphs all key on "creatures that <target Wall> blocked this turn" — a per-Wall memory of
what it blocked, surviving past the combat in which it blocked. On top of that: glyph counters with
a granted untap-suppression and upkeep-removal pair (Glyph of Delusion), a prevent-then-destroy-at-
end-step combo (Glyph of Destruction), a delayed end-of-combat mass destroy (Glyph of Doom), a
damage-triggered lifegain watcher (Glyph of Life), and a destroy-then-reanimate-from-the-right-
graveyard (Glyph of Reincarnation, which needs "the player who controlled that creature the last
time it became blocked by that Wall").
*Sketch:* a turn-scoped `blocked_by_this_turn: Vec<(blocker, attacker, controller_at_the_time)>`
ledger appended at declare-blockers and cleared at cleanup. Every Glyph is a small effect once the
ledger exists; Glyph of Delusion additionally needs #26's counter-gated untap suppression *granted*
to another creature (#32's ability-granting).

### 44. `global-characteristic-rewrite` — 2 cards, L
Depends on: #2, #5.
Gravity Sphere ("All creatures lose flying") and Living Plane ("All lands are 1/1 creatures that
are still lands"). The first is #5's removal applied globally and continuously; the second is the
engine's first *type-changing* continuous effect — lands become creatures without ceasing to be
lands, which reorders type-based legality across the whole engine (they can now attack, be
destroyed by creature removal, be affected by anthems, and die to lethal damage).
*Sketch:* a `SetTypes { filter, add_types, base_power_toughness }` continuous effect in the
type-changing layer, applied before P/T layers so #22's work composes. Land this after #22 — the
0/0-plus-counters interaction is where type-changing effects usually break.

### 45. `hazezon-tamar` — 1 card, M
Depends on: nothing.
"When Hazezon enters, create X 1/1 Sand Warrior creature tokens … at the beginning of your next
upkeep, where X is the number of lands you control at that time. When Hazezon leaves the
battlefield, exile all Sand Warriors." *Sketch:* an ETB trigger that registers a delayed trigger
for the next upkeep, with X evaluated on the *delayed* resolution; plus an LTB trigger exiling by
creature type, which is the existing exile-by-filter with a subtype axis. The token count coming
from a later evaluation is the only genuinely new part.

### 46. `hellfire` — 1 card, S
Depends on: nothing.
"Destroy all nonblack creatures. Hellfire deals X plus 3 damage to you, where X is the number of
creatures that died this way." *Sketch:* the destroy-all already exists; the new part is an
`Amount` reading how many permanents the *preceding effect in this resolution* destroyed. A
resolution-scoped counter threaded through the effect list, not a turn-scoped one.

### 47. `imprison` — 1 card, L
Depends on: nothing.
Two pay-or-destroy-this-Aura triggers: one on the enchanted creature activating a non-mana {T}
ability (counter that ability), one on it attacking or blocking (tap it, **remove it from
combat**, and un-block anything it was solely blocking). *Sketch:* removal from combat is the
expensive piece — the combat assignment must support withdrawing a creature mid-combat and
recomputing which attackers become unblocked. The "counter that ability" half needs abilities on
the stack to be counterable targets, which the engine may not model separately from spells.

### 48. `counter-unless-pays-x` — 3 cards, M
Depends on: #2 (two of the three are World enchantments).
In the Eye of Chaos and Nether Void (counter unless the caster pays a tax) and Invoke Prejudice
(same, gated on the spell not sharing a color with a creature you control). The engine has
counter effects but no "unless that player pays" prompt during another player's cast.
*Sketch:* a `CounterUnlessPays { amount: Amount }` effect where the amount can be fixed ({3}) or
the countered spell's mana value, resolved through a pending payment choice by the spell's
controller. Invoke Prejudice's condition needs a "shares a color with a creature you control"
predicate over the caster's opponents' board.

### 49. `blocks-with-toughness-filter-then-counter` — 2 cards, M
Depends on: #8.
Infinite Authority: "Whenever enchanted creature blocks or becomes blocked by a creature with
toughness 3 or less, destroy the other creature at end of combat. At the beginning of the next end
step, if that creature was destroyed this way, put a +1/+1 counter on the first creature."
Wall of Caltrops' block-count condition is #87.
*Sketch:* `PermanentFilter` needs `toughness_max` (it has `power_min`/`power_max` but no toughness
bounds), plus the delayed-destroy-at-end-combat shape (which Thicket Basilisk already uses) and a
second delayed trigger conditional on the first having resolved — an intervening-if on a delayed
trigger, which the engine has not needed.

### 50. `johan` — 1 card, M
Depends on: nothing.
"At the beginning of combat on your turn, you may have Johan gain 'Johan can't attack' until end of
combat. If you do, attacking doesn't cause creatures you control to tap this combat if Johan is
untapped." *Sketch:* a conditional vigilance-for-your-whole-team effect whose condition is checked
at attack declaration (Johan untapped), plus #10's `CantAttack` self-applied. The team-wide
"attacking doesn't cause tapping" is the existing vigilance check widened to a controller-scoped
continuous effect.

### 51. `opponents-permanents-enter-tapped` — 1 card, S
Depends on: nothing.
Kismet: "Artifacts, creatures, and lands your opponents control enter tapped." *Sketch:* an
ETB-replacement continuous effect with a filter, applied in the existing enters-tapped path (which
today only reads the permanent's own `enters_tapped`).

### 52. `exiled-with-this-face-down` — 1 card, M
Depends on: nothing.
Knowledge Vault: exile cards face down "with this artifact", return them all on sacrifice, dump
them to the graveyard on LTB. *Sketch:* an "exiled with" association between an exiled card and a
source permanent (the engine exiles cards but does not track *which* permanent exiled them), plus
face-down exile visibility. Two effects then reference the set.

### 53. `land-etb-sacrifice-replacement` — 1 card, M
Depends on: nothing.
Land Equilibrium: "If an opponent who controls at least as many lands as you do would put a land
onto the battlefield, that player instead puts that land onto the battlefield then sacrifices a
land of their choice." *Sketch:* a land-ETB replacement (the engine has no per-type ETB
replacement hook) with a comparative land-count condition, whose body is an ordinary sacrifice
choice. The replacement fires on *any* way a land enters, not only land drops.

### 54. `lesser-werewolf` — 1 card, S
Depends on: #8.
"{B}: If this creature's power is 1 or more, it gets -1/-0 until end of turn and put a -0/-1 counter
on target creature blocking or blocked by this creature. Activate only during the declare blockers
step." *Sketch:* a `-0/-1` counter kind, a power-threshold activation condition, and an
`only_during_declare_blockers` ability restriction alongside the existing
`only_during_your_upkeep`.
The targeting is the **combat-partner relation** #8 deferred: "blocking or blocked by *this
creature*" is not a global axis but a pairing, so it needs the filter's own `source` threaded
against `CombatState::blocks` — match a `(blocker, attacker)` pair in either direction. Sentinel
(#22) is the only other card that wants it; whichever of the two lands first builds it.

### 55. `granted-regenerate-via-counter` — 1 card, M
Depends on: #32 (granting an ability).
Life Matrix: "{4}, {T}: Put a matrix counter on target creature and that creature gains 'Remove a
matrix counter from this creature: Regenerate this creature.' Activate only during your upkeep."
*Sketch:* #32's ability grant, where the granted ability's cost is a counter removal on the *host*.
The grant is indefinite (#14's duration) and stacks — a creature can receive several.

### 56. `must-block-by-filter` — 1 card, M
Depends on: #12 (Marble Priest's other half).
Marble Priest: "All Walls able to block this creature do so." A block *requirement* rather than a
restriction — CR 509.1c, the harder half of the declare-blockers legality check, because the
engine must verify the declared blocks satisfy the maximum number of requirements.
*Sketch:* a `MustBlock { blocker_filter, attacker }` requirement collected at declare-blockers and
checked as "no other legal block assignment satisfies more requirements". With one requirement in
the pool this collapses to "every able Wall must be blocking it"; implement the general check
anyway — requirements compound badly if approximated.

### 57. `exchange-life-totals` — 1 card, S
Depends on: nothing.
Mirror Universe. *Sketch:* an `ExchangeLifeTotals { target_opponent }` effect. Life setting already
exists; the only subtlety is that the exchange is a single event, so life-gain/life-loss triggers
see one change each (CR 118.7).

### 58. `name-a-card` — 2 cards, M
Depends on: nothing.
Nebuchadnezzar ("choose a card name; target opponent reveals X cards at random from their hand;
discard all with that name") and Petra Sphinx ("target player chooses a card name, then reveals the
top card of their library"). *Sketch:* a `Choice::CardName` pending choice — free text validated
against the card catalog — plus **random selection from a hidden zone**, which needs the injected
RNG (the engine takes no randomness it is not given) and must stay replay-deterministic. Petra
Sphinx's chooser is the *targeted* player, not the controller.

### 59. `spend-mana-as-any-type` — 1 card, M
Depends on: nothing.
North Star: "For one spell this turn, you may spend mana as though it were mana of any type to pay
that spell's mana cost." *Sketch:* a turn-scoped, one-shot payment relaxation flag consulted by the
mana-payment validator. "One spell" means it is consumed by the first cast that uses it, which the
player chooses implicitly — so the flag is offered as an option during payment rather than applied
automatically.

### 60. `damage-redirection` — 2 cards, L
Depends on: #12, #19.
Nova Pentacle ("the next time a source of your choice would deal damage to you this turn, that
damage is dealt to target creature of an opponent's choice instead") and Shimian Night Stalker
("all damage that would be dealt to you this turn by target attacking creature is dealt to this
creature instead"). *Sketch:* a redirection registry parallel to the prevention shields —
`RedirectDamage { source_filter, new_recipient, uses }` consulted in the damage path *after*
prevention. Nova Pentacle's new recipient is chosen by an **opponent** at the time the shield is
created, which is a pending choice held by a non-controller.

### 61. `x-target-creatures` — 2 cards, M
Depends on: nothing.
Part Water and Winter Blast both take "X target creatures" — a target *count* determined by X.
*Sketch:* the targeting system takes a fixed arity today; it needs `TargetCount::Variable(Amount)`
resolved at cast time from the announced X, with the usual "as many as possible if fewer are
legal" rule. Winter Blast's damage half is #92.

### 62. `exact-power-toughness-filter` — 1 card, S
Depends on: nothing.
Pendelhaven: "Target 1/1 creature gets +1/+2 until end of turn." *Sketch:* `power_min`/`power_max`
exist; add `toughness_min`/`toughness_max` (#49 wants `toughness_max` too) and express 1/1 as all
four bounds set to 1.

### 63. `primordial-ooze` — 1 card, M
Depends on: nothing.
"Attacks each combat if able. At the beginning of your upkeep, put a +1/+1 counter on this creature.
Then you may pay {X}, where X is the number of +1/+1 counters on it. If you don't, tap this creature
and it deals X damage to you." *Sketch:* an attack *requirement* on a single creature (the mirror of
#10's ban, and much cheaper than #56's filtered block requirement), plus an optional payment whose
amount is a live counter count with an if-you-don't consequence.

### 64. `discarded-from-hand-trigger` — 1 card, M
Depends on: nothing.
Psychic Purge: "When a spell or ability an opponent controls causes you to discard this card, that
player loses 5 life." A trigger that fires **from the hand**, and only for opponent-caused
discards. *Sketch:* the trigger system watches the battlefield, stack, and graveyard; it needs a
hand-scoped trigger check for this shape, and the discard event needs to carry its cause (which
player's spell or ability, or a turn-based action).

### 65. `puppet-master` — 1 card, M
Depends on: nothing.
"When enchanted creature dies, return that card to its owner's hand. If that card is returned this
way, you may pay {U}{U}{U}. If you do, return this card to its owner's hand." The Aura triggers on
its own host dying and then returns *itself* from the graveyard. *Sketch:* a leaves-the-battlefield
trigger whose effect targets the Aura's own card in the graveyard, with an optional-payment gate.
The Aura is in the graveyard by the time the trigger resolves, so the effect must address it as a
card object, not a permanent.

### 66. `change-land-mana-production` — 1 card, M
Depends on: nothing.
Quarum Trench Gnomes: "If target Plains is tapped for mana, it produces colorless mana instead of
white mana. (This effect lasts indefinitely.)" *Sketch:* a mana-production replacement applied per
permanent — the engine resolves mana abilities directly with no interception point. Add one, keyed
by the produced color, and make it #14-indefinite.

### 67. `pump-per-attached-aura` — 1 card, S
Depends on: nothing.
Rabid Wombat: "+2/+2 for each Aura attached to it." *Sketch:* an `Amount::AurasAttachedToThis` in
the existing per-permanent-count amount family, read continuously by a self-anthem.

### 68. `rasputin-dreamweaver` — 1 card, M
Depends on: #12.
Seven dream counters that are simultaneously a mana source and a prevention resource, an upkeep
regrowth conditional on having started the turn untapped, and a hard cap of seven. *Sketch:* the
counter-as-mana-cost and counter-as-prevention-cost are two ordinary activated abilities; the new
parts are a **counter maximum** (a continuous "can't have more than N" enforced as a replacement on
counter placement, CR 122.6) and a `started_the_turn_untapped` per-permanent flag stamped in the
untap step.

### 69. `recall` — 1 card, S
Depends on: nothing.
"Discard X cards, then return a card from your graveyard to your hand for each card discarded this
way. Exile Recall." *Sketch:* an `Amount::CardsDiscardedThisWay` scoped to the resolution (#46's
resolution-scoped counter), plus self-exile on resolution, which the engine has for other cards.

### 70. `dies-this-turn-delayed-trigger` — 1 card, S
Depends on: nothing.
Reincarnation: "Choose target creature. When that creature dies this turn, return a creature card
from its owner's graveyard to the battlefield under the control of that creature's owner."
*Sketch:* a delayed trigger keyed to a specific permanent dying, expiring at end of turn. The
existing delayed-trigger machinery covers timing; the new part is keying one to a permanent
identity rather than to a step.

### 71. `remove-enchantments` — 1 card, M
Depends on: nothing.
"Return to your hand all enchantments you both own and control, all Auras you own attached to
permanents you control, and all Auras you own attached to attacking creatures your opponents
control. Then destroy all other enchantments you control, all other Auras attached to permanents
you control, and all other Auras attached to attacking creatures your opponents control."
*Sketch:* the filter needs an **owner** axis distinct from controller (the engine tracks owner but
does not filter on it) and an attachment-host axis ("attached to a permanent you control",
"attached to an attacking creature an opponent controls"). Both halves then reuse existing bounce
and destroy effects.

### 72. `reset` — 1 card, S
Depends on: nothing.
"Cast this spell only during an opponent's turn after their upkeep step. Untap all lands you
control." *Sketch:* untap-all-by-filter exists in spirit (mass untap); the new part is the cast
restriction, which is a step/turn-ownership predicate alongside the existing
`cast_only_during_declare_attackers`. Generalize that field to a small timing-restriction enum
rather than adding a second bool.

### 73. `rohgahh-of-kher-keep` — 1 card, M
Depends on: nothing.
"At the beginning of your upkeep, you may pay {R}{R}{R}. If you don't, tap Rohgahh and all creatures
named Kobolds of Kher Keep, then an opponent gains control of them." *Sketch:* an unless-you-pay
upkeep trigger (shared with #29) whose failure branch is a mass tap plus a control change to an
opponent — and *which* opponent is a choice the controller makes (CR 616 / "an opponent" means the
effect's controller chooses). The by-name anthem half is ordinary.

### 74. `stangg-twin` — 1 card, M
Depends on: nothing.
"When Stangg enters, create Stangg Twin, a legendary 3/4 red and green Human Warrior creature token.
Exile that token when Stangg leaves the battlefield. Sacrifice Stangg when that token leaves the
battlefield." A **linked pair** — each half watches the other. *Sketch:* the ETB trigger records the
created token's id on Stangg's permanent state, and two delayed triggers key off that id. The token
is legendary and named, so the legend rule applies to it.

### 75. `four-minus-cards-in-hand` — 1 card, S
Depends on: #2.
Storm World: "deals X damage to that player, where X is 4 minus the number of cards in their hand."
*Sketch:* an arithmetic `Amount` — `Amount::Difference { from: 4, subtract: CardsInHand(who) }` —
clamped at zero (a negative X deals no damage, CR 107.1b). The engine's amounts are all
non-negative counts today, so the clamp belongs in the amount, not the damage effect.

### 76. `sacrifice-any-number-as-cost` — 1 card, M
Depends on: nothing.
Sword of the Ages: "{T}, Sacrifice this artifact and any number of creatures you control: deals X
damage to any target, where X is the total power of the creatures sacrificed this way, then exile
this artifact and those creature cards." *Sketch:* a `Cost::SacrificeAnyNumber { filter }` (a
player-chosen set at activation, like #11's counter cost), an `Amount::TotalPowerSacrificedThisWay`
snapshotting power *as the cost was paid* (they are in the graveyard by resolution), and a
graveyard-exile of exactly those cards.

### 77. `sylvan-library` — 1 card, M
Depends on: nothing.
"At the beginning of your draw step, you may draw two additional cards. If you do, choose two cards
in your hand drawn this turn. For each of those cards, pay 4 life or put the card on top of your
library." *Sketch:* a per-card "drawn this turn" annotation on hand cards (#36 wants per-card hand
annotations too), plus a two-of-N choice with a per-card either/or payment. The put-on-top ordering
matters when both are returned.

### 78. `takklemaggot` — 1 card, L
Depends on: nothing.
An Aura that, when its host dies, returns to the battlefield under *your* control attached to a
creature the dead creature's *controller* chooses — or, if they can't or won't, returns as a
**non-Aura enchantment** that loses "enchant creature" and gains a wholly different triggered
ability. *Sketch:* the engine has no way for a permanent to change its own ability set and subtype
on re-entry. Model it as two distinct permanent shapes selected at ETB by a flag set on the return
effect — cleaner than mutating abilities, and honest about what is happening. The opponent-chooses-
the-host part is a pending choice held by a non-controller (#60 has the same shape).

### 79. `skip-next-two-untap-steps` — 1 card, S
Depends on: #26.
Telekinesis: "Tap target creature. Prevent all combat damage that would be dealt by that creature
this turn. It doesn't untap during its controller's next two untap steps." *Sketch:* #26's untap
suppression with a *count* (skip the next N) rather than a counter-presence condition, decremented
in the untap step. The prevention half is #34's damage-dealt-by shield with a single creature.

### 80. `opponent-chooses-the-target` — 1 card, M
Depends on: #2.
The Abyss: "At the beginning of each player's upkeep, destroy target nonartifact creature that
player controls of their choice." The target is chosen by the *upkeep player*, not by the
enchantment's controller. *Sketch:* the target chooser becomes an axis on the targeting request
(`chosen_by: Who`) rather than always the ability's controller — a pending choice routed to a
different player. #60 and #78 need the same routing; land it here and reuse.

### 81. `granted-upkeep-tax-to-all-creatures` — 1 card, M
Depends on: #32.
The Tabernacle at Pendrell Vale: All creatures have "At the beginning of your upkeep, destroy this
creature unless you pay {1}." *Sketch:* #32's ability-granting applied globally by filter, where
the granted ability is a *triggered* one whose "your" resolves per affected creature's controller.
The self-reference ("this creature") must bind to each grantee, not to the Tabernacle.

### 82. `time-elemental` — 1 card, S
Depends on: nothing.
"When this creature attacks or blocks, at end of combat, sacrifice it and it deals 5 damage to you.
{2}{U}{U}, {T}: Return target permanent that isn't enchanted to its owner's hand." *Sketch:* an
`attacks_or_blocks` trigger registering a delayed end-of-combat effect (both shapes exist), plus a
`PermanentFilter::enchanted` inverse — the filter has `enchanted` but not `unenchanted`; add the
negative axis.

### 83. `triassic-egg` — 1 card, S
Depends on: nothing.
Hatchling counters, a sacrifice ability gated on having two or more, and a two-mode choice
(reanimate from hand or from graveyard). *Sketch:* an activation condition on a counter count
threshold — the only missing piece; both modes and the counter-adding ability are expressible.

### 84. `activate-no-more-than-twice-each-turn` — 1 card, S
Depends on: nothing.
Vampire Bats. `AbilityToml` has `once_each_turn` and `nth_each_turn`; neither expresses "no more
than twice". *Sketch:* replace `once_each_turn` with `max_activations_per_turn: Option<u32>`
(1 for the existing users, 2 here) and migrate the pool in the same change.

### 85. `look-at-top-n-then-may-shuffle` — 1 card, S
Depends on: nothing.
Visions: "Look at the top five cards of target player's library. You may then have that player
shuffle that library." *Sketch:* a look-at-hidden-zone effect scoped to a *targeted* player's
library (the engine's look effects are self-scoped), with the reveal going only to the controller —
a per-player visibility grant the projection layer must honour (#35's machinery). The optional
shuffle is ordinary.

### 86. `voodoo-doll` — 1 card, M
Depends on: nothing.
Pin counters accumulate every upkeep; an end-step trigger destroys the doll and burns its
controller for the count if it is untapped; and `{X}{X}, {T}` deals that many damage where **X is
defined as the counter count**, not chosen. *Sketch:* a `Cost::Mana` whose generic amount is an
`Amount` rather than a literal — the payment validator computes it at activation. The
intervening-if on the end-step trigger ("if this artifact is untapped") is the existing
intervening-if shape.

### 87. `conditional-banding-grant` — 1 card, M
Depends on: #3.
Wall of Caltrops: "Whenever this creature blocks a creature, if at least one other Wall creature is
blocking that creature and no non-Wall creatures are blocking that creature, this creature gains
banding until end of turn." *Sketch:* a `blocks` trigger with an intervening-if that counts the
*other* blockers of the same attacker by filter — a co-blocker predicate the combat assignment can
answer once #3 has built the band/group vocabulary.

### 88. `cant-be-targeted-by-wall-only-effects` — 1 card, M
Depends on: #15.
Wall of Shadows: "can't be the target of spells that can target only Walls or of abilities that can
target only Walls." The predicate is about the *targeting restriction of the source*, not about the
source's type — the engine would have to inspect a spell's target filter and ask whether it is
Wall-exclusive. *Sketch:* a static analysis of the targeting spell's `PermanentFilter` ("does this
filter admit only Walls?"), computed from the filter's subtype axis. Feasible because filters are
data; if it turns out to need a general filter-subsumption check, approximate to "the filter names
Wall as a required subtype" and mark it.

### 89. `attack-as-though-no-defender` — 1 card, S
Depends on: nothing.
Wall of Wonder: "{2}{U}{U}: This creature gets +4/-4 until end of turn and can attack this turn as
though it didn't have defender." *Sketch:* an `IgnoresDefenderForAttacking` marker with an
until-end-of-turn duration, checked in attack legality alongside the defender check. Not keyword
removal (#5) — the creature keeps defender for everything else.

### 90. `dealt-damage-to-opponent-this-turn` — 1 card, S
Depends on: #19.
Whirling Dervish: "At the beginning of each end step, if this creature dealt damage to an opponent
this turn, put a +1/+1 counter on it." *Sketch:* a query against #19's damage ledger, filtered to
this source and opponent recipients. If #19 lands first this is a one-line condition.

### 91. `winds-of-change` — 1 card, S
Depends on: nothing.
"Each player shuffles the cards from their hand into their library, then draws that many cards."
*Sketch:* a per-player count captured before the shuffle and used for the draw — a
resolution-scoped per-player amount (#46/#69's shape, one value per player).

### 92. `damage-to-filtered-subset-of-targets` — 1 card, S
Depends on: #61.
Winter Blast: "Tap X target creatures. Winter Blast deals 2 damage to each of those creatures with
flying." *Sketch:* a second effect scoped to the *previously chosen target set* filtered down —
an `EffectTarget::PreviousTargetsMatching { filter }` the resolution threads from the first effect
to the second.

### 93. `wood-elemental` — 1 card, M
Depends on: nothing.
"As this creature enters, sacrifice any number of untapped Forests. Its power and toughness are
each equal to the number of Forests sacrificed as it entered." *Sketch:* an **as-enters**
(replacement, not ETB trigger) player choice — the engine's ETB choices all happen after the
permanent is on the battlefield — plus a characteristic-defining P/T frozen at that count
(2ed's `base_power_toughness_from_amount` with an amount snapshotted at entry rather than
recomputed).

## Increments 94–117 — raised by the Phase 3 authoring wave

Thirty-nine cards were classified as section C at intake and turned out, once authoring
actually reached them, to need engine work the intake read missed. They are reclassified to
section D and blocked on the increments below. Two clusters dominate: prevention that is keyed
to a damage *source* rather than a recipient (#94, #95 — 7 cards), and the "becomes a color"
effect, which the DSL models as a permanent, uncounted, literal-color set (#96 — 6 cards).

### 94. `source-keyed-prevention-shield` — 5 cards, L
Depends on: #12.
Lady Evangela, Subdue, Horn of Deafening, Kry Shield, Indestructible Aura.
"Prevent all combat damage that would be dealt by target creature this turn."
*Sketch:* `PreventionShield` is keyed to a damage **recipient** and is consumed by the first
damage it stands in front of. Legends wants two axes the shield does not have: a *source* key
("dealt by that creature", any recipient) and a *duration* ("this turn", uncapped rather than
one-shot). `StaticEffect::PreventCombatDamage { by_self }` is source-keyed but binds to the
ability's own source and lives as long as the permanent, so it cannot be aimed. The shield needs
a `key: Recipient(ObjectId) | Source(ObjectId)` and an `amount: Next(n) | AllThisTurn`.
Indestructible Aura is the recipient-keyed corner of the same change (all damage, turn-long);
Kry Shield's and Subdue's +0/+X halves are already expressible today.

### 95. `grant-prevention-to-attached` — 2 cards, M
Depends on: #94.
Gaseous Form, Demonic Torment.
"Prevent all combat damage that would be dealt to and dealt by enchanted creature."
*Sketch:* `StaticEffect::GrantToAttached` carries P/T and keywords but has no prevention field,
and `StaticEffect::PreventCombatDamage` binds to the ability's own source — the Aura — not to
its host. Once #94 gives the shield a source/recipient key, this is an Aura-scoped standing
shield re-keyed to `attached_to`. Demonic Torment's "can't attack" clause is `grant_to_attached
cant_attack` today; only the prevention line blocks it.

### 96. `target-becomes-color` — 6 cards, M
Depends on: nothing.
Dwarven Song, Heaven's Gate, Touch of Darkness, Sea Kings' Blessing, Sylvan Paradise,
Alchor's Tomb.
"One or more target creatures become red until end of turn."
*Sketch:* `PumpEffect::TargetBecomesColor` hardcodes `until_end_of_turn: false`, takes no
`count`, and names a literal color. The five one-mana spells need the duration and the
multi-target count; Alchor's Tomb needs the third axis — a chosen color, consuming
`ChoiceEffect::ChooseColor` — and targets a *permanent* you control rather than a creature,
indefinitely (CR 613 layer 5, no expiry). All three axes land on the one effect.

### 97. `token-profiles-without-a-scryfall-printing` — 3 cards, M
Depends on: nothing. **Gates #123.**
Boris Devilboon (Minor Demon), Serpent Generator (Snake), Master of the Hunt (Wolves of the Hunt).
*Sketch:* `create_token` keys a `data/tokens/` profile by Scryfall oracle id — `de::token_profile`
in `crates/cards/src/de.rs` resolves the string against `crates/cards/data/tokens/`, and all 44
files there carry a real one — and Legends predates printed token cards, so none of these three
tokens has an id to key. Needs a synthetic local id convention for pre-token-era tokens.
The constraint that decides the shape is **`default_print`**: it is a required plain `String`
(`crates/cards/src/toml_surface/card.rs`) with no format validation, and it feeds an image URL
straight through `client/app/board/html/inspect.ts` (`pin.print ?? card?.default_print ?? ""`), so
a synthetic id is not merely untidy — it ships a broken art tile. So either make `default_print`
optional and let `inspect.ts` take the no-art path it already has for a missing print, or give
synthetic ids a documented namespace the client recognises and refuses to build a URL from. Decide
this before authoring any of the three cards; it is a pool-wide convention, not a per-card call, and
the grind has precedent against quietly fabricating an id (the Spurnmage Advocate frame error, #120).
Serpent Generator additionally needs its token to carry an ability — "Whenever this creature deals
damage to a player, that player gets a poison counter" — which is #99's trigger on a token profile,
so it lands after that.

### 98. `nested-effects-lose-their-source` — 1 card, S — **bug** — **LANDED** (wave 1)
Depends on: nothing.
*Landed:* fixed at the root in `Game::run` rather than at the reported call site, so every
nested-effect path is covered at once — `run_sequence`, both `Conditional` branches,
`pay_sacrifice_unless`, and `answer_choose_mode`'s mid-resolution modal branch. Cosmic Horror is
faithful. Tests: `crates/engine/tests/leg_nested_source.rs`.
Cosmic Horror: "At the beginning of your upkeep, destroy this creature unless you pay
{3}{B}{B}{B}. If this creature is destroyed this way, it deals 7 damage to you."
*Sketch:* not a missing capability — a defect. A `target = "this"` effect nested inside a
`pay_or_else` `otherwise` array **silently no-ops**: `TargetSpec::ThisPermanent` is resolved to
the source only at ability placement, never for nested effects, so the nested effect runs with
`target: None` and `Game::run`'s no-target guard (`crates/engine/src/effects.rs:833`) drops it.
Resolve `ThisPermanent` for nested effects too. Regression test first, per the repo's bug-fix
rule: an unpaid Cosmic Horror upkeep must destroy it, and today it does nothing at all.

### 99. `fill-player-for-counters-on-player` — 1 card, S — **LANDED** (wave 1)
Depends on: nothing.
*Landed:* the arm is in. Pit Scorpion keeps a narrower residual — its trigger tag is
opponent-scoped where the oracle says "a player" — carried forward as **#118**.
Tests: `crates/engine/tests/leg_poison_trigger.rs`.
Pit Scorpion: "Whenever this creature deals damage to a player, that player gets a poison counter."
*Sketch:* one missing match arm. `cards::fill_player` does not rewrite
`CountersEffect::PutCountersOnPlayer`'s `who`, so a `deals_damage_to_opponent` trigger cannot
fill `damaged_player` and the engine panics on the "filled in at placement" expect. Add the arm.

### 100. `blocks-and-becomes-blocked-by-as-separate-triggers` — 1 card, M
Depends on: nothing.
Infernal Medusa: "Whenever this creature blocks a creature, destroy that creature at end of
combat. / Whenever this creature becomes blocked by a non-Wall creature, destroy that creature
at end of combat."
*Sketch:* the DSL has only the combined `blocks_or_becomes_blocked_by { filter }` tag, and this
card's two halves take *different* filters — any creature it blocks, but only non-Wall creatures
that block it. Split into `blocks { filter }` and `becomes_blocked_by { filter }`, each carrying
`blocking_partner`. The divergence is reachable rather than theoretical: Animate Wall is in the
pool, so an attacking Wall blocked by the Medusa is a real board state.

### 101. `filtered-per-turn-cast-tally` — 1 card, M
Depends on: nothing.
Ichneumon Druid: "Whenever an opponent casts an instant spell other than the first instant spell
that player casts each turn, this creature deals 4 damage to that player."
*Sketch:* `Trigger::CastSpell.nth_each_turn` tests an equality against the caster's unfiltered
`Player::spells_cast_this_turn`. The card needs a tally scoped by the trigger's own filter
(instants only) and an "every cast after the first" comparison rather than "exactly the Nth".
The engine already documents this gap at `crates/engine/src/triggers.rs:3389`.

### 102. `counter-kinds-legends-prints` — 2 cards, S
Depends on: nothing.
Osai Vultures (carrion counter), Spirit Shackle (-0/-2 counter).
*Sketch:* `CounterKind` is a fixed 14-slot enum. Add the two kinds Legends prints — a named
`Carrion` counter that only its own card reads, and `-0/-2`, which the P/T layer must apply
alongside the existing `-1/-1`.

### 103. `counter-activated-ability-payload` — 2 cards, M
Depends on: nothing.
Ayesha Tanaka, Rust: "Counter target activated ability from an artifact source unless that
ability's controller pays {W}."
*Sketch:* `MiscEffect::CounterTargetActivatedAbility` is a payload-free variant. It needs a
source filter ("from an artifact source" — the target-legality restriction both cards share) and
an `unless_pays` rider matching the one `CounterTargetSpell` already carries. Ayesha's banding
half is already expressible.

### 104. `blocking-this-creature-and-indefinite-gain-control` — 1 card, L
Depends on: nothing.
The Wretched: "At end of combat, gain control of all creatures blocking this creature for as
long as you control this creature."
*Sketch:* two gaps at once. `PermanentFilter::blocking` means "blocking *some* attacker" — there
is no axis for "blocking **this** creature". And `GainControlAllUntilEndOfTurn` is the only mass
control-change; nothing expresses the "for as long as you control this creature" duration, which
is a conditional continuous effect that ends when the source leaves or changes controller
(CR 611.2b), not a turn-scoped one.

### 105. `filter-completeness-and-disjunction` — 3 cards, M
Depends on: nothing.
Abomination ("a green or white creature"), Flash Counter ("target instant spell"),
Mana Matrix ("Instant and enchantment spells you cast").
*Sketch:* the filter types are missing both arms and combinators. `SpellFilter` has no bare
`instant` (only `instant_or_sorcery`, which would wrongly catch sorceries) and no way to say
"instant or enchantment". `ColorFilter` holds one color with no OR. Add the missing arms plus a
disjunction combinator usable by both — the union shape #8 needs for attacking-or-blocking is
the same idea one level up.

### 106. `sacrifice-filtered-permanents-as-an-alternative-cost` — 1 card, M
Depends on: nothing.
Mold Demon: "When this creature enters, sacrifice it unless you sacrifice two Swamps."
*Sketch:* `pay_or_else` settles mana only — `settle_payment` takes no sacrifice rider — and
`may_sacrifice` has neither a count nor an `otherwise` branch. Needs "sacrifice N permanents
matching a filter, otherwise `<effects>`" as a payable cost in the `pay_or_else` window.

### 107. `board-wide-attack-ban` — 1 card, M
Depends on: nothing.
Moat: "Creatures without flying can't attack."
*Sketch:* `StaticEffect::CantBeAttackedBy` is scanned only off the *defending player's own*
battlefield, so it expresses "creatures without flying can't attack **you**". Moat bans the
attack at every seat regardless of who controls the Moat. Needs the restriction evaluated
globally over the battlefield rather than per defender.

### 108. `counter-the-triggering-spell` — 1 card, M
Depends on: nothing.
Presence of the Master: "Whenever a player casts an enchantment spell, counter it."
*Sketch:* `MiscEffect::CounterTargetSpell` requires a chosen `Target`, and no `TargetSpec` names
the spell that fired a `CastSpell` trigger.
*Re-rated after wave 1 (cheaper than written):* the threading half is already done —
`queue_cast_spell_triggers` sets both `triggering_spell: Some(spell)` and
`triggering_caster: Some(spell_controller)` (`crates/engine/src/triggers.rs:3370`). All this
needs is a `TargetSpec::TriggeringSpell` that reads the existing field. Closer to **S** than M.

### 109. `exile-the-source-from-the-graveyard` — 1 card, M
Depends on: nothing.
Cyclopean Mummy: "When this creature dies, exile it."
*Sketch:* no effect mode exiles the ability's own source once it has left the battlefield.
`ExileEffect::Object.object` is `serde(skip)` — always `None` from TOML, and resolution
`.expect()`s it — while `TargetSpec::ThisPermanent` resolves empty because the source is in the
graveyard by the time a dies-trigger resolves. Needs the source's post-death card object
tracked through the trigger (CR 603.6c's look-back), which is adjacent to but distinct from #98.

### 110. `per-effect-targets` — 1 card, L
Depends on: nothing.
Psionic Entity: "{T}: This creature deals 2 damage to any target and 3 damage to itself."
*Sketch:* an ability has exactly **one** shared target — `Effect::target()` returns the first
non-`None` target of the sequence and every effect in the ability resolves against it. This card
needs a chosen "any target" step beside an untargeted self-damage step. Structural: targets must
become per-effect rather than per-ability, which touches target selection, legality re-checks on
resolution, and every existing multi-effect ability. Rank late.

### 111. `activate-only-before-the-combat-damage-step` — 1 card, S
Depends on: nothing.
Angus Mackenzie: "Activate only before the combat damage step."
*Sketch:* `AbilityToml` has no activation-window restriction for this. The effect it gates,
`prevent_all_combat_damage_this_turn`, already exists — only the timing clause is missing. Sits
beside the existing `only_during_your_upkeep` / `only_during_your_turn` restrictions.

### 112. `damaged-player-discards-their-hand` — 1 card, M
Depends on: nothing.
Nicol Bolas: "Whenever Nicol Bolas deals damage to an opponent, that player discards their hand."
*Sketch:* three near-misses and no hit. `ChoiceEffect::Discard`'s count `Amount` has no
damaged-player hand-size variant; `Amount::CardsInTargetPlayerHand` needs a `Target::Player` the
untargeted trigger never sets; `DiscardYourHand` is controller-scoped. Needs `who` on a
whole-hand discard, filled from the trigger the way #99 fills the counter recipient. Bolas's
other two lines are expressible today.
*Re-rated after wave 1:* #99 landed the exact pattern — `damage_recipient` is already in
`TriggerContext` and `fill_player` already rewrites a `damaged_player` placeholder. This is now
`who` on `DiscardYourHand` plus one more `fill_player` arm. Closer to **S** than M.

### 113. `gain-life-equal-to-mass-damage-dealt` — 1 card, S
Depends on: nothing.
Syphon Soul: "Syphon Soul deals 2 damage to each other player. You gain life equal to the damage
dealt this way."
*Sketch:* the `gain_life_equal_to_damage` rider exists on `DamageEffect::Target` but not on
`DamageEffect::ToPlayers`, and no `Amount` totals damage dealt across several recipients. Add
the rider to the mass form, summing what was actually dealt (prevention and redirection change
the total, so it must read the result rather than the intent).

### 114. `cast-during-any-players-declare-attackers` — 1 card, S — **LANDED** (wave 1)
Depends on: nothing.
*Landed:* the flag now means the printed thing (any player's step, CR 506.3). Camouflage — the
pool's only other user, and the one card that does print "your" — carries the seat half as
`condition = { type = "during_your_turn" }` on its spell ability, which `cast_timing_ok` now
reads as a CR 601.3e cast restriction. Teleport is faithful; Camouflage keeps its pre-existing
pile-splitting residual. Tests: `crates/engine/tests/leg_declare_attackers_window.rs`.
Teleport: "Cast this spell only during the declare attackers step."
*Sketch:* `playable.rs` gates `cast_only_during_declare_attackers` on
`active_player == caster`, which reads the restriction as "during *your* declare attackers".
Teleport's window is any player's. Split the flag, or drop the active-player clause and let the
existing `only_during_your_turn` express the narrower case.

### 115. `player-draws-trigger-context` — 1 card, S — **LANDED** (wave 1)
Depends on: nothing.
*Landed:* `TriggerContext::drawing_player` threaded from `queue_player_draws_triggers`.
Underworld Dreams is faithful. Tests: `crates/engine/tests/leg_player_draws.rs`.
Underworld Dreams: "Whenever an opponent draws a card, this enchantment deals 1 damage to that
player."
*Sketch:* `queue_player_draws_triggers` builds `TriggerContext::of(controller)` and never
threads the *drawing* player, so `who = "triggering_player"` panics at resolution on the
"filled in at placement" expect. Thread the drawing player into the context.

### 116. `condition-scoped-to-the-triggering-player` — 1 card, M
Depends on: nothing.
Spiritual Sanctuary: "At the beginning of each player's upkeep, if that player controls a
Plains, they gain 1 life."
*Sketch:* every `Condition` evaluates against `ctx.controller`. An `each_upkeep` trigger needs
its intervening-if clause scoped to the player whose upkeep it is (CR 603.4), which means
`Condition` gains a subject rather than always meaning "you".

### 117. `grant-to-attached-under-a-condition` — 1 card, M
Depends on: nothing.
Spectral Cloak: "Enchanted creature has shroud as long as it's untapped."
*Sketch:* `StaticEffect::GrantToAttached` has no `condition` field and
`attachment_continuous_effects` ignores `ability.condition`, so a grant cannot be gated on the
host's state. Needs the condition evaluated against the *attached* permanent, re-checked
continuously rather than latched at attach time.

### 118. `damage-to-any-player-trigger` — 1 card, S
Depends on: nothing. Raised by wave 1 (#99).
Pit Scorpion: "Whenever this creature deals damage to a player, that player gets a poison
counter."
*Sketch:* the only damage-to-a-player trigger tag is `deals_damage_to_opponent`, which
early-returns when the damaged player is the source's controller. The oracle says "a player",
not "an opponent", so damage the Scorpion deals to its own controller — redirection, or a
control swap mid-damage — poisons no one. Needs a `deals_damage_to_player` tag that watches
every seat, with `deals_damage_to_opponent` kept for the cards that really do print "opponent".

### 119. `divide-combat-damage-in-the-damage-step` — 8 cards, L
Depends on: nothing. Raised by wave 1 (#1); blocks every post-declaration pump, not just rampage.
*Sketch:* the engine raises `PendingChoice::AssignCombatDamage` from `Game::declare_blockers`,
immediately after `seal_blocks`, and `Game::assign_damage` validates the total against
`self.power(attacker)` **at that moment** — before any block trigger has resolved. CR 509.2 does
choose the damage *assignment order* at declare blockers, but CR 510.1a divides the actual amounts
in the combat damage step, reading the attacker's power *then*. So a rampage bonus, a Giant Growth
cast in response to blockers, or any other post-declaration pump can never be assigned to the
blockers: the locked division still totals the unpumped power. Only trample rescues it, because
`Game::assign_attacker_damage` computes overflow as current `power` minus *assigned*.
Split the choice in two: an order at declare blockers, amounts at the damage step. This changes
when the choice is raised for every multi-block in the engine, so the existing `game.rs` suite is
the real cost — hence L, not M.
**Schedule this before #3's slice 3**, which adds a "who chooses" axis to the same choice — see #3.
Proven by `rampage_bonus_cannot_be_divided_because_the_division_is_locked_before_the_trigger_resolves`
in `crates/engine/tests/leg_rampage.rs`, which asserts today's behavior; this increment makes the
pumped total legal. All 8 of #1's cards carry an `approximates` until it lands.

### 120. `return-graveyard-cards-to-owners-hand` — 1 card, M
Depends on: nothing. Raised by #8, which went looking for Spurnmage Advocate's stale blocking-axis
note and found the whole card fabricated.
Spurnmage Advocate's real oracle is "{T}: Return two target cards from an opponent's graveyard to
their hand. Destroy target attacking creature." — a {W} 1/1 Human **Nomad**. The pool TOML was
authored in the `political-puppets` grind (#218 there, `soc` in the `game.rs` test comment) against
text no printed card has: a {1}{W} 2/2 Human
Cleric whose ability *exiles* the two graveyard cards as an activation cost and then *removes the
attacker from combat and taps it* — the second clause is Labyrinth of Skophos' text, not this
card's. The front matter (cost, P/T, subtypes, oracle) is corrected as of #8 and the body carries a
precise `approximates`; the body itself is this increment.
*Sketch:* the two clauses are both resolution effects, not costs. Add a graveyard-to-owner's-hand
effect over the same `card_in_graveyard` target form `Effect::Exile(ExileEffect::Target)` already
uses, with a target count of 2, and swap the second clause to a plain destroy on
`{ types = "creature", attacking = true }`. That leaves `AbilityCost::graveyard_exile_target_count`
and its `PendingChoice::ChooseActivationCostTargets` machinery — built for this card and used by
nothing else — with no card behind them, so **delete both** in the same change unless another card
has claimed them by then; `spurnmage_advocate_two_target_clauses` in `crates/engine/tests/game.rs`
goes with them.

### 121. `declare-bands-from-the-client` — 0 cards, M
Depends on: #3 slice 1 (landed). Carved out of #3 slice 2 deliberately — slice 2 is a rules slice
and stays testable in the engine, while this is client-surface work.
`Intent::DeclareAttackersInBands` exists and is exercised by `crates/engine/tests/`, but it has no
`WireIntent` counterpart, so the proto and the client's attack UI still see a flat attacker list: a
human player cannot declare a band, only a test can. No card is blocked by this — the six banding
cards are already as faithful as the rules slices make them — but the grind's live smoke game
cannot show banding at all until it lands, so **it belongs in the client catch-up phase, before the
smoke game, not after.**
*Sketch:* add a repeated group of attacker ids to the declare-attackers wire intent, map it at the
gRPC edge to the `bands` argument, and give the attack UI a way to group selected attackers. The
engine already rejects an illegal band (`Game::band_is_legal`), so the client needs no legality
logic of its own — surface `Reject::IllegalDeclaration` as it does for any other bad declaration.

### 122. `show-the-game-ended-in-a-draw` — 0 cards, S
Depends on: #27 (landed). Raised by #27, which stopped at the wire edge on purpose.
The engine ends a draw correctly — `GameOutcome::Draw`, `Event::GameDrawn`, and every subsequent
intent rejected (CR 104.4b) — and `crates/server/src/session.rs` reads `outcome()` rather than a
winner so the table reaches `Disposition::GameOver`. But `VisibleEvent::GameDrawn` maps to `None` at
`crates/server/src/grpc/map/stream.rs`: the wire owns no draw variant, and no `PlayerLost` fires in a
draw, so from the client's side the game simply stops responding with nothing said. Divine
Intervention is faithful in the engine and invisible in the UI.
*Sketch:* add a `VisibleEventGameDrawn` message to `stream.proto`, drop the `return None`, and render
it wherever elimination is rendered today. The client has no end-of-game panel at all yet, so scope
this to the same treatment `PlayerLost` gets rather than building one. If a dedicated end-of-game
surface does get built, the `Reject::WrongTiming` ponytail in `Game::submit_inner` (which stands in
for a `Reject::GameOver` the client could not display) should be revisited with it.

### 123. `token-named-band-quality` — 1 card, M
Depends on: #3 slice 4 (landed) and **#97**, which owns the token-id convention this needs.
Master of the Hunt — "{2}{G}{G}: Create a 1/1 green Wolf creature token named Wolves of the Hunt.
It has 'bands with other creatures named Wolves of the Hunt.'" — is the set's only card whose
`bands with other [quality]` quality is a **card name** (CR 702.22b) rather than a supertype. Split
out of #3 slice 4, which landed the four remaining banding lands and stopped here on purpose: the
rules half is small and was **built and proven** in that slice before being reverted rather than
shipped as surface with no card behind it, and the blocker is entirely #97's — "Wolves of the Hunt"
has no Scryfall printing at all (there has never been a 1/1 green Wolf token product), and the
existing `tokens/wolf.toml` is Garruk's 2/2 black-green Wolf, so the name that carries the band
quality cannot come from it.
*Sketch:* land #97 first, then the rules half, which is **known to work** and should not be
re-derived. Add `BandsWithQuality::Named` carrying the name; because `Keyword` is `Copy` and serde's
`'de` lifetime cannot yield a `&'static str`, give `BandsWithQuality` a hand-written `Deserialize`
that takes an owned `String` and `Box::leak`s it (card data loads once and lives for the process).
Match it in `Game::matches_bands_with_quality` against the printed name (CR 201.2), and replace the
hardcoded `[BandsWithQuality::Legendary]` list in `Game::band_is_legal` with a helper that discovers
the qualities the band's own members carry by reading `Game::effective_keywords` — with a
parameterised quality there is nothing left to look up by exact match through `Game::has_keyword`.
New arms are owed in `Game::keyword_token` (`crates/engine/src/message.rs`) and in both
`wire_keyword` and `keyword_label` (`crates/schema/src/catalog.rs`). The card is then a plain
activated ability, and the three tests slice 4 wrote are the coverage: the token carries the keyword
and the Master does not, a two-wolf pack is blocked as a group, and a differently-named creature
cannot join the band.
