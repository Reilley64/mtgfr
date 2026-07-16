# Card-Definition TOML DSL Reference

One TOML file per card in `crates/cards/data/*.toml`, loaded into `engine::CardDef`.
This documents every field and tag the deserializer accepts. Source of truth:
`crates/engine/src/types.rs` (the derived/annotated shapes) and `crates/engine/src/de.rs`
(manual impls for shapes that differ structurally from the TOML).
Filename is arbitrary; the card is keyed by its `name`.

**Rule:** if a card needs something not listed here, do not force-script it — flag it.
Simplifications the pool already makes are marked with `# ponytail:` comments in the TOML.

## 0. File header — Oracle text first

**Every card file begins with its verbatim Oracle text as a comment, above `name`.** This is
the faithfulness anchor: the reviewer sees what the card *actually* does before reading how the
pool models it. Copy the current Scryfall Oracle text exactly (line breaks become separate
comment lines); then, on the lines below it, keep any modeling/`# ponytail:` notes.

```toml
# Whenever this creature or another creature dies, target player loses 1 life and
# you gain 1 life.
# ponytail: modeled as a watch-others death trigger, so its own death does not fire it.
name = "Blood Artist"
approximates = "does not trigger on its own death — \"this creature or another creature dies\" is trimmed to \"another creature dies\""
```

The header quote is the bare verbatim text — no `Oracle:` prefix. All comment lines (header,
per-ability, per-effect, `ponytail:`) wrap at 120 characters.
Vanilla cards (basic lands, French-vanilla creatures) still get the header line even when
it is empty or just keywords — e.g. `# (Basic land — taps for one white mana.)` or
`# Flying, vigilance`. If the model diverges from the Oracle text at all, that divergence
must also be captured in the machine-readable [`approximates`](#1-top-level-fields) field, not the
comment alone (the comment is for humans; `approximates` is what the catalog and audits read).
The same verbatim text also goes in the machine-readable `oracle` field (§1) for the catalog's
read-the-text hover.

**Comment discipline.** Comments never *assert* faithfulness — no "faithful to the printed
card" notes. The only comments a card file carries are Oracle quotes, `# ponytail:` notes naming
a deliberate simplification, and modeling notes pairing with `approximates`. Silence means
faithful.

**Every `[[abilities]]` block is directly preceded by a `#` comment quoting the exact Oracle
sentence(s) it implements.** The reviewer maps model to text ability by ability, never by
re-deriving the pairing.

## 1. Top-level fields

| Field | Type | Default | Notes |
|-------|------|---------|-------|
| `name` | string | (required) | Registry key; must be unique. |
| `cost` | `[cost]` table | empty (free) | See §2. Omit for lands. |
| `kind` | `[kind]` table | (required) | See §3. |
| `legendary` | bool | `false` | Legendary supertype (commanders must be legendary creatures). |
| `uncounterable` | bool | `false` | "This spell can't be countered" (CR 701.5g — Altered Ego). |
| `enchant` | filter | (none) | An Aura's enchant restriction (CR 303.4a): a permanent filter (§7) the cast target/host must match. Absent = the ordinary "Enchant creature". |
| `enchant_graveyard` | bool | `false` | Animate Dead's own "enchant creature card in a graveyard" (CR 303.4a): unlike `enchant` above, the cast target is a creature *card in a graveyard*, not a battlefield permanent — `required_target` reports `TargetSpec::CreatureCardInAnyGraveyard` for the card instead of `enchant`'s battlefield-permanent spec. Kind stays `enchantment` (not `aura`) since the Aura resolution path needs a battlefield host to already exist. The choice threads to the card's own `etb` reanimation effect via `target = "this_auras_graveyard_target"` (§7) — a fixed reference, not a fresh choice. A bare bool: the pool has exactly one such card, unrestricted ("a graveyard", not "your graveyard"). |
| `modal` | bool | `false` | "Choose N" spell (CR 700.2): each `spell`-timed ability is a mode; caster picks exactly `choose` of them. Also covers a modal *triggered* ability (Shadrix Silverquill's begin-combat "you may choose two") — a card with no `spell`-timed ability instead treats every ability of its (single) triggered timing as a mode; see the `begin_combat` timing note (§5) and `optional` (§5) for the "may" gate. |
| `choose` | u8 | `1` | Only meaningful with `modal = true`. Number of distinct modes the caster picks (Prismari/Quandrix/Witherbloom Command = 2), or the *minimum* of a "choose one or more" range when `choose_max` is set. |
| `choose_max` | u8 | (none) | Only meaningful with `modal = true`. The max of a "choose one or more" range (CR 700.2d — Casualties of War: `choose = 1, choose_max = 5`, its five printed modes). Omit for a fixed count (every "choose one"/"choose two" card) — the caster then picks exactly `choose` modes. Entwine/escalate/"choose one, two, or three" with per-pick riders aren't modeled; this is a plain min/max range. |
| `choose_max_if_commander` | bool | `false` | Only meaningful with `choose_max` set. Gates the `choose_max` range on the caster controlling a commander at cast time (CR 700.2 — Nexus Mentality: "Choose one. If you control a commander as you cast this spell, you may choose both instead" is `choose = 1, choose_max = 2, choose_max_if_commander = true`). Without a commander in play, the legal count collapses to the plain `choose`. A bare bool, not a general condition-gated max — the pool has one card that needs this specific gate. |
| `keywords` | array of strings/tables | `[]` | See §4. |
| `conditional_keywords` | array of tables | `[]` | Keywords held only while a condition holds (Primordial Hydra's "has trample as long as it has ten or more +1/+1 counters"): `[{ condition = { type = "source_has_counters", at_least = 10 }, keyword = "trample" }]`. Each `condition` is a §5 `Condition`. |
| `abilities` | array of `[[abilities]]` | `[]` | See §5. |
| `identity` | array of color names | `[]` | *Extra* color-identity pips (CR 903.4) the simplified model would otherwise drop — the trimmed half of a flattened dual/pain/filter land, a cut colored activation. Deck-building color identity only; the engine's rules never read it. |
| `colors` | array of color names | `[]` | Explicit colors (CR 105.2a color indicator / a token's stated color) overriding the cost-pip derivation. Mainly token profiles (no mana cost to derive from); empty = derive from cost pips. |
| `enters_tapped` | bool | `false` | This permanent enters the battlefield tapped, **unconditionally** (CR 614.13). Almost always a land ("This land enters tapped"). |
| `may_choose_not_to_untap` | bool | `false` | "You may choose not to untap this during your untap step" (CR 502.2 — Rubinia Soulsinger). The untap turn-based action pauses this permanent's controller on a yes/no per such permanent they control (a `DeclineUntap` choice); declined ones stay tapped. Pairs with `gain_control_while`'s "remains tapped" duration (§6). |
| `enters_tapped_unless` | `[enters_tapped_unless]` table | (none) | *Conditional* enters-tapped (check lands, slowlands, reveal lands): tapped **unless** the condition holds, checked once at the land's ETB. A `Condition` — same shape and `type =` tags as `[abilities.condition]` (§5) — including the four land-focused arms (`controls_lands_with_subtype`, `controls_basic_lands`, `opponents_control_lands`, `hand_has_land_with_subtype`). Mutually exclusive with `enters_tapped` in practice (no pool card needs both). |
| `approximates` | string | (none) | Machine-readable note of how the model diverges from the real card. Set it whenever the pool trims/approximates the Oracle text (see §0); leave it unset for faithful cards. Surfaced on the card catalog so the deck builder and audits read the same gap the engine runs. |
| `oracle` | string | (none) | The card's verbatim printed Oracle text (line breaks as `\n`; a DFC joins its faces with `//`). Catalog metadata for the read-the-text hover — the engine never parses it; behavior comes from `abilities`/`keywords`. |
| `cycling` | `[cost]` table | (none) | Cycling {N} (CR 702.29a — "{N}, Discard this card: Draw a card," from hand): `cycling = { generic = 2 }`. Same shape as `[cost]` (§2). |
| `hand_ability` | `[hand_ability]` table | (none) | A hand-activated, discard-this-card ability whose payload isn't cycling's fixed draw-1 (CR 113.6/602.5e — Magma Opus's "{U/R}{U/R}, Discard this card: Create a Treasure token."): `[hand_ability.cost]` (same shape as `[cost]`, §2 — hybrid pips included) plus `[[hand_ability.effects]]` (the standard §5 effects-array shape). The general sibling of `cycling` — don't overload `cycling` for an authored payload. `Intent::ActivateHandAbility` / the `activate_hand_ability` action pays the cost, discards the card, then puts the payload on the stack as a real activated ability (a single stack ability, `Sequence`-wrapped when it has more than one effect — respondable, countered by `counter_target_activated_ability`), like cycling's own draw. |
| `flashback` | `[cost]` table | (none) | Flashback (CR 702.34): cast from your graveyard for this alternative cost, then exile. `[flashback]`, same shape as `[cost]` — may carry a `[flashback.additional]` rider (Deep Analysis's `pay_life = 3`). |
| `echo` | `[cost]` table | (none) | Echo (CR 702.31): "At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost." `[echo]`, same shape as `[cost]` (Karmic Guide's `[echo] generic = 3, white = 2`). Pauses on a pay-or-sacrifice choice at the controller's first upkeep after the permanent enters; not re-asked once paid. |
| `delve` | bool | `false` | Delve (CR 702.66): while casting, exile any number of graveyard cards, each paying {1} of the generic cost. |
| `escape` | `[escape]` table | (none) | Escape (CR 702.19): an `[escape.cost]` sub-table (same shape as `[cost]`) plus `exile = N` (other graveyard cards exiled as an additional cost) and `plus_one_plus_one_counters = N` (default 0 — CR 702.19c "escapes with N +1/+1 counters", Woe Strider's 2). |
| `retrace` | bool | `false` | Retrace (CR 702.83): cast from your graveyard by discarding a land card in addition to paying its other costs. Unlike flashback/escape this is **not** an alternative cost — the caster pays the card's normal `[cost]`, plus `discard_land = true` in `[cost.additional]` (below). The resolved spell returns to the graveyard as normal (no exile rider), so it's repeatable. |
| `graveyard_cast_cost` | `[cost]` table | (none) | Cast-from-graveyard alternative cost for a *permanent* (CR 118.9, Raffine's Guidance): `[graveyard_cast_cost]`, same shape as `[flashback]`. Unlike flashback/escape, the card is a permanent — it enters the battlefield normally on resolution, no exile rider (a permanent never reaches the graveyard-or-exile fork those two gate). Unlike retrace, it *replaces* the printed `[cost]` rather than adding an additional cost on top of it. |
| `cascade` | bool | `false` | Cascade (CR 702.85): a rules-keyword (no `[[abilities]]` block). When you cast this spell, a triggered ability goes on the stack above it and resolves first — exile from the top of your library until a nonland card of lesser mana value, you may cast it without paying its mana cost, then bottom the exiled cards in a random order. Reuses the free-cast-from-exile permission and the reveal-until top-walk. Single mana-value bound baked in at cast (no `{X}`-cost cascade card modeled). |
| `demonstrate` | bool | `false` | Demonstrate (CR 702.147): a rules-keyword (no `[[abilities]]` block). When you cast this spell, a triggered ability goes on the stack above it (like `cascade`) and pauses on a "copy it?" choice; declining copies nothing. Accepting mints one copy under the controller (offering the usual CR 707.10c retarget), then the controller chooses an opponent, who gets a second copy the same way. Creative Technique, Replication Technique. |
| `devour` | u32 | (none) | Devour N (CR 702.82): a rules-keyword (no `[[abilities]]` block) — `devour = 2` (Mycoloth), `devour = 1` (Ribtruss Roaster). As this creature enters, its controller may sacrifice any number of the *other* creatures they control (an as-enters pause; declining is legal, 0 counters); it then gains `N × (count sacrificed)` +1/+1 counters, routed through the CR 614 counter-replacement path so doublers (Doubling Season, Hardened Scales) apply. ponytail: modeled as an as-enters *step* (counters placed after the entry), not a true CR 614.13 replacement — not observable for the pool (both devour cards read their counters at a later upkeep/end step). |
| `enter_as_copy` | `{ .. }` table | (none) | Enter-as-a-copy replacement (CR 706/707.2): a rules-keyword (no `[[abilities]]` block) — `enter_as_copy = { until_eot = false, extra_counters = "0", gains_haste = false, of = "creature" }`. As this permanent enters, its controller **may** have it become a copy of any *other* object of type `of` on the battlefield (an as-enters pause; declining is legal — it stays its printed self; no candidate ⇒ no pause). Fields (all optional): `until_eot` (bool, default `false` — the copy reverts to the printed permanent at cleanup, CR 514.2; Cursed Mirror), `extra_counters` (amount, default `0` — additional +1/+1 counters the copy enters with, routed through the CR 614 counter-replacement path; Altered Ego's `"x"`), `gains_haste` (bool, default `false` — grants the copy haste; Cursed Mirror's "except it has haste"), `of` (`"creature"` \| `"enchantment"`, default `"creature"` — the copyable candidate's type; `"enchantment"` also matches an Aura, CR 303.2). Altered Ego = `{ extra_counters = "x" }`; Cursed Mirror = `{ until_eot = true, gains_haste = true }`; Copy Enchantment = `{ of = "enchantment" }`. Copying an Aura this way enters unattached and then pauses again on `ChooseAttachHost` (the same deployed-Aura attach path a searched-out or reanimated Aura uses, CR 303.4f) to pick a host among legal enchant targets. ponytail: the copyable values are the chosen object's printed `CardDef` (CR 707.2), not a full read of copy-layer modifications already on it; and the copied object's own ETB triggers don't fire (`def` is overwritten after the enter event). |
| `back` | `[back]` table | (none) | A "prepare" DFC's back face (soc/sos): a full inline card table — its own `name`/`[back.cost]`/`[back.kind]`/`[[back.abilities]]`, the same shape as a top-level card. The front face carries a `become_prepared` ability (§6); while prepared, its controller may cast a copy of this face, which unprepares it (Kirol, History Buff). |
| `suspend` | `[suspend]` table | (none) | Suspend N—[cost] (CR 702.62): a rules-keyword (no `[[abilities]]` block) — `[suspend]` with `counters = N` and a `[suspend.cost]` sub-table (the same `[cost]` shape). Rather than cast the card, its owner may pay the suspend cost to exile it from hand with N time counters (`Intent::Suspend` / the `suspend` action); a time counter is removed at each of the owner's upkeeps, and when the last is gone they may cast it from exile without paying its mana cost. Rousing Refrain's `counters = 3`, `[suspend.cost]` = `{1}{R}`. ponytail: the free cast is modeled as a "may cast free from exile this turn" permission rather than the forced triggered cast + haste of real suspend (CR 702.62e/f) — indistinguishable for a sorcery with no haste-relevant body. |
| `encore` | `[encore]` table | (none) | Encore [cost] (CR 702.140, Angel of Indemnity): a rules-keyword (no `[[abilities]]` block) — `[encore]` holds the encore **mana** cost, the same `[cost]` shape as `[flashback]` (`generic = 6, white = 2` = `{6}{W}{W}`). A graveyard-activated, sorcery-speed special action (`Intent::Encore` / the `encore` action): pay the encore mana cost **plus exiling this card from your graveyard** (the exile half is intrinsic, not a pip), then for each opponent create under you a token copy of this card that must attack that opponent this turn if able, gains haste, and is sacrificed at the beginning of the next end step. ponytail: the token uses the card's printed `CardDef` copyable values (CR 707.2), not a full copy-layer read (as #127's copy slices). |
| `bestow` | `[cost]` table | (none) | Bestow [cost] (CR 702.103, Eidolon of Countless Battles): a rules-keyword (no `[[abilities]]` block) — `[bestow]` holds the bestow **mana** cost, the same `[cost]` shape as `[echo]` (`generic = 2, white = 2` = `{2}{W}{W}`). The card keeps its printed creature `[kind]`; casting for the bestow cost (`Intent::CastBestow` / the `cast-bestow` action) puts it on the stack as an **Aura spell with enchant creature** targeting a creature, and on resolution it enters *attached* to that creature. While attached it's an Aura enchantment (with the Aura subtype), **not** a creature (CR 702.103e) — off combat, out of creature filters/wraths; when it stops being attached it becomes a creature again (CR 702.103i, a state-based action). To pump the enchanted creature, pair the card's `self_only` `anthem_static`(s) with matching `grant_to_attached` static(s) — the grant is inert until the card is bestowed (attached). ponytail: pays the flat bestow cost (no cost-reduction/ward pipeline) — Eidolon is the pool's only bestow card. |
| `morph` | `[cost]` table | (none) | Morph [cost] (CR 702.37, Willbender): a rules-keyword (no `[[abilities]]` block) — `[morph]` holds the card's **morph** cost, the same `[cost]` shape as `[bestow]` (`generic = 1, blue = 1` = `{1}{U}`). Any card with `[morph]` may be cast **face down** for a flat generic **{3}** (`Intent::CastFaceDown` / the `cast-face-down` action, CR 702.37b — independent of the morph cost), landing as a face-down 2/2 colorless creature with no name/types/abilities (CR 708.2). Turn it face up any time for its **morph** cost (`Intent::TurnFaceUp` — the same action a manifest uses, but a `[morph]` card pays this cost instead of its printed cost), revealing its real characteristics and firing any `turned_face_up` trigger (below). ponytail: pays the flat {3} (no cost-reduction/ward pipeline); manifest-of-a-morph-card dual-cost (CR 702.37j) isn't modeled — no pool card manifests a morph card. |
| `evoke` | `[cost]` table | (none) | Evoke [cost] (CR 702.74, Mulldrifter): a rules-keyword (no `[[abilities]]` block) — `[evoke]` holds the card's **evoke** cost, the same `[cost]` shape as `[echo]` (`generic = 2, blue = 1` = `{2}{U}`). Casting for the evoke cost (`evoked = true` on the cast intent) charges `[evoke]` instead of the printed `[cost]` and records it on the resulting spell; the card enters as its ordinary creature — evoke doesn't change its nature, unlike bestow. The instant it enters, it's sacrificed (CR 702.74a): the sacrifice is queued as its own triggered ability alongside the permanent's own ETB triggers, landing *underneath* them on the stack so an ETB payoff (Mulldrifter's draw two) resolves first. ponytail: no `OrderTriggers` choice is offered for the controller's own simultaneous triggers — the ETB and the sacrifice are queued as two separate single-ability groups rather than one the controller orders; grow into a real choice if an evoke card ever needs the other order. |
| `adventure` | `[adventure]` table | (none) | An adventure card's adventure half (CR 715, soc/sos): a full inline card table — its own `name`/`[adventure.cost]`/`[adventure.kind]` (an instant/sorcery)/`[[adventure.abilities]]`, the same shape as a top-level card and as `[back]`. The **creature front face is the card's own top-level fields**. You cast the adventure from hand (`Intent::CastAdventure`); on resolution the card is exiled "on an adventure" instead of going to the graveyard, and its owner may cast the creature half from exile later **at its normal `[cost]`** (an ordinary `Intent::Cast` from exile — the permission never expires). Brazen Borrower's Petty Theft, Elusive Otter's Grove's Bounty. |
| `set` | string | `""` | Set/edition code (Scryfall's lowercase code, e.g. `"soc"`). Pure catalog metadata — the engine never reads it; it exists so deck-builder search can match on set. Backfilled from Scryfall by `tooling/backfill-card-meta.mjs`; don't hand-edit unless adding a card the tool won't resolve. |
| `subtypes` | array of strings | `[]` | Printed subtypes (creature types like `["Goblin", "Wizard"]`; also artifact/enchantment subtypes). Gameplay-relevant: the permanent filter's `subtypes` axis, `anthem_static`'s `subtypes`, the spell filter's `has_subtype`, and `you_control_no_subtype` all match against it. A **land's** types stay under `[kind].subtypes` (§3, rules use those); the catalog unions the two. Backfilled by `tooling/backfill-card-meta.mjs`. |
| `otags` | array of strings | `[]` | Scryfall Tagger oracle-tag slugs for thematic deck-builder search (e.g. `["typal-spirit", "cost-reducer-enchantment"]`). Pure catalog metadata — the engine never reads it. Backfilled from Scryfall by `tooling/backfill-otags.mjs`; don't hand-edit unless adding a card the tool won't resolve. |
| `functions_in_graveyard` | bool | `false` | CR 112.6/603.6e: this card's ability(-ies) work from the graveyard instead of the battlefield (Nether Traitor's death-watch self-return, Teacher's Pest's activated `{B}{G}` self-return, Vanguard of the Restless's enters-trigger self-return). A flagged card's trigger/activated-ability scans fire **only** while it's in the graveyard — not also while it's a live permanent. |

## 2. `[cost]`

All fields optional, default `0`/`false`. No `[cost]` table = free (lands, tokens).

```toml
[cost]
generic = 2      # {2}
white = 1        # {W}   (u8 pip count)
blue = 0         # {U}
black = 1        # {B}
red = 0          # {R}
green = 0        # {G}
colorless = 0    # {C}   colorless pips (NOT a color; payable only by colorless mana)
x = true         # cost includes {X}; or an integer pip count — x = 3 for {X}{X}{X} (Astral Cornucopia).
                 # A top-level [cost]'s {X} is chosen at cast time (Intent::Cast's x); an
                 # [abilities.cost]'s {X} (an optional trigger's own pay-{X} rider — Decree of
                 # Justice's "you may pay {X}") is chosen when accepting it (Intent::PayOptionalCostX).
hybrid = [["white", "black"]]   # hybrid pips (CR 107.4e): one two-color array per {A/B} symbol
reduce_own_generic = "per_creature_on_battlefield"   # an Amount (below); shaves this spell's OWN generic
```

`[cost.additional]` is an additional cost paid alongside the mana (CR 601.2f), any of:
`discard = N` (Big Score's "discard a card"), `discard_land = true` (Throes of Chaos's retrace
— "discard a land card"; shares the same `Intent::Cast` `discard_cost` slot as `discard`, with
the named card required to be a land), `pay_life = N` (a fixed life payment — Deep
Analysis's flashback rider), `pay_life = "x"` (the chosen `{X}` is paid as life instead of
mana — Toxic Deluge), `sacrifice = { count = "one_or_more", filter }` (an entirely optional
"sacrifice any number of permanents matching `filter`" — Plumb the Forbidden's "you may sacrifice
one or more creatures"; `count` is a marker, only `"one_or_more"` is modeled — a mandatory or
fixed-count sacrifice cast cost needs its own shape), `[cost.additional.kicker]` (Kicker,
CR 702.33 — Rite of Replication's "Kicker {5}"; a `[cost]`-shaped sub-table, e.g.
`[cost.additional.kicker]` / `generic = 5`), `[cost.additional.buyback]` (Buyback, CR 702.27 —
Capsize's "Buyback {3}"; the same `[cost]`-shaped sub-table, e.g. `[cost.additional.buyback]` /
`generic = 3`), `[cost.additional.strive]` (Strive, CR 702.42 —
Twinflame's "This spell costs {2}{R} more to cast for each target beyond the first"; the same
`[cost]`-shaped sub-table, e.g. `[cost.additional.strive]` / `generic = 2` / `red = 1`), or
`[cost.additional.replicate]` (Replicate, CR 702.108 — Changing Loyalty's "Replicate {2}"; the
same `[cost]`-shaped sub-table, e.g. `[cost.additional.replicate]` / `generic = 2`). The two
`pay_life` spellings are mutually exclusive. The chosen permanents are named in the
`Intent::Cast`'s `sacrifice_cost`; the count actually paid is recorded on the resolved spell (read
via `Game::spell_sacrifice_count`) for a future copy-per-sacrifice rider to consume. Kicker is
entirely optional (CR 702.33d): the caster opts in via `Intent::Cast`'s `kicked = true` (its mana
folds into the total cost), and the choice is recorded on the resolved spell, read via
`Game::spell_was_kicked` — see the `if_kicked`/`else` `Amount` table below. Buyback is the same
entirely-optional shape (CR 702.27c): the caster opts in via `Intent::Cast`'s `bought_back = true`
(its mana folds into the total cost the same way kicker's does), and the choice is recorded on the
resolved [`Spell::bought_back`] — when set, `Game::finish_instant_sorcery_resolution` returns the
resolved instant/sorcery to its owner's hand (CR 702.27d) instead of the graveyard, in place of the
usual flashback/escape exile or graveyard fork. Strive scales with a
caster-declared target count rather than a paid cost: the caster names it up front on
`Intent::Cast`'s `strive_count` (CR 601.2c precedes 601.2f — targets are chosen before the total
cost is locked, but this engine puts a spell on the stack before pausing to choose multi-targets,
so the count must be committed at cast time), `Game::cast_cost` multiplies
`[cost.additional.strive]` by `strive_count.saturating_sub(1)` and adds it in, and the same
declared count substitutes as the spell's own multi-target clamp — see `strive_scaled` in the
target-count table below. Replicate scales with a caster-declared *payment* count, committed the
same pre-stack way as Strive's target count: the caster names it up front on `Intent::Cast`'s
`replicate_count` (0 for none), `Game::cast_cost` multiplies `[cost.additional.replicate]` by the
count directly (each payment is a full extra instance of the cost, unlike Strive's "beyond the
first") and adds it in, and the declared count is recorded on the resolved spell (read via
`Game::spell_replicate_count`). At the cast choke, a nonzero `replicate_count` mints that many
copies of the spell immediately (CR 702.108b — reusing the same `Game::mint_spell_copies` rider
`copy_this_spell` uses, including each copy's own CR 707.10c retarget pause); a copy of a
permanent spell (not just instant/sorcery) resolves as a **token** (CR 707.10a) that ceases to
exist once it leaves the battlefield, including falling unattached as an Aura (CR 111.7/704.5m).

`reduce_own_generic` is a *self* cost reducer (CR 601.2f/702.41 "this spell costs N less to cast")
— an [`Amount`](#amounts) resolved with the spell's own controller/source/`{X}` at cast time and
subtracted from `[cost].generic`, floored at 0 (never below). It affects only this card's own
casting cost — contrast the *other-spells* reducer (`reduce_spell_cost`, §6), which is a static
that discounts spells *other* permanents' controllers cast. Blasphemous Act ("costs {1} less for
each creature on the battlefield") is `"per_creature_on_battlefield"`; Tomik's affinity for
planeswalkers is `{ per_permanent = { types = ["planeswalker"] } }`; Volcanic Salvo's "{X} less …
where X is total power of creatures you control" is `"total_power_you_control"`; Mortality
Spear's "{2} less if you gained life this turn" is a conditional amount (§"Amounts" below).

**Unsupported:** Phyrexian mana (`{W/P}`), snow, `{X}` in activated-ability costs. Hybrid *is*
supported (`hybrid`, above — also in an `activation_cost`, Fetid Heath). No way to express the
rest — flag the card.

## 3. `[kind]`

`type =` selects the variant; extra fields depend on it.

| `type` | Extra fields | Maps to |
|--------|-------------|---------|
| `"creature"` | `power` (i32), `toughness` (i32), `also` (type-name or list, optional) | `CardKind::Creature` |
| `"instant"` | — | `Spell{Instant}` |
| `"sorcery"` | — | `Spell{Sorcery}` |
| `"enchantment"` | — | `Enchantment` |
| `"aura"` | — | `Aura` (enters attached; use a `grant_to_attached`/`set_attached_base_p_t`/`control_attached` static, §6) |
| `"artifact"` | — | `Artifact` (mana rocks, equipment bodies) |
| `"planeswalker"` | `loyalty` (i32) | `Planeswalker` (starting loyalty) |
| `"land"` | `produces` (mana symbol, §8, **optional**), `subtypes` (array of strings, default `[]`), `basic` (bool, default `false`) | `Land` |

A land's `produces` is **optional sugar** for the common "{T}: Add one mana" free base tap: a
single mana symbol (§8) — a color name, `"colorless"`, `"any"`, or a **color array of 2 to 4
distinct colors** for a fixed choice: exactly two is a dual ("{T}: Add {G} or {U}" → `produces =
["green", "blue"]`), three or four is a triome-style choice ("{T}: Add {G}, {W}, or {U}" →
`mana = [["green", "white", "blue"]]` on an `add_mana` effect — Treva's Ruins). Either way it
adds one *credit* spendable as any of the listed colors, resolved at payment time — no choice on
tap, and every listed color counts toward color identity and castability. Two literal strings are
also accepted: `"commander_identity"` — one
mana of any color in your commander's color identity (CR 903.4, Command Tower) — and
`"opponent_colors"` — one mana of any color a land an opponent controls could produce (Exotic
Orchard); both resolve their credit from table state at tap time.

**Omit `produces`** for a land with no free base tap. Two cases:
- **Fetch-only lands** (Prismatic Vista, Terramorphic Expanse) — played only to be sacrificed;
  their sole ability is the search (§6). No `produces` ⇒ no tap-for-mana; a `TapForMana` intent is
  rejected.
- **Lands whose mana is all explicit `add_mana` abilities** (§6) — painlands, filter lands, the
  `{1},{T}` karoos. Their tap modes carry costs a bare `produces` can't express (self-damage, a
  hybrid cost, a two-mana output), so each mode is an ordinary `timing = "activated"` `add_mana`
  ability instead. These are **mana abilities** (marked by the `add_mana` effect): they resolve
  immediately, use no stack, and open no priority window (CR 605), exactly like a `produces` tap.
  Color identity and the auto-pass castability heuristic read the produced colors off the abilities.

`subtypes` is the land's printed land types (CR 305 — "Forest", "Island", …); empty for a land
with none (a check land, an untyped scry land). It drives type-specific search
(`CardFilter::LandWithSubtype`, §7) and type-gated conditions
(`controls_lands_with_subtype`/`hand_has_land_with_subtype`, §5) — nothing else reads it.

`basic` is the "Basic" supertype (CR 205.4a), set on exactly the five basics (`forest.toml`,
`island.toml`, …). It is **not** derived from `subtypes`: a nonbasic dual routinely carries the
same type strings as a basic without being one (Tangled Islet is `subtypes = ["Forest",
"Island"]`, `basic = false` — a nonbasic "Land — Forest Island" dual) — `basic` is what
`CardFilter::BasicLand`/`"basic_land"` actually tests.

A creature's `also` carries any *additional* card types — Artifact Creature, Enchantment Creature
(CR 305.4). Write one type name or a list: `also = "artifact"`, `also = ["artifact", "enchantment"]`.
Omit it for a plain creature. It makes "is this an artifact?" queries and artifact/enchantment
filters (§7) count the creature, so an Artifact Creature is sacrificed by an artifact edict and hit
by artifact removal.

```toml
[kind]
type = "creature"
power = 2
toughness = 2
also = ["artifact"]    # an Artifact Creature (Solemn Simulacrum, Stonecoil Serpent); omit for a plain creature
```
```toml
[kind]
type = "land"
produces = "green"    # a color, "colorless", or "any"
```
```toml
[kind]
type = "land"
produces = ["green", "blue"]    # a dual: "{T}: Add {G} or {U}"
```

## 4. Keywords

Bare-string keywords (snake_case): `flying`, `first_strike`, `vigilance`, `haste`, `trample`,
`deathtouch`, `reach`, `menace`, `double_strike`, `lifelink`, `defender`, `indestructible`, `flash`,
`hexproof` (can't be targeted by *opponents'* spells/abilities, CR 702.11), `shroud` (can't be
targeted by anything, CR 702.18), `prowess` (CR 702.108 — the whole triggered ability *is* the
keyword; the engine synthesizes the +1/+1-until-EOT trigger, never author it as an `[[abilities]]`),
`unblockable` (can't be blocked this turn/permanently — a fixed subset of "unblockable" with no
"except by …" carve-out; checked in `Game::can_block`, e.g. Rogue's Passage), `skulk` (CR 702.72a
— can't be blocked by creatures with greater power), `shadow` (CR 702.28 — can only block or be
blocked by other Shadow creatures, checked both directions in `Game::can_block`), and
`lesser_power_cant_block` (Elusive Otter's printed evasion, "creatures with power less than this
creature's power can't block it" — not a named MTG keyword; a card-specific tag riding the same
`Game::can_block` check), `cant_block` (CR 509.1a — Bloodghast's "This creature can't block"; never
a legal blocker), `can_block_only_flyers` (Brazen Borrower's "can block only creatures with
flying" — not a named MTG keyword; a card-specific tag on the same `Game::can_block` check, so pair
it with `flying` if the creature should still block flyers), `decayed` (CR 702.148 — can't
block, checked in `Game::can_block`; and "when it attacks, sacrifice it at the beginning of the
end of combat step" (CR 702.148c), a rules-defined delayed trigger scheduled per attacker in
`Game::declare_attackers` rather than authored ability text — see `jadar_ghoulcaller_of_nephalia.toml`),
and `myriad` (CR 702.114 — like Prowess, the whole triggered ability *is* the keyword; the engine
synthesizes the per-opponent tapped-and-attacking token mint in `Game::queue_myriad_triggers`,
never author it as an `[[abilities]]`. No pool card prints this keyword yet — Muddle, the
Ever-Changing grants it to itself temporarily via `become_copy_of_target_creature_gaining_myriad`,
§6).

Parametrized keywords are a single-key table:

```toml
keywords = ["flying", { ward = 2 }, { protection = "red" }]
```

| Tag | Shape | Meaning |
|-----|-------|---------|
| `ward` | `{ ward = N }` (u8) | Ward N (CR 702.21): an opponent targeting this must pay `{N}` or the spell/ability is countered. Modeled as a cast-time tax (`Game::cast`). |
| `protection` | `{ protection = "<value>" }` | Protection from a fixed color (CR 702.16, the "can't be blocked/targeted/damaged/blocked-by/dealt-damage-by that quality" core), or one of the non-color qualities `"creatures"` (a card type) / `"multicolored"` (CR 105.4, ≥2 colors). No "protection from everything" or "choose a color/type as this enters" — the value is fixed at print. `Creatures` is checked at the blocking and combat-damage sites (both have the source's `ObjectId`); at the targeting site it's never evaluated (no source `ObjectId` threaded there — see `Game::protection_blocks_source_colors`'s doc). |

All keywords are engine-effective (affect combat/timing/targeting/casting). Used on the card, in
token profiles, in `conditional_keywords` (§1), in `grant_to_attached`'s `keywords` array (e.g.
granting `ward` or `indestructible` to an enchanted/equipped creature — see
`chains_of_custody.toml`, `darksteel_mutation.toml`), in `anthem_static`'s `keywords` (a
continuous grant to your creatures — Ohran Frostfang's deathtouch), and in
`pump_until_end_of_turn`/`pump_creatures_you_control_until_end_of_turn`'s `keywords` array (§6) —
"target creature gains indestructible until end of turn" (Yahenni, Rogue's Passage) and
"creatures you control gain flying until end of turn" (Selfless Spirit, Moonshaker Cavalry) are
both expressible now.

A keyword granted only while a condition holds is the top-level `conditional_keywords` field
(§1), not a keyword spelling. Flashback, echo, delve, escape, and cycling are top-level *card*
fields (§1), not keywords.

**Unsupported keywords** (not in the enum — flag the card): kicker, cascade, adventure, etc.

## 5. `[[abilities]]`

Each ability is a `[[abilities]]` array entry with a `timing` and one or more
`[[abilities.effects]]` blocks. **Effects are always the `effects` array-of-tables** — even a
single-effect ability uses `[[abilities.effects]]` (there is no singular `effect` key; it is a
load error).

```toml
[[abilities]]
timing = "etb"

[[abilities.effects]]
type = "draw_cards"
count = 1
```
Inline form also works: `effects = [{ type = "put_counters", count = 2, target = "creature" }]`,
but the `[[abilities.effects]]` block form is standard. **When an ability has more than one effect,
each `[[abilities.effects]]` block is directly preceded by a `#` comment quoting the specific Oracle
clause that effect implements** (the same discipline §0 applies to `[[abilities]]`, one rung finer):
the ability's `#` comment quotes the whole sentence, and each effect's comment quotes its fragment
of it.

### `effects = [..]` — an ordered effect sequence

An ability may run **several effects in order as one resolution**, sharing the ability's target
and `{X}` (Faithless Looting's "Draw two cards, **then** discard two cards" is one ability, not
two). Give more than one `[[abilities.effects]]` block (or list more than one element inline):

```toml
[[abilities]]
timing = "spell"

[[abilities.effects]]
type = "draw_cards"
count = 2

[[abilities.effects]]
type = "discard"
count = 2
```

A single-effect ability is just the one-element case — a lone `[[abilities.effects]]` block stays
a bare effect (no `Sequence` wrapper). A step that pauses for a choice (`surveil`,
`discard`) defers the remaining steps until that choice is answered (Prismari Charm's "Surveil 2,
then draw a card" surveils, pauses, then draws once the arrange choice is made). Within one
`Sequence`, the shared target is the first step that needs one; a plain second targeting step
(Killian's `goad_target` after `tap_target`) shares that one chosen target. A step that needs its
*own* independent target set instead reads a **second target clause** (below).

**Independent target clauses.** A **non-modal spell** may carry more than one targeting
`[[abilities]]` block, each choosing its *own* target set at cast, in printed order (CR 601.2c —
Magma Opus's "4 damage divided among any number of targets. Tap two target permanents." is a
divided `deal_damage` clause followed by a separate `tap_target` clause). The engine chooses each
multi-target clause in sequence at cast; each clause resolves against only its own targets. Two
clauses max per spell (clause 0 → `Spell.targets`, clause 1 → `Spell.targets_second`); no pool
spell prints a third.

A **triggered ability** may likewise carry a *second* independent target clause, chosen as the
trigger goes on the stack (CR 603.3d, not at resolution — shroud/hexproof/protection are enforced
and responders can react to the specific set). Clause 0 is the ability's shared target (`ctx.target`);
clause 1 is a step whose effect reads its own `targets_second` list — today only
`double_counters_on_target_creatures` (Kinetic Ooze's "destroy up to one target artifact… If X is
10 or more, double the number of +1/+1 counters on any number of other target creatures"). The
second clause carries its own `target`/`count`, and — inside a `conditional` — its targets are only
chosen when the intervening-if gate holds at placement (CR 603.4). Two clauses max per ability.

When a step's own `count`/`targets` is multi-target (§7), the whole `Sequence` re-runs once per
chosen target — a per-target rider naturally follows an X-scaled exile/destroy/etc. with no extra
wiring (Curse of the Swine's "for each creature exiled this way, its controller creates a 2/2
Boar" is just a second `[[abilities.effects]]` step in the same ability). A rider that must instead
fire **once total**, independent of how many targets the multi-target step actually hit, can't live
in that `Sequence` — give it its own separate `[[abilities]]` block instead (same `timing = "spell"`,
no target of its own); the shared `Spell.targets`/`{X}` are still visible to it, but it isn't
re-run per target (Pest Infestation's "up to X target artifacts and/or enchantments. Create twice X
... tokens" — the token count is `Amount::TwiceX`, not per-destroyed-target, so it's a second,
untargeted `[[abilities]]`).

### `timing =` values

| Tag | Meaning |
|-----|---------|
| `"spell"` | The one-shot effect of an instant/sorcery (or a mode of a `modal` spell). |
| `"static"` | Continuous ability (anthem, cost reducer, aura/equip grant, counter replacement, enters-with-counters). |
| `"activated"` | Activated ability; see cost fields below. |
| `"etb"` | Triggers when this permanent enters. (`"etb_triggered"` is an accepted alias.) |
| `"turned_face_up"` | When this permanent is turned face up (CR 702.37f — a morph turned-face-up trigger; scans the now-revealed object's own abilities). |
| `"attacks"` | When this creature attacks. |
| `"dies"` | When this creature dies. |
| `"creature_dies"` | When *another* creature dies (watch-others, self-excluded). |
| `"creature_you_control_dies"` | When a creature you control dies (self-excluded). |
| `"creature_dies_including_this"` | "This creature or another creature dies" — `creature_dies` plus a self-fire off the dying source's last-known information (CR 603.6c/603.10a — Blood Artist). |
| `"creature_you_control_dies_including_this"` | The you-control twin of the above (Zulaport Cutthroat). |
| `"creature_you_control_dies_nontoken"` | `creature_you_control_dies` plus a token-death exclusion — a dying token never fires it (Blight Mound). |
| `"creature_you_control_dies_including_this_nontoken"` | `creature_you_control_dies_including_this` plus the same token-death exclusion on the *other*-creature half; the watcher's own death still self-fires unconditionally (Pawn of Ulamog). |
| `"creature_an_opponent_controls_dies"` | When a creature an opponent controls dies (Yahenni). |
| `"enchantment_you_control_dies"` | When an enchantment you control is put into a graveyard from the battlefield (Starfield Mystic). |
| `"upkeep"` | At the beginning of your upkeep. |
| `"each_upkeep"` | At the beginning of *each* player's upkeep — fires for the ability's controller regardless of whose turn (Beledros Witherbloom, Ophiomancer). |
| `"begin_combat"` | At the beginning of combat on your turn. Scoped to the active player only (an "each player" variant is unlanded). With card-level `modal = true` and no `spell`-timed ability, every `begin_combat` ability on the card is a mode of one modal trigger (Shadrix Silverquill's "you may choose two. Each mode must target a different player."): the controller picks `choose` distinct modes (§1), each with its own `target = "player"` effect (`create_token { controller = "target_player" }`, `put_counters_each { target_player = true }`, or a `target_player_draws`/`target_player_loses_life` `Sequence`), pairwise-distinct across the chosen modes — a restriction hardcoded to this one card's need, not a general modal-trigger axis. |
| `"end_step"` | At the beginning of your end step. |
| `"each_end_step"` | At the beginning of *each* end step — the every-player twin of `end_step` (Relic Retriever). |
| `"each_other_player_untap_step"` | At the beginning of every *other* player's untap step, excluding the controller's own (Drumbellower). The mirror image of `each_upkeep`/`each_end_step`, which include the controller's own. |
| `"you_gain_life"` | When you gain life. |
| `"you_lose_life_first_time_each_turn"` | When you lose life for the first time each turn (CR 118.9/119.3 — a *decrease* only; a second loss the same turn doesn't re-fire). "You"-scoped, fieldless (Intermediate Chirography's level 2). |
| `"magecraft"` | When you cast/copy an instant or sorcery. |
| `"player_attacks_your_opponent"` | When a player attacks one of your opponents (Breena). |
| `"you_attack_with_creatures"` | Once per combat, when this permanent's controller has attacked with the sibling `at_least` or more creatures this combat, any defenders (Firemane Commando's "whenever you attack with two or more creatures"). |
| `"opponent_attacks_you_with_creatures"` | Once per attacking opponent, when that opponent has attacked this permanent's controller with the sibling `at_least` or more creatures this combat (Mangara/Tomik's "an opponent attacks with creatures, if two or more of those are attacking you") — per-opponent, not pod-wide; the attacking opponent is addressable as "that opponent" (`attacker_loses_life_you_draw`, §6). |
| `"another_player_attacks_with_creatures"` | Once per attacking player other than this permanent's controller, when that player has attacked with the sibling `at_least` or more creatures this combat, *and none of those creatures attacked this permanent's controller* (Firemane Commando's "whenever another player attacks with two or more creatures, they draw a card if none of those creatures attacked you") — the "none attacked you" clause is a gate on the trigger firing, not a target restriction; the attacking player is addressable as "they" (`attacking_player_draws`, §6). |
| `"enchanted_creature_attacks"` (alias `"equipped_creature_attacks"`) | Whenever the creature/permanent this Aura or Equipment is attached to attacks — fires for the *attached permanent's* controller (the Impetus cycle for Auras; Fractal Harness's "whenever equipped creature attacks" for Equipment — same underlying trigger, either tag on an Equipment card reads the same). |
| `"enchanted_creature_dies"` | Aura only: when the enchanted creature dies (Angelic Destiny's self-return). |
| `"enchanted_creature_deals_damage"` | Whenever the creature this Aura is attached to deals damage, combat or noncombat alike (Armadillo Cloak: "you gain that much life") — fires for the *Aura's own* controller, not the host's, off `Game::deal_creature_damage`/`Game::damage_player`'s events. Distinct from lifelink: a separate triggered ability, so it stacks additively with real lifelink on the same creature. Pair with `"triggering_damage_dealt"` (§5 Amounts). |
| `"an_enchanted_creature_dies"` | Watch-others twin of `enchanted_creature_dies`: on any permanent, whenever *any* enchanted creature dies, gated on this permanent's controller having controlled at least one Aura attached to it (Hateful Eidolon's "draw a card for each Aura you controlled that was attached to it" — pair with `"auras_you_controlled_attached_to_dying_creature"`, §5 Amounts). |
| `"creature_enchanted_by_your_aura_attacks"` | Once per combat, when the sibling `at_least` or more of this combat's whole attacker set (any defender) are each enchanted by an Aura this permanent's controller controls (Killian, Decisive Mentor's second ability, "one or more creatures ... attack" — `at_least = 1`). |
| `"you_sacrifice"` | Whenever this permanent's controller sacrifices a permanent matching `filter` (Smothering Abomination: "whenever you sacrifice a creature"). |
| `"any_player_sacrifices"` | Whenever *any* player sacrifices a permanent matching `filter` — a watch-others trigger (Mazirek: "whenever a player sacrifices another permanent", via `filter`'s `other = true`). |
| `"you_discard"` | Whenever you discard a card (Containment Construct; pairs with the `exile_from_graveyard_may_play`/`exile_discarded_with_this` payoffs, §6). |
| `"deals_combat_damage_to_player"` | Whenever a creature deals combat damage to a player, scoped by the sibling `who` field (below). |
| `"zero_base_power_creatures_deal_combat_damage"` | Whenever one or more creatures this permanent's controller controls, each with **base** power 0, deal combat damage to a player — batch-once per defending player, summed across every qualifying attacker that combat (Primo, the Unbounded). The summed damage rides in `"combat_damage_dealt"` (§5 Amounts). Base-power-0 filter is hard-coded, not a sibling field — the pool's only consumer. |
| `"cast_spell"` | Whenever a player casts a spell matching the sibling `spell_filter`, scoped by `caster`/`nth_each_turn`/`from_hand` (Monologue Tax, Sram, Dirgur Focusmage). |
| `"player_draws"` | Whenever a player draws, scoped by the sibling `drawer`/`nth_each_turn` (Faerie Mastermind's "an opponent draws their second card each turn"). |
| `"activate_ability"` | Whenever a player puts an activated ability whose activation cost contains `{X}` on the stack, scoped by the sibling `caster` (reuses `"cast_spell"`'s `caster` — Unbound Flourishing's "or activate an ability … copy that ability" is `caster = "you"`). Gated on the *cost* containing `{X}`, not the chosen value (an `{X}` = 0 activation still fires). Pairs with the `copy_triggering_ability` effect (§6). |
| `"permanent_enters"` | Whenever *another* permanent matching `filter` enters, scoped by the sibling `controller` — constellation, landfall-watch (Ajani's Chosen, Archaeomancer's Map). Self-excluded. |
| `"permanent_enters_including_this"` | Same, plus a self-fire on this permanent's own entry (Doomwake Giant's constellation). |
| `"cards_leave_your_graveyard"` | Whenever one or more cards leave your graveyard — batch-once, not per card (Quintorius, Kirol). |
| `"cards_exiled_from_your_library_or_graveyard"` | Whenever one or more cards are exiled from your library and/or graveyard — batch-once (Laelia). |
| `"you_create_token"` | Whenever you create one or more creature tokens — batch-once (Staff of the Storyteller). |
| `"becomes_targeted"` | Whenever this permanent becomes the target of a spell (Goldspan Dragon). (`"becomes_the_target"` is an accepted alias.) |
| `"spell_targets_this"` | Whenever a player casts a spell matching the sibling `spell_filter` whose *only* target is this permanent (Mirrorwing Dragon: "an instant or sorcery spell that targets only this creature"). Reuses `"cast_spell"`'s `spell_filter` sibling; a `"becomes_targeted"` twin narrowed to "only" — same single-target `Event::SpellCast` field, so (like `"becomes_targeted"`) it only sees a spell whose own spec is single-target, not a multi-target spell's post-cast selection. |
| `"when_you_cast_this"` | "When you cast this spell" (CR 601.2i): scanned off the cast card's own abilities, not a battlefield watcher — placed on the stack above the spell, controller = caster, and resolves independently even if the spell is later countered (Hydroid Krasis). |
| `"cycled"` | "When you cycle this card" (CR 702.29e): scanned off the cycled card's own abilities at `Game::cycle`'s discard, like `"when_you_cast_this"`'s self-scan — not a battlefield watcher. Cycling's draw itself is a real activated ability on the stack (CR 702.29a — see `cycling`/`hand_ability` in §1, respondable by Azorius Guildmage's `counter_target_activated_ability`), and the cycled trigger is placed *above* it, so it resolves first (Krosan Tusker's "you may search your library for a basic land card … (Do this before you draw.)"). Pairs with `optional = true` and, for a paid rider, an `[abilities.cost]` carrying `x = true`/`x = N` (Decree of Justice's "you may pay {X}. If you do, create X 1/1 white Soldier creature tokens.") — answered by `Intent::PayOptionalCostX { pay, x }`, a dedicated wire shape (not `PayOptionalCost`) so the chosen `{X}` threads onto the placed ability's stack item for its own `Amount::X` to read (same mechanism an activated ability's `{X}` cost uses). |
| `"this_put_into_graveyard"` | When THIS permanent is put into a graveyard from the battlefield (CR — Fallen Ideal's Aura-death rider). Self-referential like `"dies"`, but not creature-scoped: fires for any permanent kind. Guarded to a live battlefield permanent immediately before the move, so a milled/discarded copy of the same card does not fire it. |
| `"this_leaves_battlefield"` | When THIS permanent leaves the battlefield to *any* zone — destroy/exile → graveyard/exile, bounce → hand, tuck → library (Animate Dead's "when this Aura leaves the battlefield, that creature's controller sacrifices it"). Broader than `"this_put_into_graveyard"` (graveyard-only): fires off any battlefield-exit event. Carries no filter of its own, but the permanent this was attached to at the instant it left (CR 603.10a last-known information) feeds a following `sacrifice_enchanted_creature` effect. |
| `"spend_mana_to_cast"` | When you spend mana **this permanent produced** to cast a spell matching the sibling `spend_predicate` — Study Hall / Path of Ancestry. Source-scoped and provenance-driven: fires only for the specific land whose `track_provenance` `add_mana` (§6) tagged the spent mana, and only when that mana funds a qualifying cast. Fires at the cast payment (off `SpellCast`, reading the paid-with spend multiset). |

Watch-others death triggers see only creatures still on the battlefield; the `*_including_this`
arms are the "this creature or another" wording — they additionally self-fire off the dying
creature's own last-known information, so amounts like `"source_power"` read pre-death values.

### Activated-ability cost fields (only with `timing = "activated"`)

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `taps_self` | bool | `false` | `{T}` in the cost. |
| `[abilities.activation_cost]` | `[cost]` table | free | Mana cost (same shape as §2, hybrid included — Fetid Heath). |
| `sacrifice` | string or table | `"none"` | `""`/`"none"`, `"this"` (sac this permanent), `"creature"` (sac a creature you control, count 1), a `{ creature = { … }, count = N }` table of permanent-filter overrides (§7) plus an optional sacrifice count — Izoni's "Sacrifice another creature" is `sacrifice = { creature = { other = true } }`; Priest of Forgotten Gods's "Sacrifice two other creatures" is `sacrifice = { creature = { other = true }, count = 2 }` — or a `{ permanent = { … }, count = N }` table for a **non-creature** sacrifice cost (Gyome, Master Chef's "Sacrifice a Food" is `sacrifice = { permanent = { subtypes = ["Food"] } }`). `count` omitted = 1. `creature`'s table forces `types` to creature; `permanent`'s leaves the filter's own `types`/`subtypes` unforced. The activator names exactly `count` distinct matching permanents they control in the activating intent — an illegal/short/duplicated choice rejects the activation (CR 602.2b). |
| `pay_life` | amount | `0` | Life paid as part of the cost (fetchlands' "Pay 1 life"; War Room's `"commander_color_count"`) — any Amount (§7), resolved with `x = 0`. Can't activate if life < this. |
| `remove_counters` | u8 | `0` | +1/+1 counters removed from the source as part of the cost (Steelbane Hydra's "Remove a +1/+1 counter"). Fewer on the source ⇒ can't activate (CR 602.2b). |
| `remove_counters_kind` | string | absent | Which counter kind `remove_counters` removes: `"charge"`/`"story"`/`"study"`/`"vow"` (Staff of the Storyteller's "remove a story counter"). Absent = the +1/+1 path. |
| `self_damage` | u8 | `0` | Damage the source deals to the activator as a rider on the effect (painlands'/Talismans' "This land deals 1 damage to you"). Unlike `pay_life` it is **not** a cost — it never gates activation (you may tap a painland at 1 life). Modeled as life loss. |
| `return_self` | bool | `false` | "Return this to its owner's hand" as part of the cost (Rootha, Mercurial Artist's "Return Rootha to its owner's hand"). A token ceases to exist instead (CR 111.7). |
| `mill_self` | u8 | `0` | "Mill a card" as part of the cost (Millikin's "Mill a card") — the activator mills this many of their own cards. A library with fewer cards can't pay it (CR 602.2b). |
| `exile_self` | bool | `false` | "Exile this artifact"/"exile this permanent" as part of the cost (Perpetual Timepiece's "Exile this artifact"). A token ceases to exist instead (CR 111.7), same fork as `return_self`. |
| `loyalty` | i32 | absent | Present ⇒ planeswalker loyalty ability (e.g. `loyalty = 1`, `-4`); absent ⇒ ordinary activated ability. |
| `once_each_turn` | bool | `false` | "Activate only once each turn" (CR 602.2b — Beledros). Also legal on a *triggered* ability: "triggers only once each turn" (Morbid Opportunist). |
| `sorcery_speed` | bool | `false` | "Activate only as a sorcery" (CR 602.5b — Ozolith, the Shattered Spire). |

### `min_level` (Class level gate, CR 717.5)

Any `[[abilities]]` block (triggered, static, or activated) may carry `min_level = N` (u8,
`#[serde(default)]` 0): the ability functions only while its source permanent is at Class level
`N` or higher. `0` (the default) is unconditional; every ordinary permanent is trivially level 1.
Only a Class enchantment (§9) raises its level past 1; a level-up ability itself keeps
`min_level` 0. See §9 for the full Class layout.

```toml
# {T}, Sacrifice a creature: You gain 1 life.
[[abilities]]
timing = "activated"
taps_self = true
sacrifice = "creature"

[[abilities.effects]]
type = "gain_life"
amount = 1
```
```toml
# {T}, Sacrifice two other creatures: ...
[[abilities]]
timing = "activated"
taps_self = true
sacrifice = { creature = { other = true }, count = 2 }
```
```toml
# {1}, Sacrifice a Food: ...
[[abilities]]
timing = "activated"
sacrifice = { permanent = { subtypes = ["Food"] } }

[abilities.activation_cost]
generic = 1
```
```toml
# {2}, Return this to its owner's hand: Copy target instant or sorcery spell you control.
[[abilities]]
timing = "activated"
return_self = true

[abilities.activation_cost]
generic = 2

[[abilities.effects]]
type = "copy_target_spell"
```
```toml
# {T}, Mill a card: Add {C}.
[[abilities]]
timing = "activated"
taps_self = true
mill_self = 1

[[abilities.effects]]
type = "add_mana"
mana = ["colorless"]
```
```toml
# {2}, Exile this artifact: Shuffle any number of target cards from your graveyard into your library.
[[abilities]]
timing = "activated"
exile_self = true

[abilities.activation_cost]
generic = 2

[[abilities.effects]]
type = "shuffle_target_cards_from_graveyard_into_library"
```
```toml
# {T}: Add {R} or {W}. This land deals 1 damage to you. (a painland's colored mode)
[[abilities]]
timing = "activated"
taps_self = true
self_damage = 1

[[abilities.effects]]
type = "add_mana"
mana = [["red", "white"]]
```
```toml
# Equip {1}
[[abilities]]
timing = "activated"

[abilities.activation_cost]
generic = 1

[[abilities.effects]]
type = "equip"
```

### `optional` (optional trigger, CR 603.2c "may")

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `optional` | bool | `false` | Only meaningful on a `Timing::Triggered` ability. `true` raises a yes/no "may" choice before the trigger goes on the stack; declining discards it. |
| `[abilities.cost]` | `[cost]` table | free | A non-free "may" cost (CR 603.2c, a pay-N-or-decline trigger) — Trudge Garden's "you may pay {2}." Only meaningful with `optional = true`; omitted = a plain free "may". Same shape as a top-level `[cost]` table (§2), including `x = true`/`x = N` for a pay-`{X}` rider (Decree of Justice's "you may pay {X}" — the chosen `{X}` is picked when accepting, via `Intent::PayOptionalCostX`, and feeds the placed ability's own `Amount::X`). |

```toml
# "When this creature dies, you may draw a card."
[[abilities]]
timing = "dies"
optional = true

[[abilities.effects]]
type = "draw_cards"
count = 1
```

```toml
# "Whenever you gain life, you may pay {2}. If you do, create a 4/4 green Fungus Beast."
[[abilities]]
timing = "you_gain_life"
optional = true

[abilities.cost]
generic = 2

[[abilities.effects]]
type = "create_token"
count = 1
[abilities.effects.token]
name = "Fungus Beast"
power = 4
toughness = 4
keywords = ["trample"]
```

Only a single optional trigger per ability is wired (an optional trigger that's also one of
several simultaneous triggered abilities isn't modeled, see ADR 0006).

A modal *triggered* ability's "you may choose N" (§5 `begin_combat` note) puts `optional = true`
on every one of the card's modes — the whole modal choice is one "may," not a per-mode one;
declining (choosing zero modes) drops the ability entirely, same as a plain optional trigger.



### Trigger sibling fields (`filter`, `controller`, `who`, `spell_filter`, `caster`, `drawer`, `nth_each_turn`)

Some triggers carry parameters as flat sibling fields next to `timing`; each is ignored by every
other timing.

| Field | Type | Default | Used by | Meaning |
|-------|------|---------|---------|---------|
| `filter` | permanent filter (§7) | matches everything | `you_sacrifice`, `any_player_sacrifices`, `permanent_enters(_including_this)` | Which sacrificed/entering permanent the trigger cares about. `other = true` means "another permanent" — excludes the ability's own source (Mazirek). |
| `controller` | `"you"` (default), `"opponent"`, `"any_player"` | `"you"` | `permanent_enters(_including_this)` | Whose permanent the watch cares about, relative to the ability's controller (constellation's "you control"; Archaeomancer's Map's "an opponent controls"). |
| `who` | `"this"` (default), `"your_creatures"`, `"your_tokens"` | `"this"` | `deals_combat_damage_to_player` | Whose combat damage counts (Leitmotif Composer / Ohran Frostfang / Curiosity Crafter). |
| `spell_filter` | spell filter (§7) | `"all"` | `cast_spell` | Which cast spells count (Sram's `{ has_subtype = ["Aura", "Equipment", "Vehicle"] }`). |
| `caster` | `"you"` (default), `"opponent"`, `"any_player"` | `"you"` | `cast_spell`, `activate_ability` | Whose cast/activation counts (Monologue Tax's `"opponent"`; Unbound Flourishing's `activate_ability` half is `"you"`). |
| `drawer` | `"you"` (default), `"opponent"`, `"any_player"` | `"you"` | `player_draws` | Whose draw counts (Faerie Mastermind's `"opponent"`). |
| `nth_each_turn` | u8 | absent (every) | `cast_spell`, `player_draws` | Fire only on exactly the watched player's Nth spell/draw that turn ("their second … each turn" = `2`). Filter-scoped for `cast_spell` when `spell_filter = "has_x"`: counts only {X}-spells cast this turn (Nev, Zimone Infinite Analyst's "first spell with {X} … each turn"), not the whole-turn spell count. Every other `spell_filter` still reads the unfiltered whole-turn tally (Monologue Tax/Mangara's "second spell"). |
| `at_least` | u8 | `0` | `you_attack_with_creatures`, `opponent_attacks_you_with_creatures`, `another_player_attacks_with_creatures`, `creature_enchanted_by_your_aura_attacks` | The attacker-count threshold ("two or more creatures" = `2`; "one or more" = `1`). |
| `from_hand` | bool, `#[serde(default)]` | `false` | `cast_spell` | Restrict to a spell cast from its controller's hand — `false` (default) fires on a cast from any zone (flashback/escape, command zone, impulse-play); `true` excludes those (Dirgur Focusmage's "you cast … from your hand"). |
| `spend_predicate` | `"commander"`, `"creature_sharing_type_with_commander"` | `"commander"` (unread otherwise) | `spend_mana_to_cast` | Which cast the spend-to-cast rider accepts: your commander (Study Hall, Opal Palace) or a creature spell sharing a creature type with your commander (Path of Ancestry). |

```toml
# "Whenever you sacrifice a creature, draw a card." (Smothering Abomination)
[[abilities]]
timing = "you_sacrifice"
filter = "creature"

[[abilities.effects]]
type = "draw_cards"
count = 1
```
```toml
# "Whenever an opponent draws their second card each turn, you draw a card." (Faerie Mastermind)
[[abilities]]
timing = "player_draws"
drawer = "opponent"
nth_each_turn = 2

[[abilities.effects]]
type = "draw_cards"
count = 1
```
```toml
# "Whenever an opponent attacks with creatures, if two or more of those creatures are attacking
# you and/or planeswalkers you control, that opponent loses 3 life and you draw a card."
# (Tomik, Wielder of Law)
[[abilities]]
timing = "opponent_attacks_you_with_creatures"
at_least = 2

[[abilities.effects]]
type = "attacker_loses_life_you_draw"
life_loss = 3
```
```toml
# "Whenever another player attacks with two or more creatures, they draw a card if none of
# those creatures attacked you." (Firemane Commando)
[[abilities]]
timing = "another_player_attacks_with_creatures"
at_least = 2

[[abilities.effects]]
type = "attacking_player_draws"
count = 1
```

On a sacrifice-watch, only the type and `other` axes are read (the sacrificed permanent has
already left the battlefield by the time the filter is checked, so the battlefield-only axes —
`tapped`, `enchanted`, `mv_max`, `token` — never apply).

### `[abilities.condition]` (optional intervening-if, CR 603.4)

The one `Condition` shape appears in six places: an ability's intervening-if (here — on an
*activated* ability it's an activation restriction instead, CR 602.5: Temple of the False God,
Mistveil Plains), the top-level `enters_tapped_unless` table (§1), a `conditional_keywords` entry
(§1), a conditional Amount (§7's `{ condition, then }`), the `conditional` effect step (§6), and
`anthem_static`'s own `condition` axis (§6 — gates the whole anthem, e.g. "as long as you have
the city's blessing").

| `type` | Extra fields |
|--------|-------------|
| `"you_control_at_least_creatures"` | `count` (u32) |
| `"attacked_opponent_has_more_life_than_another_opponent"` | — |
| `"controls_lands_with_subtype"` | `subtypes` (array of land-type strings), `count` (u32) — "control `count`+ lands whose type line has any of `subtypes`" |
| `"controls_basic_lands"` | `count` (u32) — "control `count`+ basic lands" |
| `"opponents_control_lands"` | `count` (u32) — "opponents control `count`+ lands, combined" |
| `"hand_has_land_with_subtype"` | `subtypes` (array of land-type strings) — modeled as an automatic hand scan, not a real reveal choice (see the reveal-land examples below) |
| `"opponent_controls_more_lands"` | — (Land Tax). Paired with an entering-permanent trigger it narrows to *that* permanent's controller (Archaeomancer's Map's "if that player controls more lands than you"). |
| `"you_control_lands"` | `at_least` (u32) — "you control N or more lands" (Fabled Passage's untap gate; Temple of the False God's activation restriction) |
| `"you_gained_life_this_turn"` | — (Witch of the Moors; Mortality Spear's cost reduction) |
| `"modified_creature_died_this_turn"` | — "a modified creature died under your control this turn" (CR 700.4/701.29; Intermediate Chirography's Level 3). Reads the controller's turn-scoped `Player::modified_creature_died_this_turn` flag, set at the death choke by last-known information (the dying creature's own counters/Aura/Equipment, read before the zone change tears them down). |
| `"card_left_your_graveyard_this_turn"` | — (Relic Retriever) |
| `"cast_instant_or_sorcery_this_turn"` | — (Hall of Oracles's activation restriction) |
| `"you_control_no_subtype"` | `subtypes` (array) — "you control no Snakes" (Ophiomancer) |
| `"you_control_no_creature_with_keyword"` | `keyword` (a §4 keyword string) — "you control no creatures with decayed" (Jadar, Ghoulcaller of Nephalia); the effective-keyword sibling of `you_control_no_subtype` |
| `"source_has_counters"` | `at_least` (u32) — N+ +1/+1 counters on the source (Primordial Hydra's `conditional_keywords` trample gate; also usable as an `[abilities.condition]` intervening-if — Ingenious Prodigy's upkeep may-draw) |
| `"source_has_no_counters_of_kind"` | `kind` (`"charge"`/`"story"`/`"study"`/`"vow"`) — "has no charge counters on it" (Mana Bloom) |
| `"you_control_color_permanents"` | `color` (color name), `at_least` (u32) — "two or more white permanents" (Mistveil Plains's activation restriction) |
| `"this_permanent_entered_untapped"` | — (Mystic Sanctuary's ETB intervening-if) |
| `"triggering_spell_mana_value_at_least"` | `at_least` (u8) — "if that spell's mana value is N or greater" on a `timing = "cast_spell"` ability (Prismari Pianist's "create three instead"). Resolved when the ability is placed on the stack (CR 603.4), not re-checked at resolution — use inside an `{ type = "conditional", condition = …, then = […] }` step, not `[abilities.condition]`. |
| `"you_have_citys_blessing"` | — "as long as you have the city's blessing" (CR 702.131, Ascend — Tendershoot Dryad's Saproling anthem). Reads the controller's sticky city's-blessing flag, granted by a state-based action once they control ten or more permanents and never cleared. |
| `"any_player_hand_size_at_most"` | `at_most` (u32) — "a player has `at_most` or fewer cards in hand" (naktamun_lorespinner's "if a player has one or fewer cards in hand"). An existential over *every* seated player, not just the controller — holds as soon as one living player's hand is small enough. |
| `"instant_or_sorcery_cards_in_your_graveyard_at_least"` | `count` (u32) — "`count`+ instant and/or sorcery cards in your graveyard" (Animist's Awakening's spell mastery). Counts the same way `Amount::InstantOrSorceryCardsInYourGraveyard` does (any card whose kind is `spell` — creature/land cards never count). |
| `"artifact_or_creature_cards_in_your_graveyard_at_least"` | `count` (u32) — "`count`+ artifact and/or creature cards in your graveyard" (Lorehold Archivist's prepare trigger). Counts the controller's graveyard cards whose kind is artifact or creature. |
| `"target_power_at_least"` | `at_least` (u32) — "if that creature has power `at_least` or greater" (Yavimaya Bloomsage's "Then if that creature has power 7 or greater, this creature becomes prepared"). Reads the *resolving effect's own chosen target's* power, not `TriggerContext` — only reachable inside `{ type = "conditional", … }`, not `[abilities.condition]` (an intervening-if has no chosen target yet). |
| `"an_opponent_has_life_at_most"` | `at_most` (u32) — "as long as an opponent has `at_most` or less life" (Bloodghast's conditional haste). An existential over the ability controller's living opponents (CR 104.3a); holds as soon as one is at or below `at_most`. Evaluated live, so a life change across the boundary flips a gated `anthem_static` on or off. |
| `"source_entered_with_x_at_least"` | `at_least` (u32) — "if X is `at_least` or more" (CR 601.2b; Kinetic Ooze's "If X is 5 or more, you draw a card"). Reads the source permanent's *locked cast* `{X}` (`Permanent::entered_with_x`), not any live board state (e.g. counters that happen to equal X at ETB but can change before the ability resolves) — only reachable inside `{ type = "conditional", … }`, not `[abilities.condition]` (same shape as `target_power_at_least` above). |
| `"all"` | `conditions` (array of `Condition`) — a composed AND of every element (Zimone, All-Questioning's "if a land entered the battlefield under your control this turn and you control a prime number of lands"). Holds iff every nested condition holds. |
| `"land_entered_under_your_control_this_turn"` | — "a land entered the battlefield under your control this turn" (Zimone, All-Questioning). CR landfall's own "enters," not "played" — a cast, fetched, or token land all set it. |
| `"you_control_prime_number_of_lands"` | — "you control a prime number of lands" (Zimone, All-Questioning). Trial division over `Game::lands_controlled`. |
| `"during_your_turn"` | — "during your turn" (Restless Spire's animated form: "During your turn, this creature has first strike"). Holds iff the controller is the active player right now; re-evaluated live, not cached at animation time. Pair with `anthem_static`'s `self_only = true` to gate a manland's own conditional keyword. |
| `"color_was_spent_to_cast_this"` | `color` (a color) — "if `color` was spent to cast this" (CR 106.9; Court Hussar's "unless {W} was spent to cast it" — pair with `negate = true` on the enclosing `conditional`, §6). Reads the source permanent's locked-in `Permanent::spent_colors`, snapshotted from the casting spell's actual mana payment (which colors funded which pips, including the generic) — only reachable inside `{ type = "conditional", … }`, not `[abilities.condition]` (same shape as `source_entered_with_x_at_least` above). ponytail: only a literally-colored mana source counts — a dual/filter/"any" credit spent toward a pip isn't attributed to either of its colors (see `ManaPool::colors_spent`'s doc); no pool card's mana base exercises that gap yet. |

```toml
[abilities.condition]
type = "you_control_at_least_creatures"
count = 3
```

```toml
# Clifftop Retreat: "This land enters tapped unless you control a Mountain or a Plains."
[enters_tapped_unless]
type = "controls_lands_with_subtype"
subtypes = ["Mountain", "Plains"]
count = 1
```
```toml
# Eclipsed Steppe: "This land enters tapped unless you control two or more basic lands."
[enters_tapped_unless]
type = "controls_basic_lands"
count = 2
```
```toml
# Turbulent Fen: "This land enters tapped unless your opponents control eight or more lands."
[enters_tapped_unless]
type = "opponents_control_lands"
count = 8
```
```toml
# Vineglimmer Snarl: "As this land enters, you may reveal a Forest or Island card from your
# hand. If you don't, this land enters tapped." Revealing is strictly better with no downside,
# so there's no real choice — modeled as an automatic scan of the controller's hand.
[enters_tapped_unless]
type = "hand_has_land_with_subtype"
subtypes = ["Forest", "Island"]
```

## 6. Effects (`[[abilities.effects]] type =`)

Every accepted tag. "Target" column: does it read a `target` field (§7)?

| `type` | Fields | Target |
|--------|--------|--------|
| `deal_damage` | `amount` (amount), `target`, `count` (target-count, §7 — default one mandatory target; "up to two target" is `{ max = 2 }`, each taking `amount`), `divided` (bool, default `false` — one `amount` total split across the chosen targets, ≥1 each; Magma Opus's "divided as you choose among any number of targets" is `target = "any"`, `count = { max = 4 }`, `divided = true` — the targets are whatever `target` admits, so `"any"` lets the split fall on **players and planeswalkers** as well as creatures) | yes |
| `draw_cards` | `count` (amount) | no |
| `discard` | `count` (u32), `target_player` (bool, default `false`), `or_one_matching` (card filter, §7, optional) | no by default (the controller discards — fewer in hand discards the whole hand); `target_player = true` targets a player instead (Prismari Command); `or_one_matching` is an escape valve — a single card matching the filter satisfies the whole discard in place of the `count`-card answer (Compulsive Research's "discards two cards unless they discard a land card", `or_one_matching = "land"`); no matching card in hand collapses back to the plain `count`-card discard |
| `target_player_draws` | `count` (amount), `opponent` (bool, default `false` — restrict to "target opponent", Secret Rendezvous) | intrinsic (a player) |
| `target_player_may_draw` | `count` (amount), `opponent` (bool, default `false` — restrict to "target opponent", Questing Phelddagrif's "Target opponent may draw a card") | intrinsic (a player) — the optional twin of `target_player_draws`: pauses the *targeted* player (not the ability's controller) on a `MayYesNo`; "yes" draws them `count` cards directly, no pay window |
| `gain_life` | `amount` (amount) | no |
| `lose_life` | `amount` (amount) | no (the controller loses — Reanimate's "you lose life equal to its mana value" via `"target_mana_value"`) |
| `gain_life_target_controller` | `amount` (amount) | no (the *shared target's* controller gains — Swords to Plowshares' rider; pair with the targeting step in an `effects` sequence, before the target leaves the battlefield) |
| `manifest` | — | no (the *shared target's* controller manifests — Reality Shift's rider, CR 701.34: that player puts their top library card onto the battlefield face down as a 2/2 creature. Pair with the targeting step in an `effects` sequence; reads the target's controller like `gain_life_target_controller`. No-op on an empty library) |
| `add_mana` | `mana` (array of mana symbols, §8), `repeat` (amount, default 1 — adds the batch `repeat` times, e.g. Mana Geyser's `{R}` per tapped opponent land), `identity` (u8, default 0 — commander-color-identity credits, Arcane Signet), `opponent_colors` (u8, default 0 — "any color an opponent's land could produce" credits, Fellwar Stone), `restriction` (§8, default none — "spend this mana only to…", wraps the whole batch, Troyan Gutsy Explorer), `single_color` (bool, default `false` — CR 106.4 "add N mana of any one color": locks every `"any"` credit in `mana` × `repeat` to one color the controller chooses when the ability resolves, pausing on a `ChooseManaColor` choice instead of resolving straight to mana; Lotus Field's three mana, Kami of Whispered Hopes's power-many mana. Only the `mana` field's `"any"` credits are locked — no pool card combines it with `identity`/`opponent_colors`/colored/colorless credits on the same ability), `track_provenance` (bool, default `false` — record each credit this ability produces against its own source, so a later spell-cast payment can fire the source's `spend_mana_to_cast` trigger, "When you spend this mana to cast …"; Study Hall / Path of Ancestry. Cleared with the pool at every step/phase boundary), `target` (§7 target spec, default none — a player this mana ability targets, whose hand size a `"cards_in_target_player_hand"` `repeat` reads; Rousing Refrain's `target = "opponent"`. Only ever set on a `timing = "spell"` ability — an activated mana ability can't target, CR 605.1a), `persist_until_end_of_turn` (bool, default `false` — CR 500.4's "until end of turn, you don't lose this mana as steps and phases end" exception; Rousing Refrain, the only pool card that prints it. The batch still empties at the turn's actual end, CR 514.2 cleanup) | no |
| `pump_until_end_of_turn` | `power` (amount), `toughness` (amount), `target`, `keywords` (array, default `[]`) | yes |
| `pump_self_until_end_of_turn` | `power`, `toughness` (amounts), `keywords` (array, default `[]` — Questing Phelddagrif's "This creature gains protection from black and from red until end of turn" / "…gains flying until end of turn") | no (the ability's own source — no choice, so unlike `pump_until_end_of_turn`'s `target = "this"` it never claims the enclosing `Sequence`'s shared target — pair it with a step that *does* target, e.g. an opponent-directed rider, in one activated ability) |
| `pump_creatures_you_control_until_end_of_turn` | `power`, `toughness` (amounts), `keywords` (array, default `[]`), `filter` (§7 permanent filter, default: any) | no (mass version of `pump_until_end_of_turn`; every creature you control matching `filter` — subtype-scope with `filter = { subtypes = [...] }`, Quintorius) |
| `grant_keywords_to_permanents_you_control_until_end_of_turn` | `keywords` (array, default `[]`), `filter` (§7 permanent filter, default: any) | no (keyword-only twin of `pump_creatures_you_control_until_end_of_turn` — no P/T, no creature gate, so it reaches noncreature permanents; Silkguard's "Auras, Equipment … you control gain hexproof", `filter = { subtypes = ["Aura", "Equipment"] }`) |
| `keyword_anthem_static` | `keywords` (array, default `[]`), `filter` (§7 permanent filter, default: any) | no (static twin of `grant_keywords_to_permanents_you_control_until_end_of_turn` — same "you control" scan + filter shape, but read fresh on every keyword recompute instead of resolved once; Sterling Grove's "Other enchantments you control have shroud" is `keywords = ["shroud"]`, `filter = { types = ["enchantment"], other = true }`) |
| `set_base_pt_target_until_end_of_turn` | `power`, `toughness` (amounts), `target` | yes (sets the target creature's base P/T until end of turn — a CR 613.3(7b) base SET, so counters/pumps layer on top; Quandrix Charm mode 3's "base power and toughness 5/5") |
| `set_base_pt_creatures_you_control_until_end_of_turn` | `power`, `toughness` (amounts), `other` (bool, default false) | no (mass base-P/T SET on every creature you control until end of turn — the base-set twin of `pump_creatures_you_control_until_end_of_turn`; Biomass Mutation's "base power and toughness X/X". `other = true` excludes the source itself — Tanazir's "*other* creatures you control … become equal to Tanazir's power and toughness", with `power`/`toughness` = `"source_power"`/`"source_toughness"`) |
| `animate_self_until_end_of_turn` | `add_types` (type set, default none), `add_subtypes` (array, default `[]`), `base_power`, `base_toughness` (plain `i32`), `keywords` (array, default `[]`), `add_colors` (array of colors, default `[]` — unioned onto `Game::colors_of` while the animation is live, CR 105.2a) | no (manland self-animation: the ability's own source gains `add_types`/`add_subtypes` (CR 613.4), has its base P/T SET to `base_power`/`base_toughness` (613.3(7b)), and gains `keywords`/`add_colors`, all until end of turn — Restless Spire → "becomes a 2/1 blue and red … Elemental creature. It's still a land"; a noncreature land animating into a creature. A *conditional* grant like Restless Spire's "During your turn, this creature has first strike" doesn't belong in `keywords` here — pair a `self_only` `anthem_static` with `condition = { type = "during_your_turn" }` instead, since this effect's own `keywords` are unconditional for the whole until-EOT window) |
| `pump_other_attackers_attacking_your_opponents` | `power`, `toughness` (plain `i32`) | no (the Impetus cycle's "other attacker" anthem, Martial Impetus; scans the committed attacker set at resolution — excludes the ability's own enchanted host and any attacker whose defender is the ability's controller; pair with `timing = "enchanted_creature_attacks"`) |
| `enchanted_attacker_pump_attacking_opponent_else_controller_loses_life` | `power`, `toughness` (plain `i32`), `life` (`u32`) | no (Scriv, the Obligator's Contract token: "it gets +power/+toughness until end of turn if it's attacking one of your opponents. Otherwise, its controller loses `life` life." Reads the enchanted host off the ability's own source; "one of your opponents" = the host's declared defender being a player other than the Aura's controller; pair with `timing = "enchanted_creature_attacks"`) |
| `anthem_static` | `power`, `toughness` (amounts, default 0 — may scale off a live count: Storm-Kiln Artist's "+1/+0 for each artifact"), `keywords` (array, default `[]` — Ohran Frostfang's deathtouch), `subtypes` (array, default `[]` — "Spirits you control", Quintorius), `colors` (array of colors, default `[]` — restricts to creatures whose color set intersects it (CR 105.2, `Game::colors_of`); `[]` matches every color; Balefire Liege's "Other **red** creatures you control get +1/+1" is two separate `anthem_static` effects, `colors = ["red"]` and `colors = ["white"]`), `chosen_subtype` (bool, default `false` — restricts to creatures of the source's own as-enters chosen creature type, `Permanent::chosen_subtype`; `None` matches nothing; Patchwork Banner. ANDs with `subtypes` if both set — no pool card combines them), `self_only` (bool — pumps *only* its own source), `exclude_source` (bool, default `false` — CR "**other** … creatures you control": excludes the source from the creatures it buffs, still buffing the rest of the team — the opposite restriction from `self_only`; Balefire Liege/Creakwood Liege), `tokens_only` (bool, default `false` — restricts to token creatures the controller controls, checked against `Permanent::token`; Brudiclad, Telchor Engineer's "Creature tokens you control have haste"), `attacking_only` (bool — "attacking creatures you control"), `commander_only` (bool — "commander creatures you control", Guardian Augmenter), `has_counters` (bool, default `false` — "creatures you control with counters on them", Nev the Practical Dean's trample grant; read live off [`Game::has_any_counter`] — +1/+1, every named kind, and the finality counter, CR 122.1's unqualified "counter"), `condition` (optional §5 `Condition`, default none — gates the whole anthem, e.g. "as long as you have the city's blessing", Tendershoot Dryad's Saproling anthem; evaluated against the anthem source's own controller), `from_graveyard` (bool, default `false` — the anthem runs from the source card's **graveyard**, not the battlefield: Anger's "as long as this card is in your graveyard … creatures you control have haste"; also set `functions_in_graveyard = true` on the card, §1. A `false` anthem applies only while the source is a battlefield permanent, a `true` one only while it's a graveyard card, so a card with both a battlefield anthem and a graveyard-functional ability — Vanguard of the Restless — never leaks either across zones), `all_players` (bool, default `false` — drops the "source's controller controls the candidate" gate entirely, so the anthem reaches every creature on the battlefield instead of just its controller's; CR "**all** creatures" — Concordant Crossroads's "All creatures have haste." Every other axis above still applies) | no (static; buffs your creatures, or every creature's with `all_players`) |
| `grant_mana_ability` | `filter` (§7), `cost` (an activation-cost table: `taps_self`, `sacrifice`, …), `mana` (array of mana symbols), `restriction` (§8, default none — Galazeth Prismari) | no (static: matching permanents you control gain the activated mana ability — Goldspan Dragon's Treasures) |
| `trigger_doubling_static` | `source_subtypes` (array of strings, default `[]` — the triggering ability's SOURCE permanent must carry one of these subtypes; Harmonic Prodigy's `["Shaman", "Wizard"]`; `[]` doesn't gate on source subtype), `source_other` (bool, default `false` — exclude the doubler's own source permanent, CR "another"; Harmonic's "another Wizard"), `caused_by_instant_or_sorcery_cast` (bool, default `false` — the triggered ability must have been placed in the same event batch as an instant/sorcery cast or copy by the doubler's controller; Veyran's magecraft cause) | no (static: CR 603.3c — at trigger placement, each matching doubler makes the triggered ability trigger one additional time; two doublers → three instances. Approximates "causes … to trigger" as same-batch cause — exact for the pool) |
| `no_maximum_hand_size` | — | no (static — Reliquary Tower) |
| `prevent_noncombat_damage_to_other_creatures_you_control` | — | no (static, CR 615 — Tajic, Legion's Edge: prevents all *noncombat* damage (effect + fight damage, CR 701.12) dealt to the controller's **other** creatures; combat damage, damage to the source itself, and opponents' creatures are untouched. Scanned by `Game::noncombat_damage_prevented_to_creature` at each noncombat creature-damage choke) |
| `play_from_graveyard_once_per_turn` | — | no (static, CR 118.9 — Serra Paragon: once during each of the controller's turns, they may play a land or cast a permanent spell with mana value 3 or less from their own graveyard, capped by the per-turn `graveyard_play_used_this_turn` flag; a permanent played/cast this way gains "when it's put into a graveyard from the battlefield, exile it and its controller gains 2 life" — modeled as a rules-driven exile-and-lifegain redirect, not a stack trigger) |
| `reduce_spell_cost` | `amount` (amount — a bare number for a fixed reduction, or `{ per_permanent = <filter>, zone = … }` etc. to scale off a live count, e.g. Pearl-Ear's affinity for Auras: `{ per_permanent = { subtypes = ["Aura"], controller = "you" } }`), `filter` (spell filter, §7), `first_x_spell_each_turn` (bool, default `false` — gates the reducer to the controller's first spell this turn matching `filter`, read off the existing `has_x` `x_spells_cast_this_turn` tally, CR 601.2f; Zimone, Infinite Analyst's "The first spell you cast with {X} in its mana cost each turn costs {1} less…", paired with `filter = "has_x"` and `amount = "per_counter_on_source"`. ponytail: wired to the {X}-spell tally specifically — a differently-filtered once-per-turn reducer would need its own per-turn tally) | no (static) |
| `attack_tax` | `amount` (u8) | no (static pillow-fort: creatures can't attack you unless their controller pays `{N}` each — Ghostly Prison) |
| `counter_scaled_attack_tax` | — | no (static pillow-fort, per-attacker: an attacker aimed at this ability's controller that has one or more counters can't attack unless its controller pays `{X}` = the number of counters on that attacker; a counterless attacker owes nothing — Nils, Discipline Enforcer. Read in `Game::attack_tax_owed` alongside `attack_tax`. The "or planeswalkers you control" clause is unobservable while attack targets are always players) |
| `cant_be_attacked_by` | `filter` (§7 permanent filter) | no (static attack restriction, CR 509.1a: a permanent matching `filter` can't be declared attacking this ability's controller — Combat Calligrapher's "Inklings", Eriette's "enchanted by an Aura you control". Read in `Game::declare_attackers`. The printed "or planeswalkers you control" clause is unobservable while attack targets are always players) |
| `prevent_combat_damage_to_you_creating_tokens` | `token` (§8 token profile) | no (Inkshield, CR 615: "Prevent all combat damage that would be dealt to you this turn. For each 1 damage prevented this way, create a … token." Arms a this-turn per-player shield protecting the ability's controller; consulted at the combat-damage-to-a-player choke (`Game::damage_player`) — a shielded player takes no combat damage (no life loss, no commander damage, no lifelink, CR 702.15e) and instead mints one `token` per point prevented, routed through the token-creation replacements (Doubling Season). "this turn" is modeled as "until the next untap". Combat-damage-to-a-*player* only, no token — `prevent_all_combat_damage_this_turn` below is the table-wide, no-token scope generalization (#150); noncombat damage and per-source/N-point shields remain unlanded) |
| `prevent_all_combat_damage_this_turn` | — | no (Moment's Peace, CR 615, #150 — the table-wide scope generalization of `prevent_combat_damage_to_you_creating_tokens`: "Prevent all combat damage that would be dealt this turn." No target — every player's combat damage is prevented, not just this ability's controller's, and nothing is minted. Arms a this-turn `CombatExtras::prevent_all_combat_damage_this_turn` flag consulted at all three combat-damage chokes — `Game::deal_creature_damage` (fight/single-blocker damage), `Game::assign_attacker_damage` (a blocked attacker's own inlined damage-marking path), and `Game::damage_player` (which still emits `Event::CombatDamagePrevented` for observability, amount-only, no token). "this turn" is modeled as "until the next untap", same idiom as the per-player shield) |
| `place_vow_counters` | `filter` (§7 permanent filter) | no (Promise of Loyalty's rider, run as an `each_player_sacrifices` `then`: places a `"vow"` counter on each battlefield creature matching `filter` — every survivor a keep-one creature edict just left — marking the ability's controller as the player it "can't attack … for as long as it has a vow counter on it." The restriction is read live in `Game::declare_attackers`, so removing the counter lifts it. Same "or planeswalkers you control" unobservability as `cant_be_attacked_by`) |
| `destroy_target` | `target`, `count` (target-count, §7, default `{1, 1}` — Pest Infestation's "up to X target artifacts and/or enchantments" is `count = { min = 0, max = 0, x_scaled = true }`), `cant_be_regenerated` (bool, default `false` — CR 701.15d "It can't be regenerated": turns off any regeneration shield on the target so a shielded creature dies anyway — Rapid Hybridization) | yes (creature *or* noncreature — see §7) |
| `regenerate_shield` | `target` (§7) | yes (grants the target creature one regeneration shield, CR 701.15b: the next time it would be destroyed this turn, instead tap it, remove it from combat, and heal all damage; a `destroy_target` without `cant_be_regenerated` consumes one shield, and unused shields expire at cleanup. Bare primitive — no pool card grants a shield yet) |
| `destroy_all` | `filter` (§7 permanent filter) | no (mass destruction; no target) |
| `exile_all` | `filter` (§7 permanent filter) | no (mass exile — Oversimplify; indestructible does not save, CR 701.18a) |
| `damage_each_creature` | `amount` (amount), `opponents_only` (bool, default `false` — Volcanic Torrent's "each creature ... your opponents control") | no (damage to every creature) |
| `weaken_each_creature` | `power`, `toughness` (amounts), `opponents_only` (bool, default `false` — Doomwake Giant's "creatures your opponents control") | no (each creature gets -power/-toughness EOT) |
| `strip_keywords_from_opponents_creatures` | `keywords` (array) | no (CR 702.11e/702.18d "lose ... and can't have": every creature the ability's controller's opponents control loses `keywords` until end of turn and can't regain them this turn — arcane_lighthouse. Implemented as a negative until-EOT keyword record, `Permanent::temp_lost_keywords`, filtered out of the final unioned keyword set every recompute — so "can't have" falls out of the same mechanism as "lose," with no per-grant-site check needed) |
| `put_counters` | `count` (amount), `target`, `targets` (target-count, §7 — "on each of up to X target creatures", Silkguard), `kind` (`"charge"`/`"story"`/`"study"`/`"vow"`, default +1/+1), `divided` (bool, default `false` — `deal_damage`'s `divided` twin: `count` becomes one total split across the chosen `targets`, ≥1 each; Grove's Bounty's "distribute X +1/+1 counters among any number of target creatures you control" is `count = "x"`, `targets = { min = 0, x_scaled = true }`, `divided = true`. Only meaningful with `kind` unset — no pool card divides a named counter kind) | yes |
| `double_counters` | `target` | yes (double the target's +1/+1 counters — Primordial Hydra, `target = "this"`) |
| `double_counters_on_attached_creature` | — | no (the no-target sibling of `double_counters`, pinned to the permanent this ability's own source is attached to rather than a chosen target — Fractal Harness's "double the number of +1/+1 counters on it [equipped creature]"; a no-op if the source is currently unattached, pair with `timing = "enchanted_creature_attacks"`/`"equipped_creature_attacks"`) |
| `double_counters_on_target_creatures` | `target` + `count` | yes (double the +1/+1 counters on each of `count` chosen creatures — Kinetic Ooze's X≥10 "any number of other target creatures", `target = { permanent = { types = "creature", other = true } }`, `count = { min = 0, max = 6 }`. A triggered ability's **second** independent target clause: its targets are chosen on the stack, CR 603.3d — see "Independent target clauses" above. Read at resolution from `StackItem::Ability.targets_second`) |
| `put_counters_each` | `filter` (§7), `count` (amount), `target_player` (bool, default `false`) | no by default (a counter on each permanent matching `filter` from the *controller's* perspective — Mazirek); `target_player = true` reads `filter`'s `you`/`opponent` axis from a chosen Player target instead (Shadrix Silverquill's begin-combat "target player puts a +1/+1 counter on each creature they control") — intrinsic (a player) in that case |
| `counter_replacement` | `add` (i32, default 0), `times` (i32, default 1), `other` (bool, default `false` — "another creature you control", excludes its own source; Benevolent Hydra) | no (static; +1/+1 counter replacement) |
| `token_replacement` | `times` (i32, default 1) | no (static; token-creation replacement — "creates twice that many of those tokens", Doubling Season → `times = 2`; multiplies tokens created under the controller, Treasures and token copies included) |
| `life_gain_replacement` | `plus` (i32, default 0) | no (static; life-gain replacement — "you gain that much life plus 1 instead", Pest Rescuer → `plus = 1`; adds to life the controller would gain, additive across statics; gaining 0 is not "gaining life", so no addend) |
| `cast_x_replacement` | `times` (i32, default 1) | no (static; cast-context X modification — "double the value of X", Unbound Flourishing → `times = 2`; multiplies the stored `{X}` on the controller's *permanent* X-spells after payment, so enters-with-X counters / `Amount::X` read the doubled value; instant/sorcery/land spells untouched; multiplicative across statics) |
| `enters_with_counters` | `count` (amount), `kind` (`"charge"`/`"story"`/`"study"`/`"vow"`, default +1/+1 — Astral Cornucopia's "X charge counters") | no (static) |
| `creatures_you_control_enter_with_counters` | `filter` (§7 permanent filter, applied to the *entering* permanent — Gorma, the Gullet's "nontoken creatures you control" is `{ types = "creature", token = "nontoken" }`), `count` (amount, resolved with the static's own permanent as source — Gorma's "for each creature that died under your control this turn" is `"creatures_died_this_turn"`) | no (static; the *other*-permanent-wide sibling of `enters_with_counters` — CR 614.1c: watches every permanent the static's own controller casts/puts onto the battlefield, applying at the ETB counter choke in `Game::resolve_spell` via `Game::additional_enter_counters`; a static never modifies its own permanent's entry (an ETB replacement isn't functioning until the permanent is on the battlefield — Master Biomancer / Corpsejack Menace ruling). Currently wired only at the cast-resolution choke, not reanimation/blink) |
| `proliferate` | `times` (amount, usually `1` or `"x"` — Expansion Algorithm's "Proliferate X times") | no (untargeted; pauses on a select-any-number choice over every battlefield permanent with a counter, giving each one more of every kind already there — CR 701.27; no player in this pool ever carries a counter, so the offered set is permanents-only) |
| `move_counters` | `target` (the moved-*from* permanent), `to_filter` (§7 permanent filter — the resolution-time destination, "onto another…"), `all_kinds` (bool, default `false` — `false` moves only +1/+1, `true` also moves every named kind present), `distributed` (bool, default `false` — the moved-*from*'s live +1/+1 count divided across any number of `to_filter`-matching destinations instead of one, ≥1 each, up to the source's count and not required to move them all; Forgotten Ancient's "move any number of +1/+1 counters from this creature onto other creatures". Only meaningful with `all_kinds = false`) | yes (`target` is the moved-from permanent, chosen at cast/placement; the destination(s) are chosen at *resolution* — one, mirroring `fight`'s cast/resolution split, unless `distributed`, which pauses on a target→amount map like `put_counters`' `divided`) |
| `remove_all_counters_then_draw` | `target` | yes (every counter, every kind, off `target`; the controller draws one card per counter removed — Nexus Mentality) |
| `remove_counter_from_self` | — | no (removes one +1/+1 counter from the ability's own source, a CR 608.2c effect-internal sub-action, not an activation cost — Ingenious Prodigy's "you may remove a +1/+1 counter from it"; a no-op with zero on it) |
| `grant_flash_this_turn` | — | no (CR 601.3a permission, unfiltered — Alchemist's Refuge's "you may cast spells this turn as though they had flash." Sets a per-player turn flag, `Player::flash_permission_this_turn`, consulted by the cast-timing gate `CardDef::is_instant_speed`; cleared at the next Untap step alongside the other per-turn player flags. Modeled as a resolved one-shot rather than a continuous static — see the effect's doc comment) |
| `grant_channel_colorless_mana_this_turn` | — | no (CR 605 mana permission, unfiltered — Yavimaya Bloomsage's Channel back face's "Until end of turn, any time you could activate a mana ability, you may pay 1 life. If you do, add {C}." Sets a per-player turn flag, `Player::channel_colorless_mana_this_turn`; the granted pay-1-life-for-{C} action is driven by the standalone `Intent::ChannelColorlessMana` — a player-scoped mana ability, unlike `grant_mana_ability`'s permanent-keyed grant, since Channel has no surviving source once its copy resolves. Cleared at the next Untap step alongside the other per-turn player flags) |
| `create_token` | `count` (amount, default 1), `token` (§8), `controller` (`"you"` default, `"target_controller"`, `"each_opponent"`, `"one_per_opponent"`, `"target_player"` — the ability's own chosen Player target, Shadrix Silverquill's "Target player creates a ... Inkling ...", `"target_opponent"` — same, restricted to an opponent, Questing Phelddagrif's "Target opponent creates a 1/1 ... Hippo ... token"), `enters_with` (amount, default 0 — "put X +1/+1 counters on it," Deekah's Magecraft Fractal: places that many +1/+1 counters on each minted token through the same doubler/Hardened-Scales replacement pipeline as any other counter placement), `set_base_pt` (amount, optional — "create an X/X … token …, where X is …", Manaform Hellkite/Rootha, Mastering the Moment: overrides the minted token's *base* power **and** toughness to this resolved amount before it enters, a genuine X/X body baked into the minted def, not `enters_with`'s counters — differs from counters under -1/-1 effects and counter removal; omit for a token that keeps its profile's printed P/T, every existing `create_token` unchanged), `exile_at_next_end_step` (bool, default `false` — "Exile that token at the beginning of the next end step," Manaform Hellkite, CR 603.7b: schedules a delayed exile against that specific minted token, mirroring `create_token_copy`'s `sacrifice_at_next_end_step`), `enters_tapped_and_attacking` (bool, default `false` — Combat Calligrapher: "that attacking player creates a tapped … token … that's attacking that opponent." Valid only on an ability whose trigger populates the attack context (`timing = "player_attacks_your_opponent"`): the token mints under the *attacking* player (overrides `controller`) and enters tapped, attacking the same defending player, via a dedicated `Event::TokenEnteredAttacking` — CR 508.4, a token put onto the battlefield attacking was never declared as an attacker, so it does not re-fire attack-triggered abilities), `must_attack_defender` (bool, default `false` — "tokens … that attack that opponent this turn if able," Furygale Flocking: each minted token gets a this-turn requirement to attack an opponent of the controller, enforced in `Game::declare_attackers` the same way goad is (CR 508.1a "if able"), cleared at the next turn boundary. Under `controller = "one_per_opponent"`, each opponent's own batch is bound to *that* opponent — Furygale Flocking's "for each opponent, create two … tokens … that attack that opponent"; every other `controller` value binds to the single flattened opponent — the one legal defending player in a 1v1 game) | no by default (`"target_controller"` reads the *ability's* shared target — see below); `"target_player"` targets a player directly (its own target, not shared with a preceding step) |
| `create_treasure` | `count` (amount, default 1), `target_player` (bool, default `false` — "target player creates…", Prismari Command), `tapped` (bool, default `false` — "create a number of tapped Treasure tokens…", Goldvein Hydra: each minted Treasure enters already tapped) | no (Treasure is an engine-provided artifact token) |
| `create_token_copy` | `count` (amount — copies minted *per chosen target*, e.g. Rite of Replication's `{ if_kicked = 5, else = 1 }`), `target`, `targets` (target count, §9 below, default `{1, 1}` — how many *distinct* targets are chosen, named apart from `count` the same way `put_counters`' `targets` sits apart from its own `count`; Twinflame's "any number of target creatures you control" is `{ strive_scaled = true }` — one copy is minted per chosen target, no special-cased resolution loop needed, the ordinary multi-target step expansion already runs the effect once per target), `sacrifice_at_next_end_step` (bool, default `false` — Determined Iteration: each minted copy is sacrificed at the beginning of the next end step via a delayed trigger, CR 603.7b), `exile_at_next_end_step` (bool, default `false` — Twinflame: "Exile those tokens at the beginning of the next end step," mirroring `create_token`'s own field of the same name; distinct from `sacrifice_at_next_end_step` because exile skips dies-triggers, mutually exclusive in every pool card), `haste` (bool, default `false` — Determined Iteration: "the token created this way gains haste," an until-EOT `Keyword::Haste` grant on each minted token) | yes |
| `copy_each_entered_this_turn_token_tapped_attacking` | — | no (Redoubled Stormsinger: "for each creature token you control that entered this turn, create a tapped and attacking token that's a copy of that token. At the beginning of the next end step, sacrifice those tokens." Valid only on an ability whose trigger populates the attack context (`timing = "attacks"`, i.e. `Trigger::Attacks`): enumerates the attacking player's own battlefield internally — `token && creature && controller_you && entered_this_turn` — rather than taking a target; each match mints a copy under the attacker via `create_token`'s `enters_tapped_and_attacking` path (attacking the same defender) and schedules its own `sacrifice_at_next_end_step` delayed trigger, both reused verbatim) |
| `grant_to_attached` | `power` (amount, default 0), `toughness` (amount, default 0 — Sage's Reverie's "+1/+1 for each Aura you control that's attached to a creature" is `{ per_permanent = { subtypes = ["Aura"], controller = "you" } }`, resolved live off the attached Aura/Equipment), `keywords` (array, default `[]`), `goad` (bool, default `false` — host is goaded while attached; the Impetus cycle), `protection_from_chosen_color` (bool, default `false` — Flickering Ward's "Enchanted creature has protection from the chosen color": grants the host `{ protection = <this Aura's chosen_color> }` read live off the Aura's own `Permanent::chosen_color`, set by a `choose_color` ETB; grants nothing until a color is chosen — a card-specific dynamic-scope axis, not a general grant-a-runtime-keyword surface), `granted_ability` (default none — a `{ cost = <activation-cost table>, effects = [...] }` sub-table granting the host an activated ability; Fallen Ideal's "Sacrifice a creature: +2/+1 until end of turn", the non-mana twin of `grant_mana_ability`), `cant_attack` (bool, default `false` — Faith's Fetters/Prison Term: host can't be declared as an attacker while attached; the reverse of `goad`'s "must attack", read live in `declare_attackers`), `cant_block` (bool, default `false` — the block-legality twin of `cant_attack`, read live in `can_block`), `activated_abilities` (string enum, default unset — `"none"` (Prison Term: no activated ability of the host's may be activated, mana or not) or `"mana_only"` (Faith's Fetters: every activated ability except a mana ability, CR 605.3a, is banned); read live in `ability_activation_gate`) | no (aura/equipment static) |
| `set_attached_base_p_t` | `power` (i32), `toughness` (i32) | no (aura static; sets the host's *base* P/T while attached — see §9) |
| `set_attached_types` | `add_types` (type set, default none — card types unioned onto the host, Darksteel Mutation → artifact), `add_subtypes` (array, default `[]` — creature subtypes unioned on, Angelic Destiny → Angel "in addition to its other types"), `set_subtypes` (array, default `[]` — when non-empty, *replaces* the host's creature subtypes, Darksteel Mutation → `[Insect]`), `lose_all_abilities` (bool, default false — CR 613.1e/701; strips the host's *own* printed abilities + keywords, Darksteel Mutation → "loses all other abilities"; the Aura's own grants survive) | no (aura static; CR 613.4/613.1e type/subtype/ability layer while attached — see §9) |
| `control_attached` | — | no (aura static; its controller controls the enchanted permanent, CR 720 — see §9) |
| `gain_control_until_end_of_turn` | `target` | yes (one-shot control change, reverting at cleanup — Besmirch) |
| `gain_control` | `target` | yes (permanent control change, no cleanup reversion — Entrancing Melody's "gain control of target creature with mana value X," typically paired with a `mv_eq_x` target filter) |
| `gain_control_while` | `target`, `while_source_tapped` (bool, default `false`) | yes (condition-scoped control change, CR 611.2b — Rubinia Soulsinger's "for as long as you control Rubinia and Rubinia remains tapped"): the steal reverts automatically the instant the source leaves the battlefield, changes controller, or (when `while_source_tapped`) untaps. Pairs with `may_choose_not_to_untap` (§1) — the `{T}` activation cost taps the source, starting the "remains tapped" clause true) |
| `grant_source_abilities_until_end_of_turn` | — | no (Backup's rider, CR 702.166 — Guardian Scalelord's "if that's another creature, it gains the following abilities until end of turn": grants the source's *other* abilities + keywords, read live off the source's `CardDef`, to the enclosing sequence's shared target until cleanup; no-op when that target is the source itself. Author as the trailing `[[abilities.effects]]` of a Backup ETB whose first effect is the `put_counters` on `target = "creature"` — the two effects share that one target) |
| `equip` | — | no (pairs with an activated cost) |
| `exile_target` | `target`, `count` (target-count, §7, default `{1, 1}` — Curse of the Swine's "exile X target creatures" is `count = { min = 1, max = 1, x_scaled = true }`) | yes |
| `exile_until_source_leaves` | `target` | yes (the O-Ring pattern, CR 603.6e linked exile: returns when this source leaves; a token ceases to exist instead) |
| `exile_target_minting_illusion_on_leave` | `target` | yes (Skyclave Apparition's linked exile: unlike `exile_until_source_leaves`, the exiled card is *never* returned — when this source leaves the battlefield, its owner instead gets an X/X blue Illusion token, X = the exiled card's mana value; a token ceases to exist instead of being exiled) |
| `flicker_target` | `target`, `return_at` (a `Step` name, optional — omit for an immediate return, `"end"` for Mistmeadow Witch's "at the beginning of the next end step"; only `"upkeep"`/`"end"` are wired, mirroring `schedule_at_next_upkeep`'s `fire_at`) | yes (CR 400.7 — a new object: exile the target creature, then return it to the battlefield under its **owner's** control, either in this same resolution — Momentary Blink — or as a real CR 603.7 delayed triggered ability at `return_at`'s step — Mistmeadow Witch. Fresh ETBs fire, Auras fall off, counters/damage are wiped, summoning sickness resets; a token ceases to exist instead (CR 111.7), and a commander diverted to the command zone instead of exile (CR 903.9b) was never exiled, so nothing returns for it) |
| `exile_top_may_play` | `count` (amount), `until_next_turn` (bool, default `false` — Atsushi, the Blazing Sky: the play permission lasts until the end of your *next* turn instead of expiring at this turn's cleanup) | no (impulse draw: exile the top `count` library cards face up, controller may play them until end of turn) |
| `exile_top_cast_matching_free` | `count` (u32), `filter` (card filter, §7) | no (Herald of Amity's ETB: exile the top `count` library cards face up, pause on a choose-up-to-one over the ones matching `filter` to grant the free-cast permission (CR 118.5) — the chosen card stays in exile; every other exiled card, including any not offered, goes to the bottom of the library. No candidates matching `filter` skips the pause and bottoms the whole batch) |
| `exile_from_graveyard_may_play` | — | no (`you_discard` payoff: exile the just-discarded card, may play it this turn — Containment Construct) |
| `exile_random_from_graveyard_may_play` | — | no (exile a card from the controller's own graveyard at random, may play it this turn — Advanced Reconstruction's base ability; the pick uses the engine's injected RNG, same seeded idiom as shuffling; a no-op on an empty graveyard) |
| `exile_discarded_with_this` | — | no (`you_discard` payoff: exile the just-discarded card into this source's linked pile — Currency Converter) |
| `cash_out_exiled_with_this` | — | no (put a card from this source's linked exile pile into its owner's graveyard, then Treasure if land / 2/2 token otherwise — Currency Converter's `{T}`) |
| `exile_target_from_graveyard_with_this` | — | yes (a fixed `noncreature_nonland` filter over your own graveyard, hardcoded — not TOML-authored; exile the chosen card into this source's own linked pile, no impulse-play permission — Quintorius, Loremaster's end step) |
| `exile_target_graveyard_spell_cast_free` | `filter` (card filter, §7), `count` (target-count, §7, default `{1, 1}` — Renegade Bull's "up to one target" is `count = { min = 0, max = 1 }`) | yes ("up to one" target over your own graveyard, matching `filter`; exile the chosen card and grant the free-cast permission (CR 118.5, "without paying its mana cost") for it — `Event::CastFromExileFreePermissionGranted`, the same plumbing `cast_exiled_with_this_free` grants — so the controller may cast it as a genuine cast (firing real "whenever you cast" watchers) at their next opportunity. No target (empty graveyard, or declined) is a no-op — Renegade Bull's attack trigger, `filter = "instant_or_sorcery"`) |
| `exile_target_from_graveyard_create_token_copy` | `filter` (card filter, §7) | yes (mandatory single target over your own graveyard, matching `filter`; exile the chosen card, then mint a token copy of its copiable characteristics (CR 707.2) onto the battlefield under the controller — Restore Relic, `filter = "artifact_or_creature"`) |
| `exile_target_graveyard_card_then_if_creature` | `then` (`[effect]`, default empty) | yes (a fixed unrestricted `any_card` filter over *any* graveyard, hardcoded — not TOML-authored, unlike its noncreature-nonland/authored-filter siblings above; exile the chosen card, then run `then` only if the just-exiled card's own printed type is a creature card — CR "if a creature card is exiled this way" reads what was just exiled, not LKI. `then` is `[[abilities.effects.then]]`, the same array-of-effects shape `each_player_sacrifices`'s `then` uses — Feral Appetite, `then = [{ create_token ... }]`) |
| `exile_target_graveyard_card_record_mana_value` | `filter` (card filter, §7) | yes (mandatory single target over your own graveyard, matching `filter`; exile the chosen card and snapshot its id + mana value onto the engine's resolution-scoped slot — no free-cast permission granted here, unlike `exile_target_graveyard_spell_cast_free` above. Feeds `"exiled_card_mana_value_this_way"` (§ Amount) and `schedule_this_turn_combat_damage_copy` below, both sharing this same `effects` sequence — Surge to Victory, `filter = "instant_or_sorcery"`) |
| `schedule_this_turn_combat_damage_copy` | — | no-target-of-its-own (reads `exile_target_graveyard_card_record_mana_value`'s snapshot from earlier in the same `effects` sequence) — arms a CR 603.7 delayed watch: every creature the controller controls that deals combat damage to a player, any time later **this turn**, mints a free copy (CR 118.5) of the exiled card. Controller-scoped (not one watched creature) and repeatable — never removed on fire, unlike `arm_combat_damage_watch` below — cleared unconsumed at the next turn's Untap. Surge to Victory: "Whenever a creature you control deals combat damage to a player this turn, copy the exiled card. You may cast the copy without paying its mana cost." |
| `mint_free_copy_of_exiled_card` | — | engine-internal only, not author-facing — a `schedule_this_turn_combat_damage_copy`-armed payoff: mints one free copy (CR 118.5) of the card the watch names onto the stack via the Storm/Twincast copy machinery. Its `card` field is filled in by the delayed watch when it fires (`Option<ObjectId>`, always `None`/no-op if authored directly) |
| `cast_exiled_with_this_free` | — | no (pauses on a card-pick choice over this source's linked exile pile — up to one, or decline; grants a free-cast permission (CR 118.5, "without paying its mana cost") for the chosen card instead of cashing it out — Quintorius, Loremaster's activated ability) |
| `exile_graveyard` | — | intrinsic (target player; exile their whole graveyard — Bojuka Bog) |
| `exile_all_graveyards` | — | no (mass twin of `exile_graveyard`; exiles *every* player's graveyard, no target — Final Act's "Exile all graveyards" mode) |
| `return_to_hand` | `target`, `count` (target-count, §7, default 1) | yes (bounce a permanent; `count = 6` bounces six target permanents — Aether Gale) |
| `return_this_to_hand` | — | no (return the ability's own source to hand from wherever it is — Angelic Destiny) |
| `phase_out` | — | no (pauses on a select-any-number choice over the *other* creatures you control — each chosen creature and everything attached to it phases out, CR 702.26; phases back in at your next untap. Guardian of Faith's ETB. Fixed filter "other creatures you control"; targets are chosen at resolution, not on the stack) |
| `return_all_to_hand` | `filter` (§7 permanent filter) | no (mass bounce; a token ceases to exist instead) |
| `mill` | `count` (amount), `target` | yes |
| `exile_self_with_time_counters` | `counters` (u32) | no — "Exile [this card] with N time counters on it" (CR 702.62, Rousing Refrain): the resolving instant/sorcery exiles *itself* with `counters` time counters (visible in exile) instead of going to the graveyard. Ticked at the owner's upkeep like a `[suspend]` card. |
| `drain_target` | `amount` (i32), `opponent` (bool, default `false` — "target opponent") | intrinsic (target player loses / you gain) |
| `target_player_gains_life` | `amount` (i32), `opponent` (bool, default `false` — restrict to "target opponent", Questing Phelddagrif's "Target opponent gains 2 life") | intrinsic (target player, no matching loss — the gain-only twin of `drain_target`) |
| `each_opponent_drain` | `amount` (amount), `sum_gain` (bool, default `false` — gain the *total* lost across every opponent, Exsanguinate's "life lost this way", instead of the flat per-opponent `amount` Zulaport Cutthroat prints) | no |
| `each_opponent_loses_life` | `amount` (amount) | no (no controller lifegain half — avoids re-triggering "whenever you gain life") |
| `return_from_graveyard_to_hand` | `target` | yes (`your_graveyard`) |
| `reanimate_to_battlefield` | `target`, `finality` (bool, default `false` — enters with a finality counter, CR 614.12: dies ⇒ exiled instead; Excava), `becomes` (inline table, default none — an **indefinite** (as-long-as-on-battlefield, CR 611.2c) type-set + subtype-add + base-P/T-set + keyword-add applied to the *reanimated* permanent as it enters: `add_types` (type set, default none — Excava adds `"creature"`, animating a reanimated noncreature), `add_subtypes` (array, default `[]` — Excava → `["Spirit"]`), `base_power`, `base_toughness` (plain `i32` — SET, a 7b layer before counters/pumps), `keywords` (array, default `[]` — Excava → `["flying"]`); Excava's "It's a 1/1 Spirit creature with flying in addition to its other types" — the indefinite twin of `animate_self_until_end_of_turn`, keyed on the reanimated object rather than the source) | yes (`any_graveyard`, a filtered `card_in_graveyard`, or `this_auras_graveyard_target` — §7) |
| `return_this_from_graveyard_to_battlefield` | `tapped` (bool, default `false`) | no (self-return; needs `functions_in_graveyard = true` on the card, §1) |
| `tuck_from_graveyard` | `target`, `to_top` (bool, default `false` — bottom of the library; `true` = top, Mystic Sanctuary) | yes (a graveyard card — Mistveil Plains) |
| `mass_return_from_graveyard` | `filter` (card filter, §7) | no (return every matching card in *your* graveyard to the battlefield — Replenish / Eiganjo Dynastorian's back face) |
| `shuffle_target_cards_from_graveyard_into_library` | `max` (u32, default `0` = unbounded), `target_player` (bool, default `false`) | `target_player = false`: no (pauses on a resolution-time choice over the *controller's* whole graveyard — the controller picks up to `max` (or any subset if `max = 0`), including none or all, to shuffle into the library; Perpetual Timepiece). `target_player = true`: yes (a player — the caster still picks the cards at resolution, but the graveyard/library affected is the *targeted player's*, up to `max`; Quandrix Command) |
| `shuffle_target_permanent_into_library_then_reveal` | `target` | yes (`{ permanent = {} }` — any permanent, no restriction; Chaos Warp) — deterministic, no pause: the target's *owner* shuffles it into their library, then reveals the new top card; a permanent card goes onto the battlefield under the owner (not necessarily this effect's controller), anything else stays on top. A token target ceases to exist instead of entering a library (CR 111.7) — no shuffle, no reveal. |
| `tuck_permanent_into_library` | `target`, `to_top` (bool, default `false` — bottom of the owner's library; `true` = top, Temporal Spring) | yes (a battlefield permanent — `{ permanent = {} }` for any permanent (Temporal Spring), or a filtered `{ permanent = { types = "creature", attacking = true } }` for "target attacking creature" (Condemn)) — the standalone tuck-only half of `shuffle_target_permanent_into_library_then_reveal` above: no shuffle, no reveal, just a fixed-position move to the target's owner's library. A token target ceases to exist instead (CR 111.7). |
| `scry` | `count` (amount, §5 — a bare int for a fixed scry, or a derived keyword like `"commander_casts_from_command_zone"` for Study Hall's "scry X") | no |
| `surveil` | `count` (u32) | no |
| `look_at_top` | `count` (u32), `filter` (card filter, default any card), `up_to` (u32, default 1), `min` (u32, default 0), `dest` (`"hand"`/`"battlefield"`), `dest_tapped` (bool, default `false` — gates a `"battlefield"` `dest`, ignored for `"hand"`), `rest` (`"bottom"`, the default, or `"hand"`), `mv_budget` (u32, optional — omit for uncapped) | no (look at top N, select up to `up_to` matches into `dest`, rest to `rest` — Quandrix Apprentice; `min` floors the selection below "may" for a mandatory pick — Dig Through Time's "put two of them into your hand", bounded by however many were actually looked at on a short library; `dest = "battlefield"` puts each selected card onto the battlefield under the controller instead — Armored Skyhunter's "put an Aura or Equipment card from among them onto the battlefield" — and auto-pauses a deployed Aura/Equipment on the same choose-host `PendingChoice` a cast Aura/Equip uses: an Aura's host is mandatory (CR 303.4f), Equipment's is optional to a creature you control (CR 301.5c); `mv_budget = N` caps the **summed** mana value of the selected cards at `N`, rejecting an over-budget answer — Ao, the Dawn Sky's "put any number of nonland permanent cards with total mana value 4 or less onto the battlefield" is `up_to = count, min = 0, mv_budget = 4`; omit for no cap; `rest = "bottom"` bottoms the non-selected looked-at cards in a real PRNG-shuffled random order — CR "in a random order", same `bottom_pile_in_library` idiom `reveal_until` uses; `rest = "hand"` shares the `RestDest` enum `reveal_until`/`reveal_top_cards` use — no pool `look_at_top` card needs it yet) |
| `distribute_top` | `count` (u32), `to_hand` (u32), `to_bottom` (u32), `to_exile_may_play` (u32) | no (look at top N, then route exactly `to_hand`/`to_bottom`/`to_exile_may_play` of them one-per-slot into hand / library bottom / exile-with-permission-to-play-this-turn, mandatory per slot — Expressive Iteration's "put one into your hand, one on the bottom, and exile one"; fixed named slots, not a generic destination list — grow toward one only when a second card needs a different mix) |
| `reveal_top_to_hand` | `filter` (card filter, §7) | no, subject read from attack context (reveal the *defending* player's top card publicly; if it matches `filter`, that player puts it into their hand, else it stays on top — Goblin Guide's attack trigger) |
| `reveal_top_and_drain_mutual` | — | intrinsic (`opponent`; the ability's controller and the chosen opponent each reveal their top card publicly, each loses life equal to the mana value of the *other's* revealed card, then each puts their own revealed card into their hand — Keen Duelist) |
| `reveal_until` | `filter` (card filter, §7), `count` (amount, §6 — the stop count), `matched_dest` (§7, `"battlefield"`/`"hand"`), `matched_tapped` (bool, default `false`), `rest_dest` (`"bottom"`, the default, or `"hand"`) | no (reveal the controller's own top cards one at a time until `count` match `filter` or the library runs out — CR 120-style "as many as possible" on a short library; each match goes to `matched_dest` (`matched_tapped` gates a `"battlefield"` destination), every other revealed card to `rest_dest` — Open the Way's "reveal until X lands, put them onto the battlefield tapped, rest on the bottom"; non-matching cards go to the bottom in library order — a deterministic stand-in for "random order" (no `rand`); unlike `look_at_top`'s `rest`, this one isn't yet routed through the PRNG shuffle; `rest_dest = "hand"` puts each non-matching revealed card into hand instead — Coiling Oracle's "Otherwise, put that card into your hand") |
| `reveal_top_cards` | `count` (amount, §6 — how many to reveal), `filter` (card filter, §7), `matched_dest` (§7, `"battlefield"`/`"hand"`), `matched_tapped` (bool, default `false`), `rest_dest` (`"bottom"`, the default, or `"hand"`), `deploy_untapped_if` (condition, §5's `[abilities.condition]` table, optional) | no (`reveal_until`'s sibling: reveal exactly the top `count` cards — not "until N match" — stopping early on a short library, CR 120.3 "as many as possible"; every match goes to `matched_dest`, every other revealed card to `rest_dest`; non-matching cards bottom in library order, the same deterministic "random order" stand-in `reveal_until` uses; `rest_dest = "hand"` puts each non-matching revealed card into hand instead — Coiling Oracle's `count = 1` "reveal the top card... if it's a land card, put it onto the battlefield. Otherwise, put that card into your hand"; `deploy_untapped_if`, when it holds at resolution, deploys matches untapped instead of per `matched_tapped` — Animist's Awakening's spell mastery "then untap those lands" bakes to "enters untapped" since nothing can respond to the intermediate tapped state) |
| `reveal_until_may_deploy` | `filter` (card filter, §7) | no (`reveal_until`'s routed-pause sibling: reveal the controller's own top cards one at a time, bottoming each non-match — same per-card loop as `reveal_until`, in library order — until the first match or the library runs out; a hit pauses on a battlefield-or-hand choice over exactly that card, left unmoved on top until answered — accepting puts it onto the battlefield untapped, declining puts it into hand; a whiff never pauses — Songbirds' Blessing's "reveal cards from the top of your library until you reveal an Aura card. You may put that card onto the battlefield. If you don't, put it into your hand." Kept separate from `reveal_until` rather than adding a pause axis to it — `reveal_until` stays deterministic/no-pause) |
| `reveal_until_exile_cast_free` | `filter` (card filter, §7) | no (`reveal_until_may_deploy`'s sibling: same reveal-until-first-match loop, but the hit is exiled face-up and pauses on the shared `exile_top_cast_matching_free`/`cascade` choice — accepting grants the free-cast permission (CR 118.5), declining bottoms it; a whiff never pauses — Creative Technique's "reveal cards from the top of it until you reveal a nonland card. Exile that card... You may cast the exiled card without paying its mana cost", usually preceded by a `shuffle_library` step in the same ability) |
| `shuffle_library` | — | no (shuffle the controller's own library, no target — Creative Technique's "Shuffle your library, then reveal…" lead-in, a preceding `[[abilities.effects]]` step ahead of `reveal_until_exile_cast_free` in the same sequence) |
| `exile_top_until_stop_cast_free_under_budget` | `budget` (u32) | no (Dance with Calamity's push-your-luck loop: "As many times as you choose, you may exile the top card of your library. If the total mana value of the cards exiled this way is `budget` or less, you may cast any number of spells from among those cards without paying their mana costs." Pauses before each exile on a yes/no `AnswerMay` choice, running a live tally of the exiled cards' summed mana value; when the caster stops or the library empties, a tally `<= budget` lets them cast any number of the exiled (nonland) cards free (CR 118.5), a bust (`> budget`) grants nothing and every exiled card stays exiled either way. Usually preceded by a `shuffle_library` step in the same sequence) |
| `opponent_splits_exile_piles` | — | no (Abstract Performance: exile the top four then the next four of the controller's library into two face-up piles, then hand off to the shared "an opponent" chooser — with two or more opponents alive, the controller picks which one on a `ChooseSplittingOpponent` pause (a settled ruling, not a hardcoded APNAP-next default); with one, it resumes immediately — that opponent picks one pile, which goes to the controller's graveyard while the controller may cast up to one card from the other pile free (CR 118.5) with the rest going to hand. The first pile's "face-down" hidden-ness isn't modeled — both exile face-up — and the free cast is offered at a later priority window, not mid-resolution) |
| `reveal_top_split_piles` | — | no (Fact or Fiction: "Reveal the top five cards of your library. An opponent separates those cards into two piles. Put one pile into your hand and the other into your graveyard." Reveals the top five (all public; a short library reveals only what's there), then hands off to the same shared "an opponent" chooser `opponent_splits_exile_piles` uses. The chosen opponent partitions the revealed cards into two piles on a `PartitionRevealed` pause — a free subset split, either pile may be empty — then the controller picks which pile to keep in hand on a `ChoosePileForHand` pause; the other pile is milled into the graveyard (the revealed cards never left the library)) |
| `each_player_exiles_until_nonland_opponent_picks` | — | no (Plargg and Nassari's upkeep trigger: each player, APNAP order, exiles from the top of their own library until a nonland (all face-up), pause on an **opponent-addressed** choice — the next opponent in turn order picks one exiled nonland — then the controller may cast up to two of the *other* exiled cards free (CR 118.5); the picked and any uncast cards stay exiled. Same next-opponent-hardcoded and later-priority-window approximations `opponent_splits_exile_piles` used to carry — not yet migrated to the shared chooser, since no pool card needs "an opponent" here to be a genuine controller choice) |
| `goad_target` | `target` | yes |
| `tap_target` | `target`, `count` (target-count, §7 — Magma Opus's "tap two target permanents") | yes |
| `untap_target` | `target` | yes |
| `untap_all` | `filter` (§7 permanent filter) | no (untaps every matching permanent the controller controls) |
| `untap_searched_land` | — | no (untaps the permanent this same ability's own `search_library` just found — Fabled Passage; pair via `conditional`) |
| `each_player_draws` | `count` (u32) | no (every player draws, not just the controller) |
| `target_player_loses_life` | `amount` (i32) | intrinsic (target player, no matching gain) |
| `sacrifice_own` | `filter` (§7 permanent filter), `count` (u32) | no (controller sacrifices `count` of their own matching permanents — a mandatory `PendingChoice::ChooseOwnSacrifices` the controller directs when more than `count` match, CR 701.16a; with `count` or fewer, all of them go immediately, no pause — CR 700.2) |
| `defending_player_sacrifices` | `count` (u8) | no, subject read from attack context (annihilator N, CR 702.86a: the *defending* player — not the ability's controller — sacrifices `count` of their own permanents of their choice, any type; same `ChooseOwnSacrifices` machinery as `sacrifice_own` with an unrestricted filter and the defender standing in for the controller; pair with `timing = "enchanted_creature_attacks"` — Eldrazi Conscription) |
| `sacrifice_self_unless_pay` | `cost` (`[cost]` table, same shape as `[echo]`) | no (Rupture Spire: "sacrifice it unless you pay {1}" — a real `timing = "etb"` triggered ability, CR 603.3b, NOT the Echo keyword, though it shares Echo's pay-or-sacrifice resolution shape: pauses on a `PendingChoice::SacrificeUnlessPay`, answered by `Intent::PayOptionalCost`; paying settles `cost` from the controller's mana pool, declining sacrifices the source) |
| `sacrifice_self_unless_return_land` | `filter` (§7 permanent filter) | no (Treva's Ruins: "sacrifice it unless you return a non-Lair land you control to its owner's hand" — the land-bounce twin of `sacrifice_self_unless_pay`; `filter` names the qualifying lands, `{ types = "land", controller = "you", nonlair = true }` for Treva's Ruins — pauses on a `PendingChoice::SacrificeUnlessReturnLand` offering the controller's matches, answered by `Intent::ReturnLandOrSacrifice`; no matching land skips the pause and sacrifices the source outright) |
| `sacrifice_source` | none | no (sacrifices the ability's own source unconditionally, no pause — Court Hussar's "sacrifice it unless {W} was spent to cast it": the `then` of a `negate`d `conditional` step, §6 below. Distinct from the engine-internal `sacrifice_object`, which is never authored directly — always pair `sacrifice_source` with a `conditional`/`intervening_if` gate rather than reaching for it unconditionally on its own ability) |
| `mill_self` | `count` (amount) | no (untargeted — always the controller's own library; contrast `mill`, which targets a player) |
| `copy_target_spell` | — | intrinsic (an instant/sorcery spell on the stack; Twincast) |
| `copy_this_spell` | `count` (amount, default one), `cast_from_graveyard_only` (bool, default false), `optional` (bool, default false) | no (storm/Gravestorm-style rider: mints `count` copies of the resolving spell itself, each offered the same CR 707.10c retarget `copy_target_spell` offers — Plumb the Forbidden's `"spell_sacrifice_count"`, Ominous Harvest's `"permanents_died_this_turn"`; `cast_from_graveyard_only` gates the mint on the resolving spell having been cast via flashback — Sevinne's Reclamation's "if this spell was cast from a graveyard"; `optional` pauses on a `PendingChoice::MayYesNo` before minting, declining runs nothing — Sevinne's Reclamation's "you may copy this spell") |
| `retarget_spell_copy` | `copy` (object id) | — internal only, minted by `copy_this_spell`'s own resolution; never authored in a card TOML |
| `counter_target_spell` | `unless_pays` (amount, optional — "unless its controller pays {N}", Quandrix Charm), `filter` (spell filter §7, default `"all"` — Decisive Denial's `"noncreature"`), `countered_dest` (optional, only `"library_top_or_bottom"` today — Hinder's "if that spell is countered this way, put that card on your choice of the top or bottom of its owner's library instead of into that player's graveyard," CR 701.5b: pauses this ability's controller on a `PendingChoice::ChooseCounteredSpellDestination`/`Intent::ChooseTopOrBottom { top }` before the countered card moves; a flashback/escape spell exiles instead (CR 702.34e/702.19d), leaving nothing for the rider to redirect — never combined with `unless_pays` in the pool) | intrinsic (a spell on the stack; Counterspell) |
| `counter_target_activated_ability` | — (no fields) | intrinsic (`"activated_ability_on_stack"` — Azorius Guildmage's "Counter target activated ability"). Removes an *activated* ability from the stack (CR 701.5c/112.7a); unlike `counter_target_spell` there is no card to move — it just ceases to exist. Mana abilities never reach the stack (CR 605.3b) so they're unreachable; triggered abilities are excluded by the target axis. Cycling and `hand_ability` activations are real stack abilities (see §5), so this can counter them. |
| `may_draw_unless_pays` | `cost` (amount — "unless that player pays {1}", Rhystic Study) | no (untargeted; the ability's own controller). Pairs with `timing = "cast_spell"`, `caster = "opponent"` (§5, "Trigger sibling fields"): resolution first pauses the ability's own controller on a `PendingChoice::MayYesNo` — do they want to draw at all (the card's ruling: declining is quiet, no pay window is ever offered) — and only a "yes" there raises the *triggering opponent* (not the ability's controller — `Game::queue_cast_spell_triggers` bakes their identity in via `TriggerContext::triggering_caster`, same context-fill shape as `copy_triggering_spell`'s `triggering_spell`) on a `PendingChoice::PayOrControllerDraws`/`Intent::PayOptionalCost`: paying stops the draw, declining lets it happen. |
| `change_target_of_target_spell_or_ability` | `target` (§7 target spec, default none — `"single_target_spell_on_stack"`) | `"single_target_spell_on_stack"` (the spell to bend, chosen as the trigger goes on the stack, CR 603.3d). Willbender's turned-face-up payload: "change the target of target spell or ability with a single target." At resolution the controller chooses a *different* legal target for that spell (CR 114.6b — must change if able) and it overwrites the stored one, via the same `ChooseSpellTargets`/`SpellTargetsChosen` write-back a multi-target choice uses. No-op if the spell left the stack (CR 608.2b already fizzles the trigger) or has no legal alternate. ponytail: CR's "or ability" half is unmodeled — stack abilities have no object identity to target in this engine (see the `single_target_spell_on_stack` spec); spells only. |
| `fight` | `ally_is_shared_target` (bool, default `false`) | intrinsic — default shape: target creature an opponent controls; your own fighter is chosen at resolution. `ally_is_shared_target = true` mirrors this: the ability's shared cast target is instead your own creature (a preceding `pump_until_end_of_turn` step's target, e.g.), and the enemy ("fights up to one target creature you don't control") is chosen at an *optional* resolution-time pause instead — no legal enemy, or the ally no longer being a creature, is a no-op (no pause, no fight). Primal Might: `effects = [{ type = "pump_until_end_of_turn", power = "x", toughness = "x", target = "creature_you_control" }, { type = "fight", ally_is_shared_target = true }]` |
| `schedule_at_next_upkeep` | `who` (`"you"` default, `"target_spell_controller"`), `then` (one effect) | no (delayed trigger, CR 603.7: `then` fires at the next upkeep — Arcane Denial's draws) |
| `schedule_next_cast_trigger` | `filter` (spell filter, §7 below), `then` (array of effects) | no (arms a CR 603.7 delayed *one-shot*, event- rather than step-armed: the next time the ability's own controller casts a spell matching `filter` **this turn**, `then` runs as its own triggered ability, with the triggering spell's chosen `{X}` filled into `then`'s `Amount::X`/`"half_x_rounded_down"` same as a `cast_spell` trigger's own `{X}` — Brass Infiniscope's "{T}: Add {C}{C}. When you next cast a spell with {X} in its mana cost this turn, you draw a card and gain half X life, rounded down." — a `taps_self` mana ability with two `effects` blocks: `add_mana` then this. Fires at most once — CR 603.7's "next" — and expires unconsumed at the next turn's Untap if no matching cast happens first) |
| `copy_triggering_spell` | `count` (amount), `may_choose_new_targets` (bool, default false) | no — a `schedule_next_cast_trigger` `then` payoff, or the direct effect of a live `cast_spell` trigger / `you_cast_this`: mints `count` copies of the spell that fired the watch (not a chosen target, not this ability's own spell), each offered the CR 707.10c retarget choice `copy_target_spell` offers when `may_choose_new_targets = true` (every current consumer sets this; `false` mints keeping the triggering spell's own targets instead, CR 707.10c's declined case, unexercised so far). No-ops if the triggering spell already left the stack (countered in response, CR 603.4) before the copying ability resolved. Thunderclap Drake's delayed one-shot: "{2}{U}, Sacrifice this creature: When you next cast an instant or sorcery spell this turn, copy it for each time you've cast your commander from the command zone this game. You may choose new targets for the copies." — `then = [{ type = "copy_triggering_spell", count = "commander_casts_from_command_zone", may_choose_new_targets = true }]`. Unbound Flourishing's live `cast_spell` watch ("Whenever you cast an instant or sorcery spell … copy that spell") and Owlin Spiralmancer's optional one ("your first spell with {X} … each turn, you may copy it") instead put this straight in `abilities.effects` on a `timing = "cast_spell"` ability — `Game::queue_cast_spell_triggers` bakes the live cast's own spell id into `triggering_spell`, same context-fill machinery as the delayed-trigger case. |
| `copy_triggering_ability` | `may_choose_new_targets` (bool, default false) | no — an `activate_ability` trigger's own payoff: mints one copy of the activated ability that fired the watch, put on the stack above the original (CR 707.10c — the copy isn't "activated"), carrying its source/effect/target and its chosen `{X}` unchanged (CR 706.10 — an already-doubled X isn't re-doubled). No-ops if the triggering ability already left the stack (countered in response, CR 603.4). ponytail: `may_choose_new_targets` (CR 707.10c) currently mints the copy keeping the original's targets — the re-pick isn't offered for the ability half, since no pool card is a *targeted* `{X}`-cost activated ability. Unbound Flourishing's ability half: `timing = "activate_ability"`, `caster = "you"`, `effects = [{ type = "copy_triggering_ability", may_choose_new_targets = true }]`. |
| `copy_triggering_spell_for_each_other_creature_you_control` | — | no — a `spell_targets_this` trigger's own payoff: mints one copy of the spell that fired the watch (not this ability's controller, but the SPELL's own controller — "that player copies") per *other* creature that controller controls that the spell could legally target (a legal target of the spell's own spec, excluding this permanent — hexproof/protection honored, ponytail: exact for the pool's single-target instant/sorcery consumers), each copy's target set directly to a distinct one of those creatures (CR 707.10's "each copy targets a different one" — engine-chosen assignment, not offered as a player choice). No-ops if the triggering spell already left the stack (countered in response, CR 603.4). Mirrorwing Dragon: "Whenever a player casts an instant or sorcery spell that targets only this creature, that player copies that spell for each other creature they control that the spell could target. Each copy targets a different one of those creatures." — `timing = "spell_targets_this"`, `spell_filter = "instant_or_sorcery"`, `effects = [{ type = "copy_triggering_spell_for_each_other_creature_you_control" }]` |
| `commander_enters_with_bonus_counters` | `count` (amount) | no — a `spend_mana_to_cast` (`spend_predicate = "commander"`) rider only: as the just-funded commander spell resolves into a permanent, it enters with `count` additional +1/+1 counters (run through the same doublers/replacements as printed `enters_with_counters`). The funded spell is threaded in from trigger context; no-ops if it already left the stack (countered in response, CR 603.4). Opal Palace's "If you spend this mana to cast your commander, it enters with a number of additional +1/+1 counters on it equal to the number of times it's been cast from the command zone this game." — `count = "commander_casts_from_command_zone"` |
| `attacker_draws_controller_counters` | `counters` (u32) | no (target is hardcoded to any creature; attacker from trigger context — Breena) |
| `attacker_loses_life_you_gain` | `amount` (u32) | no (the triggering attacker's controller loses, you gain — Parasitic Impetus) |
| `attacker_loses_life_you_draw` | `life_loss` (u32) | no (`opponent_attacks_you_with_creatures` payoff: that attacking opponent loses `life_loss` life, this ability's controller draws a card — Tomik, Wielder of Law) |
| `attacking_player_draws` | `count` (u32) | no (`another_player_attacks_with_creatures` payoff: the attacking player, not this ability's controller, draws `count` — Firemane Commando's "they draw a card if none of those creatures attacked you") |
| `deal_damage_to_entering_permanent` | `amount` (i32), `then_if_subtype` (array of strings, default empty), `then` (array of effects, default empty) | no (`permanent_enters` payoff: damage the permanent that just entered. `then_if_subtype`/`then` are an optional gated follow-up: `then` runs only if the entering permanent's printed subtypes intersect `then_if_subtype` AND the damage actually landed (not prevented by protection) — Marauding Raptor's "If a Dinosaur is dealt damage this way, this creature gets +2/+0 until end of turn" is `then_if_subtype = ["Dinosaur"]`, `then = [{ type = "pump_self_until_end_of_turn", power = 2, toughness = 0 }]`. Empty `then_if_subtype` (the default) never matches, so `then` never runs) |
| `reanimate_dying_enchanted_creature` | `under_owner` (bool, default `false`) | no (`enchanted_creature_dies` payoff: reanimate the specific creature this Aura was attached to when it died — CR 603.10a last-known information, "that card" — under this ability's own controller by default (`under_owner = false` — Changing Loyalty's "under your control") or under the card's owner (`under_owner = true` — Gift of Immortality's "under its owner's control"); no-ops if that card is no longer in a graveyard when the trigger resolves, e.g. exiled in response) |
| `sacrifice_enchanted_creature` | — | no (a `this_leaves_battlefield` payoff: the creature this Aura was attached to the instant it left the battlefield (CR 603.10a last-known information) is sacrificed by *its own* controller — Animate Dead's "When this Aura leaves the battlefield, that creature's controller sacrifices it." No-ops if the trigger never captured a host (this permanent wasn't attached to anything when it left), or if that creature is no longer on the battlefield (it died first, or was bounced/exiled in response)) |
| `exile_dead_creature_create_copy_with_subtype` | `add_subtypes` (array of subtype strings, default `[]`), `leaves_returns_exiled` (bool, default `false`) | no (a `creature_you_control_dies*` payoff — Hofri Ghostforge's "exile it. If you do, create a token that's a copy of that creature, except it's a Spirit in addition to its other types and it has 'When this token leaves the battlefield, return the exiled card to its owner's graveyard.'": exile the just-dead creature (CR 603.10a last-known information, the card now in a graveyard), mint a token copy of its printed def (CR 707.2) under this ability's controller, and union `add_subtypes` onto the token indefinitely (CR 613.4 — `["Spirit"]`); no-ops if that card is no longer in a graveyard when the trigger resolves. ponytail: the copiable snapshot is the source's *printed* `CardDef`, not full CR 707.2 copyable values. `leaves_returns_exiled`, when `true`, grants the minted token a synthesized `Trigger::ThisPermanentLeavesBattlefield` ability (engine-internal `return_exiled_card_to_owners_graveyard` payload, not TOML-facing) baking in the specific exiled card's id at mint time — mirrors the Myriad/Prowess synthesized-ability pattern; no-ops on resolution if that card is no longer in exile) |
| `attach_triggering_aura_to_minted_token` | — | no (`permanent_enters` payoff: if the entering permanent is an Aura, attach it to the token this same `effects` sequence's preceding `create_token` step just minted — Ajani's Chosen; a no-op if the entering permanent isn't an Aura) |
| `attach_self_to_entering` | — | no (`permanent_enters` payoff: attach this ability's own source — an Aura — to the entering permanent, moving it off any host it's already attached to; pair with `optional = true` for a "you may attach" — Shielded by Faith, Prison Term. Re-checks the Aura's own `enchant` filter against the entering permanent (CR 303.4f-style legality) — a no-op if it isn't a legal host, even if the "may" was accepted) |
| `attach_self_to_reanimated` | — | no (attach this ability's own source — an Aura — to the creature this same `effects` sequence's preceding `reanimate_to_battlefield` step just put onto the battlefield — Animate Dead) |
| `attach_self_to_minted_token` | — | no (`permanent_enters`/`etb` payoff: attach this ability's own source — an Equipment — to the token this same `effects` sequence's preceding `create_token` step just minted; a no-op if no token was minted this resolution — Fractal Harness's "and attach this Equipment to it") |
| `attach_minted_aura_to_target` | `target` (§6, the shared target) | yes (the mirror of `attach_self_to_minted_token`: attach the **Aura token** this same `effects` sequence's preceding `create_token` step just minted to the ability's chosen `target` — Scriv, the Obligator's "create a … Aura … token … attached to target creature an opponent controls." Pair a `create_token` step whose `token` is an Aura profile with this step sharing a `target = { permanent = { types = "creature", controller = "opponent" } }`; a no-op if the minted token isn't an Aura or no token was minted this resolution) |
| `schedule_return_this_aura_attached_to_reanimated` | — | no (must follow a `reanimate_dying_enchanted_creature` step in the same `effects` sequence: reads the creature that step just reanimated and schedules a CR 603.7 delayed trigger — fires at the next end step, returning this ability's own source — an Aura, now a graveyard card — to the battlefield attached to that creature. No-ops if the enchanted creature wasn't reanimated. The delayed payload itself (`return_this_aura_attached_to`, internal only, `creature` filled in at schedule time) no-ops if the Aura has since left the graveyard or the creature has since left the battlefield — Gift of Immortality's "Return this card to the battlefield attached to that creature at the beginning of the next end step.") |
| `return_this_aura_from_graveyard_attached_to_chosen_host` | — | no (`enchanted_creature_dies` payoff: unlike `return_this_aura_attached_to` above, the return isn't to the *same* look-back creature — this Aura's own host stays dead. Moves this ability's own source — an Aura, now a graveyard card — from the graveyard to the battlefield unattached, then pauses on the same `ChooseAttachHost` choice a deployed Aura already uses (§ deploy-time attach) over every legal host on the battlefield; no legal host leaves it unattached for the Aura-legality state-based action to sweep. No-ops if the Aura has since left the graveyard — Screams from Within's "return this card from your graveyard to the battlefield") |
| `schedule_return_this_aura_from_graveyard_attached_to_chosen_host` | — | no (the delayed sibling of `return_this_aura_from_graveyard_attached_to_chosen_host`: schedules that same effect against this ability's own source at the next end step, CR 603.7, mirroring `schedule_return_this_aura_attached_to_reanimated`'s emit shape — the choose-host pause happens when the delayed trigger fires, not now — Ghoulish Impetus's "return this card to the battlefield at the beginning of the next end step") |
| `search_library` | `filter` (card filter, §7), `to_zone` (§7), `tapped` (bool, default `false`), `count` (u8, default `1` — "up to N" finds, one shuffle), `searcher` (`"you"` default, `"target_controller"` — Path to Exile's compensation search), `overflow` (§7, default none — where every find *after the first* goes, if different from `to_zone`; Cultivate) | no (searcher picks; tutors/ramp/fetchlands) |
| `each_player_sacrifices` | `scope` (§7, default `"all_players"`), `filter` (§7, default `"creature"`), `keep_one` (bool, default `false`), `life_loss` (i32, default `0`), `then` (array of effects, default `[]`) | no (multi-player sacrifice edict; see below) |
| `each_player_exiles_from_graveyard` | — | no (multi-player fan-out: each player, in APNAP order, exiles one card from *their own* graveyard — mandatory when they have any, empty graveyard skipped — Augusta, Order Returned. Carries no payoff of its own: put the reflexive payoff as a following `effects` step reading `"nonland_cards_exiled_this_way"` (§6), resumed once every player has answered) |
| `target_player_exiles_from_graveyard` | `target` | yes (Relic of Progenitus's "Target player exiles a card from their graveyard": the *targeted* player, not the caster/activator, picks one card from their own graveyard — mandatory when non-empty, a no-op when empty. The one-player special case of `each_player_exiles_from_graveyard`'s fan-out — same pause, no payoff) |
| `each_player_controller_chooses_counter_target` | — | no (multi-player fan-out — Nils, Discipline Enforcer's "for each player, put a +1/+1 counter on up to one target creature that player controls": for each living player in APNAP order, the *effect's controller* picks up to one creature that player controls and puts one +1/+1 counter on it — pausing on a controller-answered choice — a player with no creature is skipped. A single-purpose effect, no fields. Caster-answered twin of `caster_keeps_one_of_each_type_per_player`. ponytail: "up to one target creature" is really a target chosen as the ability goes on the stack, modeled as a resolution-time pick) |
| `caster_keeps_one_of_each_type_per_player` | — | no (Tragic Arrogance's "For each player, you choose from among the permanents that player controls an artifact, a creature, an enchantment, and a planeswalker. Then each player sacrifices all other nonland permanents they control": for each living player in APNAP order, the *caster* picks up to one of that player's nonland permanents of each type to keep — pausing on a caster-answered choice — and every other nonland permanent that player controls is sacrificed by its controller. A single-purpose effect, no fields. ponytail: the pool has no planeswalker permanent type, so the "…a planeswalker" slot is unreachable — the four-type keep collapses to artifact/creature/enchantment) |
| `councils_dilemma_vote` | `options` (array of strings — the ballot labels; today only `["past", "present"]`) | no (council's dilemma, CR 701.32 — Fateful Tempest's "Starting with you, each player votes for past or present": each living player, in turn order starting with the caster, votes for one of `options`, pausing on a vote choice. Carries no payoff of its own: put the tally-scaled outcomes as following `effects` steps reading `"past_votes"` / `"present_votes"` (§6), resumed once every player has voted — the vote-round twin of `each_player_exiles_from_graveyard`. ponytail: the two tallies and their `§6` amounts are hardcoded to the past/present ballot — Fateful Tempest is the only voting card; generalize to a label→tally map when a differently-balloted council's-dilemma / will-of-the-council card lands) |
| `each_player_creates_fractal_from_exiled_power` | `token` (§8) | no (Oversimplify's "Each player creates a 0/0 green and blue Fractal creature token and puts a number of +1/+1 counters on it equal to the total power of creatures they controlled that were exiled this way": mints one `token` per living player, in APNAP order, with +1/+1 counters equal to *that player's own* share of the power a preceding `exile_all` step exiled — routed through the same doubler/Hardened-Scales pipeline as `create_token`'s `enters_with`. No player choice, so it never pauses. Requires a preceding `exile_all` step in the same `effects = [...]` sequence — its per-controller power snapshot is internal engine state, not a `§6` amount, since only this effect reads it) |
| `each_player_discards_hand_then_draws` | `count` (§6 amount) | no (Wheel of Fortune's "Each player discards their hand, then draws seven cards": every living player, in APNAP order, discards their *entire* hand — not a chosen subset — then draws `count`. No player choice, so it never pauses, unlike `discard`'s partial-hand card-pick) |
| `each_other_token_becomes_copy_of_chosen` | — | no (Brudiclad's "Then you may choose a token you control. If you do, each other token you control becomes a copy of that token": pauses so the controller may choose one token they control — declinable ("you may"), no token ⇒ no pause — then every *other* token they control becomes an indefinite copy of it (CR 706/707.2, reusing the same `BecameCopy` primitive as `enter_as_copy`; permanent, CR 400.7). Only tokens are chosen or converted, only the controller's own. ponytail: copyable values are the chosen token's `CardDef` (CR 707.2), same note as `enter_as_copy`) |
| `put_counter_then_may_become_copy_of_card_from_list` | — | no (Spirit of Resilience's "put a +1/+1 counter on this creature, then you may have this creature become a copy of an artifact or creature card from among those cards until end of turn": a `cards_leave_your_graveyard` (§5) payoff. Places one +1/+1 counter on the ability's own source, then — if any artifact/creature card left the graveyard this batch — pauses so the controller may pick one (declinable "you may", no copyable card ⇒ counter only, no pause); the pick becomes an *until-end-of-turn* copy of it (CR 706/707.2/514.2, reusing `enter_as_copy`'s `BecameCopy` with `until_eot: true`). The leave-batch's card ids ride in trigger context — no TOML field. ponytail: copyable values are the chosen card's printed `CardDef` (CR 707.2), same note as `enter_as_copy`) |
| `become_copy_of_target_creature_gaining_myriad` | `target` (§7 target — a real cast/activation-time target, e.g. `{ permanent = { types = "creature", controller = "you", other = true, nonlegendary = true } }`) | yes (Muddle, the Ever-Changing's magecraft: "Muddle becomes a copy of up to one target nonlegendary creature you control until end of turn, except it has myriad." "Up to one" is modeled by pairing this effect with `optional = true` on the ability — same shape as Skyclave Apparition's "up to one target" (declining or having no legal target both fizzle harmlessly). On resolution: an until-end-of-turn `BecameCopy` (CR 706/707.2, reverted at cleanup) plus an until-end-of-turn `myriad` keyword grant, the same "gains a keyword" `TempBoost` shape `enter_as_copy`'s `gains_haste` uses) |
| `myriad_token_copies` | — | never authored in TOML — the payload `Keyword::Myriad` resolves into when its carrier attacks (CR 702.114a), synthesized by `Game::queue_myriad_triggers` the same way Prowess's pump is synthesized from the keyword. For each opponent other than the defending player, mints a tapped-and-attacking token copy of the attacker's current characteristics (never `AttackerDeclared` — CR 508.4, so a minted copy can't re-trigger myriad), then schedules it to be exiled at the true end of combat (`Step::EndCombat`, already a delayed-trigger `fire_at` boundary). ponytail: "you may create a token copy" per opponent is modeled as mandatory, matching the pool's existing tapped-attacking-copy convention (`copy_each_entered_this_turn_token_tapped_attacking`, Encore); no planeswalker permanent type exists in the pool, so "that player or a planeswalker they control" narrows to the player, the same standing pool-wide limitation every attack effect carries |
| `may_sacrifice` | `filter` (§7, default: any permanent), `then` (array of effects, default `[]`) | no (resolution-time optional cost: the controller may sacrifice one permanent matching `filter`; `then` runs only "if you do" — CR 601.2f-style "You may sacrifice a permanent. If you do, …", Witherbloom Charm; also fits a triggered ability's own optional-cost gate, Springbloom Druid's ETB) |
| `may_return_from_graveyard` | `filter` (card filter, §7), `if_you_sacrificed_this_way` (bool, default `false`) | no (resolution-time optional rider: the controller may return one card from *their own* graveyard matching `filter` to hand, or decline — CR 601.2f-style "you may return … to your hand", Deadly Brew. `if_you_sacrificed_this_way` gates the whole rider — no pause at all, same as declining — on the controller having actually sacrificed a permanent during this resolution's own preceding `each_player_sacrifices` edict, tracked by an edict-scoped scratch flag that resets each edict and is set only by the edict's own controller's sacrifice) |
| `reflexive_trigger` | `then` (array of effects) | no (a real reflexive "when you do" triggered ability, CR 603.3b: put this step after a `create_token` step; when it resolves — the "you do" being that the preceding `create_token` minted a token this resolution — each `then` effect is placed on the stack as its **own separate, respondable ability** (its own priority window, its own target chosen at placement, CR 601.2c), with the minted token threaded in. No token minted this resolution → no reflexive trigger — Forum Filibuster's "When you do, …") |
| `return_from_graveyard_attached_to_token` | `filter` (card filter, §7) | yes (`{0,1}` — "return up to one target"; the reflexive-ability body placed by `reflexive_trigger`: targets an Aura/Equipment card in *your own* graveyard matching `filter`, and at resolution returns it to the battlefield attached to the minted token threaded in at placement — Forum Filibuster. Fizzles (CR 608.2b) if that token has left the battlefield by resolution) |
| `may_discard` | `then` (array of effects, default `[]`) | no (resolution-time optional sub-action, CR 608.2c — the controller may discard one card from their own hand; `then` runs only "if you do" — Quintorius, History Chaser's +1 "You may discard a card. If you do, draw two cards, then mill a card." Distinct from a cost gate: activating/triggering the ability always happens, the discard is a choice made while it resolves) |
| `put_land_from_hand` | `tapped` (bool, default `false`) | no (controller may put a land from hand onto the battlefield, CR 305.9 — Eureka Moment; doesn't use the land drop) |
| `cast_creature_face_down` | — | no (controller may cast a creature card from hand — mana value at most the `{X}` this ability was activated for (CR 107.3, read from the resolving ability's context) — **face down as a 2/2 creature spell** (CR 708.2) without paying its mana cost; "you may," so no payable creature is a no-op — Illusionary Mask's `{X}` ability. Pair with `timing = "activated"`, `activation_cost = { x = true }`, `sorcery_speed = true`. ponytail: `mana value <= X` approximates the printed "the mana you spent on {X} could pay its cost" color-subset test) |
| `choose_one` | `modes` (array of effects) | no ("Choose one —" on a *triggered* ability: the controller picks one non-targeting mode at resolution — Atsushi) |
| `become_prepared` | — | no (marks the source prepared so its `[back]` face may be cast — prepare DFCs, §1) |
| `level_up` | `level` (u8) | no (a Class's "Level N" ability, CR 717.2: sets the source's level to `level`. Always on a `timing = "activated"`, `sorcery_speed = true` ability; the engine offers it only while the source is at `level - 1`, so each level is gained exactly once. See the Class worked example, §9) |
| `arm_combat_damage_watch` | — | no-target-of-its-own (reads the enclosing `effects` sequence's shared target) — arms a CR 603.7 delayed watch on that shared target: the ability's own source becomes prepared the first time the watched creature deals combat damage to a player, any time later this combat, then the watch is removed (cleared unconsumed at end of combat). Object-armed like `schedule_next_cast_trigger` is filter-armed. Always resolves to `become_prepared`, no `then` list — Stensian Sanguinist's "target creature gains deathtouch until end of turn. Whenever that creature deals combat damage to a player this combat, this creature becomes prepared", authored as `pump_until_end_of_turn` (deathtouch) then this in the same `effects` sequence |
| `choose_creature_type` | — | no (CR 614.12/700.9-style "as ~ enters, choose a creature type" — pauses on a `ChooseCreatureType` choice for the controller over the pool's known creature types, `CREATURE_TYPES`; the chosen type is stored on the ability's own source, `Permanent::chosen_subtype`, read back by `anthem_static`'s `chosen_subtype` axis — Patchwork Banner) |
| `choose_color` | — | no (CR 614.12/700.9-style "as ~ enters, choose a color" — pauses on a `ChooseColor` choice for the controller over the fixed five colors; the chosen color is stored on the ability's own source, `Permanent::chosen_color`, read back by `grant_to_attached`'s `protection_from_chosen_color` axis — Flickering Ward) |
| `conditional` | `condition` (§5 shape), `then` (array of effects), `negate` (bool, default `false`) | no (runs `then` only if `condition` holds — or, with `negate = true`, only if it *doesn't* — when *this step* resolves; a per-step gate inside an `effects` sequence. Plain: Zimone's "draw two instead". Negated: Court Hussar's "sacrifice it unless {W} was spent to cast it" is `condition = { type = "color_was_spent_to_cast_this", color = "white" }`, `negate = true`, `then = [{ type = "sacrifice_source" }]` — CR's "unless" is `negate`d "if") |

`counter_replacement`: adder → `add = 1`; doubler → `times = 2` (omit `add`).
`token_replacement`: doubler → `times = 2` (Doubling Season). Pure multiply — no `add`/`other`.
`life_gain_replacement`: adder → `plus = 1` (Pest Rescuer). Pure addend — additive across statics.
`cast_x_replacement`: doubler → `times = 2` (Unbound Flourishing). Permanent X-spells only, applied after payment; multiplicative across statics.

`pump_until_end_of_turn`'s `keywords` grants keywords to the target alongside the P/T change
(`power`/`toughness` may be `0` for a keyword-only grant); `pump_creatures_you_control_until_end_of_turn`
is the same grant applied to every creature the controller controls, no target:

```toml
# Selfless Spirit: "Sacrifice this creature: Creatures you control gain indestructible until end
# of turn."
[[abilities]]
timing = "activated"
sacrifice = "this"

[[abilities.effects]]
type = "pump_creatures_you_control_until_end_of_turn"
power = 0
toughness = 0
keywords = ["indestructible"]
```

`filter` narrows the mass grant to a subset of the controller's creatures (default: every creature,
the unfiltered shape above):

```toml
# Quintorius, History Chaser: "−4: Spirits you control gain double strike and vigilance until
# end of turn."
[[abilities]]
timing = "activated"
loyalty = -4

[[abilities.effects]]
type = "pump_creatures_you_control_until_end_of_turn"
power = 0
toughness = 0
keywords = ["double_strike", "vigilance"]
filter = { subtypes = ["Spirit"] }
```

```toml
# Rogue's Passage: "{4}, {T}: Target creature can't be blocked this turn."
[[abilities]]
timing = "activated"
taps_self = true

[abilities.activation_cost]
generic = 4

[[abilities.effects]]
type = "pump_until_end_of_turn"
power = 0
toughness = 0
target = "creature"
keywords = ["unblockable"]
```

`each_player_sacrifices` (Deadly Brew, Priest of Forgotten Gods): each affected player (`scope`)
loses `life_loss` life, then chooses one of their permanents matching `filter` to sacrifice (or, with
`keep_one`, keeps one and sacrifices the rest) — resolved player by player in APNAP order. `then`
runs afterward for the edict's *controller* only (Priest's "add {B}{B} and draw a card"):

```toml
# Priest of Forgotten Gods: "{T}, Sacrifice two other creatures: Any number of target players each
# lose 2 life and sacrifice a creature of their choice. You add {B}{B} and draw a card."
[[abilities.effects]]
type = "each_player_sacrifices"
scope = "targeted_players"
filter = "creature"
life_loss = 2

[[abilities.effects.then]]
type = "add_mana"
mana = ["black", "black"]

[[abilities.effects.then]]
type = "draw_cards"
count = 1
```

`scope = "targeted_players"` (CR 601.2c/608.2b "any number of target players") pauses the
controller on a subset-of-players pick — every living player is legal, zero is a legal choice,
and the caster may include themselves — before the per-player sacrifice fan-out runs over exactly
the chosen set (not the scope-derived `"all_players"`/`"each_opponent"` set).

An edict's `filter` is the composable **permanent filter** (§7), so an artifact- or nontoken-only
edict *is* expressible now — Lorehold Charm's "each opponent sacrifices a nontoken artifact" is
`filter = { types = "artifact", token = "nontoken" }`. The filter's `controller` axis is read
relative to the *sacrificing* player, so leave it `"any"` (the default) on an edict.

`may_sacrifice` is the single-player, entirely-optional cousin — declining runs nothing, no
`life_loss`, no APNAP sequencing:

```toml
# Witherbloom Charm mode 0: "You may sacrifice a permanent. If you do, draw two cards."
[[abilities.effects]]
type = "may_sacrifice"

[[abilities.effects.then]]
type = "draw_cards"
count = 2
```

```toml
# Springbloom Druid: "When this creature enters, you may sacrifice a land. If you do, search
# your library for up to two basic land cards, put them onto the battlefield tapped, then
# shuffle." — the same effect gates a *triggered* ability's rider, not just a spell's.
[[abilities]]
timing = "etb"

[[abilities.effects]]
type = "may_sacrifice"
filter = "land"

[[abilities.effects.then]]
type = "search_library"
filter = "basic_land"
to_zone = "battlefield"
tapped = true
count = 2
```

## 7. Targets (`target =`) and filters

`target` string on a targeted effect:

| Tag | Legal targets | Used by |
|-----|--------------|---------|
| `"none"` | (default) no target | — |
| `"creature"` | a creature on the battlefield | destroy/exile/pump/put_counters/bounce/goad/token_copy |
| `"creature_you_control"` | a creature the choosing player controls | (Twinflame's `create_token_copy`) |
| `"creature_token_you_control"` | a creature *token* the choosing player controls (Populate, CR 701.32) | `create_token_copy` |
| `"player"` | any living player | mill, damage |
| `"opponent"` | a living player other than the choosing player ("target opponent") | Witherbloom Command mode 3 |
| `"any"` | "any target" (CR 115.4 — a creature, a player, or a planeswalker) | deal_damage |
| `"creature_or_planeswalker"` | a creature or planeswalker on the battlefield | deal_damage (Rip Apart) |
| `"player_or_planeswalker"` | a living player or a planeswalker | deal_damage (Balefire Liege) |
| `"artifact_enchantment_or_planeswalker"` | a battlefield artifact, enchantment (incl. Aura), or planeswalker | destroy_target (Fracture) — sugar; equals `{ permanent = { types = ["artifact", "enchantment", "planeswalker"] } }` |
| `{ permanent = <filter> }` | a battlefield permanent matching a composable **permanent filter** (below) | any targeted battlefield effect (destroy/exile/bounce…) — Anguished Unmaking, Abrade, Skyclave Apparition |
| `"this"` | the ability's own source — no real choice, no `PendingChoice`, and (CR-faithfully) no shroud/hexproof check, since these abilities don't say "target" | put_counters/double_counters on self (Hangarback Walker, Primordial Hydra) |
| `"enchanted_creature"` | the creature this Aura/Equipment is attached to — same non-targeted treatment as `"this"` | exile_target (Redemption Arc) |
| `"this_auras_graveyard_target"` | Animate Dead only: the graveyard creature card this Aura was cast targeting (`enchant_graveyard`, above), captured as it enters — same non-targeted, no-pause treatment as `"this"`/`"enchanted_creature"`; empty (CR 603.3c: the ability is dropped) once that card has left the graveyard | reanimate_to_battlefield (Animate Dead) |
| `"your_graveyard"` | a creature card in your graveyard | return_from_graveyard_to_hand |
| `"any_graveyard"` | a creature card in any graveyard | reanimate_to_battlefield |
| `{ card_in_graveyard = { whose, filter } }` | a graveyard card matching a **card filter** (below); `whose` = `"yours"`/`"any"` | reanimate_to_battlefield/tuck_from_graveyard with a non-creature or MV-gated filter — Sevinne's Reclamation, Mystic Sanctuary, Excava |
| `"single_target_spell_on_stack"` | a spell on the stack with exactly one target (CR 114.6 "single target"); targets the stack object, any controller's | `change_target_of_target_spell_or_ability` (Willbender). ponytail: CR's "or ability" is unreachable — stack abilities carry no object identity to target — so only spells qualify |
| `"activated_ability_on_stack"` | an *activated* ability on the stack (CR 112.7a), any controller's; keyed by the ability's `source` id (not a chosen object of its own) | `counter_target_activated_ability` (Azorius Guildmage). Mana abilities never reach the stack (CR 605.3b); triggered abilities are excluded. ponytail: keyed by source id — if two activated abilities on the stack shared a source, resolution counters the topmost; no pool card produces that |

`target = { permanent = <filter> }` is the general targeted-permanent form; `<filter>` is a
permanent-filter table or shorthand string (below). It scales to new narrowings (nonland, nontoken,
opponent-controlled, mana-value gate) without a new target tag. The bare `"creature"` and
`"artifact_enchantment_or_planeswalker"` tags above stay as convenient sugar for their common shapes.

Two more targets exist as intrinsic-only (they are never a settable `target =` field — the effect
has no `target` field at all and always targets this): `copy_target_spell` always targets an
instant/sorcery spell on the stack; `counter_target_spell` always targets a spell on the stack
(narrowed by its own `filter` field, §6).

**Target counts** — a multi-target effect's `count` (or `put_counters`' `targets`) field is a
*target count* (CR 601.2c): a bare integer `N` for an exact "N target" (Aether Gale's `count = 6`),
or a `{ min, max, x_scaled }` table for an "up to"/"one or two" range (`min` defaults to 0 — a
fully declinable "up to N"; Volcanic Salvo's "up to two" is `count = { max = 2 }`). Only spell-timed
abilities widen to *multiple* targets — a *triggered* ability's target still goes through the
single-target `ChooseTarget` path, but that path does read a `{ max = 1 }`-style `min = 0`:
declining (submit no target), or there being no legal target to begin with, still puts the ability
on the stack with no target chosen (CR 601.2c treats choosing zero of "up to one" as a complete,
legal choice) — *unless* every step of the ability needs that same missing target, in which case it
drops outright as a pure no-op (`Effect::has_target_independent_step`; Killian, Decisive Mentor's
"tap up to one target creature and goad it" — `tap_target`'s `count = { max = 1 }`, shared by the
following untyped `goad_target` step since a `Sequence` shares one target, and goad has nothing to
goad without a tapped creature). An ability with a *target-independent* step alongside the
declinable one still runs that step (Kinetic Ooze's "destroy up to one target artifact or
enchantment … If X is 5 or more, you draw a card" — the draw runs even when nothing gets destroyed).
A *spell-timed* multi-target ability declined the same way (0 of a `min = 0` count chosen) follows
the identical rule at resolution: it drops outright unless a target-independent step rides along,
in which case that step still runs (Zimone's Hypothesis' "you may put a +1/+1 counter on a
creature" primer ahead of its untargeted mass parity-bounce — declining the counter still bounces).

A multi-target `count` works inside a `modal` spell's mode too (Prismari Charm mode 1's "deals 1
damage to each of one or two targets" is `deal_damage` with `count = { min = 1, max = 2 }` as one
mode among several) — the caster picks the mode first, then, if that mode's effect is
multi-target, chooses its targets the same post-cast way a non-modal multi-target spell does. A
modal spell resolves only its chosen mode(s), so at most one multi-target clause is ever live at
once; the other modes keep their ordinary single-target-or-none shape.

`x_scaled = true` (default `false`) marks `min`/`max` as placeholders substituted at cast time by
the spell's own chosen `{X}` (CR 601.2b — X is fixed before targets are chosen): `{ min = 0,
max = 0, x_scaled = true }` is "up to X target(s)" (Silkguard's "up to X target creatures you
control"); `{ min = 1, max = 1, x_scaled = true }` is "exactly X target(s)" (Curse of the Swine's
"exile X target creatures"). The substituted X is clamped to `MAX_TARGETS` (6) — a spell that
needs to target more would need that ceiling bumped. `x_scaled` works on any `count`/`targets`
field (`exile_target`, `destroy_target`, `put_counters`'s `targets`, …), not just the effects named
above — any multi-target effect's count can opt in.

`sacrifice_scaled = true` (default `false`) is the sibling for a spell whose X is never chosen as
`{X}` but is instead *defined* by an additional sacrifice cost (CR 601.2f) — Immoral Bargain's "As
an additional cost to cast this spell, sacrifice X creatures. Destroy X target nonland
permanents." is `[cost.additional] sacrifice = { count = "one_or_more", filter = "creature" }`
paired with `destroy_target`'s `target = { permanent = { types = "nonland" } }`,
`count = { sacrifice_scaled = true }`. `min`/`max` need not be given (both default 0); at cast
time they're always overridden to "exactly X", X being however many were actually sacrificed
(`Game::spell_sacrifice_count`) — always mandatory, unlike `x_scaled`'s declinable "up to X" case.
`min`/`max`/`x_scaled` don't matter when `sacrifice_scaled = true`; no pool card sets both.

`strive_scaled = true` (default `false`) is Strive's own sibling (CR 601.2c/601.2f/702.42) — a
spell whose target count is neither a chosen `{X}` nor a paid sacrifice cost but a bare number the
caster commits to *before* the stack (CR 601.2c precedes 601.2f, but this engine puts a
multi-target spell on the stack and only then pauses to choose targets, so the count must be
declared up front). Twinflame's "Choose any number of target creatures you control" paired with
"This spell costs {2}{R} more to cast for each target beyond the first" is `[cost.additional]
strive = { generic = 2, red = 1 }` paired with `create_token_copy`'s `targets = { strive_scaled =
true }`. `min`/`max` need not be given; at cast time they're always overridden to "exactly N", N
being the caster's declared `Intent::Cast.strive_count` (`Game::spell_strive_count`) — always
mandatory once declared, like `sacrifice_scaled`'s "exactly X". `min`/`max`/`x_scaled`/
`sacrifice_scaled` don't matter when `strive_scaled = true`; no pool card sets more than one of
the three `_scaled` flags.

"A creature you control" for any targeted battlefield effect is
`target = { permanent = { types = "creature", controller = "you" } }` (the bare
`"creature_you_control"` tag remains as sugar). Effects with no `target` field can't be
narrowed at all — e.g. `attacker_draws_controller_counters`' counter target is hardcoded (see
`breena.toml`).

**Amounts** — a numeric field on *any* effect (`amount`/`count`/`power`/`toughness`/`repeat`). One
resolver (`Game::resolve_amount`) evaluates every form, so any amount works on any numeric effect.
Write one of:

- a plain number: `amount = 3`
- the spell's `{X}`: `"x"` (only spells choose `X`; a triggered/activated ability resolves `"x"`
  as 0 — except a `when_you_cast_this` ability, whose `"x"`/`"half_x"`/`"half_x_rounded_down"` are
  filled at placement from the triggering cast's chosen `{X}`; and a self `timing = "etb"`
  ability, whose `"x"`/`"half_x"` read the entering permanent's own already-placed `+1/+1`
  counters as an X proxy — The Goose Mother's "create half X Food tokens" — same reasoning as
  `mv_max_x` below), or `"half_x"` (rounded up) / `"half_x_rounded_down"` (the explicit
  round-down override some cards print instead — Hydroid Krasis) / `"twice_x"`
- a board count of creatures: `"per_creature_you_control"` / `"per_creature_on_battlefield"`
  (Chain Reaction uses the latter)
- a filtered count over a zone: `{ per_permanent = <filter (below)>, zone = "battlefield" | "graveyard" }`
  (`zone` defaults to `"battlefield"`). Subsumes per-artifact / per-aura / "per creature card in
  your graveyard" (Izoni). Example: `{ per_permanent = { types = "creature", controller = "you" }, zone = "graveyard" }`.
  A shorthand type-only filter also works with `subtypes` (Pearl-Ear's affinity for Auras — "per
  Aura you control" — is `{ per_permanent = { subtypes = ["Aura"], controller = "you" } }`).
- a characteristic read: `"source_power"` / `"source_toughness"` (the effect's own permanent's
  power/toughness — Goldvein Hydra; Tanazir's attack P/T snapshot),
  `"target_power"` / `"target_toughness"` / `"target_mana_value"` (the chosen target's —
  Swords to Plowshares' power rider; Condemn's toughness rider), `"per_counter_on_source"` (+1/+1
  counters on the source), `{ per_counter_of_kind = "charge" }` (named-kind counters on the
  source — Astral Cornucopia), `"total_power_you_control"` (summed power of the controller's
  creatures — Volcanic Salvo's self cost reduction), `"commander_color_count"` (colors in your
  commander's identity — War Room's pay-life), `"sacrificed_creature_power"` /
  `"sacrificed_creature_toughness"` (the power/toughness of the creature just paid as this
  ability's sacrifice cost — Dina; Miren, the Moaning Well; only meaningful with a `sacrifice`
  cost). On a `timing = "dies"` ability, both `"source_power"` and `"per_counter_on_source"` read
  the source's CR 603.10a last-known-information snapshot — its power/counters the instant before
  it died (Lifeblood Hydra, Hangarback Walker, Goldvein Hydra) — not its now-graveyard-card values.
  On a `timing = "an_enchanted_creature_dies"` ability, `"auras_you_controlled_attached_to_dying_creature"`
  reads the same last-known-information snapshot's count of Auras this permanent's controller
  controlled that were attached to the dying creature (Hateful Eidolon; only meaningful on that
  timing).
- a turn tally: `"life_gained_this_turn"` / `"spells_cast_this_turn"` /
  `"creatures_died_this_turn"` (died under your control — Gorma) /
  `"nontoken_creatures_entered_this_turn"` (nontoken creatures that entered under your control
  this turn, tokens excluded — Gyome, Master Chef's end-step Food count) /
  `"permanents_died_this_turn"` (any permanent, any controller, put into a graveyard from the
  battlefield — Ominous Harvest's Gravestorm; game-wide, unlike `creatures_died_this_turn`'s
  per-controller tally) / `"greatest_instant_or_sorcery_mana_value_cast_this_turn"` (the highest
  mana value among instant/sorcery spells you've cast this turn, 0 if none — Rootha, Mastering
  the Moment's begin-combat "X is the greatest mana value among instant and sorcery spells
  you've cast this turn") / `"one_plus_instants_and_sorceries_cast_this_turn"` (one plus the
  count of instant/sorcery spells you've cast this turn — Rionya, Fire Dancer's begin-combat "X
  is one plus the number of instant and sorcery spells you've cast this turn"; a copied spell
  doesn't bump the underlying tally) (reset each turn at untap)
- `"spell_sacrifice_count"` — how many creatures were sacrificed to pay the resolving spell's
  additional sacrifice cost (CR 601.2f); only meaningful on that spell's own effect (Plumb the
  Forbidden's copy rider)
- `{ auras_attached_to_source = {} }` — the count of Auras (any controller) currently attached to
  the effect's source (CR 303.4) — Kor Spiritdancer's "gets +2/+2 for each Aura attached to it"
  (Amounts carry no per-unit coefficient, so the printed +2/+2 is two summed self-only anthem
  statics each reading this amount — see the `per_permanent`/`self_only` example above for the
  idiom). A bare `{}` presence flag, live-read (no controller filter, no placeholder-fill) —
  distinct from `"auras_you_controlled_attached_to_dying_creature"` above, which is
  controller-scoped and only meaningful on a dying-creature watch.
- `"instant_or_sorcery_cards_in_your_graveyard"` — the count of instant and sorcery cards in the
  effect's controller's graveyard (Furygale Flocking's self cost reduction, "{1} less to cast for
  each instant and sorcery card in your graveyard"). Not a `per_permanent`/`zone = "graveyard"`
  filter: an instant/sorcery card's `TypeSet` is empty (permanents are the other card kinds), so no
  `PermanentFilter` axis can select it — this reads the card's `CardKind::Spell` directly instead.
- `{ permanents_destroyed_this_way = <filter (below), default matches all> }` — a "destroyed this
  way" rider count (CR): how many permanents *this same resolution's own* preceding `destroy_all`
  step destroyed, restricted to `filter` (Ceaseless Conflict's "for each nontoken creature you
  controlled that was destroyed this way" is `{ permanents_destroyed_this_way = { types =
  "creature", controller = "you", token = "nontoken" } }`; Culling Ritual's unfiltered "for each
  permanent destroyed this way" is `{ permanents_destroyed_this_way = {} }`). Resolution-scoped,
  not a turn tally like `"permanents_died_this_turn"` above — pair with the `destroy_all` step in
  the same `effects = [...]` sequence (§"effect-sequencing"). Only the `types`/`subtypes`/
  `controller`/`token` filter axes apply (the destroyed permanents are already off the
  battlefield by the time this reads them — no live tapped/mv/power context).
- `"nonland_cards_exiled_this_way"` — how many *nonland* cards *this same resolution's own*
  preceding `each_player_exiles_from_graveyard` step exiled across every player (Augusta, Order
  Returned's "put that many +1/+1 counters"). Resolution-scoped like `permanents_destroyed_this_way`
  above; pair it with the `each_player_exiles_from_graveyard` step in the same `effects = [...]`
  sequence.
- `"past_votes"` / `"present_votes"` — how many past / present votes *this same resolution's own*
  preceding `councils_dilemma_vote` step tallied (Fateful Tempest's "mill a card for each past
  vote" / "Exile the top card for each present vote"). Resolution-scoped like
  `nonland_cards_exiled_this_way`; pair with the `councils_dilemma_vote` step in the same
  `effects = [...]` sequence.
- `"total_mana_value_milled_this_way"` — the total mana value of the cards *this same resolution's
  own* preceding `mill_self` step just milled (Fateful Tempest's "damage to each opponent equal to
  the total mana value of cards milled this way"). Resolution-scoped; pair with the `mill_self` step
  in the same `effects = [...]` sequence.
- `"exiled_card_mana_value_this_way"` — the mana value of the card *this same resolution's own*
  preceding `exile_target_graveyard_card_record_mana_value` step just exiled (Surge to Victory's
  "Creatures you control get +X/+0 until end of turn, where X is that card's mana value").
  Resolution-scoped; pair with that step in the same `effects = [...]` sequence.
- a game tally off the target: `"commander_casts_from_command_zone"` — how many times the
  *targeted player* has cast a commander from the command zone this game (Commander's Insight)
- `"cards_in_target_player_hand"` — the number of cards in the resolving spell's chosen player
  target's hand (Rousing Refrain's "Add {R} for each card in target opponent's hand"). The ability
  must target a player (e.g. an `add_mana` with `target = "opponent"`).
- `"cards_in_your_hand"` — the number of cards in the effect's own controller's hand, no target
  needed (Empyrial Armor's `grant_to_attached` "+1/+1 for each card in your hand"). Read live at
  every characteristic recompute, so the buff tracks the hand as it grows or shrinks — the
  no-target sibling of `"cards_in_target_player_hand"` above.
- `"triggering_spell_mana_value"` — the mana value of the spell that fired a `timing = "cast_spell"`
  or `timing = "magecraft"` ability (Renegade Bull's "+X/+0 … where X is that spell's mana value";
  Deekah's Magecraft Fractal's `create_token` `enters_with`). Only meaningful on that ability's own
  effect; resolved when the ability is placed on the stack (both the cast and copy halves of
  Magecraft thread the triggering spell's mana value the same way).
- `"triggering_spell_mana_spent"` — the mana actually spent (CR 601.2h) to cast the spell that
  fired a `timing = "cast_spell"` ability (Manaform Hellkite's `create_token` `set_base_pt`: "X is
  the amount of mana spent to cast that spell"). The mana-*spent* sibling of
  `"triggering_spell_mana_value"` above, which reads the printed mana value instead (CR 202.3b
  treats `{X}` as 0 outside the stack, so the two diverge for an `{X}` spell). Only meaningful on a
  `cast_spell` ability's own effect; resolved when the ability is placed on the stack from the
  cast's preceding mana payment.
- `"combat_damage_dealt"` — the summed combat damage a
  `timing = "zero_base_power_creatures_deal_combat_damage"` watch's whole batch of qualifying
  attackers just dealt one defending player (Primo, the Unbounded's `create_token` `enters_with`:
  "Put a number of +1/+1 counters on it equal to the damage dealt"). Only meaningful on that
  ability's own effect; resolved when the ability is placed on the stack (CR 603.10a last-known
  information), same shape as `"triggering_spell_mana_value"` above.
- `"triggering_damage_dealt"` — the amount of damage — combat or noncombat alike — the enchanted
  host of a `timing = "enchanted_creature_deals_damage"` watch just dealt (Armadillo Cloak's
  `gain_life` `amount`: "you gain that much life"). Only meaningful on that ability's own effect;
  resolved when the ability is placed on the stack (CR 603.10a last-known information), same shape
  as `"combat_damage_dealt"` above (which this doesn't reuse: that one is specifically the summed
  *combat* damage a base-power-0 batch dealt *a player*).
- a conditional amount: `{ condition = <Condition (§5)>, then = <Amount> }` — `then` if `condition`
  holds for the effect's controller, else 0 (Mortality Spear's "costs {2} less to cast if you
  gained life this turn": `{ condition = { type = "you_gained_life_this_turn" }, then = 2 }`).
- a kicked-branch amount: `{ if_kicked = <Amount>, else = <Amount> }` (CR 702.33d) — `if_kicked` if
  the resolving spell's kicker cost (`[cost.additional.kicker]`, above) was paid, else `else`
  (Rite of Replication's "create five of those tokens instead": `{ if_kicked = 5, else = 1 }`).
  Reads `Game::spell_was_kicked` off the effect's `source` (the resolving spell itself), the
  kicked-flag sibling of `"spell_sacrifice_count"` above.

A non-negative-count field (draw/mill/token/counter) clamps a negative result to 0.

**Spell filter** (`reduce_spell_cost`/`counter_target_spell`/`schedule_next_cast_trigger`'s
`filter`, a `cast_spell` trigger's `spell_filter`): `"all"`, `"creature"`, `"noncreature"`,
`"targets_a_creature"` (Killian),
`"aura"`, `"instant_or_sorcery"`, `"enchantment"` (an Aura counts, CR 303.4a — Starfield Mystic),
`"artifact_or_enchantment"` (Quandrix Command), `{ has_subtype = ["Aura", "Equipment",
"Vehicle"] }` (matches the card's printed `subtypes` — Sram), `"has_x"` (the cast card's cost
contains `{X}` — `def.cost.x > 0`; Nev/Zimone/Elementalist's Palette), `"instant_or_sorcery_with_x"`
(`"instant_or_sorcery"` AND `"has_x"` combined — no general And-combinator exists, so this is a
single-purpose arm; Unbound Flourishing's copy ability), `"historic"` (CR
702.135a: artifact, legendary, or Saga — Teshar), `"aura_targets_modified_permanent_you_control"`
(an Aura spell whose chosen target is "modified" — CR 701.29 — and controlled by the caster;
Pearl-Ear, Imperial Advisor), `"cast_from_non_hand_zone"` (a spell cast from anywhere other
than the caster's hand — a flashback/escape from a graveyard, an impulse-play from exile, a
command-zone cast; Advanced Reconstruction's level 3 "Spells you cast from anywhere other than
your hand cost {2} less"), or `{ color = "red" }` (the cast spell's own colors, CR 105.1/202.2 —
a multicolored spell matches any of its colors; Balefire Liege's "cast a red spell" /
"cast a white spell"). Only `reduce_spell_cost` reads the cast-from zone; every other
consumer treats a spell as a plain hand cast.

**Permanent filter** (the `filter` on `destroy_all` / `return_all_to_hand` / `each_player_sacrifices`,
and the `<filter>` in a `{ permanent = … }` target): one composable predicate over a battlefield
permanent. Every axis is independent; an unset axis imposes no restriction. Write it as a table or,
for a common type set, a shorthand string:

| Axis | Values | Meaning |
|------|--------|---------|
| `types` | a card-type name or a list — `"creature"`, `"artifact"`, `"enchantment"`, `"planeswalker"`, `"land"`, `"nonland"` (the four nonland types), a two-type union shorthand (`"artifact_or_enchantment"`, `"creature_or_planeswalker"`, `"artifact_or_creature"`), or e.g. `["artifact", "enchantment"]` (their union) | the permanent must have *any* of these types (empty/omitted = any type). An Artifact Creature (a creature with `also = "artifact"`) matches both `"creature"` and `"artifact"`. |
| `subtypes` | array of strings, default `[]` | restrict to permanents carrying any of these printed subtypes (Goldspan Dragon's "Treasures you control" — `["Treasure"]`); empty matches all |
| `controller` | `"any"` (default), `"you"`, `"opponent"` | whose permanents qualify, relative to the effect's controller |
| `token` | `"any"` (default), `"token"`, `"nontoken"` | token-ness restriction |
| `other` | bool (default `false`) | "another permanent" — excludes the filter's own source |
| `enchanted` | bool, optional | `true` requires an attached Aura, `false` requires none (Winds of Rath's "aren't enchanted"); omit to not care |
| `attached_to_creature` | bool, optional | `true` requires the candidate (an Aura) be attached to a creature (Sage's Reverie's "each Aura you control that's attached to a creature", CR 303), `false` requires the opposite; omit to not care. The mirror of `enchanted`, which reads the host side instead |
| `enchanted_by_you` | bool (default `false`) | `true` requires an attached Aura controlled by "you" (the effect's controller) — Eriette's "enchanted by an Aura you control"; narrower than `enchanted`, which counts any attached Aura |
| `mv_max` | u8, optional | mana-value ceiling (Skyclave's "4 or less", Culling Ritual's "2 or less"); omit to not gate on mana value |
| `mv_eq_x` | bool (default `false`) | `true` requires mana value *exactly* the casting spell's chosen `{X}` (Entrancing Melody's "creature with mana value X") — only meaningful on a cast-time target filter |
| `mv_max_x` | bool (default `false`) | `true` requires mana value at most a *triggered* ability's own source's entered `{X}` (Kinetic Ooze's "mana value X or less", X = the counters it entered with) — only meaningful on a triggered ability's own target filter |
| `tapped` | bool, optional | `true` requires the permanent be tapped (Mana Geyser's "tapped land"), `false` untapped; omit to not care (ignored in the graveyard zone) |
| `power_max` | u8, optional | power ceiling (Silverquill Charm's "power 2 or less"); omit to not gate on power |
| `power_parity` | `"even"`/`"odd"`, optional | power parity gate (Zimone's Hypothesis; zero is even); omit to not care |
| `noncreature` | bool (default `false`) | `true` excludes creature-typed permanents ("noncreature artifact" — Haywire Mite) |
| `color` | `"any"` (default), `"monocolored"`, `"white"`/`"blue"`/`"black"`/`"red"`/`"green"` | color-count restriction (Vanishing Verse's "monocolored permanent") or a specific color (Oran-Rief, the Vastwood's "each green creature") |
| `modified` | bool (default `false`) | `true` requires the permanent be "modified" (CR 701.29: has any counter, is enchanted by an Aura, or is equipped — Silkguard's "modified creatures you control gain hexproof") |
| `attacking` | bool (default `false`) | `true` restricts to creatures declared as attackers this combat (Tajic's Mentor — "target attacking creature") |
| `power_less_than_source` | bool (default `false`) | `true` requires power strictly less than the filter's own source permanent's power (Mentor, CR 702.121a "lesser power"); meaningless without a source, so only meaningful on a targeted ability's own target filter |
| `entered_this_turn` | bool (default `false`) | `true` requires the permanent entered the battlefield this turn (Oran-Rief, the Vastwood's "each green creature that entered this turn"); distinct from summoning sickness, which clears one step earlier |
| `nonbasic` | bool (default `false`) | `true` excludes basic lands (CR 205.4a's "Basic" supertype — White Orchid Phantom's "target nonbasic land"); meaningful only alongside `types = "land"` |
| `name` | string, optional | restrict to permanents with this exact printed name (CR 201.2 — Leitmotif Composer's "creatures named Leitmotif Composer can't be blocked"); omit to not gate on name. Printed-name equality only — no pool card changes a permanent's name. |
| `nonlegendary` | bool (default `false`) | `true` excludes legendary permanents (CR 205.4a's "Legendary" supertype — Muddle, the Ever-Changing's "up to one target nonlegendary creature you control") |
| `nonlair` | bool (default `false`) | `true` excludes the "Lair" land subtype (Treva's Ruins' "return a non-Lair land you control"); reads the printed land-type list directly, not `subtypes`; meaningful only alongside `types = "land"` |

Shorthand strings (equivalent type-set tables): `"creatures"`/`"creature"`, `"nonland_permanents"`/`"nonland"`,
`"artifact"`, `"artifact_or_enchantment"`, `"creature_or_planeswalker"`, `"artifact_or_creature"`,
and any single type name. Examples:
`filter = "creatures"`, `filter = { types = "creature", enchanted = false }` (Winds of Rath),
`filter = { types = "nonland", mv_max = 2 }` (Culling Ritual),
`target = { permanent = { types = "nonland", token = "nontoken", controller = "opponent", mv_max = 4 } }`
(Skyclave Apparition). For mass effects, each destroyed permanent goes to its owner's graveyard (a
commander diverts to the command zone); each bounced nontoken permanent goes to *its own owner's*
hand; a token hit by either ceases to exist instead (CR 111.7).

**`each_player_sacrifices` `scope =`:** `"all_players"` (default; Deadly Brew, Promise of
Loyalty), `"each_opponent"` (Witch of the Moors, Lorehold Charm), or `"targeted_players"` (Priest
of Forgotten Gods' "any number of target players" — a controller-chosen subset of living players,
possibly empty or including the controller themselves, in place of a scope-derived set). Its
`filter` is the same permanent filter (default `"creature"`).

**Card filter** (`search_library`/`look_at_top`/`reveal_top_to_hand`/`mass_return_from_graveyard`/
`may_return_from_graveyard`'s `filter`, and the `card_in_graveyard` target's): which *card* (in a
library or graveyard) matches.
One of: `"basic_land"` (a land with the `basic` flag set — §3), `"land"` (any land), `"nonland"`
(the inverse of `"land"` — a creature, artifact, enchantment, planeswalker, instant, or sorcery
card; Creative Technique's "reveal cards from the top of it until you reveal a nonland card"),
`"creature"`, `"any_card"`, `"instant_or_sorcery"` (Mystic Sanctuary), `"enchantment"` (an Aura counts —
Replenish), `"permanent"` (any card with a permanent type, no mana-value bound — Deadly Brew's
"another permanent card"), `"noncreature_nonland"` (neither a creature nor a land — Quintorius,
Loremaster's "target noncreature, nonland card"), `{ land_with_subtype = [...] }` (a land whose
`subtypes` intersect the given list — Nature's Lore's "a Forest card", which also matches a
nonbasic Forest-typed dual like Tangled Islet), `{ basic_land_with_subtype = [...] }` (the same
subtype intersection, gated on the `basic` flag too — Archaeomancer's Map's "a basic Plains
card", which a nonbasic Plains-typed dual like Eclipsed Steppe does NOT match), or one of the
MV-gated tables:
`{ permanent_with_mana_value_at_most = 3 }`
(Sevinne's Reclamation), `{ nonland_permanent_with_mana_value_at_most = 3 }` (Sun Titan),
`{ artifact_or_creature_with_mana_value_at_most = 2 }` (Lorehold Charm),
`{ artifact_creature_or_non_aura_enchantment_with_mana_value_at_most = 3 }` (Excava),
`{ creature_with_mana_value_at_most = 3 }` (Teshar),
`"creature_with_mana_value_at_most_combat_damage"` — a `card_in_graveyard`-only placeholder
resolved to `creature_with_mana_value_at_most` bounded by the amount of combat damage the
`deals_combat_damage_to_player` trigger's source just dealt (CR 510.2/603.10a last-known
information; Venerable Warsinger's "mana value X or less … where X is the amount of damage this
creature dealt to that player"). Never matched live — baked at trigger placement.
`"nonland_permanent_with_mana_value_at_most_source_power"` — the same-shape `card_in_graveyard`-only
placeholder for an `attacks` trigger, resolved to `nonland_permanent_with_mana_value_at_most`
bounded by the attacking source's power (CR 510.2/603.10a; Guardian Scalelord's "return target
nonland permanent card with mana value X or less … where X is this creature's power"). Never
matched live — baked at trigger placement. Or
`"aura_or_equipment"` (a card whose printed `subtypes` include "Aura" or "Equipment" — Armored
Skyhunter's "an Aura or Equipment card from among them"), `look_at_top`-only today, `"aura"`
(a card whose printed `subtypes` include "Aura", no Equipment — `exile_top_cast_matching_free`'s
Herald of Amity: "cast an Aura spell from among them"), or `"artifact_or_creature"` (an artifact or
creature card, no mana-value bound — Restore Relic's "target artifact or creature card from your
graveyard"; the unbounded twin of `{ artifact_or_creature_with_mana_value_at_most = N }`).

**`search_library` `to_zone =`:** `"hand"` (tutors) or `"battlefield"` (ramp/fetchlands;
`tapped = true` to enter tapped). The searcher picks up to `count` matches (default `1`; or
none — "fail to find" is always legal at any point and ends the search), each moved to `to_zone`
as it's found; the library is shuffled once, after the last pick (CR 701.19f — Land Tax's "up to
three basics" is `count = 3`). `searcher = "target_controller"` hands the search to the ability's
shared target's controller instead (Path to Exile's/Assassin's Trophy's basic-land compensation).
In this model a search reveals nothing to opponents.

**`search_library` `overflow =`:** a second destination (§7) for every find *after the first* —
`to_zone` still receives the first pick, `overflow` receives every later one (Cultivate: "put one
onto the battlefield tapped and the other into your hand" is `to_zone = "battlefield", tapped =
true, overflow = "hand"`, `count = 2`). Omit for the common single-destination search (every find
goes to `to_zone`). `tapped` applies only to a `Battlefield` destination — a find routed to
`overflow = "hand"` has no tapped concept. ponytail: only first-vs-rest is modeled; a real
per-pick destination list is the generalization a third destination would need.

```toml
# Diabolic Tutor: search for any card, put it into your hand, then shuffle.
[[abilities.effects]]
type = "search_library"
filter = "any_card"
to_zone = "hand"
```
```toml
# Terramorphic Expanse / Prismatic Vista — a fetchland: "{T}, [Pay 1 life,] Sacrifice this:
# Search your library for a basic land card, put it onto the battlefield [tapped], then shuffle."
[[abilities]]
timing = "activated"
taps_self = true
pay_life = 1          # omit or 0 for a no-life fetch (Terramorphic/Fabled Passage)
sacrifice = "this"

[[abilities.effects]]
type = "search_library"
filter = "basic_land"
to_zone = "battlefield"
tapped = false        # true to enter tapped (Rampant Growth, Terramorphic, Fabled Passage)
```
```toml
# Nature's Lore: "Search your library for a Forest card, put it onto the battlefield, then
# shuffle." — matches the basic Forest and any nonbasic Forest-typed land.
[[abilities.effects]]
type = "search_library"
filter = { land_with_subtype = ["Forest"] }
to_zone = "battlefield"
tapped = false
```
```toml
# Archaeomancer's Map: "search your library for up to two basic Plains cards, reveal them, put
# them into your hand, then shuffle." — the Basic supertype excludes a nonbasic Plains-typed dual.
[[abilities.effects]]
type = "search_library"
filter = { basic_land_with_subtype = ["Plains"] }
to_zone = "hand"
count = 2
```
```toml
# Land Tax: "search your library for up to three basic land cards ... put them into your hand,
# then shuffle." One search, up to three picks, a single shuffle after the last one.
[[abilities.effects]]
type = "search_library"
filter = "basic_land"
to_zone = "hand"
count = 3
```
```toml
# Cultivate: "search your library for up to two basic land cards ... put one onto the
# battlefield tapped and the other into your hand, then shuffle." One search, two destinations,
# one shuffle after the last pick.
[[abilities.effects]]
type = "search_library"
filter = "basic_land"
count = 2
to_zone = "battlefield"
tapped = true
overflow = "hand"
```

```toml
# Open the Way: "Reveal cards from the top of your library until you reveal X land cards. Put
# those land cards onto the battlefield tapped and the rest on the bottom of your library in a
# random order." Non-matching reveals go to the bottom in library order (deterministic, no rand).
[[abilities.effects]]
type = "reveal_until"
filter = "land"
count = "x"
matched_dest = "battlefield"
matched_tapped = true
rest_dest = "bottom"
```

## 8. Mana symbols and tokens

**Mana symbol** (for `produces` and `add_mana`): `"white"`, `"blue"`, `"black"`, `"red"`,
`"green"`, `"colorless"`, `"any"`, or a **2-to-4-color array** for a fixed choice among those
colors — one credit that picks its color at payment time, no choice on tap (§3). Exactly two
(`["green", "blue"]`) is a dual ("either of two colors"); three or four (`["green", "white",
"blue"]` — Treva's Ruins' "{G}, {W}, or {U}") is a triome-style choice. Write the colors in the
card's printed order — a 2-color array is normalized to WUBRG internally (a 3-/4-color array
just needs its colors distinct; order doesn't matter, it collapses to a bitmask).

**Multi-mana** — `add_mana` takes a list, one symbol per mana produced:
```toml
[[abilities.effects]]
type = "add_mana"
mana = ["colorless", "colorless"]   # Sol Ring: {C}{C}
```
Add `repeat` (an amount, §7) to scale the batch — e.g. Mana Geyser's "{R} per tapped land your
opponents control":
```toml
type = "add_mana"
mana = ["red"]
repeat = { per_permanent = { types = "land", controller = "opponent", tapped = true } }
```
A list of *dual* symbols is a two-mana filter output — Fetid Heath's "{W/B},{T}: Add {W}{W},
{W}{B}, or {B}{B}" is two `["white","black"]` credits for a `{1}` activation cost (§3):
```toml
[[abilities]]
timing = "activated"
taps_self = true
activation_cost = { generic = 1 }   # {W/B} hybrid has no exact spelling; {1} is the closest fit

[[abilities.effects]]
type = "add_mana"
mana = [["white", "black"], ["white", "black"]]
```

**Spend restriction** (`restriction` on `add_mana`/`grant_mana_ability`) — CR 106.9's "spend
this mana only to…", wrapping *every* credit the batch produces this resolution (not per-symbol).
One of:
- `"instant_or_sorcery"` — only to cast an instant or sorcery spell (Galazeth Prismari's granted
  artifact mana).
- `{ mana_value_at_least_or_has_x = N }` — only to cast a spell with mana value `N` or greater,
  *or* any spell with `{X}` in its printed mana cost regardless of mana value (Troyan, Gutsy
  Explorer: `N = 5`).
- `"has_x"` — only on costs that contain `{X}` (Elementalist's Palette).

A credit under a restriction the spell being cast doesn't satisfy simply can't be spent on it —
it isn't an error, the payment planner just looks past it to other mana:
```toml
# Troyan, Gutsy Explorer: "{T}: Add {G}{U}. Spend this mana only to cast spells with mana value
# 5 or greater or spells with {X} in their mana costs."
[[abilities.effects]]
type = "add_mana"
mana = ["green", "blue"]
restriction = { mana_value_at_least_or_has_x = 5 }
```
Ability activation costs never satisfy a restriction (CR 106.9's restrictions name a *spell*) —
a restricted credit floating in the pool can't fund one, even one that would otherwise fit.

**Token profile** (`[abilities.effects.token]` for `create_token`): either the creature-token
*sugar* — `name` + base `power`/`toughness` + evergreen `keywords`, plus the token's stated
`colors`/`subtypes` (CR 105.2a/111.4 — a token has no mana cost to derive color from) — or, when
a token must be an artifact or carry an ability, a **full inline card table** (its own
`[…token.kind]`, `[[…token.abilities]]`, …), the same shape a top-level card takes.
```toml
[[abilities.effects]]
type = "create_token"
count = 1   # count is an amount (§7): a number, "x", or a derived count like
            # { per_permanent = { types = "creature", controller = "you" }, zone = "graveyard" } (Izoni)

[abilities.effects.token]
name = "Elephant"
power = 3
toughness = 3
keywords = ["trample"]   # optional, evergreen keywords only
colors = ["green"]       # optional stated color(s)
subtypes = ["Elephant"]  # optional stated creature type(s)
```
Copy-tokens aren't spelled via this shape (use `create_token_copy`). A token profile may be a
non-creature — including an **Aura** (`[…token.kind] type = "aura"`, with an `enchant = { … }`
restriction and its own `[[…token.abilities]]`) minted attached to a target via
`attach_minted_aura_to_target` (Scriv, the Obligator's Contract token).

**`controller`** (default `"you"`): who ends up controlling the minted token(s).
- `"you"` — the ability's own controller (CR 111.4's default; most `create_token` cards).
- `"target_controller"` — the controller of the effect's shared target, read at the point this
  step resolves (Beast Within's "its controller creates a 3/3 Beast" — pair with a preceding
  targeted step in an `effects = [...]` sequence, §"effect-sequencing", so both steps share one
  target):
  ```toml
  effects = [
      { type = "destroy_target", target = { permanent = {} } },
      { type = "create_token", controller = "target_controller",
        token = { name = "Beast", power = 3, toughness = 3 } },
  ]
  ```
- `"each_opponent"` — one copy of the token per opponent of the ability's controller, each under
  that opponent (a hostile edict; no pool card yet).
- `"one_per_opponent"` — one copy of the token per opponent of the ability's controller, but every
  copy stays under the ability's own controller (Eccentric Pestfinder's Turn Stones, "For each
  opponent, you create a 1/1 ... Pest ..."). Distinct from `"each_opponent"` above, which hands a
  token to each opponent instead. CR 111.4.
- `"target_player"` — the ability's own chosen Player target (CR 111.4 — Shadrix Silverquill's
  begin-combat "Target player creates a 2/1 white and black Inkling creature token with flying").
  Unlike every other `controller` value, this makes the whole `create_token` step itself
  Player-targeted (`Effect::target` reports `TargetSpec::Player`) rather than reading a target
  shared with a preceding step:
  ```toml
  [[abilities.effects]]
  type = "create_token"
  controller = "target_player"
  [abilities.effects.token]
  name = "Inkling"
  power = 2
  toughness = 1
  ```
- `"target_opponent"` — the same shape as `"target_player"` above, restricted to an opponent (CR
  "target opponent" — Questing Phelddagrif's "Target opponent creates a 1/1 ... Hippo ..."):
  `Effect::target` reports `TargetSpec::OpponentPlayer` instead of `TargetSpec::Player`, same
  `Target::Player` resolution otherwise. Pair with a no-target step (e.g.
  `pump_self_until_end_of_turn`, not `pump_until_end_of_turn target = "this"` — see that entry's
  note) in the same `effects` sequence so the whole ability's one shared target stays the chosen
  opponent.

**Treasure tokens** are engine-provided (`engine::treasure_token`: a colorless artifact token with
"{T}, Sacrifice this artifact: Add one mana of any color"). Make them with `create_treasure`
(§6) — no token profile needed:
```toml
[[abilities.effects]]
type = "create_treasure"
count = 2
```

## 9. Aura/Equipment statics (`grant_to_attached`, `set_attached_base_p_t`, `set_attached_types`, `control_attached`)

An Aura or Equipment's continuous effect on its host is a `timing = "static"` ability read during
characteristic recompute (ADR 0003 — additive layers, not full CR 613):

- **`grant_to_attached`**: adds `power`/`toughness` and `keywords` to the host. `power`/
  `toughness` are an [`Amount`] (a bare int like `4` parses as `Amount::Fixed(4)`), so a grant can
  scale off a live board count instead of a flat number (Sage's Reverie's "+1/+1 for each Aura you
  control that's attached to a creature") — resolved fresh on every characteristic recompute with
  the attached Aura/Equipment as source and its controller as "you", never cached. Keywords can
  include parametrized ones (`{ ward = 2 }`), so an Aura can grant ward or indestructible.
  `goad = true` additionally goads the host for as long as the Aura stays attached (the Impetus
  cycle) — continuous, unlike `goad_target`'s until-your-next-turn one-shot.
  `protection_from_chosen_color = true` grants the host protection from **this Aura's own
  `chosen_color`** (Flickering Ward's "Enchanted creature has protection from the chosen color") —
  a runtime scope read live off `Permanent::chosen_color` (set by the Aura's `choose_color` ETB),
  which can't ride the static `keywords` slice; grants nothing until a color is chosen.
  `granted_ability` (default none) grants the host an *activated* ability beyond statics/keywords
  (Fallen Ideal's "Sacrifice a creature: This creature gets +2/+1 until end of turn.") — a sub-table
  `{ cost = <activation-cost table: sacrifice, taps_self, mana, …>, effects = [<effect>, …] }`. The
  non-mana twin of `grant_mana_ability`: surfaced on the host past its own abilities and activated
  exactly like one, read live off the attachment scan so it disappears the instant the Aura leaves.
  The effects resolve against the host as their source (so `pump_self_until_end_of_turn` pumps the
  host); a one-effect grant runs directly, multiple as a `Sequence`.
  `cant_attack`/`cant_block` (bools, default `false`) are the Pacifism-family "enchanted
  permanent/creature can't attack or block" restrictions (Faith's Fetters, Prison Term) — continuous
  grants read live off the attachment scan, the reverse of `goad`'s "must attack"; they vanish the
  instant the Aura leaves, just like `goad`. `activated_abilities` (string, default unset — `"none"`
  or `"mana_only"`) bans the host's own activated abilities while attached: `"none"` (Prison Term)
  bans all of them, `"mana_only"` (Faith's Fetters) exempts a mana ability (CR 605.3a) but bans
  everything else; read live in `ability_activation_gate`, so it lifts the instant the Aura leaves.
- **`set_attached_base_p_t`**: overrides the host's *base* power/toughness to a fixed value
  (Darksteel Mutation's "base power and toughness 0/1"); counters/pumps/anthems still add on top.
  Only one set-base effect is ever active on a given creature in the pool, so there's no
  layer-7b "last one wins" ordering. This is a fixed printed value, not "until end of turn" and
  not "equal to another creature's power and toughness" — a spell-timed or dynamic base-P/T-set
  effect (Quandrix Charm's third mode, Tanazir Quandrix's attack trigger) still isn't expressible.
- **`set_attached_types`**: adds card types and/or changes the host's creature subtypes while
  attached (CR 613.4 type/subtype layer, read live at the type/subtype match chokes so it reverts
  the instant the Aura leaves). `add_types` (a type set) unions card types onto the host (Darksteel
  Mutation → Insect *artifact* creature); `add_subtypes` unions creature subtypes on (Angelic
  Destiny's "is an Angel in addition to its other types"); `set_subtypes`, when non-empty,
  *replaces* the host's own creature subtypes (Darksteel Mutation's "is an Insect", dropping the
  host's printed types). `lose_all_abilities` (bool, default false — CR 613.1e/701) strips the
  host's *own* printed abilities and keyword abilities while attached (Darksteel Mutation's "it
  loses all other abilities": its flying, its activated/triggered abilities, its static abilities
  all stop functioning). The Aura's *own* grants (this same type change, its `set_attached_base_p_t`,
  its `grant_to_attached` keywords) are unaffected — they sit after the removal in CR 613 order.
  Only one type-changing Aura per host in the pool, so there's no CR 613.7 layer ordering. Ability
  *addition* / "loses all abilities except …" / partial removal is still not expressible — flag it.
- **`control_attached`**: the Aura's controller controls the host for as long as it stays
  attached (CR 720), reverting automatically when the Aura leaves. The one-shot siblings are
  `gain_control_until_end_of_turn` (§6 — Besmirch), which reverts at cleanup, and `gain_control`
  (§6 — Entrancing Melody), which doesn't; leaves-the-battlefield-triggered
  reanimation-under-your-control is still inexpressible (Changing Loyalty is approximated as a
  continuous control change instead of its real death-trigger reanimation).

## 10. NOT expressible / unsupported — flag these cards

- **Phyrexian mana** (`{W/P}`), snow, `{X}` in activated-ability costs. (Hybrid IS supported —
  §2's `hybrid`.)
- **Layer-system interactions** beyond §9's additive set: no CDA `*/*`, no "becomes a copy of
  target creature", no dynamic/until-end-of-turn base-P/T set.
- **Keywords** not in §4: kicker, cascade, adventure, etc. `ward` is a single fixed value (no
  "ward equal to X"); `protection` is a single fixed color or quality (creatures/multicolored —
  no "protection from everything", no "choose a color/type as this enters"). A keyword grant is
  *while-attached* (`grant_to_attached`, §9), *until end of turn* (the pump effects' `keywords`,
  §6), a continuous anthem
  (`anthem_static`'s `keywords`, §6), or condition-gated on the card itself
  (`conditional_keywords`, §1) — no "gains a keyword for the rest of the game" grant.
- **Modal *triggered* abilities** are supported only for *non-targeting* modes (`choose_one`,
  §6 — Atsushi); a triggered mode that needs a freshly chosen target isn't. Modal spells take
  `choose`/`choose_max` (§1), but there are no per-mode riders (entwine, escalate).
- **Day/night, monarch, and initiative** are still out. (Permanent one-shot control change IS
  expressible — `gain_control`, §6 — Entrancing Melody.)
- Three-or-more-color "or" producers (a triome's tap) — `produces` duals stop at two colors
  (`"commander_identity"`/`"opponent_colors"` are the only wider credits, §3). Painlands, filter
  lands, and `{1},{T}` two-mana lands ARE expressible (omit `produces`, use `add_mana` abilities
  with `self_damage` / a `hybrid` `activation_cost` / a multi-symbol batch — §3, §6). Tokens —
  including artifact tokens and tokens with abilities — ARE expressible via the full-form token
  profile (§8).
- **Filter axes** still missing from the permanent filter (§7): a power *floor* ("power 4 or
  greater") and any toughness axis. Type, subtype, controller, token-ness, "another",
  enchanted-ness, mana-value ceiling, power ceiling, power parity, specific color, and
  entered-this-turn are all expressible.
- **Delayed triggered abilities** beyond "at the beginning of the next upkeep"
  (`schedule_at_next_upkeep`, §6) — no general "at the beginning of your next end step / in two
  turns" scheduling.

When a card needs any of the above, script only the faithful core with a `# ponytail:`
note naming what was dropped — or flag it as not-yet-expressible. Do not invent tags.

## 9. Class enchantments (CR 717)

A **Class** is an enchantment (`[kind] type = "enchantment"`, `subtypes = ["Class"]`) with a
level counter that starts at 1. Each printed "Level N" is a **separate `[[abilities]]`** activated
ability — `timing = "activated"`, `sorcery_speed = true`, its mana cost, and a single
`{ type = "level_up", level = N }` effect. The engine offers each level-up only while the source
is at `level - 1`, so levels are gained one at a time, exactly once. The level-gated
triggered/static/activated abilities carry `min_level = N` (§5) and function only at that level or
higher. The always-on base ability (level 1) carries no `min_level`.

```toml
# Level 1 (base): "When this Class enters, create a 2/1 … Inkling …"
[[abilities]]
timing = "etb"

[[abilities.effects]]
type = "create_token"
count = 1
[abilities.effects.token]
name = "Inkling"
power = 2
toughness = 1

# {1}{B}: Level 2
[[abilities]]
timing = "activated"
sorcery_speed = true

[abilities.activation_cost]
generic = 1
black = 1

[[abilities.effects]]
type = "level_up"
level = 2

# Whenever you lose life for the first time each turn, put a +1/+1 counter on target creature you control.
[[abilities]]
timing = "you_lose_life_first_time_each_turn"
min_level = 2

[[abilities.effects]]
type = "put_counters"
count = 1
target = "creature_you_control"
```

A level whose effect the DSL can't yet express is dropped (its "Level N" activated ability is
simply not authored, so the Class can never reach it) and named in `approximates`.
