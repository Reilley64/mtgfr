//! Zone objects: the per-object runtime state a card takes on as it changes zones.
//!
//! Split out of the old `types::card` when the card-DSL vocabulary moved to the `cards`
//! crate: what stayed here is the state machine's own object model ([`Card`], [`Spell`],
//! [`Permanent`], [`Object`], [`Player`]) plus the constructors that mint it from a
//! [`CardDef`].

use crate::*;

/// A permanent entering the battlefield: all per-object state at its defaults.
pub(crate) fn fresh_permanent(
    def: CardId,
    owner: PlayerId,
    summoning_sick: bool,
    commander: bool,
) -> Permanent {
    let printed = card_def(def);
    Permanent {
        def,
        owner,
        level: 1,
        tapped: printed.enters_tapped,
        summoning_sick,
        entered_this_turn: true,
        started_turn_untapped: false,
        attacked_this_turn: false,
        attacked_on_last_own_turn: false,
        monstrous: false,
        plus_counters: 0,
        kind_counters: [0; CounterKind::COUNT],
        text_swap: None,
        attachment_lost_keywords: &[],
        set_base_pt: None,
        set_base_pt_timestamp: 0,
        added_types: TypeSet::NONE,
        added_types_timestamp: 0,
        added_subtypes: &[],
        subtypes_set_while_source_remains: None,
        granted_keywords: &[],
        marked_damage: 0,
        deathtouched: false,
        commander,
        token: false,
        attached_to: None,
        continuous_timestamp: 0,
        loyalty: starting_loyalty(&printed),
        loyalty_activated: false,
        finality_counter: false,
        cant_be_regenerated_this_turn: false,
        exile_instead_of_dying_this_turn: false,
        regeneration_shields: 0,
        prepared: false,
        echo_unpaid: printed.echo.is_some(),
        chosen_subtype: None,
        chosen_color: None,
        chosen_opponent: None,
        entered_with_x: 0,
        entered_multikicker_count: 0,
        cast_time_enchant_target: None,
        linked_twin: None,
        enchant_rewrite_host: None,
        vow_protected: None,
        phased_out: false,
        serra_recursion: false,
        bestowed: false,
        face_down: false,
        flipped: false,
        masked: false,
        evoked: false,
        spent_colors: [false; Color::COUNT],
        cast_from_hand: false,
        copy_rider_keywords: &[],
    }
}

/// A planeswalker's printed starting loyalty (CR 606.5b — it enters with that many loyalty
/// counters); 0 for any other card kind.
pub(crate) fn starting_loyalty(def: &CardDef) -> i32 {
    match def.kind {
        CardKind::Planeswalker { loyalty } => loyalty,
        // Battles store starting defense in the same `Permanent::loyalty` slot.
        CardKind::Battle { defense } => defense,
        _ => 0,
    }
}

/// A token entering the battlefield: like [`fresh_permanent`], but flagged as a token
/// (ceases to exist when it leaves the battlefield) and summoning-sick.
pub(crate) fn fresh_token(def: CardId, controller: PlayerId) -> Permanent {
    Permanent {
        token: true,
        ..fresh_permanent(def, controller, true, false)
    }
}
/// A card at rest in a hidden/graveyard/command zone: identity only, no battlefield state.
#[derive(Debug, Clone)]
pub(crate) struct Card {
    pub(crate) def: CardId,
    pub(crate) owner: PlayerId,
    /// One of Library / Hand / Graveyard / Exile / Command.
    pub(crate) zone: Zone,
    /// Whether this is (a form of) its owner's commander — carried across zone changes.
    pub(crate) commander: bool,
    /// Whether this exiled card is face down and hidden from every viewer but `owner` (CR
    /// 701.9 — Abstract Performance's first pile: exiled face-down while its opponent-chooser
    /// pauses on which pile to take). `false` for every ordinarily-visible card in a
    /// hidden/graveyard/command/exile zone. A fresh object is minted on every zone change (see
    /// [`Object::Moved`]'s doc), so this clears for free the moment the card leaves the pile —
    /// there is no separate "reveal" event.
    pub(crate) face_down: bool,
}

/// A spell on the stack (a cast card waiting to resolve).
#[derive(Debug, Clone)]
pub(crate) struct Spell {
    pub(crate) def: CardId,
    pub(crate) controller: PlayerId,
    /// The chosen targets (CR 601.2c). A single-target spell fills one slot; Aether Gale fills up
    /// to six. Empty until a multi-target spell's targets are chosen (see [`Event::SpellTargetsChosen`]).
    /// This is the spell's *first* independent target clause (clause 0); [`Self::targets_second`]
    /// holds a second one.
    pub(crate) targets: TargetList,
    /// A *second* independent target clause's chosen targets (CR 601.2c — Magma Opus's "Tap two
    /// target permanents" alongside its divided-damage clause). Filled by a second
    /// [`Event::SpellTargetsChosen`] with `clause == 1`; empty for the single-clause majority.
    /// ponytail: exactly two independent target clauses per spell — clause 0 in `targets`, clause 1
    /// here. No pool spell prints three; add a `[TargetList; N]` array (with `MAX_TARGET_CLAUSES`)
    /// if one ever does.
    pub(crate) targets_second: TargetList,
    pub(crate) commander: bool,
    /// The chosen `{X}` value, read by X-scaled effects at resolution (0 if the spell has no `{X}`).
    pub(crate) x: u32,
    /// The color named by a resolution-time [`Effect::Choice(ChoiceEffect::ChooseColor)`] step
    /// earlier in this spell's own effect `Sequence` (Bathe in Light's "Choose a color. Target
    /// creature ... gain protection from the chosen color"). The permanent-side twin is
    /// [`Permanent::chosen_color`] — a spell isn't a permanent while resolving, so it needs its
    /// own slot rather than sharing that one. `None` until the choice is answered, and for every
    /// spell without such a choice.
    pub(crate) chosen_color: Option<Color>,
    /// A CR 613.3c layer-5 color SET on this spell, granted by the copy effect that minted it
    /// (Fork's "except that the copy is red"). *Replaces* the copiable color derived from the
    /// card's pips rather than unioning with it, exactly like the permanent-side registered
    /// `ModifierKind::SetColor` — see [`Game::colors_of`]. `None` for every cast spell and
    /// for a copy from an effect that doesn't recolor (Twincast).
    pub(crate) set_color: Option<Color>,
    /// A CR 612.1 text change made to this spell while it is on the stack ("change the text of
    /// target spell or permanent …" — Magical Hack, Sleight of Mind). The permanent-side twin is
    /// [`Permanent::text_swap`]; a spell isn't a permanent, so it needs its own slot for the same
    /// reason `chosen_color` above does. `None` for every untouched spell.
    pub(crate) text_swap: Option<TextSwap>,
    /// A modal spell's chosen modes (CR 700.2), each with its own target. An empty selection for
    /// a non-modal spell (which uses `target` and runs every effect).
    pub(crate) modes: Modes,
    /// Whether this spell is a *copy* (CR 707.10) rather than a cast card: it was put on the
    /// stack by a copy effect (Twincast), pays no cost, and ceases to exist when it resolves
    /// instead of going to a graveyard.
    pub(crate) copy: bool,
    /// Whether this spell was cast with flashback (CR 702.34): from the graveyard for its
    /// flashback cost. When set, the resolved spell is exiled instead of moved to the graveyard
    /// (CR 702.34e). A copy of a flashback spell inherits the flag but ceases to exist first, so
    /// it never reaches the exile branch.
    pub(crate) flashback: bool,
    /// Whether this spell was cast via escape (CR 702.19): from the graveyard for its escape
    /// cost, exiling other graveyard cards as an additional cost. Mirrors [`Self::flashback`]'s
    /// exile-on-resolve treatment for a noncreature/nonland escape spell (CR 702.19d); a creature
    /// or Aura escape spell instead becomes a permanent and never reaches that branch.
    pub(crate) escape: bool,
    /// Whether this spell was cast from its controller's hand (CR 601's default cast zone) —
    /// `false` for a flashback/escape/retrace cast from a graveyard, a commander cast from the
    /// command zone, or an impulse-play permission cast from exile. Feeds
    /// [`Trigger::CastSpell`]'s `from_hand` gate (Dirgur Focusmage's "you cast … from your
    /// hand"); read at [`Event::SpellCast`] apply time off the source card's zone before it
    /// moves to the stack (see `apply.rs`).
    pub(crate) cast_from_hand: bool,
    /// Whether this spell was cast during its controller's own precombat or postcombat main phase
    /// (CR 505.1a/505.1b) — Sulfurous Blast's "If you cast this spell during your main phase..."
    /// rider, Return to Dust's optional second target. Computed the way `cast_from_hand` is: read
    /// off ambient game state (active player/step) at [`Event::SpellCast`] apply time, not a
    /// player-declared cost (unlike `kicked`/`multikicker_count` below, so no wire field needed).
    pub(crate) cast_during_main_phase: bool,
    /// CR 601.2d's damage division for a `divided: true` `Effect::Damage(DamageEffect::Target)` on this spell
    /// (Magma Opus's "4 damage divided as you choose"): `(target, assigned amount)` pairs,
    /// settled right after `targets` above by [`Game::maybe_begin_damage_division`]. Empty for a
    /// spell with no divided-damage effect. Reuses [`DamageAssignment`], the same `Copy`
    /// division shape combat's [`Event::CombatDamageDivided`] uses (CR 510.1c) — a divided
    /// spell's targets are always permanents (see [`Effect::Damage(DamageEffect::Target)`]'s doc), so the same
    /// `ObjectId`-keyed shape fits without a parallel type.
    pub(crate) damage_division: DamageAssignment,
    /// CR 601.2d's *player* shares of a `divided: true` `Effect::Damage(DamageEffect::Target)`'s division (Magma
    /// Opus's "any number of targets" includes players): `(player, assigned amount)` pairs,
    /// settled alongside [`Self::damage_division`] by [`Game::maybe_begin_damage_division`]. A
    /// separate fixed `Copy` array (not `DamageAssignment`, which is `ObjectId`-keyed and shared
    /// with combat — a player isn't an object) so `Spell` stays `Copy`; `[None; MAX_TARGETS]` for a
    /// spell with no player among its divided targets.
    pub(crate) damage_division_players: [Option<(PlayerId, i32)>; MAX_TARGETS],
    /// CR 601.2d's counter division for a `divided: true` `Effect::Counters(CountersEffect::PutCounters)` on this spell
    /// (Grove's Bounty's "Distribute X +1/+1 counters among any number of target creatures you
    /// control"): `(target, assigned count)` pairs, settled right after `targets` above by
    /// [`Game::maybe_begin_counter_division`]. Empty for a spell with no divided-counters effect.
    /// Reuses [`DamageAssignment`], the same `Copy` division shape [`Self::damage_division`] uses
    /// — a divided spell's targets are always permanents, so the same `ObjectId`-keyed shape fits.
    pub(crate) counter_division: DamageAssignment,
    /// How many permanents were sacrificed to pay [`AdditionalCost::sacrifice`] (CR 601.2f —
    /// Plumb the Forbidden's "you may sacrifice one or more creatures"), 0 if the spell has no
    /// such cost or the caster declined. Read by a copy-per-sacrifice rider once one exists (no
    /// pool card reads it yet); recorded here the way `x` is, for the same reason.
    pub(crate) sacrifice_count: u8,
    /// The total mana value of the permanents counted by [`Self::sacrifice_count`] (Sacrifice's
    /// "Add an amount of {B} equal to the sacrificed creature's mana value"), 0 when nothing was
    /// sacrificed. Recorded here because the fodder is a graveyard card by the time the spell
    /// resolves, exactly like [`Self::revealed_creature_mana_value`] below. Read by
    /// [`Amount::SpellSacrificedManaValue`] via [`Game::spell_sacrificed_mana_value`].
    /// ponytail: a *total*, because a spell that eats several (Plumb the Forbidden) would
    /// otherwise need an arbitrary pick; the one card that reads it eats exactly one, so the
    /// total and "the sacrificed creature's" agree.
    pub(crate) sacrificed_mana_value: u8,
    /// The mana value of the creature card revealed to pay this spell's
    /// [`AdditionalCost::reveal_creature_from_hand`] (CR 601.2g — Disaster Radius's "reveal a
    /// creature card from your hand"), 0 if the spell has no such cost. Chosen automatically at
    /// cast time as the highest-mana-value creature card in the caster's hand (see the field's
    /// own doc); read by [`Amount::RevealedCreatureManaValue`] via
    /// [`Game::revealed_creature_mana_value`], the reveal-cost sibling of
    /// [`Self::sacrifice_count`]'s read.
    pub(crate) revealed_creature_mana_value: u8,
    /// Whether the caster paid this spell's kicker cost (CR 702.33d — [`AdditionalCost::kicker`]),
    /// `false` for a spell with no kicker or a decline. Read by [`Condition::SpellWasKicked`] (Rite
    /// of Replication's "If this spell was kicked, create five of those tokens instead") via
    /// [`Game::spell_was_kicked`], the kicked-flag sibling of [`Self::sacrifice_count`]'s read.
    pub(crate) kicked: bool,
    /// Whether the caster paid this spell's buyback cost (CR 702.27c — [`AdditionalCost::buyback`]),
    /// `false` for a spell with no buyback or a decline. Read by
    /// [`Game::finish_instant_sorcery_resolution`] (Capsize's "put this card into your hand as it
    /// resolves" instead of the graveyard), the buyback-flag sibling of [`Self::kicked`]'s read.
    pub(crate) bought_back: bool,
    /// The caster's declared Strive target count (CR 702.42 — [`AdditionalCost::strive`]), 0 if
    /// the spell has no Strive cost. Settled before the spell hits the stack (CR 601.2c precedes
    /// 601.2f) and recorded here the way `sacrifice_count`/`kicked` are; read back by
    /// [`TargetCount::strive_scaled`]'s cast-time target-count substitution in
    /// [`Game::choose_spell_targets`](crate::Game::choose_spell_targets).
    pub(crate) strive_count: u8,
    /// How many times the caster paid this spell's Replicate cost (CR 702.108 —
    /// [`AdditionalCost::replicate`]), 0 if the spell has no Replicate cost or the caster paid it
    /// zero times. Settled before the spell hits the stack (CR 601.2b) and recorded here the way
    /// `strive_count` is; read at the [`Event::SpellCast`] choke to mint that many copies via
    /// [`Game::mint_spell_copies`] (CR 702.108b).
    pub(crate) replicate_count: u8,
    /// How many times the caster paid this spell's Multikicker cost (CR 702.33c —
    /// [`AdditionalCost::multikicker`]), 0 if the spell has no Multikicker cost or the caster
    /// paid it zero times. Settled before the spell hits the stack the way `replicate_count` is;
    /// read by [`Game::spell_multikicker_count`] (an [`Amount::SpellMultikickerCount`] read, and
    /// [`TargetCount::multikicker_scaled`]'s cast-time target-count substitution).
    pub(crate) multikicker_count: u8,
    /// Whether this spell was cast from a graveyard under Serra Paragon's permission (CR 118.9 —
    /// [`Effect::Static(StaticEffect::PlayFromGraveyardOncePerTurn)`]). Copied onto the resulting
    /// [`Permanent::serra_recursion`] when the spell resolves ([`Event::PermanentEntered`]), so the
    /// recurred permanent carries the granted "exile-and-gain-2-life" rider. `false` for any other
    /// cast (from hand, flashback, escape, …).
    pub(crate) serra_recursion: bool,
    /// Whether this spell was cast via bestow (CR 702.103 — Eidolon of Countless Battles): for its
    /// [`CardDef::bestow`] cost, as an Aura spell with enchant creature. When set, the spell
    /// resolves through the Aura attach path ([`Game::resolve_spell`]) rather than entering as a
    /// creature, and the resulting permanent carries [`Permanent::bestowed`]. `false` for an
    /// ordinary creature cast.
    pub(crate) bestowed: bool,
    /// Whether this spell was cast face down (CR 702.37b — a morph cast, [`Intent::CastFaceDown`]):
    /// a 2/2 colorless creature spell whose real characteristics are hidden. Copied onto the
    /// resulting [`Permanent::face_down`] when the spell resolves ([`Event::PermanentEntered`]),
    /// so the permanent enters face down (CR 708). `false` for an ordinary face-up cast.
    pub(crate) face_down: bool,
    /// Whether this face-down spell was cast by Illusionary Mask's `{X}` ability (CR 615). Copied
    /// onto the resulting [`Permanent::masked`], which carries the card's self-replacement: a
    /// masked face-down creature that would assign or deal damage, be dealt damage, or become
    /// tapped is turned face up first. Only Illusionary Mask sets it; a plain morph/manifest
    /// face-down cast leaves it `false`.
    pub(crate) masked: bool,
    /// Whether this spell was cast for its evoke cost (CR 702.74a — [`CardDef::evoke`]). Copied
    /// onto the resulting [`Permanent::evoked`] when the spell resolves ([`Event::PermanentEntered`]),
    /// so the permanent is sacrificed the instant it enters. `false` for an ordinary cast.
    pub(crate) evoked: bool,
    /// The colors of mana actually spent to cast this spell (CR 106.9 — Court Hussar's "unless
    /// {W} was spent to cast it"), snapshotted from [`ManaPool::colors_spent`] against the
    /// [`Event::ManaSpent`] [`Game::settle_payment`](crate::Game::settle_payment) appends right
    /// before this spell hits the stack. Copied onto the resulting [`Permanent::spent_colors`]
    /// when the spell resolves, the same "read the spell's own info before it's gone" idiom as
    /// `entered_with_x`. `[false; Color::COUNT]` for a spell that paid no mana (a copy, a free
    /// cast) or a cast form (adventure, prepared copy) this snapshot isn't wired through yet — no
    /// pool card checks color-spent off those forms.
    pub(crate) spent_colors: [bool; Color::COUNT],
    /// How many of this spell's Phyrexian mana pips (CR 107.4f — Vraska, Betrayal's Sting's
    /// Compleated `{B/P}`) were paid with life instead of mana, snapshotted right before this
    /// spell hits the stack (see `phyrexian_life_paid_from` in `cast.rs`). Copied onto the
    /// resulting permanent's as-enters loyalty when the spell resolves
    /// ([`Event::PermanentEntered`]) — CR 107.4f: "If life was paid, this planeswalker enters
    /// with two fewer loyalty counters" per pip so paid. `0` for a spell with no Phyrexian pips
    /// or one that paid them all with mana.
    pub(crate) phyrexian_life_paid: u8,
}

/// A permanent on the battlefield, with its mutable per-object state.
#[derive(Debug, Clone)]
pub(crate) struct Permanent {
    pub(crate) def: CardId,
    pub(crate) owner: PlayerId,
    /// This permanent's Class level (CR 717.4 — a Class enchantment's level counter). Raised one
    /// step at a time by [`Effect::Counters(CountersEffect::LevelUp)`] (via [`Event::LeveledUp`]); read by every
    /// level-gated ability's [`Ability::min_level`] check. Runtime state, not TOML-authored —
    /// **defaults to 1** at every construction site (a Class enters at level 1; every ordinary
    /// permanent is trivially level 1, so a `min_level = 0/1` ability always functions). Not
    /// wire-mirrored (like `finality_counter`/`regeneration_shields`).
    pub(crate) level: u8,
    /// Whether the permanent is tapped.
    pub(crate) tapped: bool,
    /// Whether it entered this turn (can't attack / use tap abilities without haste);
    /// cleared at its controller's untap step.
    pub(crate) summoning_sick: bool,
    /// Whether this permanent entered the battlefield this turn (CR "entered the battlefield
    /// this turn" — Oran-Rief, the Vastwood's "each green creature that entered this turn").
    /// Distinct from `summoning_sick`, which is scoped to the permanent's own *controller's*
    /// next untap (CR 302.6, [`Event::LostSummoningSickness`]): this instead clears for every
    /// battlefield permanent at *every* Untap step (whichever player's turn is beginning — see
    /// [`Event::StepBegan`]'s turn-boundary reset block, alongside the `*_this_turn` tallies),
    /// since a new turn — anyone's — ends "this turn" for CR purposes. `true` for every
    /// permanent minted by [`fresh_permanent`]/[`fresh_token`] (an ETB, by definition, is "this
    /// turn"); the `spawn_on_battlefield`/`spawn_token_on_battlefield` test helpers override it
    /// back to `false` to keep their "as if it had been there since before the turn" contract,
    /// the same way they override `summoning_sick`.
    pub(crate) entered_this_turn: bool,
    /// Whether this permanent was untapped as the turn began — Rasputin Dreamweaver's "if
    /// Rasputin started the turn untapped", backing
    /// [`Condition::SourceStartedTheTurnUntapped`](cards::Condition). Stamped for every
    /// battlefield permanent in [`Event::StepBegan`]'s Untap block, which runs *before* the
    /// step's untapping turn-based action — so a permanent that was tapped last turn still reads
    /// `false` at the upkeep that follows, which is the whole point of the clause. `false` for a
    /// permanent minted mid-turn (it did not exist when the turn started).
    pub(crate) started_turn_untapped: bool,
    /// Whether this permanent was declared as an attacker this turn (CR 508.1, [`Event::AttackerDeclared`]).
    /// Backs [`Condition::SourceAttackedThisTurn`] (Agent Frank Horrigan's "has indestructible as
    /// long as it attacked this turn"). Turn-scoped like `entered_this_turn` above — set the
    /// instant the permanent is declared an attacker and cleared for *every* battlefield
    /// permanent at every Untap step (see [`Event::StepBegan`]'s turn-boundary reset block), not
    /// just its controller's. Distinct from [`PermanentFilter::attacking`], which only holds
    /// while the permanent is still in combat this turn: this stays `true` after end of combat,
    /// after the permanent is removed from combat, or after it changes controllers mid-combat.
    pub(crate) attacked_this_turn: bool,
    /// Whether this permanent attacked during its controller's *previous* turn (Giant Turtle's
    /// "This creature can't attack if it attacked during your last turn", CR 508.1a). `attacked_this_turn`
    /// above can't answer that on its own — it is cleared at *every* Untap, so the intervening
    /// opponents' turns wipe it long before its controller's next combat. This one is rolled once
    /// per turn, at the active player's cleanup step, from that same flag: a fact recorded during
    /// the controller's turn N reads back throughout their turn N+1 and is overwritten at N+1's
    /// cleanup.
    ///
    /// ponytail: rolled for the permanents its controller holds *at that cleanup*, so a creature
    /// that changes controllers between attacking and its owner's next turn carries the flag to the
    /// new controller rather than staying tied to the seat it attacked under.
    pub(crate) attacked_on_last_own_turn: bool,
    /// Whether this permanent has become monstrous (CR 701.28b) — a one-way state, not
    /// turn-scoped: it is never cleared at any Untap step, and a permanent that leaves the
    /// battlefield and returns is a new object that starts `false` again ([`fresh_permanent`]).
    /// Set by [`Event::BecameMonstrous`] the moment a [`CountersEffect::Monstrosity`] resolves
    /// (Alpha Deathclaw's "{5}{B}{G}: Monstrosity 4"), even if a replacement effect drove the
    /// accompanying +1/+1 counters to zero (CR 701.28c only silences a *second* activation).
    pub(crate) monstrous: bool,
    /// Net +1/+1 counters (each adds +1 power and +1 toughness).
    pub(crate) plus_counters: i32,
    /// Named non-P/T counters (CR 122.1 — charge, story, …), indexed by [`CounterKind`] as
    /// `usize`; `0` = none of that kind. Kept separate from `plus_counters` above — no
    /// replacement effect (Hardened Scales, a doubler) reads or grows this map.
    pub(crate) kind_counters: [u8; CounterKind::COUNT],
    /// A CR 612.1 text change made to this permanent ("change the text of target spell or
    /// permanent by replacing all instances of one basic land type with another" — Magical Hack;
    /// the color-word twin — Sleight of Mind). Read back at CR 613.4 layer 3 by
    /// [`Game::effective_subtypes`](crate::Game::effective_subtypes),
    /// [`Game::effective_keywords`](crate::Game::effective_keywords) and
    /// `Game::functional_abilities` — see [`TextSwap`] for what a swap does and does not reach.
    /// One slot, so a second text change on the same permanent replaces the first rather than
    /// composing with it.
    /// ponytail: two text-changers on one object is the composition this drops; no pool card
    /// makes it likely, and a `Vec` here would cost [`Permanent`] its `Copy`.
    pub(crate) text_swap: Option<TextSwap>,
    /// Keywords the creature this Aura is attached to loses, indefinitely (Earthbind's "this Aura
    /// gains 'Enchanted creature loses flying'"). Set on the *Aura*, not on its host (see
    /// [`Event::AttachedKeywordsLost`]), and read through the Aura's live attachment at the end of
    /// [`Game::compute_effective_keywords_uncached`] — so it lapses on its own when the Aura leaves
    /// the battlefield, and follows the Aura if it is moved to a new host. Never cleared at
    /// cleanup, unlike the registered `ModifierKind::LoseKeywords` an until-EOT strip makes.
    pub(crate) attachment_lost_keywords: &'static [Keyword],
    /// An *indefinite* base-P/T SET (CR 611.2c — Excava, the Risen Past's "It's a 1/1 Spirit
    /// creature with flying"): the indefinite twin of `ModifierKind::BasePtSet`, `Some((p, t))` while
    /// active, emitted as the same 7b `BasePtSet` layer by [`Game::pt_layers`] (before the 7c
    /// counters/pumps). Written once as the reanimated permanent enters (see
    /// [`Event::ReanimatedCreatureBecame`]) and **never cleared at cleanup** — it naturally resets
    /// because a permanent that leaves the battlefield becomes a new object (CR 400.7). Runtime
    /// bookkeeping, never a `CardDef`/TOML surface.
    pub(crate) set_base_pt: Option<(i32, i32)>,
    /// The CR 613.7 timestamp of [`Permanent::set_base_pt`].
    pub(crate) set_base_pt_timestamp: u64,
    /// Card types added indefinitely (CR 611.2c — Excava's "It's a … creature … in addition to its
    /// other types", turning a reanimated noncreature into a creature): the indefinite twin of
    /// `ModifierKind::Became`, unioned onto the printed types by [`Game::effective_types`], never
    /// cleared at cleanup (resets with the object per CR 400.7).
    pub(crate) added_types: TypeSet,
    /// The CR 613.7 timestamp of [`Permanent::added_types`] / [`Permanent::added_subtypes`].
    pub(crate) added_types_timestamp: u64,
    /// Creature subtypes added indefinitely by the same set (Excava → "Spirit"): the indefinite
    /// twin of `ModifierKind::Became`'s subtypes, unioned onto the printed subtypes by
    /// [`Game::effective_subtypes`]. `&'static` — copied straight from the granting ability's
    /// already-leaked `CardDef` data, no runtime leak.
    pub(crate) added_subtypes: &'static [&'static str],
    /// The land-type line another permanent replaced for as long as *it* stays on the battlefield
    /// (CR 613.4, CR 305.7 — Gaea's Liege's "target land becomes a Forest until this creature
    /// leaves the battlefield"): the new subtypes, the source that set them, and the CR 613.7
    /// timestamp. Read back by [`Game::effective_subtypes`] only while the source is still a
    /// permanent, which is the whole duration model — nothing clears this, the read just stops
    /// finding a live source. Resets with the object itself per CR 400.7.
    pub(crate) subtypes_set_while_source_remains: Option<(&'static [&'static str], ObjectId, u64)>,
    /// Keywords granted indefinitely by the same set (Excava → flying): the indefinite twin of a
    /// registered until-EOT keyword grant, unioned onto the effective keywords by
    /// [`Game::compute_effective_keywords_uncached`], never cleared at cleanup. `&'static`.
    pub(crate) granted_keywords: &'static [Keyword],
    /// Damage marked this turn (compared against toughness by a state-based action).
    pub(crate) marked_damage: i32,
    /// Set when dealt damage by a deathtouch source — lethal regardless of amount.
    pub(crate) deathtouched: bool,
    pub(crate) commander: bool,
    /// A token (CR 111): created directly on the battlefield, not from a card. When it
    /// leaves the battlefield it ceases to exist (a state-based action).
    pub(crate) token: bool,
    /// The permanent this is attached to, for an Aura/Equipment (CR 301.5/303.4). `None`
    /// when unattached. Its grant (see [`Effect::Static(StaticEffect::GrantToAttached)`]) applies to that host.
    pub(crate) attached_to: Option<ObjectId>,
    /// The CR 613.7 timestamp of this permanent's own static continuous effects while on the
    /// battlefield. Set when it enters, and refreshed when an attached grant takes hold on a host.
    pub(crate) continuous_timestamp: u64,
    /// A planeswalker's current loyalty (its loyalty counters, CR 606.5b). 0 for a non-planeswalker.
    pub(crate) loyalty: i32,
    /// Whether a loyalty ability was activated on this planeswalker this turn (CR 606.3 — at most
    /// one per turn). Cleared at its controller's untap.
    pub(crate) loyalty_activated: bool,
    /// Whether this permanent has a finality counter (CR 122.3g/614.12): if it would be put into
    /// a graveyard from the battlefield, it's exiled instead (see `Game::graveyard_or_command`).
    /// A permanent either has one or it doesn't — no pool card stacks or removes them, so this is
    /// a flag rather than a count (unlike `plus_counters`). Set only by a reanimation with
    /// `finality = true` (Excava, the Risen Past); default `false`.
    pub(crate) finality_counter: bool,
    /// Whether a damage rider has marked this creature "it can't be regenerated this turn"
    /// (Disintegrate, CR 701.15d). Read by the lethal-damage state-based action alongside
    /// `regeneration_shields`, and by the same shield check the destroy path runs — the flag is
    /// the permanent-side twin of [`Effect::Destroy(DestroyEffect::DestroyTarget)::cant_be_regenerated`],
    /// which is carried by the destruction itself and so needs no marking. Set by
    /// [`Event::DamageMarked`](crate::Event); cleared at the next Untap step with the other
    /// "this turn" state.
    pub(crate) cant_be_regenerated_this_turn: bool,
    /// Whether a damage rider has marked this creature "if it would die this turn, exile it
    /// instead" (Disintegrate). Read at the single dies choke `Game::graveyard_or_command`, where
    /// it does exactly what a `finality_counter` does — the difference is only that this one is a
    /// nameless turn-scoped mark rather than a real counter, so it shows up in no counter count
    /// and expires at the next Untap step. Set by [`Event::DamageMarked`](crate::Event).
    pub(crate) exile_instead_of_dying_this_turn: bool,
    /// How many regeneration shields this permanent currently has (CR 701.15b): each is a
    /// replacement effect that replaces the next "destroy" this turn with a regeneration (tap,
    /// remove from combat, heal all damage). Consumed one at a time by the destroy path unless
    /// the destruction carries [`Effect::Destroy(DestroyEffect::DestroyTarget)::cant_be_regenerated`] (CR 701.15d); all
    /// reset to 0 at cleanup (CR 701.15b's "this turn"). Runtime state, not TOML-authored,
    /// defaulted 0 like `finality_counter`. Granted by [`Effect::Control(ControlEffect::RegenerateShield)`].
    pub(crate) regeneration_shields: u8,
    /// Whether this permanent is "prepared" (soc/sos prepare DFCs — CR-style status): a front-face
    /// ability ([`Effect::Misc(MiscEffect::BecomePrepared)`]) set it, and while set its controller may cast a copy of
    /// its back-face spell ([`CardDef::back`], via [`Game::cast_prepared`]), which clears the flag.
    /// The status persists across turns until the copy is cast (it is *not* reset at turn
    /// boundaries). `false` for every ordinary permanent.
    pub(crate) prepared: bool,
    /// Echo (CR 702.31e) unpaid: set when a permanent with [`CardDef::echo`] enters, cleared at
    /// its controller's first upkeep after entering (whether echo was paid or the permanent was
    /// sacrificed) — the honest "came under your control since your last upkeep" flag, distinct
    /// from [`Self::summoning_sick`] (which clears at the *untap* step, one step earlier).
    /// `false` for every permanent without echo.
    pub(crate) echo_unpaid: bool,
    /// The creature type named by an as-enters choice (CR 614.12/700.9-style "as ~ enters,
    /// choose a creature type" — Patchwork Banner), read back by a chosen-type-gated anthem
    /// ([`Effect::Static(StaticEffect::Anthem)`]'s `chosen_subtype`). `None` until the choice is answered (see
    /// [`Effect::Choice(ChoiceEffect::ChooseCreatureType)`]), and for every permanent without such a choice.
    pub(crate) chosen_subtype: Option<&'static str>,
    /// The color named by an as-enters choice (CR 614.12/700.9-style "as this Aura enters, choose
    /// a color" — Flickering Ward), read back by a `protection_from_chosen_color`
    /// [`Effect::Static(StaticEffect::GrantToAttached)`] to confer [`Keyword::ProtectionFrom`] of that color on the
    /// enchanted creature. `None` until the choice is answered (see [`Effect::Choice(ChoiceEffect::ChooseColor)`]), and
    /// for every permanent without such a choice.
    pub(crate) chosen_color: Option<Color>,
    /// The opponent named by an as-enters choice (CR 614.12-style "as this artifact enters, choose
    /// an opponent" — Black Vise), read back by a [`Condition::ChosenPlayersUpkeep`] gate on this
    /// permanent's own `each_upkeep` trigger. `None` until the choice is answered (see
    /// [`Effect::Choice(ChoiceEffect::ChooseOpponent)`]), and for every permanent without one.
    pub(crate) chosen_opponent: Option<PlayerId>,
    /// The {X} chosen for the spell that became this permanent (CR 601.2b), fixed for the rest
    /// of this permanent's existence — read by [`Game::ability_source_x`] so a later-resolving
    /// ability (an ETB trigger, an `mv_max_x` filter) can still reference "X" once the casting
    /// spell has left the stack and X would otherwise revert to 0 (CR 107.3i). For a hydra-style
    /// card this duplicates `plus_counters` (both are set to the same cast X); Fractal Harness's
    /// "put X +1/+1 counters on [a separate token]" ETB is the case that actually needs it, since
    /// nothing places counters on Fractal Harness itself. 0 for a token or a permanent with no
    /// {X} in its cost.
    ///
    /// An as-enters replacement effect can write it too ([`Event::EnteredWithXSet`]): Wood
    /// Elemental's "the number of Forests sacrificed as it entered" is a number fixed at entry and
    /// read back by an ability, which is exactly what this slot holds.
    pub(crate) entered_with_x: u32,
    /// How many times the spell that became this permanent paid its Multikicker cost (CR
    /// 702.33c), fixed for the rest of this permanent's existence — copied from
    /// [`Spell::multikicker_count`] as it enters, the same "read the spell's own info before
    /// it's gone" idiom as `entered_with_x` above (Lightkeeper of Emeria's "gain 2 life for each
    /// time it was kicked" ETB fires after the spell is already this permanent). Read back by
    /// [`Game::spell_multikicker_count`]. 0 for a token, a permanent with no Multikicker cost, or
    /// one paid zero times.
    pub(crate) entered_multikicker_count: u8,
    /// The graveyard-card object id this Aura targeted when cast (CR 303.4a's "enchant creature
    /// card in a graveyard" — [`CardDef::enchant_graveyard`]), locked in as it enters, the same
    /// "read the spell's own info before it's gone" idiom as `entered_with_x` above (the spell
    /// object is destroyed by the time this permanent's own ETB ability resolves). Read back by
    /// [`TargetSpec::ThisAurasGraveyardTarget`] as a fixed reference, not a fresh choice — empty
    /// once it's left the graveyard (CR 603.3c: the ETB ability then has no legal target and is
    /// dropped, rather than reanimating whatever moved in). `None` for every permanent whose
    /// spell had no chosen target, or wasn't cast with `enchant_graveyard` set.
    pub(crate) cast_time_enchant_target: Option<ObjectId>,
    /// The permanent this one is paired with — Stangg's Twin token and Stangg himself, linked as
    /// the token is created ([`Event::TwinLinked`]) and read back by
    /// [`TargetSpec::LinkedTwin`](cards::TargetSpec). Held on *both* halves so either one's
    /// "when the other leaves the battlefield" ability can still find its partner: the leaving
    /// permanent's own record is gone by the time its trigger resolves, so the lookup always
    /// scans the battlefield for the survivor that points back at it. `None` for every permanent
    /// that isn't half of a printed pair.
    pub(crate) linked_twin: Option<ObjectId>,
    /// The creature this [`CardDef::enchant_graveyard`] Aura reanimated and attached itself to —
    /// the object its rewritten enchant ability names (CR 613.3/702: "it loses 'enchant creature
    /// card in a graveyard' and gains 'enchant creature put onto the battlefield with this
    /// Aura'"). Set when the Aura's own ETB attaches it; consulted by
    /// [`Game::attachment_host_legal`], so the CR 704.5m sweep holds the Aura to exactly that
    /// object rather than the default enchant-creature filter. `None` until the rewrite happens
    /// (the pre-attach window keeps its printed graveyard-card enchant, which no battlefield
    /// host can satisfy).
    pub(crate) enchant_rewrite_host: Option<ObjectId>,
    /// The player this creature "can't attack … for as long as it has a vow counter on it" (CR
    /// 122.1 — Promise of Loyalty): set alongside a [`CounterKind::Vow`] counter by
    /// [`Event::VowCountersPlaced`], read in [`Game::declare_attackers`]. `None` for any creature
    /// with no vow counter. Engine-internal, not wire-mirrored (like `entered_with_x`/`echo_unpaid`);
    /// the restriction is read live off `kind_counters[Vow]` + this, so removing the counter lifts it.
    pub(crate) vow_protected: Option<PlayerId>,
    /// Whether this permanent is *phased out* (CR 702.26): treated as though it doesn't exist —
    /// excluded from every battlefield scan (statics, combat, SBAs, targeting, board counts) until
    /// it phases in at the start of its controller's next turn (CR 702.26f, before untapping).
    /// Set by [`Effect::Choice(ChoiceEffect::PhaseOut)`] (Guardian of Faith's ETB) and on anything attached to a
    /// phased-out permanent (CR 702.26g — indirect phasing); cleared at that untap step. `false`
    /// for every permanent that hasn't phased out.
    /// ponytail: a plain "did/didn't phase out" flag — no "phased in tapped" bit (CR 702.26e: a
    /// permanent phases in tapped if it phased out tapped). Guardian phases out untapped creatures
    /// and the flag doesn't touch `tapped`, so tapped state is preserved for free; add a companion
    /// bit if a card ever phases out a tapped permanent whose re-tap must be observable.
    pub(crate) phased_out: bool,
    /// Whether this permanent was played/cast from a graveyard under Serra Paragon's permission
    /// (CR 118.9) and so carries the granted rider "when this permanent is put into a graveyard
    /// from the battlefield, exile it and you gain 2 life." Set as it enters (from the casting
    /// [`Spell::serra_recursion`], or directly for a land-play); read at the [`Event::MovedToGraveyard`]
    /// apply choke ([`Game::apply`]) into `Game`'s batch scratch — the permanent is genuinely
    /// dying, so `Game::enqueue_triggers` fabricates the real placed trigger off that scratch
    /// (see [`crate::Effect::Zone(ZoneEffect::ExileGraveyardObjectGainLife)`]). Runtime state, not TOML-authored,
    /// defaulted `false` like `finality_counter`.
    pub(crate) serra_recursion: bool,
    /// Whether this permanent was cast via bestow (CR 702.103 — Eidolon of Countless Battles) and
    /// so is a dual-nature Aura/creature. While set *and* attached ([`Permanent::attached_to`] is
    /// `Some`), it's an Aura enchantment and **not** a creature (CR 702.103e) — the "attached?"
    /// gate, not this flag alone, decides which nature is live. When it stops being attached it
    /// becomes a creature again (CR 702.103i). Set as it enters from the casting [`Spell::bestowed`];
    /// runtime state, not TOML-authored, defaulted `false` like `serra_recursion`.
    pub(crate) bestowed: bool,
    /// Whether this permanent is *face down* (CR 708 — a manifested card, CR 701.34): while set,
    /// its real `def` is hidden and it is a 2/2 colorless creature with no name, no card types
    /// other than creature, no subtypes, no mana cost, and no abilities (CR 708.2 — the
    /// characteristics overrides in [`Game::effective_types`]/`pt_base`/`functional_abilities`/
    /// `effective_subtypes`/`compute_effective_keywords_uncached` all short-circuit on it). The
    /// real card stays in `def` so it can be revealed by the turn-face-up special action
    /// ([`Intent::TurnFaceUp`]), which clears this flag; the wire redaction layer anonymizes it.
    /// ponytail: the face-down 2/2 status is the shared substrate for both plain manifest (CR
    /// 701.34) and the morph family (CR 702.37) — a morph card (Willbender, Chromeshell Crab) adds
    /// its face-down cost ([`CardDef::morph`]) + the morph keyword on top of this status. Megamorph
    /// and disguise aren't in the pool yet; they'd layer their own extras on the same flag.
    pub(crate) face_down: bool,
    /// Whether this permanent has *flipped* (CR 712 — a Kamigawa flip card, Nezumi Graverobber →
    /// Nighteyes the Devourer): while set, its live characteristics come from its [`CardDef::back`]
    /// face (name, P/T, types, subtypes, abilities) instead of its front `def` — read through
    /// [`Game::def_of`], which every characteristic accessor funnels through. Unlike morph's
    /// `face_down`, flipping is one-way and permanent: nothing clears it. The object itself is
    /// unchanged (CR 712.5), so counters, attachments, and tapped state ride across untouched.
    /// Set by [`Effect::Misc(MiscEffect::FlipSource)`](crate::Effect::Misc(MiscEffect::FlipSource)) via [`Event::Flipped`]; runtime
    /// state, not TOML-authored, defaulted `false` like `face_down`.
    pub(crate) flipped: bool,
    /// Whether this face-down permanent was put onto the battlefield by Illusionary Mask (CR 615):
    /// while `masked && face_down`, it turns face up for free (no morph/manifest cost) the instant
    /// it would assign or deal damage, be dealt damage, or become tapped (the printed "instead it's
    /// turned face up and ..." self-replacement — consulted at the damage/tap chokes). Set from the
    /// casting [`Spell::masked`]; `false` for a plain morph/manifest face-down permanent, which is
    /// never turned face up by interaction. Runtime state, not TOML-authored.
    pub(crate) masked: bool,
    /// Whether this permanent was cast for its evoke cost (CR 702.74a — [`CardDef::evoke`]): it is
    /// sacrificed the instant it enters, via a self-sacrifice trigger queued alongside its own ETB
    /// triggers so an ETB payoff (Mulldrifter's draw two) resolves first. Set as it enters from the
    /// casting [`Spell::evoked`]; runtime state, not TOML-authored, defaulted `false` like
    /// `bestowed`.
    pub(crate) evoked: bool,
    /// The colors of mana spent to cast the spell that became this permanent (CR 106.9), fixed
    /// for the rest of this permanent's existence — copied from [`Spell::spent_colors`] as it
    /// enters, the same "read the spell's own info before it's gone" idiom as `entered_with_x`.
    /// Read by [`Condition::ColorWasSpentToCastThis`] (Court Hussar's "unless {W} was spent to
    /// cast it"). `[false; Color::COUNT]` for a token, a reanimated/reconstructed permanent, or
    /// any permanent whose casting spell paid no mana or isn't wired through yet (see
    /// [`Spell::spent_colors`]'s doc).
    pub(crate) spent_colors: [bool; Color::COUNT],
    /// Whether the spell that became this permanent was cast from its controller's hand (CR
    /// 601) — copied from [`Spell::cast_from_hand`] as it enters, the same "read the spell's own
    /// info before it's gone" idiom as `entered_with_x`/`spent_colors` above. Read by
    /// [`Condition::CastFromHand`] (Dread Cacodemon's/Reiver Demon's "if you cast it from your
    /// hand" ETB intervening-if, CR 603.4). `false` for a token, a reanimated/searched/flickered/
    /// manifested permanent, or anything else that never went through [`Event::PermanentEntered`]
    /// — every one of those is, by definition, not a hand cast.
    pub(crate) cast_from_hand: bool,
    /// Copy-effect *exception* keywords that are part of this object's **copiable** values (CR
    /// 707.2 — a copy made "except it has haste"/"except it has myriad": Twinflame, Cursed
    /// Mirror, Muddle, Determined Iteration, Rionya). Unlike an ordinary until-end-of-turn
    /// keyword grant (a `TempBoost`, unrelated EOT pump), these ride along when the object is
    /// copied *again* — so a second-generation copy (Brudiclad, Rite of Replication) keeps the
    /// rider. Unioned onto the effective keywords by [`Game::runtime_continuous_effects`] and
    /// read back by [`Game::copiable_keywords`]; set by [`Event::CopyRiderKeywordsGranted`].
    /// Not cleared at ordinary cleanup (a copiable characteristic resets with the object per CR
    /// 400.7), but an *until-end-of-turn* copy clears it when its `def` reverts (Cursed Mirror,
    /// Muddle — see [`Event::TempBoostsEnded`]). `&'static`, defaulted `&[]`.
    pub(crate) copy_rider_keywords: &'static [Keyword],
}

/// One slot in the object arena. A card's slot becomes [`Object::Moved`] when it changes
/// zones (a fresh slot/id is minted for its new form); `to` points at that new id so an
/// old id's lineage can still be followed (see [`Game::zone_of`]).
// The `Spell`/`Permanent` variants inline a whole `CardDef` and are near-equal in size (~2.3 KB);
// the id-indexed object arena needs `Object: Copy`, so boxing a variant isn't an option — the same
// carve-out the sibling `Copy` enums in this crate take.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub(crate) enum Object {
    Card(Card),
    Spell(Spell),
    Permanent(Permanent),
    Moved {
        to: ObjectId,
    },
    /// The object left the game — a token that ceased to exist (CR 111.7), a spell copy that
    /// finished resolving (CR 707.10a), or an object owned by an eliminated player (CR 800.4a).
    /// Carries last-known `def`/`owner` so stack abilities keyed by this id (Food sacrifice,
    /// Dies triggers off a token) can still project art / provenance after the arena slot dies.
    Removed {
        def: CardId,
        owner: PlayerId,
    },
}

/// The default number of seats when a constructor doesn't specify one (the 1v1 games
/// most tests build). Real tables set their own count via [`Game::with_players`].
pub(crate) const NUM_PLAYERS: u8 = 2;

/// Default starting life for a plain game; Commander games use [`COMMANDER_LIFE`].
pub(crate) const STARTING_LIFE: i32 = 20;

/// Starting life in the Commander format.
pub(crate) const COMMANDER_LIFE: i32 = 40;

/// Combat damage from a single commander that loses the game.
pub(crate) const LETHAL_COMMANDER_DAMAGE: i32 = 21;

/// Poison counters on a player that lose the game (CR 704.5c).
pub(crate) const LETHAL_POISON: u8 = 10;

/// The maximum hand size enforced by the cleanup step.
pub(crate) const HAND_SIZE: usize = 7;

/// Per-player game state that isn't tied to a single object.
#[derive(Debug, Clone, Default)]
pub(crate) struct Player {
    pub(crate) life: i32,
    /// Available mana this step (colored, colorless, and "any"). Empties between steps.
    pub(crate) mana_pool: ManaPool,
    /// The player's library, top of library first (index 0 is drawn next).
    /// ponytail: `Vec::remove(0)` to draw is O(n); trivial for a ~100-card deck.
    pub(crate) library: Vec<ObjectId>,
    /// Set when the player tried to draw from an empty library (loses via SBA).
    pub(crate) attempted_empty_draw: bool,
    /// Completed mulligans this player has taken in the pre-game mulligan phase.
    pub(crate) mulligans_taken: u8,
    /// Whether this player has kept their hand for the pre-game mulligan phase.
    pub(crate) hand_kept: bool,
    /// Lands played this turn (reset at untap; limited to one).
    pub(crate) lands_played: u8,
    /// Life this player has gained this turn (turn-scoped; reset each turn at untap). Feeds
    /// [`Amount::LifeGainedThisTurn`] and "if you gained life this turn" conditions.
    pub(crate) life_gained_this_turn: u32,
    /// Spells this player has cast this turn (turn-scoped; reset each turn at untap). Feeds
    /// [`Amount::SpellsCastThisTurn`].
    pub(crate) spells_cast_this_turn: u32,
    /// Damage dealt to this player this turn (turn-scoped; reset each turn at untap), combat and
    /// noncombat alike (CR 120.1). Feeds [`Amount::DamageTakenThisTurn`] — Simulacrum's "the
    /// damage dealt to you this turn". Damage, not life loss: a drain or a paid life cost only
    /// ever emits `Event::LifeChanged`, and neither of the two damage markers this counts.
    pub(crate) damage_taken_this_turn: u32,
    /// How many untapped lands this player controlled at the beginning of the current turn —
    /// Power Surge's "the number of untapped lands they controlled at the beginning of this
    /// turn". Snapshotted for every player when the *upkeep* begins (`Game::apply`'s
    /// `Event::StepBegan` arm), not at untap: no player receives priority between untapping and
    /// that point (CR 502.3), so the two moments hold the same count and only the later one runs
    /// after the untap step's turn-based action. Feeds
    /// [`Amount::UntappedLandsAtTurnStart`].
    pub(crate) untapped_lands_at_turn_start: u32,
    /// Spells with {X} in their mana cost this player has cast this turn (turn-scoped; reset
    /// at untap) — the filter-scoped sibling of `spells_cast_this_turn`, for the "first {X}-
    /// spell each turn" gate (Nev, Zimone Infinite Analyst). CR 107.3.
    pub(crate) x_spells_cast_this_turn: u32,
    /// Cards this player has drawn this turn (turn-scoped; reset each turn at untap) — the
    /// draw-side sibling of `spells_cast_this_turn`. Feeds [`Trigger::PlayerDraws`] (Faerie
    /// Mastermind's "an opponent draws their second card each turn"). A [`Event::CardDrawn`]
    /// bumps this; drawing from an empty library ([`Event::DrewFromEmptyLibrary`]) does not —
    /// CR 120.3, you don't draw if the library is empty.
    pub(crate) draws_this_turn: u32,
    /// How many times this player has lost life this turn (turn-scoped; reset each turn at
    /// untap) — the life-loss sibling of `draws_this_turn`. A [`Event::LifeChanged`] with a
    /// *negative* amount bumps this (CR 118.9/119.3 — only a decrease is a life loss; gaining
    /// life doesn't). Feeds [`Trigger::YouLoseLifeFirstTimeEachTurn`] (Intermediate
    /// Chirography's level-2 "whenever you lose life for the first time each turn"): the trigger
    /// fires only when the losing event's ordinal this turn is 1. A count (not a bool) so the
    /// first-loss ordinal can be recovered within a batch that carries several losses, exactly
    /// as `draws_this_turn` recovers a draw's ordinal.
    pub(crate) life_losses_this_turn: u32,
    /// Creatures that died under this player's control this turn (turn-scoped; reset each turn
    /// at untap) — the death-side sibling of `spells_cast_this_turn`. Feeds
    /// [`Amount::CreaturesDiedThisTurn`] (Gorma, the Gullet).
    pub(crate) creatures_died_this_turn: u32,
    /// Whether a *modified* creature (CR 701.29 — has a counter, is enchanted by an Aura, or is
    /// equipped) died under this player's control this turn (turn-scoped; reset each turn at
    /// untap) — the modified-scoped sibling of `creatures_died_this_turn`. Feeds
    /// [`Condition::ModifiedCreatureDiedThisTurn`] (Intermediate Chirography's Level 3
    /// morbid-of-modified end step). Set at the death choke ([`Event::MovedToGraveyard`]/
    /// [`Event::TokenCeasedToExist`] in `apply.rs`) by last-known information — `is_modified` is
    /// read *before* the dying object's attachments/counters are torn down by the zone change
    /// (CR 700.4).
    pub(crate) modified_creature_died_this_turn: bool,
    /// Nontoken creatures that entered the battlefield under this player's control this turn
    /// (turn-scoped; reset each turn at untap) — the entering-side sibling of
    /// `creatures_died_this_turn`, excluding tokens. Feeds
    /// [`Amount::NontokenCreaturesEnteredThisTurn`] (Gyome, Master Chef).
    pub(crate) nontoken_creatures_entered_this_turn: u32,
    /// Whether a land entered the battlefield under this player's control this turn (turn-scoped;
    /// reset each turn at untap) — CR landfall's own "enters" (cast, fetched, or a token land all
    /// count), not "played." Feeds [`Condition::LandEnteredUnderYourControlThisTurn`] (Zimone,
    /// All-Questioning's end step). Set at the same permanent-enters choke as
    /// `nontoken_creatures_entered_this_turn`.
    pub(crate) land_entered_under_your_control_this_turn: bool,
    /// Whether a card has left this player's graveyard this turn (turn-scoped; reset each turn at
    /// untap). Set at the object-move choke point ([`Game::create_object`]); feeds
    /// [`Condition::CardLeftYourGraveyardThisTurn`].
    pub(crate) card_left_graveyard_this_turn: bool,
    /// Whether this player has cast an instant or sorcery spell this turn (turn-scoped; reset
    /// each turn at untap). Feeds [`Condition::CastInstantOrSorceryThisTurn`] (Hall of Oracles's
    /// counter ability's activation restriction).
    pub(crate) instant_or_sorcery_cast_this_turn: bool,
    /// The greatest mana value among instant and sorcery spells this player has cast this turn
    /// (turn-scoped; reset each turn at untap, 0 if none) — Rootha, Mastering the Moment's "X is
    /// the greatest mana value among instant and sorcery spells you've cast this turn." Feeds
    /// [`Amount::GreatestInstantOrSorceryManaValueCastThisTurn`].
    pub(crate) greatest_instant_or_sorcery_mana_value_cast_this_turn: u32,
    /// How many instant and sorcery spells this player has cast this turn (turn-scoped; reset
    /// each turn at untap, 0 if none) — Rionya, Fire Dancer's "X is one plus the number of
    /// instant and sorcery spells you've cast this turn." Feeds
    /// [`Amount::InstantsAndSorceriesCastThisTurn`]. A copied spell doesn't bump this —
    /// same "cast" boundary as `instant_or_sorcery_cast_this_turn` above.
    pub(crate) instants_and_sorceries_cast_this_turn: u32,
    /// Whether this player may cast spells this turn as though they had flash (turn-scoped;
    /// reset each turn at untap) — CR 601.3a, granted by [`Effect::Misc(MiscEffect::GrantFlashThisTurn)`]
    /// (Alchemist's Refuge). Unfiltered: every spell, not a subset. Read by
    /// [`CardDef::is_instant_speed`]'s cast-timing gate.
    pub(crate) flash_permission_this_turn: bool,
    /// Whether this player may, at mana-ability timing, pay 1 life to add {C} (turn-scoped;
    /// reset each turn at untap) — Yavimaya Bloomsage's Channel back face, granted by
    /// [`Effect::Misc(MiscEffect::GrantChannelColorlessManaThisTurn)`]. Read by
    /// [`Game::channel_colorless_mana`](crate::Game::channel_colorless_mana).
    pub(crate) channel_colorless_mana_this_turn: bool,
    /// Whether this player may spend mana as though it were mana of any type to pay a spell's mana
    /// cost (turn-scoped; reset each turn at untap) — CR 609.4b, granted by
    /// [`Effect::Misc(MiscEffect::GrantSpendManaAsAnyTypeForOneSpellThisTurn)`] (North Star).
    /// Widens [`Game::mana_substitutions`](crate::Game) into every color pair while it holds, and
    /// is cleared by the next [`Event::SpellCast`] this player makes — North Star's "for one
    /// spell."
    /// ponytail: colors only. The card says "any *type*", which includes colorless {C} pips;
    /// nothing in this pool prints a {C} pip a North Star player would want to relax, and
    /// [`ManaPool::substituted`] is a color→color widening. Widen the substitution vocabulary if
    /// a {C} cost ever needs it.
    pub(crate) spend_mana_as_any_type_this_turn: bool,
    /// Whether this player has already used Serra Paragon's graveyard-play permission this turn
    /// (turn-scoped; reset each turn at untap) — CR 118.9's "once during each of your turns."
    /// Set when a land / permanent spell is played or cast from the graveyard under
    /// [`Effect::Static(StaticEffect::PlayFromGraveyardOncePerTurn)`], read by [`Game::playable_zone`] to reject a
    /// second such play the same turn. `false` until the permission is used.
    pub(crate) graveyard_play_used_this_turn: bool,
    /// Whether this player declared at least one attacker this turn (turn-scoped; reset each
    /// turn at untap) — Angelic Arbiter's "Each opponent who attacked with a creature this turn
    /// can't cast spells." Set by [`Event::AttackerDeclared`] (`Game::apply`), keyed by the
    /// attacker's own controller; read by [`Game::cant_cast_if_attacked_this_turn`].
    pub(crate) attacked_this_turn: bool,
    /// Whether a nontoken permanent entered the battlefield under this player's control this turn
    /// — the second half of Arboria's "unless that player cast a spell or put a nontoken permanent
    /// onto the battlefield during their last turn". The existing
    /// `nontoken_creatures_entered_this_turn` tally next to it is creature-only, which Arboria is
    /// not. Read once per turn, at this player's own cleanup step, and reset there (see
    /// [`Game::roll_own_turn_history`]) rather than at the shared Untap reset — the point of the
    /// pair is to survive the other seats' turns.
    pub(crate) nontoken_permanent_entered_this_turn: bool,
    /// Whether this player cast a spell or put a nontoken permanent onto the battlefield during
    /// their *previous* turn (Arboria) — the per-player twin of
    /// [`Permanent::attacked_on_last_own_turn`], rolled at the same cleanup step from
    /// `spells_cast_this_turn` and `nontoken_permanent_entered_this_turn` above. `false` for a
    /// player who has not yet taken a turn, which is what Arboria wants: no last turn, nothing done
    /// during it, no attacking them.
    pub(crate) acted_on_last_own_turn: bool,
    /// Monotonic counter for derive-per-op RNG — bumped once per random operation for this seat.
    pub(crate) op_iteration: u64,
    /// Times this player has cast their commander from the command zone (tax = 2× this).
    pub(crate) command_casts: u8,
    /// Commander combat damage taken, keyed by the source commander's owner (each player
    /// has one commander); 21 from one source loses the game.
    pub(crate) commander_damage: Vec<(PlayerId, i32)>,
    /// Named counters sitting on this *player* (CR 122.1), one slot per [`PlayerCounterKind`] —
    /// the player-side twin of [`Permanent::kind_counters`]. Ten or more poison loses the game
    /// (CR 704.5c).
    pub(crate) kind_counters: [u8; PlayerCounterKind::COUNT],
    /// Set once the player has lost the game (a state-based action).
    pub(crate) lost: bool,
    /// Whether this player has the city's blessing (CR 702.131 ascend). Sticky: set once by a
    /// state-based action when the player controls ten or more permanents, and never cleared —
    /// CR 702.130's "for the rest of the game." Feeds [`Condition::YouHaveCitysBlessing`].
    pub(crate) has_citys_blessing: bool,
    /// This player's answers to Archangel of Strife's "as this creature enters, each player
    /// chooses war or peace", one `(Archangel, true = war)` entry per copy — CR 614.12 makes the
    /// choice per permanent, so a second Archangel asks again and each copy's anthems read their
    /// own answer. Sticky like `has_citys_blessing` above: never cleared once written — an entry
    /// feeds its `war_choice` anthem for as long as that source lives, and does nothing after.
    pub(crate) war_choices: Vec<(ObjectId, bool)>,
    /// Mana-provenance side-channel (CR 106.9-adjacent "spend this mana to …" tracking, Study
    /// Hall / Path of Ancestry / Opal Palace): one `(producing source, mana kind)` entry per unit
    /// of provenance-tagged mana this player currently holds, kept beside the summed
    /// [`mana_pool`](Self::mana_pool) which can't tag individual credits. Pushed when an
    /// [`Effect::Mana(ManaEffect::Add)`] flagged `track_provenance` resolves (see `Game::activate_ability`),
    /// read at a spell-cast payment to fire the source's `Trigger::SpendManaToCast`
    /// (see `Game::queue_spend_to_cast_triggers`), and cleared wholesale with the pool at
    /// [`Event::ManaEmptied`].
    /// ponytail: a summed pool can't prove *which* physical credit paid a cast, so provenance is
    /// only cleared at pool-empty and at a matched fire — a tagged credit spent on a non-cast /
    /// non-matching payment lingers as an entry with no backing mana, an over-fire only if a
    /// same-kind credit is later spent on a qualifying cast in the same step (unobserved in the
    /// pool). The upgrade path is per-credit tagging (tag each unit in `mana_pool` itself and
    /// consume the exact tagged credit on every `ManaSpent`); no pool card observes the gap today.
    pub(crate) mana_provenance: Vec<(ObjectId, Mana)>,
    /// "Until end of turn, you don't lose this mana as steps and phases end" side-channel (CR
    /// 500.4 exception; Rousing Refrain) — a mirror pool in lockstep with the "persist" credits
    /// still floating in [`mana_pool`](Self::mana_pool), same shape as [`mana_provenance`](Self::mana_provenance)'s
    /// own side-channel. Populated by an [`Effect::Mana(ManaEffect::Add)`] flagged `persist_until_end_of_turn`
    /// (see `Game::effects.rs`'s mint arm and [`Event::ManaAdded`]'s `persist` flag). Read at
    /// [`Event::ManaEmptied`]: a mid-turn boundary keeps only the credits still present in both
    /// pools (some may have been spent since); the turn-ending boundary (CR 514.2 cleanup) clears
    /// both wholesale like everything else.
    pub(crate) persistent_mana: ManaPool,
}
