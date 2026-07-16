use super::*;
#[cfg(feature = "card-dsl")]
use crate::de;

/// A numeric quantity in an effect: a fixed number, the casting spell's chosen `{X}`, or a
/// value derived from game state (a board/graveyard count, a permanent's power, a turn tally).
/// Every amount resolves through [`Game::resolve_amount`] / [`Game::resolve_count`] — even the
/// trivial `Fixed`/`X` — so a new derived variant is a single match arm with no separate pure path.
/// ponytail: only spells choose an `x`; a triggered/activated ability resolves its amounts with
/// `x = 0` (see [`Game::run`]), so no separate "ability X" concept is needed. (CR 602, CR 403.5, CR 601)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Amount {
    /// A fixed quantity written on the card.
    Fixed(i32),
    /// The value of the casting spell's `{X}`.
    X,
    /// Half the casting spell's `{X}`, rounded up (CR: "half X, rounded up" default).
    HalfX,
    /// Half the casting spell's `{X}`, rounded down — the explicit override some cards print
    /// instead of the CR round-up default (Hydroid Krasis's "half X … rounded down"). Only
    /// meaningful on a [`Trigger::YouCastThis`] ability's effect, filled at placement from the
    /// cast's chosen `{X}` — see `fill_cast_x`.
    HalfXRoundedDown,
    /// Twice the casting spell's `{X}`.
    TwiceX,
    /// The number of creatures the effect's controller controls (board-derived).
    PerCreatureYouControl,
    /// The number of creatures on the battlefield, all controllers (Chain Reaction).
    PerCreatureOnBattlefield,
    /// The number of permanents (in `zone`) matching `filter` — subsumes per-artifact, per-aura,
    /// and "per creature card in your graveyard" (Izoni) counts.
    PerPermanentMatching {
        filter: PermanentFilter,
        zone: AmountZone,
    },
    /// The effect's source permanent's power (Goldvein Hydra: "Treasures equal to its power").
    SourcePower,
    /// The effect's source permanent's toughness — the toughness twin of [`SourcePower`](Self::SourcePower)
    /// (Tanazir Quandrix: base power and toughness "become equal to Tanazir Quandrix's power and
    /// toughness").
    SourceToughness,
    /// The targeted permanent's power (Swords to Plowshares: "life equal to its power").
    TargetPower,
    /// The targeted object's mana value.
    TargetManaValue,
    /// The number of +1/+1 counters on the effect's source.
    PerCounterOnSource,
    /// The number of `kind`-counters on the effect's source (astral_cornucopia's "add one mana
    /// of any color for each charge counter on this artifact") — the named-counter-kind sibling
    /// of [`PerCounterOnSource`](Self::PerCounterOnSource).
    PerCounterOfKindOnSource { kind: CounterKind },
    /// How much life the effect's controller has gained this turn (a turn-scoped tally).
    LifeGainedThisTurn,
    /// How many spells the effect's controller has cast this turn (a turn-scoped tally).
    SpellsCastThisTurn,
    /// The number of cards in the resolving spell's *chosen player target's* hand (Rousing
    /// Refrain's "Add {R} for each card in target opponent's hand"). Read off the spell's target,
    /// like [`CommanderCastsFromCommandZone`](Self::CommanderCastsFromCommandZone) — the ability
    /// must have chosen a `Target::Player`.
    CardsInTargetPlayerHand,
    /// The number of cards in the effect's own controller's hand (Empyrial Armor's "Enchanted
    /// creature gets +1/+1 for each card in your hand") — the no-target, "your hand" sibling of
    /// [`CardsInTargetPlayerHand`](Self::CardsInTargetPlayerHand). Read live off the controller
    /// (not the resolving spell's target) at every characteristic recompute, so a
    /// [`Effect::GrantToAttached`] static using it tracks the hand as it grows or shrinks.
    CardsInYourHand,
    /// How many times the *targeted* player has cast their commander from the command zone
    /// this game (CR "cast a commander from the command zone this game") — Commander's
    /// Insight's "an additional card for each time they've cast a commander from the command
    /// zone this game". Read off the target, not the effect's controller.
    CommanderCastsFromCommandZone,
    /// How many creatures died under the effect's controller's control this turn (a turn-scoped
    /// tally, the death-side sibling of [`SpellsCastThisTurn`](Self::SpellsCastThisTurn)) — Gorma,
    /// the Gullet's "for each creature that died under your control this turn".
    CreaturesDiedThisTurn,
    /// How many nontoken creatures entered the battlefield under the effect's controller's
    /// control this turn (a turn-scoped tally, the entering-side sibling of
    /// [`CreaturesDiedThisTurn`](Self::CreaturesDiedThisTurn), excluding tokens) — Gyome, Master
    /// Chef's "a number of Food tokens equal to the number of nontoken creatures you had enter
    /// the battlefield under your control this turn".
    NontokenCreaturesEnteredThisTurn,
    /// The total power of creatures the effect's controller controls (Volcanic Salvo's "total
    /// power of creatures you control").
    TotalPowerYouControl,
    /// `then` if `condition` holds for the effect's controller, else 0 (Mortality Spear's
    /// "costs {2} less to cast if you gained life this turn" — a conditional cost reduction).
    /// `then` is `&'static` (leaked, like other card-data slices) to keep [`Amount`] `Copy`.
    IfCondition {
        condition: Condition,
        then: &'static Amount,
    },
    /// The power of the creature just paid as this ability's sacrifice cost (Dina, Soul
    /// Steeper's "+X/+0"; Dina, Essence Brewer's "gain X life and put X counters"), where X is
    /// that creature's power. A placeholder: [`contextualize_sacrifice_effect`] rewrites it to
    /// [`Fixed`](Self::Fixed) with the sacrificed creature's power when the ability is placed on
    /// the stack (the sacrificed creature is already gone by the time the ability resolves, so
    /// there's nothing left on the battlefield to read power from at that point) — resolving this
    /// variant directly is a bug (see [`Game::resolve_amount`]'s panic).
    SacrificedCreaturePower,
    /// The number of colors in the effect's controller's commander's color identity (CR 903.4) —
    /// War Room's "pay life equal to the number of colors in your commander's color identity".
    CommanderColorCount,
    /// The mana value (CR 202.3) of the spell that fired a `Trigger::CastSpell` (magecraft)
    /// ability — Renegade Bull's "+X/+0 … where X is that spell's mana value." A placeholder:
    /// [`fill_cast_mana_value`] rewrites it to [`Fixed`](Self::Fixed) with the triggering spell's
    /// mana value when the ability is placed on the stack, same shape as
    /// [`SacrificedCreaturePower`](Self::SacrificedCreaturePower) above — resolving this variant
    /// directly never happens (see [`Game::resolve_amount`]'s fallback).
    TriggeringSpellManaValue,
    /// The mana actually spent (CR 601.2h) to cast the spell that fired a `Trigger::CastSpell`
    /// ability — Manaform Hellkite's "X is the amount of mana spent to cast that spell," #101's
    /// per-card follow-on to [`TriggeringSpellManaValue`](Self::TriggeringSpellManaValue) above
    /// (which reads the printed mana value, treating `{X}` as 0 per CR 202.3b — divergent from
    /// mana spent for an `{X}` spell). A placeholder, same shape as
    /// [`TriggeringSpellManaValue`](Self::TriggeringSpellManaValue): [`fill_cast_mana_spent`]
    /// rewrites it to [`Fixed`](Self::Fixed) with the triggering spell's actual mana spent when
    /// the ability is placed on the stack — resolving this variant directly never happens (see
    /// [`Game::resolve_amount`]'s fallback).
    TriggeringSpellManaSpent,
    /// How many creatures were sacrificed to pay the resolving spell's additional sacrifice cost
    /// (CR 601.2f) — Plumb the Forbidden's "copy this spell for each creature sacrificed this
    /// way". Reads [`Game::spell_sacrifice_count`] off the effect's `source` (the resolving
    /// spell itself).
    SpellSacrificeCount,
    /// How many permanents (any type, any controller) were put into a graveyard from the
    /// battlefield this turn — Ominous Harvest's Gravestorm ("copy it for each permanent put
    /// into a graveyard from the battlefield this turn"). Game-wide, unlike
    /// [`CreaturesDiedThisTurn`](Self::CreaturesDiedThisTurn)'s per-controller tally, which this
    /// doesn't reuse because Gravestorm counts every player's dying permanents, not just the
    /// caster's own.
    /// ponytail: counts a nontoken permanent's death only (a battlefield [`Object::Permanent`]
    /// that becomes `Event::MovedToGraveyard`, incremented in `apply.rs`); a token's death is the
    /// ambiguous `Event::TokenCeasedToExist` (also fired for exile/bounce, with no zone-source to
    /// discriminate), so it's left uncounted — add a zone-change tag on that event if a
    /// Gravestorm-adjacent card needs a dying token counted.
    PermanentsDiedThisTurn,
    /// How many permanents matching `filter` *this resolution's own* `Effect::DestroyAll` step
    /// just destroyed (CR "destroyed this way" riders) — Ceaseless Conflict's "for each nontoken
    /// creature you controlled that was destroyed this way" token count, Culling Ritual's "for
    /// each permanent destroyed this way" mana count. Resolution-scoped, not turn-scoped: reads
    /// [`Game::destroyed_this_way`], a snapshot [`Effect::DestroyAll`]'s own `run`
    /// special case overwrites (not accumulates) each time it runs, since an `Effect::Sequence`
    /// doesn't apply steps' events back to live battlefield state between steps (the destroyed
    /// permanents are already gone by the time a later step counts them). `#[serde(default)]`
    /// `filter` matches every destroyed permanent (Culling Ritual's unfiltered count).
    PermanentsDestroyedThisWay { filter: PermanentFilter },
    /// How many *nonland* cards this resolution's own [`Effect::EachPlayerExilesFromGraveyard`]
    /// step just exiled across every player (Augusta, Order Returned's "put that many +1/+1
    /// counters"). Resolution-scoped like [`PermanentsDestroyedThisWay`](Self::PermanentsDestroyedThisWay):
    /// reads [`Game::nonland_cards_exiled_this_way`], overwritten (not accumulated) each time the
    /// fan-out begins.
    NonlandCardsExiledThisWay,
    /// How many "past" votes this resolution's own [`Effect::CouncilsDilemmaVote`] round tallied
    /// (Fateful Tempest's "mill a card for each past vote"). Reads [`Game::council_past_votes`], a
    /// resolution-scoped tally reset when the vote round begins — the vote-round sibling of
    /// [`NonlandCardsExiledThisWay`](Self::NonlandCardsExiledThisWay).
    PastVotes,
    /// How many "present" votes this resolution's own [`Effect::CouncilsDilemmaVote`] round tallied
    /// (Fateful Tempest's "Exile the top card of your library for each present vote"). Reads
    /// [`Game::council_present_votes`], the present-ballot twin of [`PastVotes`](Self::PastVotes).
    PresentVotes,
    /// The total mana value of the cards this resolution's own [`Effect::MillSelf`] step just
    /// milled (Fateful Tempest's "damage to each opponent equal to the total mana value of cards
    /// milled this way"). Reads [`Game::milled_mana_value_this_way`], snapshotted at the mill choke
    /// — resolution-scoped, like [`NonlandCardsExiledThisWay`](Self::NonlandCardsExiledThisWay).
    TotalManaValueMilledThisWay,
    /// The mana value of the card this resolution's own
    /// [`Effect::ExileTargetGraveyardCardRecordManaValue`] step just exiled (Surge to Victory's
    /// "Creatures you control get +X/+0 until end of turn, where X is that card's mana value").
    /// Reads [`Game::surge_exiled_card`], snapshotted at the exile choke — resolution-scoped,
    /// like [`TotalManaValueMilledThisWay`](Self::TotalManaValueMilledThisWay). `0` if unset (the
    /// exile step never ran — unreachable in practice, since a fizzled target drops this whole
    /// ability before either step resolves, CR 608.2b).
    ExiledCardManaValueThisWay,
    /// How many Auras the effect's controller controlled that were attached to the creature
    /// whose death fired a [`Trigger::AnEnchantedCreatureDies`] watch (Hateful Eidolon's "draw a
    /// card for each Aura you controlled that was attached to it"). A placeholder, like
    /// [`SacrificedCreaturePower`](Self::SacrificedCreaturePower) above: filled to
    /// [`Fixed`](Self::Fixed) at trigger placement from the pre-move attachment snapshot (CR
    /// 603.10a last-known information) — resolving this variant directly never happens (see
    /// [`Game::resolve_amount`]'s fallback).
    AurasYouControlledAttachedToDyingCreature,
    /// `then` if the resolving spell was kicked (CR 702.33d), else `else_` — Rite of
    /// Replication's "If this spell was kicked, create five of those tokens instead." Reads
    /// [`Game::spell_was_kicked`] off the effect's `source` (the resolving spell itself), the
    /// kicked-flag sibling of [`SpellSacrificeCount`](Self::SpellSacrificeCount)'s
    /// sacrifice-count read. Both arms are `&'static` (leaked, like `IfCondition::then`) to keep
    /// [`Amount`] a fixed-size, non-recursive `Copy` type.
    IfSpellKicked {
        then: &'static Amount,
        else_: &'static Amount,
    },
    /// The greatest mana value among instant and sorcery spells the effect's controller has cast
    /// this turn (turn-scoped, 0 if none) — Rootha, Mastering the Moment's "X is the greatest
    /// mana value among instant and sorcery spells you've cast this turn." A live read (unlike
    /// [`TriggeringSpellManaValue`](Self::TriggeringSpellManaValue), no placeholder-fill needed):
    /// [`Player::greatest_instant_or_sorcery_mana_value_cast_this_turn`] is already current by
    /// the time a `Trigger::BeginCombat` ability resolves.
    GreatestInstantOrSorceryManaValueCastThisTurn,
    /// One plus the number of instant and sorcery spells the effect's controller has cast this
    /// turn (turn-scoped) — Rionya, Fire Dancer's "X is one plus the number of instant and
    /// sorcery spells you've cast this turn." A live read, like
    /// [`GreatestInstantOrSorceryManaValueCastThisTurn`](Self::GreatestInstantOrSorceryManaValueCastThisTurn):
    /// [`Player::instants_and_sorceries_cast_this_turn`] is already current by the time a
    /// `Trigger::BeginCombat` ability resolves.
    /// ponytail: bakes the printed "one plus" into the variant itself — Rionya is the pool's
    /// only "one plus a tally" card. If a second card needs the raw tally or a different offset,
    /// split this into a raw `InstantsAndSorceriesCastThisTurn` count plus a generic successor
    /// combinator instead of adding another baked-offset variant.
    OnePlusInstantsAndSorceriesCastThisTurn,
    /// The number of Auras (any controller) currently attached to the effect's source (CR 303.4)
    /// — Kor Spiritdancer's "gets +2/+2 for each Aura attached to it". A live read, unlike
    /// [`AurasYouControlledAttachedToDyingCreature`](Self::AurasYouControlledAttachedToDyingCreature)'s
    /// dying-snapshot/controller-scoped count: this reads the source's current attachments with
    /// no controller filter and no death involved, so it needs no placeholder-fill.
    AurasAttachedToSource,
    /// The number of instant and sorcery cards in the effect's controller's graveyard (CR 205 card
    /// types, CR 202 mana value) — Furygale Flocking's "costs {1} less to cast for each instant
    /// and sorcery card in your graveyard." Not [`PerPermanentMatching`](Self::PerPermanentMatching)
    /// with [`AmountZone::Graveyard`]: an instant/sorcery card is exactly [`CardKind::Spell`], and
    /// [`CardKind::Spell::types`](CardKind::types) is [`TypeSet::NONE`] (permanents are the other
    /// `CardKind` arms), so no `PermanentFilter`/`TypeSet` axis can select it — this reads
    /// `CardKind` directly instead, touching no `TypeSet` bit (several search-library filters rely
    /// on instants/sorceries having an empty `TypeSet` to stay excluded from "permanent" matches).
    InstantOrSorceryCardsInYourGraveyard,
    /// The summed combat damage a
    /// [`Trigger::ZeroBasePowerCreaturesYouControlDealCombatDamage`] watch's whole batch of
    /// base-power-0 attackers just dealt one defending player (Primo, the Unbounded's "Put a
    /// number of +1/+1 counters on it equal to the damage dealt"). A placeholder, like
    /// [`SacrificedCreaturePower`](Self::SacrificedCreaturePower) above:
    /// [`fill_combat_damage`] rewrites it to [`Fixed`](Self::Fixed) with the batch's summed
    /// damage when the watch's ability is placed on the stack — resolving this variant directly
    /// never happens (see [`Game::resolve_amount`]'s fallback).
    CombatDamageDealt,
}

impl Default for Amount {
    /// A cost/effect field's implicit "no amount specified" is a flat zero — matches the u8/i32
    /// fields ([`ActivationCost::pay_life`], the old fixed-int effect fields) this type replaced.
    fn default() -> Self {
        Amount::Fixed(0)
    }
}

/// Which zone a [`Amount::PerPermanentMatching`] counts over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AmountZone {
    /// Permanents on the battlefield (the default — a board count).
    #[default]
    Battlefield,
    /// Cards in a graveyard (Izoni's "creature card in your graveyard").
    Graveyard,
}

/// A single parametrized game action. The enum grows only as the pool demands it.
///
/// In TOML an effect is a table tagged by `type = "<snake_case variant>"` — adding a
/// variant here is all the DSL needs (no parallel deserialization arm; see the `de` module).
///
/// `Effect` is `Copy`, so any list-valued field must be `&'static [T]` (leaked/interned once
/// at TOML-load time), never a `Vec<T>`. In a TOML-parsed variant, reach for
/// `#[cfg_attr(feature = "card-dsl", serde(default, deserialize_with = "de::static_slice"))]`
/// (or `de::static_str_slice` for `&'static [&'static str]`) rather than hand-rolling a leak —
/// see [`Effect::PumpUntilEndOfTurn`]'s `keywords` field for the canonical example.
// ponytail: `CreateToken` inlines a whole `CardDef`, which is large by design and must stay `Copy`
// (no `Box`), so this enum is unavoidably big-variant. Boxing would break `Copy`; the lint is noise.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "type", rename_all = "snake_case")
)]
pub enum Effect {
    /// Deal `amount` damage to the ability's target. `count` is how many distinct targets are
    /// chosen at cast (CR 601.2c), the same [`TargetCount`] surface as [`ReturnToHand`](Self::ReturnToHand)'s
    /// `count` — the default `{1, 1}` is a single mandatory target; Volcanic Salvo's "up to two
    /// target creatures and/or planeswalkers" is `{0, 2}`, dealing `amount` to *each* chosen
    /// target. `divided` (default `false`) instead splits one `amount` total across the chosen
    /// targets (CR 601.2d — Magma Opus's "4 damage divided as you choose among any number of
    /// targets"): each target must get at least one point, summing to exactly `amount`. The
    /// split itself is a player choice recorded on [`Spell::damage_division`], settled right
    /// after targets are chosen (see [`Game::maybe_begin_damage_division`]) — a single chosen
    /// target skips the choice and takes the whole amount.
    /// ponytail: a `divided` spell's `count.max` must not exceed `amount` (each target needs
    /// ≥1) — enforced by careful authoring, not a runtime check (no pool card divides a
    /// non-fixed amount, so `count.max` is always chosen to match the printed number).
    DealDamage {
        amount: Amount,
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        divided: bool,
    },
    /// The ability's controller draws `count` cards.
    DrawCards { count: Amount },
    /// The target *player* draws `count` cards (e.g. Ancestral Recall). Unlike [`DrawCards`],
    /// the drawer is the chosen target player rather than the controller.
    /// ponytail: a distinct variant (target Player) keeps a separate shape from `DrawCards`;
    /// fold the two together if a card ever needs "you or target player draws."
    /// `opponent`: `true` restricts the target to an opponent (CR "target opponent" — Secret
    /// Rendezvous); `false` (default) is the unrestricted "target player" (Ancestral Recall).
    TargetPlayerDraws {
        count: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        opponent: bool,
    },
    /// The ability's controller gains `amount` life.
    GainLife { amount: Amount },
    /// The ability's controller loses `amount` life (Reanimate's "you lose life equal to its
    /// mana value"). The loss-only sibling of [`GainLife`](Self::GainLife) — kept distinct
    /// rather than a negated `GainLife` amount, matching how [`EachOpponentLosesLife`] stays
    /// distinct from [`EachOpponentDrain`].
    LoseLife { amount: Amount },
    /// Swords to Plowshares' life-gain rider (CR: "Its controller gains life equal to its
    /// power"): the *target's* controller — not the ability's own controller — gains `amount`
    /// life. Reads the target's owner (the engine conflates control with ownership for
    /// permanents, same simplification as [`Event::ReanimatedToBattlefield`]'s note), which
    /// stays correct across the target's own zone change (`owner_of` follows `Object::Moved`).
    /// No target field of its own — shares the enclosing [`Sequence`](Self::Sequence)'s chosen
    /// target, so it must run *before* a step that removes the target from the battlefield if
    /// `amount` reads a battlefield-only characteristic like [`Amount::TargetPower`].
    GainLifeTargetController { amount: Amount },
    /// Manifest the top card of a player's library (CR 701.34 — Reality Shift's rider): that
    /// player puts their top card onto the battlefield face down as a 2/2 creature (a
    /// [`Permanent::face_down`] permanent). No target field of its own — the subject player is the
    /// enclosing [`Sequence`](Self::Sequence)'s chosen *target's* controller (Reality Shift
    /// manifests the exiled creature's controller's top card), read the same way
    /// [`GainLifeTargetController`](Self::GainLifeTargetController) reads the target's owner, so it
    /// runs after the `ExileTarget` step that moves the target (the id still resolves via
    /// `Object::Moved`). No-op if that player's library is empty (CR 701.34d).
    Manifest,
    /// The ability's controller adds `repeat` copies of a mana batch to their pool (marks a mana
    /// ability). `mana` is the multiset produced once (e.g. Sol Ring adds `{C}{C}`, a signet two
    /// mana); `repeat` scales it (Mana Geyser: `{R}` per tapped land an opponent controls), and
    /// defaults to one so a plain fixed batch needs no `repeat` in TOML.
    AddMana {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::mana_batch")
        )]
        mana: ManaPool,
        /// A count of "one mana of any color in your commander's color identity" credits (CR
        /// 903.4, Arcane Signet) this ability also adds, alongside `mana`'s fixed batch.
        /// Resolved to a real `Mana::Color`/`Mana::Either` credit at resolution time (see
        /// `effects.rs`) — the identity depends on the *controller's* commander, which isn't
        /// known until then, so it can't be baked into the static `mana` pool.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        identity: u8,
        /// A count of "one mana of any color that a land an opponent controls could produce"
        /// credits (Fellwar Stone, Exotic Orchard's `[[abilities]]`-authored form — most cards
        /// use the `produces = "opponent_colors"` land sugar instead) this ability also adds,
        /// alongside `mana`'s fixed batch. Resolved to a real `Mana::Color`/`Mana::OfColors`/
        /// `Mana::Any` credit at resolution time (see `effects.rs`) — the producible set depends
        /// on the current board, which isn't known until then.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        opponent_colors: u8,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one_amount"))]
        repeat: Amount,
        /// "Spend this mana only to..." (CR 106.9) — wraps every credit `mana` produces this
        /// resolution as [`Mana::Restricted`] (Troyan, Gutsy Explorer's `{G}{U}`). `None`
        /// (every ordinary `add_mana` ability) leaves the batch unrestricted. Never applies to
        /// the `identity`/`opponent_colors` credits — no pool card restricts one of those.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        restriction: Option<SpendRestriction>,
        /// "…of any one color" (CR 106.4): all of `mana`'s `any` credits this resolution are the
        /// *same* color, chosen by the controller when the ability resolves (Lotus Field's three
        /// mana, Kami of Whispered Hopes's power-many mana) — rather than each `Mana::Any` credit
        /// independently picking a color at payment time. `false` (every pre-existing `add_mana`
        /// ability) is unchanged. Handled at activation (see `Game::activate_ability`), which
        /// pauses on a [`PendingChoice::ChooseManaColor`] instead of resolving straight to mana.
        /// ponytail: only locks `mana`'s `any` credits — no pool card combines this with
        /// colorless/colored/identity/opponent-color credits in the same ability.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        single_color: bool,
        /// Record each credit this ability produces against its own source in
        /// [`Player::mana_provenance`](crate::state), so a later spell-cast payment can fire the
        /// source's [`Trigger::SpendManaToCast`] ("When you spend this mana to cast …" — Study
        /// Hall, Path of Ancestry, Opal Palace). `false` (every other mana ability) leaves the
        /// batch untracked. TOML `track_provenance = true`. Populated in `Game::activate_ability`
        /// (not this pure `&self` mint arm), which walks the resolved `ManaAdded`
        /// events knowing the ability's source.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        track_provenance: bool,
        /// A player this mana ability targets (Rousing Refrain's "target opponent", whose hand
        /// size a [`Amount::CardsInTargetPlayerHand`] `repeat` reads). [`TargetSpec::None`] (every
        /// ordinary mana rock/land/ritual) takes no target. Only ever set on a `Timing::Spell`
        /// ability — an *activated* mana ability can't target (CR 605.1a), and no pool card
        /// authors one that does.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target: TargetSpec,
        /// "Until end of turn, you don't lose this mana as steps and phases end" (CR 500.4
        /// exception; Rousing Refrain — the only pool card that prints it). `false` (every
        /// ordinary mana source) leaves the batch emptying at the next step/phase like usual.
        /// TOML `persist_until_end_of_turn = true`. Threaded onto each resulting
        /// [`Event::ManaAdded`]'s `persist` flag (see `effects.rs`'s mint arm).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        persist_until_end_of_turn: bool,
    },
    /// The target creature gets +power/+toughness and gains `keywords`, until end of turn
    /// (Brute Force's pump; Yahenni/Rogue's Passage's keyword-only grants, `power`/`toughness`
    /// both 0). Read back by [`Game::power`]/[`Game::toughness`]/[`Game::has_keyword`] via the
    /// target's `temp_power`/`temp_toughness`/`temp_keywords`, cleared at cleanup.
    PumpUntilEndOfTurn {
        power: Amount,
        toughness: Amount,
        target: TargetSpec,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        keywords: &'static [Keyword],
    },
    /// The ability's own source gets +power/+toughness until end of turn, no target (prowess's
    /// "this creature gets +1/+1 until end of turn", CR 702.108). Distinct from
    /// [`PumpUntilEndOfTurn`](Self::PumpUntilEndOfTurn), which targets a chosen creature — a
    /// self-pump has no target to choose, the source is already known at resolution.
    PumpSelfUntilEndOfTurn { power: Amount, toughness: Amount },
    /// A mass version of [`PumpUntilEndOfTurn`]: every creature the ability's controller
    /// controls gets +power/+toughness and gains `keywords`, until end of turn (Selfless
    /// Spirit's "creatures you control gain indestructible"; Moonshaker Cavalry's "creatures you
    /// control gain flying and get +X/+X", `X` via [`Amount::PerCreatureYouControl`]). No
    /// target — same no-target shape as [`WeakenEachCreature`](Self::WeakenEachCreature), but
    /// scoped to the controller's creatures and additive rather than every creature's malus.
    /// `filter` narrows which of the controller's creatures qualify (Quintorius, History
    /// Chaser's "−4: Spirits you control gain double strike and vigilance until end of turn" —
    /// `subtypes = ["Spirit"]`); absent (the [`PermanentFilter`] default) matches every creature,
    /// the pre-existing unfiltered behavior.
    PumpCreaturesYouControlUntilEndOfTurn {
        power: Amount,
        toughness: Amount,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        keywords: &'static [Keyword],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        filter: PermanentFilter,
    },
    /// A keyword-only mass grant to every permanent — creature or not — the ability's
    /// controller controls matching `filter`, until end of turn (Silkguard's "Auras, Equipment
    /// … you control gain hexproof until end of turn"). The noncreature-permanent twin of
    /// [`PumpCreaturesYouControlUntilEndOfTurn`](Self::PumpCreaturesYouControlUntilEndOfTurn) —
    /// same "you control" scan and `filter` narrowing, no P/T (Auras/Equipment have none to
    /// pump) and no creature gate, so `filter = { subtypes = ["Aura", "Equipment"] }` reaches
    /// noncreature permanents that clause has no way to touch.
    GrantKeywordsToPermanentsYouControlUntilEndOfTurn {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        keywords: &'static [Keyword],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        filter: PermanentFilter,
    },
    /// Every creature the ability's controller controls has base power and toughness set to
    /// `power`/`toughness` until end of turn (Biomass Mutation's "Creatures you control have base
    /// power and toughness X/X until end of turn"). A CR 613.3(7b) base-P/T SET (each qualifying
    /// creature's [`Permanent::base_pt_set_eot`], emitted as a `BasePtSet` layer applied before the
    /// 7c counters/pumps), cleared at cleanup. No target — same "you control" scan as
    /// [`PumpCreaturesYouControlUntilEndOfTurn`](Self::PumpCreaturesYouControlUntilEndOfTurn), but a
    /// base SET rather than an additive delta. When `other` is true the source itself is excluded
    /// from the scan (Tanazir Quandrix's "base power and toughness of *other* creatures you
    /// control").
    SetBasePtCreaturesYouControlUntilEndOfTurn {
        power: Amount,
        toughness: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        other: bool,
    },
    /// The target creature has base power and toughness set to `power`/`toughness` until end of
    /// turn (Quandrix Charm mode 2's "Target creature has base power and toughness 5/5 until end of
    /// turn"). The single-target twin of
    /// [`SetBasePtCreaturesYouControlUntilEndOfTurn`](Self::SetBasePtCreaturesYouControlUntilEndOfTurn),
    /// mirroring [`PumpUntilEndOfTurn`](Self::PumpUntilEndOfTurn)'s targeted shape.
    SetBasePtTargetUntilEndOfTurn {
        power: Amount,
        toughness: Amount,
        target: TargetSpec,
    },
    /// A manland self-animation (CR 613 — Restless Spire's "Until end of turn, this land becomes a
    /// 2/1 blue and red Elemental creature with first strike. It's still a land"): the ability's own
    /// source gains `add_types`/`add_subtypes` (CR 613.4 type layer), has its base P/T SET to
    /// `base_power`/`base_toughness` (CR 613.3(7b)), gains `keywords`, and gains `add_colors` (CR
    /// 105.2a's color-change analog of the same type layer), all until end of turn. No target — the
    /// source is the animated land. Written to the source's `added_types_eot`/`added_subtypes_eot`/
    /// `base_pt_set_eot`/`temp_keywords`/`added_colors_eot`, all cleared at the same cleanup choke.
    /// The "becomes a creature" self-twin of the targeted [`SetBasePtTargetUntilEndOfTurn`], but a
    /// noncreature-into-creature type change rather than a P/T set on an already-creature.
    AnimateSelfUntilEndOfTurn {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        add_types: TypeSet,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        add_subtypes: &'static [&'static str],
        base_power: i32,
        base_toughness: i32,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        keywords: &'static [Keyword],
        /// Colors granted to the animated permanent (Restless Spire's "blue and red"): unioned
        /// onto [`Game::colors_of`] while the animation is live. Empty (default) for a manland
        /// whose animated form stays colorless.
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        add_colors: &'static [Color],
    },
    /// The Impetus cycle's "other attacker" anthem (Martial Impetus's "Whenever enchanted
    /// creature attacks, each other creature that's attacking one of your opponents gets
    /// +1/+1 until end of turn"): fired by [`Trigger::EnchantedCreatureAttacks`], so `source`
    /// is the Aura and `controller` is the Aura's own controller (CR: "you" on an Aura's
    /// ability is its controller, not the enchanted creature's). Scans the committed attacker
    /// set at resolution — "other" excludes the enchanted creature (the Aura's host) itself,
    /// and "attacking one of your opponents" excludes an attacker whose declared defender is
    /// the Aura's controller. No target — every qualifying attacker is boosted, mirroring
    /// [`PumpCreaturesYouControlUntilEndOfTurn`]'s no-target shape.
    PumpOtherAttackersAttackingYourOpponents { power: i32, toughness: i32 },
    /// The Contract Aura token's granted ability (Scriv, the Obligator): "Whenever enchanted
    /// creature attacks, it gets +`power`/+`toughness` until end of turn if it's attacking one of
    /// your opponents. Otherwise, its controller loses `life` life." Fired by
    /// [`Trigger::EnchantedCreatureAttacks`], so `source` is the Aura and `controller` is its own
    /// controller (CR: "your" on an Aura's ability is its controller). Reads the enchanted host
    /// off `source`'s attachment; "one of your opponents" = the host's declared defender being a
    /// player other than the Aura's controller. No target — both branches act on the host / its
    /// controller. The single-branch [`PumpOtherAttackersAttackingYourOpponents`] anthem sibling
    /// has no else, so this carries its own conditional rather than composing two
    /// [`Conditional`](Self::Conditional)s.
    EnchantedAttackerPumpAttackingOpponentElseControllerLosesLife {
        power: i32,
        toughness: i32,
        life: u32,
    },
    /// Every creature the ability's controller's opponents control loses `keywords` and can't
    /// have them, until end of turn (CR 702.11e/702.18d — arcane_lighthouse's "creatures your
    /// opponents control lose hexproof and shroud and can't have hexproof or shroud"). No
    /// target — every qualifying opponent creature is affected, mirroring
    /// [`PumpCreaturesYouControlUntilEndOfTurn`](Self::PumpCreaturesYouControlUntilEndOfTurn)'s
    /// no-target shape but scoped to opponents instead of the controller. Resolves to one
    /// [`Event::KeywordsStripped`] per matching creature, unioned onto that permanent's
    /// [`Permanent::temp_lost_keywords`] and filtered out of every recompute (see
    /// [`Game::compute_effective_keywords_uncached`]) until cleanup.
    StripKeywordsFromOpponentsCreatures {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        keywords: &'static [Keyword],
    },
    /// A static ability: creatures the source's controller controls (matching `subtype`/
    /// `attacking_only`, if set) get +power/+toughness and/or gain `keywords` (CR 702.2
    /// deathtouch via a keyword-granting anthem — Ohran Frostfang; a subtype-filtered P/T
    /// anthem — Quintorius, Field Historian's "Spirits you control get +1/+0"). `power`/
    /// `toughness` are an [`Amount`] rather than a flat `i32` so a static can scale off a live
    /// board/graveyard count (Storm-Kiln Artist's "+1/+0 for each artifact you control", Wight
    /// of the Reliquary's "+1/+1 for each creature card in your graveyard") — resolved fresh on
    /// every characteristic recompute via [`Game::resolve_amount`], never cached. `self_only`
    /// restricts the anthem to its own source permanent (these two cards pump only themselves,
    /// not the whole team) rather than every creature the controller controls. Read during
    /// characteristic recompute, per candidate creature; it never resolves off the stack.
    AnthemStatic {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        power: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        toughness: Amount,
        /// Restrict the anthem to its own source permanent (Storm-Kiln Artist, Wight of the
        /// Reliquary — both pump only themselves, never their controller's other creatures).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        self_only: bool,
        /// Exclude the anthem's own source from the creatures it buffs (CR "**other** red
        /// creatures you control" — Balefire Liege/Creakwood Liege's color-split anthems).
        /// Distinct from `self_only` (which restricts the anthem *to* its source): this
        /// restricts it to everything *but* its source, still buffing the rest of the team.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        exclude_source: bool,
        /// Restrict the anthem to token creatures the controller controls (Brudiclad, Telchor
        /// Engineer's "Creature tokens you control have haste") — checked against
        /// [`Permanent::token`].
        #[cfg_attr(feature = "card-dsl", serde(default))]
        tokens_only: bool,
        /// Keywords granted to each matching creature (Ohran Frostfang's `[Deathtouch]`;
        /// empty for a pure P/T anthem).
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        keywords: &'static [Keyword],
        /// Restrict the anthem to creatures carrying any of these subtypes (Quintorius's
        /// `["Spirit"]`); empty matches every creature the controller controls. A slice like
        /// every other subtype filter in the pool (see [`CardFilter::LandWithSubtype`]) rather
        /// than a scalar `Option<&'static str>` — `&'static str` alone defeats serde's derive
        /// (it special-cases borrowed `&str`/`&[u8]` fields and pins the impl to
        /// `Deserialize<'static>` even behind `deserialize_with`; `&'static [&'static str]`
        /// doesn't hit that case, matching why [`CardDef::name`] needs a fully manual impl but
        /// a leaked slice field doesn't).
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        subtypes: &'static [&'static str],
        /// Restrict the anthem to creatures whose color set (CR 105.2, [`Game::colors_of`])
        /// intersects `colors` (Balefire Liege's "Other **red** creatures you control get
        /// +1/+1" / "Other **white** creatures…" — two separate anthem effects, one per
        /// color); empty (default) matches every color, same as every anthem before this axis
        /// existed.
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        colors: &'static [Color],
        /// Restrict the anthem to creatures of the source's own as-enters chosen creature type
        /// (CR 614.12/700.9-style — Patchwork Banner's "Creatures you control of the chosen
        /// type get +1/+1"), ANDed with `subtypes` if both are set (no pool card combines them).
        /// Reads the source permanent's [`Permanent::chosen_subtype`]; `None` (no choice made
        /// yet) matches no creature. `false` (default) doesn't gate on it.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        chosen_subtype: bool,
        /// Restrict the anthem to the controller's declared attackers this combat (Ohran's
        /// "Attacking creatures you control").
        #[cfg_attr(feature = "card-dsl", serde(default))]
        attacking_only: bool,
        /// Restrict the anthem to the controller's commander(s) (Guardian Augmenter's
        /// "Commander creatures you control get +2/+2").
        #[cfg_attr(feature = "card-dsl", serde(default))]
        commander_only: bool,
        /// Restrict the anthem to creatures that currently have any counter on them (CR 122.1's
        /// unqualified "counter" — Nev, the Practical Dean's "Creatures you control with
        /// counters on them have trample"). Read live per candidate by
        /// [`Game::has_any_counter`], same as every other axis here — no stored state.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        has_counters: bool,
        /// An "as long as …" gate on the whole anthem (tendershoot_dryad's "Saprolings you
        /// control get +2/+2 as long as you have the city's blessing") — `None` (default) is
        /// unconditional, matching every anthem that had no such clause before this field
        /// existed. Evaluated per anthem source against its own controller in
        /// [`Game::matching_anthems`], not the ability-level [`Ability::condition`] (which
        /// `matching_anthems` doesn't consult — that field gates a triggered ability's
        /// placement, not a static's continuous read).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        condition: Option<Condition>,
        /// The anthem functions from its source card's **graveyard**, not the battlefield (CR
        /// 603.6e continuous-analog — Anger's "As long as this card is in your graveyard and you
        /// control a Mountain, creatures you control have haste"). `false` (default) is an
        /// ordinary battlefield anthem. [`Game::matching_anthems`] pulls a `false` anthem only
        /// from battlefield permanents and a `true` one only from the owner's graveyard
        /// (`functions_in_graveyard`) cards — so a card carrying both (Vanguard of the Restless:
        /// a battlefield Spirit anthem + a graveyard-functional return trigger) keeps its
        /// battlefield anthem off in the graveyard, and Anger's graveyard anthem off on the
        /// battlefield.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        from_graveyard: bool,
    },
    /// A static ability that makes certain triggered abilities trigger an *additional* time (CR
    /// 603.3c — Harmonic Prodigy's "a triggered ability of a Shaman or another Wizard you control
    /// … triggers an additional time"; Veyran's magecraft-cause doubling). Read at trigger
    /// placement by [`Game::place_pending_triggers`] the same way [`AnthemStatic`](Self::AnthemStatic)
    /// is read at characteristic recompute — never resolved off the stack. Each doubler on the
    /// battlefield whose filter matches a triggered ability adds one more instance of it (two
    /// matching doublers → three instances total, CR 603.3c stacking additively).
    TriggerDoublingStatic {
        /// The triggering ability's SOURCE permanent must carry one of these subtypes (Harmonic:
        /// `["Shaman", "Wizard"]`). Empty (default) doesn't gate on source subtype. A leaked slice
        /// like [`AnthemStatic`](Self::AnthemStatic)'s `subtypes`, for the same serde reason.
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        source_subtypes: &'static [&'static str],
        /// Exclude the doubler's own source permanent from matching (Harmonic's "*another* Wizard",
        /// CR "another").
        // ponytail: excludes self across ALL its subtypes at once — faithful for Harmonic, whose
        // only self-overlap with `source_subtypes` is the Wizard half (it isn't a Shaman). A card
        // that both self-overlaps on one subtype and must still double itself on another would need
        // a per-subtype exclusion axis; none in the pool does.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        source_other: bool,
        /// The triggered ability must have been placed in the same event batch as an instant or
        /// sorcery cast/copy by the doubler's controller (Veyran's magecraft cause). `false`
        /// (default) doesn't gate on the cause.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        caused_by_instant_or_sorcery_cast: bool,
    },
    /// A continuous static prevention: "Prevent all noncombat damage that would be dealt to
    /// other creatures you control" (CR 615 — Tajic, Legion's Edge). A durationless permanent
    /// static, never resolved off the stack — scanned by
    /// [`Game::noncombat_damage_prevented_to_creature`] at each noncombat creature-damage choke
    /// (effect damage and fight damage, CR 701.12), the same posture as
    /// [`AnthemStatic`](Self::AnthemStatic). Combat damage is untouched, as is damage to the
    /// source itself ("*other* creatures") and to opponents' creatures. Fieldless, so `CardDef`
    /// stays `Copy`.
    PreventNoncombatDamageToOtherCreaturesYouControl,
    /// A static ability granting a matching permanent the controller controls an activated
    /// *mana* ability it doesn't otherwise have (CR 113.3/605 — Goldspan Dragon's "Treasures
    /// you control have '{T}, Sacrifice this artifact: Add two mana of any one color.'").
    /// Read live off the board by [`Game::granted_mana_abilities`] the same way
    /// [`AnthemStatic`](Self::AnthemStatic) is — recomputed on demand, no stored state, so the
    /// grant disappears the instant its source leaves. Addressed on the matching permanent past
    /// its own abilities (see [`Game::ability_at`]); it never resolves off the stack itself.
    /// Bounded to activated mana abilities (a flat cost + one mana batch, exactly what an
    /// `AddMana` ability needs) — granting an arbitrary nonmana ability would need a nested
    /// `Effect` and no pool card asks for it yet.
    GrantManaAbility {
        /// Which of the controller's permanents receive the ability (Goldspan's "Treasures";
        /// Galazeth's "Artifacts").
        filter: PermanentFilter,
        /// The granted ability's activation cost (Goldspan: tap + sacrifice this; Galazeth:
        /// tap only).
        cost: ActivationCost,
        /// The mana batch the granted ability produces, same spelling as [`AddMana`](Self::AddMana)'s
        /// own `mana` field.
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::mana_batch")
        )]
        mana: ManaPool,
        /// "Spend this mana only to..." (CR 106.9) on the granted ability — Galazeth Prismari's
        /// granted artifact mana is instant/sorcery-only. Same spelling and meaning as
        /// [`AddMana`](Self::AddMana)'s own `restriction` field.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        restriction: Option<SpendRestriction>,
    },
    /// A static ability (CR 402.2, e.g. Reliquary Tower): the source's controller has no maximum
    /// hand size, so the cleanup step's discard-to-hand-size turn-based action never triggers for
    /// them. A characteristic-defining continuous effect (CR 611) — read by
    /// [`Game::has_no_max_hand_size`] each cleanup step; it never resolves off the stack. Fieldless
    /// and targetless, like [`ControlAttached`](Self::ControlAttached).
    NoMaximumHandSize,
    /// A static ability (CR 118.9 — Serra Paragon): once during each of the source's controller's
    /// turns, they may play a land or cast a permanent spell with mana value 3 or less from their
    /// graveyard, and it gains "when this permanent is put into a graveyard from the battlefield,
    /// exile it and you gain 2 life." The permission itself is read live in
    /// [`Game::playable_zone`](crate::Game::playable_zone) (gated by the per-turn
    /// [`Player::graveyard_play_used_this_turn`](crate::Player) flag); this variant is just the
    /// battlefield marker that grants it. Fieldless and targetless, like the other static effects;
    /// it never resolves off the stack.
    PlayFromGraveyardOncePerTurn,
    /// A static cost-reduction ability (CR 118.9): spells the source's controller casts that
    /// match `filter` cost `amount` *generic* mana less (colored/`{C}` pips are never reduced),
    /// floored at zero. Read at cast time (see [`Game::cast`]); it never resolves off the stack.
    /// `amount` is a live [`Amount`] (not a bare number) so a reducer can scale off a board count
    /// — Pearl-Ear's affinity for Auras is `{ per_permanent = { subtypes = ["Aura"] } }`,
    /// resolved fresh against the reducer's own controller/source each cast (a negative resolve
    /// clamps to 0, same as any other cost reduction).
    ///
    /// `first_x_spell_each_turn` (Zimone, Infinite Analyst — "The first spell you cast with {X}
    /// in its mana cost each turn costs {1} less..."): when set, the reducer only applies to the
    /// controller's first spell this turn matching [`SpellFilter::HasXInCost`], gated in
    /// [`Game::cost_reduction`] against the existing [`Player::x_spells_cast_this_turn`] tally.
    /// ponytail: wired specifically to the {X}-spell tally (Zimone's `filter` is always `has_x`
    /// in the pool today) rather than a general per-filter "first matching this turn" counter —
    /// a differently-filtered once-per-turn reducer would need its own per-turn tally.
    ReduceSpellCost {
        amount: Amount,
        filter: SpellFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        first_x_spell_each_turn: bool,
    },
    /// A static "pillow-fort" attack tax (CR 802, Ghostly Prison): creatures can't attack this
    /// ability's controller unless the attacking creatures' controller pays `amount` generic for
    /// each creature they control that's attacking that player. Charged as an additional cost of
    /// declaring attackers (CR 508.1g) — read by [`Game::attack_tax_owed`]; it never resolves off
    /// the stack. Fieldless target, like the other static effects.
    AttackTax { amount: u8 },
    /// A static attack restriction (CR 509.1a — Combat Calligrapher, Eriette of the Charmed
    /// Apple): a permanent matching `filter` can't be declared attacking this ability's
    /// controller. Read by [`Game::declare_attackers`]; it never resolves off the stack. Fieldless
    /// target, like the other static effects.
    /// ponytail: models only "can't attack *you*" — the printed "or planeswalkers you control"
    /// clause is unobservable while attack targets are always a `PlayerId` (planeswalker defenders
    /// aren't modeled); wire the clause through when they land.
    CantBeAttackedBy { filter: PermanentFilter },
    /// "Prevent all combat damage that would be dealt to you this turn. For each 1 damage prevented
    /// this way, create a [`token`](Self::PreventCombatDamageToYouCreatingTokens::token)."
    /// (Inkshield, CR 615.) No target — the shield always protects the ability's own controller
    /// ("dealt to *you*"). Resolving it arms a this-turn [`combat_damage_prevention_shields`](crate::state::CombatExtras::combat_damage_prevention_shields)
    /// entry consulted at [`Game::damage_player`](crate::Game::damage_player); the tokens are
    /// minted there, at the moment of prevention (one per point), not here — at resolution zero
    /// combat damage has been prevented yet.
    PreventCombatDamageToYouCreatingTokens {
        /// The creature-token profile (Inkling: 2/1 white+black flying) minted once per point of
        /// combat damage prevented — a [`de::token_profile`] table, like [`CreateToken`](Self::CreateToken)'s.
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::token_profile"))]
        token: CardDef,
    },
    /// Place a vow counter on each battlefield creature matching `filter`, marking the ability's
    /// controller as the player it "can't attack … for as long as it has a vow counter on it"
    /// (Promise of Loyalty's rider, run as the sacrifice edict's `then`). Emits an
    /// [`Event::VowCountersPlaced`] per matching creature, recording that protected player on
    /// [`Permanent::vow_protected`]; the restriction itself is read live in
    /// [`Game::declare_attackers`], so it lifts on its own if the counter is later removed.
    /// ponytail: "each of those creatures" (the vow-marked survivors) is modeled as "every
    /// creature matching `filter` this resolution left on the battlefield" — after a keep-one
    /// creature edict every remaining creature is exactly a survivor, so the scan can't touch a
    /// non-survivor. Narrow to a threaded per-player survivor list if a card ever places vow
    /// counters without a preceding board-clearing edict.
    PlaceVowCounters { filter: PermanentFilter },
    /// Destroy the target creature (straight to the graveyard, ignoring toughness). The
    /// `target` spec also allows noncreature removal (an artifact/enchantment/planeswalker).
    /// `count` is the same multi-target surface [`ExileTarget`](Self::ExileTarget)'s `count` is
    /// (default `{1, 1}`, every existing single-destroy card unchanged) — Pest Infestation's "up
    /// to X target artifacts and/or enchantments" is `count = { min: 0, max: 0, x_scaled: true }`.
    DestroyTarget {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
        /// The "can't be regenerated" rider (CR 701.15d — Rapid Hybridization): the destruction
        /// turns off any regeneration shield on the target, so a shielded creature dies anyway.
        /// `false` (default) for an ordinary destroy, which a shield may replace (CR 701.15b).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        cant_be_regenerated: bool,
    },
    /// Grant the target creature a regeneration shield (CR 701.15b — a replacement effect: the
    /// next time it would be destroyed this turn, instead tap it, remove it from combat, and heal
    /// all its damage). Increments [`Permanent::regeneration_shields`]; a shield is consumed by
    /// the destroy path unless the destruction carries [`DestroyTarget::cant_be_regenerated`], and
    /// all shields expire at cleanup (CR 701.15b's "this turn").
    /// ponytail: a bare shield-granting primitive tested directly — no pool card grants a shield
    /// yet (Rapid Hybridization only *defeats* one). The upgrade path when a granter lands: a
    /// per-source shield ("regenerate {this}" via a shield the ability tracks) or a "regenerate up
    /// to one target creature" grant would reuse this same counter and destroy-time check.
    RegenerateShield { target: TargetSpec },
    /// Destroy every battlefield permanent matching `filter` (mass removal — Winds of Rath).
    /// Each goes to the graveyard (a commander diverts to the command zone); a token ceases to
    /// exist. Takes no target.
    DestroyAll { filter: PermanentFilter },
    /// Exile every battlefield permanent matching `filter` (mass exile — Oversimplify's "Exile
    /// all creatures"). Unlike [`DestroyAll`](Self::DestroyAll), indestructible does not save a
    /// permanent (CR 701.18a: exile isn't "destroy," CR 702.12b's protection is destroy-only);
    /// each goes to exile (a commander diverts to the command zone, CR 903.9b) and a token
    /// ceases to exist (CR 111.7). Takes no target.
    ExileAll { filter: PermanentFilter },
    /// Deal `amount` damage to each creature on the battlefield (Blasphemous Act = 13; Chain
    /// Reaction = a board-derived count). Damage is marked, then an SBA sweep clears the dead.
    /// `opponents_only` scopes the sweep to creatures controlled by opponents of the ability's
    /// controller (Volcanic Torrent's "each creature ... your opponents control"), the same axis
    /// as [`WeakenEachCreature`](Self::WeakenEachCreature)'s `opponents_only`.
    DamageEachCreature {
        amount: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        opponents_only: bool,
    },
    /// Each creature on the battlefield gets -`power`/-`toughness` until end of turn (Toxic
    /// Deluge = -X/-X). A creature reduced to 0-or-less toughness dies to the next SBA; survivors
    /// recover at cleanup like any until-end-of-turn boost. `opponents_only` scopes the sweep to
    /// creatures controlled by opponents of the ability's controller (Doomwake Giant's
    /// constellation: "creatures your opponents control get -1/-1").
    WeakenEachCreature {
        power: Amount,
        toughness: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        opponents_only: bool,
    },
    /// Put `count` +1/+1 counters on each of `targets`-many chosen target creatures. Permanent
    /// (unlike a pump): each adds +1/+1 via the additive recompute (ADR 0003). The quantity is a
    /// single replaceable step per target (CR 614) — see [`Game::counters_after_replacements`].
    /// `targets` is the same [`TargetCount`] multi-target surface as
    /// [`ReturnToHand`](Self::ReturnToHand)'s `count` (named differently here since `count`
    /// already means "counters per target") — the default `{1, 1}` is a single mandatory target;
    /// a "one +1/+1 counter on each of up to X target creatures" distribution (Silkguard's shape)
    /// is `count = 1, targets = { max = X }`. `kind = None` (default) is a +1/+1 counter, through
    /// the replaceable-counters-placement pipeline (CR 614); `kind = Some(k)` instead places
    /// `count` of `k`'s named counters in the kind-keyed map (Staff of the Storyteller's "put a
    /// story counter on this artifact") and **bypasses** that replacement pipeline entirely — a
    /// named counter kind has no doubler/Hardened-Scales interaction in the pool, mirroring
    /// [`EntersWithCounters`](Self::EntersWithCounters)'s own `kind` split. `divided` (default
    /// `false`) is [`DealDamage`](Self::DealDamage)'s `divided` twin (CR 601.2d): `count` becomes
    /// a total split across the chosen `targets` instead of a per-target amount, each target
    /// getting at least one (Grove's Bounty — Elusive Otter's adventure — "Distribute X +1/+1
    /// counters among any number of target creatures you control"). The split is recorded on
    /// [`Spell::counter_division`], settled right after targets are chosen (see
    /// [`Game::maybe_begin_counter_division`]) — a single chosen target skips the choice and
    /// takes the whole total. Only meaningful with `kind: None` (no pool card divides a named
    /// counter kind).
    PutCounters {
        count: Amount,
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        targets: TargetCount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        kind: Option<CounterKind>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        divided: bool,
    },
    /// Double the +1/+1 counters on the target: place as many more as it already has
    /// (Primordial Hydra's upkeep, Tanazir Quandrix's ETB). Zero counters doubles to zero (no
    /// event). Like [`PutCounters`](Self::PutCounters), the placement is one replaceable step
    /// (CR 614) — see [`Game::counters_after_replacements`].
    /// ponytail: no "this permanent, no target" shape exists, so a self-only doubler (Primordial
    /// Hydra) is expressed as a `target` the controller chooses among (creature/creature-you-
    /// control) rather than one pinned to the source; add a self target spec if a card needs the
    /// choice actually removed.
    DoubleCounters { target: TargetSpec },
    /// Fractal Harness's attack trigger: "double the number of +1/+1 counters on it [equipped
    /// creature]" — the no-target sibling [`DoubleCounters`](Self::DoubleCounters)'s own doc
    /// anticipates ("add a self target spec if a card needs the choice actually removed"), pinned
    /// to the permanent the ability's own source (this Equipment) is attached to rather than a
    /// chosen target. No-op if the source is currently unattached (guard-return) — Equipment can
    /// sit on the battlefield unequipped, and only an *equipped* creature's counters double.
    DoubleCountersOnAttachedCreature,
    /// Put `count` +1/+1 counters on each battlefield permanent matching `filter` (Mazirek,
    /// Kraul Death Priest: "put a +1/+1 counter on each creature you control"). Untargeted by
    /// default; each matching permanent's placement is its own replaceable step, same as
    /// [`PutCounters`](Self::PutCounters). `target_player` (default `false`): `true` evaluates
    /// `filter`'s `you`/`opponent` controller axis from a chosen Player target's perspective
    /// instead of the ability's own controller (Shadrix Silverquill's begin-combat "Target player
    /// puts a +1/+1 counter on each creature they control" — CR 111.4).
    PutCountersEach {
        filter: PermanentFilter,
        count: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target_player: bool,
    },
    /// Proliferate `times` times (CR 701.27): each time, choose any number of permanents
    /// and/or players that have a counter, then give each another counter of each kind
    /// already there. Pauses once per iteration on a [`PendingChoice::Proliferate`]. `times`
    /// is usually `Amount::Fixed(1)` (a plain "Proliferate."); Expansion Algorithm's "Proliferate
    /// X times" is `Amount::X`.
    /// ponytail: no player in this pool ever carries a counter (poison/energy aren't tracked),
    /// so the offered set is permanents-only — see [`Game::proliferate_options`]. Add players
    /// back in if a poison/energy card ever lands. (CR 701.27, CR 122)
    Proliferate { times: Amount },
    /// Move all counters of a kind (CR: not itself a keyword action — Nexus Mentality's "Move
    /// all counters"/Forgotten Ancient's "move any number of +1/+1 counters") from `target`
    /// (the moved-*from* permanent, chosen at cast/placement) onto a second permanent chosen at
    /// *resolution* — mirrors [`Fight`](Self::Fight)'s cast/resolution target split (see its
    /// doc; #31 real simultaneous multi-targeting is unlanded). `to_filter` scopes the
    /// resolution-time destination (excluding `target` itself, "onto *another*"). `all_kinds`
    /// (default `false`) moves every counter kind present (+1/+1 and every named kind — Nexus
    /// Mentality's unqualified "all counters"); `false` moves only +1/+1 (Forgotten Ancient's
    /// "+1/+1 counters" specifically).
    /// `distributed` (default `false`, CR 601.2d — Forgotten Ancient's "distributed as you choose
    /// among any number of target creatures") splits the move across any number of destinations
    /// instead of one: pauses on [`PendingChoice::DivideMovedCounters`] with a cap equal to
    /// `from`'s live +1/+1 count (a *move* can't exceed what's actually there, unlike
    /// [`PutCounters`](Self::PutCounters)'s divided `count`/`amount` total) and moves zero up to
    /// that many, each chosen destination getting at least one ("any number" permits zero). Only
    /// meaningful with `all_kinds: false` — no distributed-move pool card moves named kinds.
    MoveCounters {
        target: TargetSpec,
        to_filter: PermanentFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        all_kinds: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        distributed: bool,
        /// The already-resolved `target` (the source permanent), stashed across the
        /// resolution-time destination pause — never set in a card template.
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        from: Option<Target>,
    },
    /// Remove every counter (every kind) from `target`, then the controller draws one card per
    /// counter removed (Nexus Mentality's other mode: "Remove all counters from target nonland
    /// permanent you control. Draw a card for each counter removed this way.").
    RemoveAllCountersThenDraw { target: TargetSpec },
    /// A static +1/+1-counter *replacement* effect (CR 614): when counters would be put on a
    /// permanent this source's controller controls, `add` more are added and the total is then
    /// multiplied by `times`. Hardened Scales → `{ add: 1, times: 1 }`; a "twice that many"
    /// doubler (Doubling Season) → `{ add: 0, times: 2 }`. Read on demand as counters are placed
    /// (see [`Game::counters_after_replacements`]); it never resolves off the stack.
    CounterReplacement {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        add: i32,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one"))]
        times: i32,
        /// Exclude the replacement's own source from "another creature you control"
        /// doublers (Benevolent Hydra doesn't double its own counters).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        other: bool,
    },
    /// A static token-creation *replacement* effect (CR 614 — Doubling Season): when an effect
    /// would create one or more tokens under this source's controller, the number created is
    /// multiplied by `times` ("twice that many"). Read on demand as tokens are minted (see
    /// [`Game::token_count_after_replacements`]); it never resolves off the stack. Pure multiply —
    /// no `add`, and "under your control" is a recipient match, not a self-exclusion.
    TokenReplacement {
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one"))]
        times: i32,
    },
    /// A static life-gain *replacement* effect (CR 614 — Pest Rescuer): when this source's
    /// controller would gain life, they gain that much life plus `plus` instead. Read on demand as
    /// life is gained (see [`Game::life_gain_after_replacements`]); it never resolves off the
    /// stack. Pure addend — gaining 0 is not "gaining life", so the addend does not apply there.
    LifeGainReplacement {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        plus: i32,
    },
    /// A static cast-context X *modification* (CR 107.3 — Unbound Flourishing): when this source's
    /// controller casts a *permanent* spell whose mana cost contains `{X}`, the value of X is
    /// multiplied by `times` ("double the value of X"). Read on demand at the cast choke as the
    /// spell's `x` is frozen (see [`Game::cast_x_after_replacements`]); it never resolves off the
    /// stack. The doubled value is what downstream effects see (enters-with-X counters, `Amount::X`)
    /// — the cost was already paid at the announced X, so payment is untouched (the CR ordering).
    CastXReplacement {
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one"))]
        times: i32,
    },
    /// A static "this permanent enters with `amount` +1/+1 counters on it" ability (hydras:
    /// `amount = Amount::X`, the casting spell's `{X}`) — when `kind` is `None`, the default.
    /// Applied as the permanent enters (see [`Game::resolve_spell`]); the entry counters route
    /// through the same replacement path as any other placement, so a doubler / Hardened Scales
    /// grows a hydra's entry. `kind = Some(k)` instead places `amount` of `k`'s counters in the
    /// kind-keyed map (mana_bloom/astral_cornucopia's "enters with X charge counters") and
    /// **bypasses** the +1/+1 replacement pipeline entirely — a named counter kind has no
    /// doubler/Hardened-Scales interaction in the pool. Never resolves off the stack. (Spelled
    /// `count` in TOML, matching the other counter effects.)
    EntersWithCounters {
        #[cfg_attr(feature = "card-dsl", serde(rename = "count"))]
        amount: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        kind: Option<CounterKind>,
    },
    /// A static "as-enters" counter grant over *other* entering permanents (CR 614.1c — Gorma,
    /// the Gullet's third ability: "Nontoken creatures you control enter with an additional
    /// +1/+1 counter on them for each creature that died under your control this turn"). Unlike
    /// [`EntersWithCounters`](Self::EntersWithCounters), which is authored on the entering
    /// permanent's own card, this lives on a *different* permanent already on the battlefield and
    /// watches every permanent its controller casts/puts onto the battlefield. It does not modify
    /// its own permanent's entry — a static's ETB replacement isn't functioning until the
    /// permanent is on the battlefield (see [`Game::additional_enter_counters`]). `filter` narrows
    /// which entering permanents
    /// qualify ("nontoken creature" is `types = "creature", token = "nontoken"`); `count` is
    /// resolved per matching entry, read with the static's own permanent as `source` (so a
    /// source-relative amount like [`Amount::CreaturesDiedThisTurn`] resolves off the static's
    /// controller). Applied at the ETB counter choke (see [`Game::additional_enter_counters`]),
    /// summed with every other qualifying static, then routed through the same
    /// [`Game::counters_after_replacements`] doubler/Hardened-Scales pipeline as any other
    /// counter placement. Never resolves off the stack.
    CreaturesYouControlEnterWithCounters {
        filter: PermanentFilter,
        count: Amount,
    },
    /// Create `count` tokens with the characteristics of `token`, under `controller`'s control
    /// (default "you" — CR 111.4). Takes no target of its own — a `target_controller`
    /// `controller` reads the *ability's* shared target (typically shared with an earlier
    /// [`Sequence`](Self::Sequence) step, e.g. destroy-then-compensate). (`token` is a
    /// creature-token profile in TOML — see [`de::token_profile`] — not a full card table.)
    CreateToken {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::token_profile"))]
        token: CardDef,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one_amount"))]
        count: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        controller: TokenController,
        /// "Put X +1/+1 counters on it" (Deekah, Fractal Theorist's Magecraft Fractal): each
        /// minted token gets this many +1/+1 counters as it enters, routed through the same
        /// [`Game::counters_after_replacements`] doubler/Hardened-Scales pipeline as a spell's
        /// own `EntersWithCounters`. Defaults to `Amount::Fixed(0)` — every existing `create_token`
        /// TOML omits this key and mints with no counters, unchanged.
        #[cfg_attr(feature = "card-dsl", serde(default = "de::zero_amount"))]
        enters_with: Amount,
        /// "…create an X/X … token …, where X is …" (Manaform Hellkite): overrides the minted
        /// token's *base* power **and** toughness (a square X/X body) to this resolved `Amount`
        /// before it enters — a genuine base-P/T set, not [`enters_with`](Self::CreateToken::enters_with)'s
        /// counters, so the two differ under -1/-1 effects and counter removal. `None` (the
        /// default) leaves the token profile's printed P/T untouched — every existing
        /// `create_token` TOML omits this key, unchanged. Baked straight into the minted `CardDef`
        /// (see [`Game::run`]'s `CreateToken` arm) — no new `Permanent` field.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        set_base_pt: Option<Amount>,
        /// "Exile that token at the beginning of the next end step" (Manaform Hellkite, CR
        /// 603.7b): schedules a delayed [`Effect::ExileObject`] for each minted token, targeting
        /// that specific token — mirrors [`CreateTokenCopy`](Self::CreateTokenCopy)'s
        /// `sacrifice_at_next_end_step`. Defaults to `false` for a plain token that just persists.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        exile_at_next_end_step: bool,
        /// "…that attacking player creates a tapped … token … that's attacking that opponent"
        /// (Combat Calligrapher): the token enters tapped and attacking under the *attacking*
        /// player's control, per the [`Trigger::PlayerAttacksYourOpponent`] trigger's context —
        /// CR 508.4, a token put onto the battlefield attacking was never declared as an attacker.
        /// Only meaningful on an ability whose trigger populates [`TriggerContext::attack`].
        /// Defaults to `false` — every existing `create_token` TOML omits this key, unchanged.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        enters_tapped_and_attacking: bool,
        /// The baked `(attacker, attacked)` pair backing [`enters_tapped_and_attacking`](Self::CreateToken::enters_tapped_and_attacking)
        /// — filled by [`contextualize_effect`] from [`TriggerContext::attack`] at trigger
        /// placement, never authored in TOML (`de.rs` always constructs this `None`).
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        attacking_context: Option<(PlayerId, PlayerId)>,
        /// "…tokens … that attack that opponent this turn if able" (Furygale Flocking): each
        /// minted token gets a this-turn attack *requirement* ([`Event::MustAttackDeclared`])
        /// naming an opponent of the controller, enforced in [`Game::declare_attackers`] the same
        /// way a goad requirement is (CR 508.1a / 701.38a "if able"). Under `controller =
        /// "one_per_opponent"`, each opponent's own batch is bound to *that* opponent; every
        /// other `controller` value binds to the single flattened opponent (the one legal
        /// defending player in a 1v1 game). Defaults to `false` — every existing `create_token`
        /// TOML omits this key, unchanged.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        must_attack_defender: bool,
    },
    /// Create `count` Treasure tokens under the controller's control (CR: Treasure) — the common
    /// "create N Treasure(s)" effect. A Treasure is a colorless artifact token with
    /// "{T}, Sacrifice this artifact: Add one mana of any color" (see [`treasure_token`]). A thin
    /// wrapper over [`CreateToken`](Self::CreateToken)'s minting with a shared def.
    /// `target_player`: `false` (default) creates under the ability's controller ("you create");
    /// `true` creates under a chosen target player instead (Prismari Command's "target player
    /// creates a Treasure token" — CR 111.4). `tapped`: `false` (default) creates untapped;
    /// `true` creates the Treasures already tapped (Goldvein Hydra's "create a number of
    /// tapped Treasure tokens").
    CreateTreasure {
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one_amount"))]
        count: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target_player: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        tapped: bool,
    },
    /// Create `count` token(s) that are copies of the target creature (Rite of Replication):
    /// each token has the target's copiable characteristics — its printed `CardDef` (name, cost,
    /// types, P/T, keywords, abilities) — and enters under the controller's control, summoning-sick,
    /// firing ETB triggers via the same path as [`CreateToken`](Self::CreateToken).
    /// ponytail: copies only copiable values (CR 707.2) — no counters, tapped status, or
    /// attachments carry over, and "as a copy, except …" modifications (1/1 clones, +1/+1 riders)
    /// are out of scope; grow those from a card that needs them.
    CreateTokenCopy {
        target: TargetSpec,
        count: Amount,
        /// How many distinct targets are chosen at cast (CR 601.2c), the [`PutCounters::targets`]-
        /// style sibling of [`Self::count`](Self::CreateTokenCopy::count) (named apart from it —
        /// `count` stays "copies of *the* target," this is "how many targets"). The default `{1,
        /// 1}` is a single mandatory target, so every existing single-target
        /// `create_token_copy` is unchanged. Twinflame's "Choose any number of target creatures
        /// you control" is `{ strive_scaled = true }` (see [`TargetCount::strive_scaled`]):
        /// resolution mints one copy of *each* chosen target — the ordinary multi-target step
        /// expansion in `effects.rs`'s `resolve_spell` runs this arm once per target, so no
        /// special-cased iteration lives in the resolution arm itself.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        targets: TargetCount,
        /// CR 603.7's "sacrifice it at the beginning of the next end step" rider (populate's
        /// Determined Iteration/myriad-family cleanup): schedules a delayed
        /// [`Effect::SacrificeObject`] for each minted token, targeting that specific token (not
        /// a re-scan). Defaults to `false` for a plain copy that just persists.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        sacrifice_at_next_end_step: bool,
        /// "Exile that token at the beginning of the next end step" (Twinflame, CR 603.7b):
        /// schedules a delayed [`Effect::ExileObject`] for each minted token, mirroring
        /// [`Self::sacrifice_at_next_end_step`](Self::CreateTokenCopy::sacrifice_at_next_end_step)
        /// and [`CreateToken::exile_at_next_end_step`](Self::CreateToken::exile_at_next_end_step)
        /// — distinct from the sacrifice rider because exile skips dies-triggers (CR 700.4).
        /// Mutually exclusive with it in every pool card (no card sacrifices *and* exiles the
        /// same copy). Defaults to `false`.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        exile_at_next_end_step: bool,
        /// "The token created this way gains haste" (Determined Iteration): grants
        /// [`Keyword::Haste`] to each minted token via an until-EOT [`Event::TempBoost`].
        /// ponytail: the printed card grants haste permanently, but every card in scope pairs
        /// this with `sacrifice_at_next_end_step`/`exile_at_next_end_step` — the token never
        /// survives to see the grant expire, so until-EOT is behaviorally exact here. A token
        /// that keeps haste past this turn needs a permanent per-object keyword grant instead
        /// (unbuilt — grow it from a card that needs it).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        haste: bool,
    },
    /// Redoubled Stormsinger's attack trigger: "for each creature token you control that
    /// entered this turn, create a tapped and attacking token that's a copy of that token. At
    /// the beginning of the next end step, sacrifice those tokens." Unlike
    /// [`CreateTokenCopy`](Self::CreateTokenCopy), which copies one chosen target, this copies
    /// *every* battlefield permanent matching `token && creature && controller_you &&
    /// entered_this_turn` — no target of its own. Each mint reuses
    /// [`CreateToken`](Self::CreateToken)'s tapped-and-attacking rider
    /// (`Event::Tapped`/`Event::TokenEnteredAttacking`, CR 508.4 — never declared, so it can't
    /// re-trigger this or any other Attacks watcher) and `CreateTokenCopy`'s
    /// `sacrifice_at_next_end_step` delayed-trigger scheduling.
    CopyEachEnteredThisTurnTokenTappedAttacking {
        /// The `(attacker, defender)` pair backing the minted copies' tapped-and-attacking entry
        /// — filled by [`contextualize_effect`] from [`TriggerContext::attack`] at trigger
        /// placement, never authored in TOML (`de.rs` always constructs this `None`). Mirrors
        /// [`CreateToken::attacking_context`](Self::CreateToken::attacking_context).
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        attacking_context: Option<(PlayerId, PlayerId)>,
    },
    /// A static ability on an Aura/Equipment: while it is attached, its host creature gets
    /// +power/+toughness and gains `keywords`. Read during recompute (ADR 0003 — additive,
    /// no CR 613 layers), never resolved off the stack.
    GrantToAttached {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        power: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        toughness: Amount,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        keywords: &'static [Keyword],
        /// Whether the host is also goaded (CR 701.38a) for as long as this Aura stays
        /// attached — the Impetus cycle's "is goaded" clause. This is continuous, unlike the
        /// one-shot [`Event::Goaded`](crate::Event::Goaded)/`goaded` vec: read live by
        /// [`Game::is_goaded`]/[`Game::goaders_of`] off the attachment scan, so it needs no
        /// turn-boundary expiry and disappears the instant the Aura leaves.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        goad: bool,
        /// Flickering Ward's "Enchanted creature has protection from the chosen color": while
        /// attached, the host also has [`Keyword::ProtectionFrom`] of this Aura's own
        /// [`Permanent::chosen_color`] (the color named by its [`Effect::ChooseColor`] ETB). The
        /// scope can't ride the static `keywords` slice above because it's runtime state, not a
        /// print-time color — it's read live during the keyword recompute (grants nothing until a
        /// color is chosen). ponytail: a single card-specific "chosen color" axis, not a general
        /// grant-a-dynamically-scoped-keyword surface — grow that from a card that needs another. (CR 702.16, CR 702.21, CR 303.4)
        #[cfg_attr(feature = "card-dsl", serde(default))]
        protection_from_chosen_color: bool,
        /// An *activated* ability the Aura grants its host beyond statics/keywords (Fallen Ideal's
        /// "Sacrifice a creature: This creature gets +2/+1 until end of turn."). `None` for a
        /// statics-only Aura. Surfaced on the host by [`Game::granted_attachment_abilities`] and
        /// addressed past the host's own abilities by [`Game::ability_at`] — the non-mana twin of
        /// [`Effect::GrantManaAbility`]. Read live off the attachment scan, so the grant disappears
        /// the instant the Aura leaves.
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::opt_static_granted_ability")
        )]
        granted_ability: Option<&'static GrantedAbility>,
    },
    /// A static ability on an Aura: while it is attached, its host's *base* power/toughness
    /// becomes this fixed value (Darksteel Mutation's "base power and toughness 0/1"). Read
    /// during recompute (ADR 0003 — additive, no CR 613 layers): the set base replaces the
    /// printed base, then counters/pumps/anthems still add on top. Never resolved off the stack.
    /// ponytail: last-applied would win under CR 613 layer 7b, but the pool never stacks two
    /// set-base effects on one creature, so a single override is enough — grow ordering from a
    /// card that needs it.
    SetAttachedBasePT { power: i32, toughness: i32 },
    /// A static ability on an Aura: while it is attached, its host gains card types and/or has its
    /// creature subtypes changed (CR 613.4 type/subtype layer). `add_types` are unioned onto the
    /// host's printed types (Darksteel Mutation → +Artifact). `add_subtypes` are unioned onto the
    /// host's creature subtypes (Angelic Destiny → "is an Angel in addition to its other types").
    /// `set_subtypes`, when non-empty, *replaces* the host's own creature subtypes (Darksteel
    /// Mutation → "is an Insect", dropping the host's printed types). Read live during the
    /// type/subtype match chokes ([`Game::effective_types`]/[`Game::effective_subtypes`]), so the
    /// change reverts the instant the Aura leaves. Never resolved off the stack.
    /// `lose_all_abilities` (CR 613.1e/701 "loses all abilities") suppresses the *host's* own
    /// printed abilities and keyword abilities while attached (Darksteel Mutation → "it loses all
    /// other abilities"); the Aura's own grants (this type change, its base-P/T set, its
    /// `grant_to_attached` keywords) are unaffected — they sit after the removal in CR 613 order.
    /// Read live via [`Game::host_loses_all_abilities`], so it reverts the instant the Aura leaves.
    /// ponytail: takes the first such Aura per axis; the pool never stacks two type-changing Auras
    /// on one creature, so CR 613.7 dependency/timestamp ordering is deferred to the slice that
    /// needs it.
    SetAttachedTypes {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        add_types: TypeSet,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        add_subtypes: &'static [&'static str],
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        set_subtypes: &'static [&'static str],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        lose_all_abilities: bool,
    },
    /// A static ability on an Aura: while it is attached, its controller controls the host
    /// (a continuous control-changing effect, CR 720). Read by [`Game::controller_of`] as an
    /// additive override of the base owner (ADR 0003 — no CR 613 layers), so control reverts
    /// on its own when the Aura leaves. Never resolves off the stack.
    ControlAttached,
    /// A one-shot control change (CR 720): the ability's controller gains control of the
    /// target creature until end of turn (Besmirch). Unlike [`ControlAttached`], this isn't
    /// tied to an attached permanent — it's recorded in [`Game::control_overrides`] and
    /// reverts on its own at cleanup (CR 514.2), the same lifetime as
    /// [`PumpUntilEndOfTurn`](Self::PumpUntilEndOfTurn)'s boost.
    GainControlUntilEndOfTurn { target: TargetSpec },
    /// A permanent control change with no stated duration (CR 720 — Entrancing Melody's "gain
    /// control of target creature with mana value X"): unlike
    /// [`GainControlUntilEndOfTurn`](Self::GainControlUntilEndOfTurn), it is never reverted at
    /// cleanup. Recorded in [`Game::permanent_control_overrides`].
    GainControl { target: TargetSpec },
    /// Equipment's Equip ability (CR 702.6): attach this Equipment to the target creature its
    /// controller controls, detaching it from any prior creature. Sorcery-speed.
    Equip,
    /// Ajani's Chosen: "create a 2/2 white Cat creature token. If that enchantment is an Aura,
    /// you may attach it to the token." Attaches the triggering enchantment (`entering`, filled
    /// from [`TriggerContext::entering`] at trigger placement — `None` in a card template) to the
    /// token minted by the preceding [`CreateToken`](Self::CreateToken) step in the same
    /// [`Sequence`](Self::Sequence): read back the most recent [`Event::TokenCreated`] this same
    /// resolution already produced, mirroring [`UntapSearchedLand`](Self::UntapSearchedLand)'s
    /// "act on what this ability just made" pattern. A no-op if `entering` isn't an Aura
    /// (guard-return).
    /// ponytail: "you may" is modeled as always-yes — declining to attach your own Aura to your
    /// own token is never correct, so no `PendingChoice` is wired for the decline.
    AttachTriggeringAuraToMintedToken {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        entering: Option<ObjectId>,
    },
    /// A reflexive "when you do" triggered ability (CR 603.3b — Forum Filibuster's "When you do,
    /// return up to one target Aura or Equipment card from your graveyard to the battlefield
    /// attached to that token"). When this step resolves as part of its parent ability, it does
    /// NOT act inline: it enqueues each effect in `then` as a *separate* reflexive triggered
    /// ability that goes on the stack via the normal APNAP placement path
    /// ([`Game::place_pending_triggers`]) — a real, respondable stack object with its own priority
    /// window and its own target chosen at placement (CR 601.2c). The "you do" condition is that
    /// the parent's preceding [`CreateToken`](Self::CreateToken) step minted a token this
    /// resolution (read back from the most recent [`Event::TokenCreated`], the same idiom
    /// [`AttachTriggeringAuraToMintedToken`](Self::AttachTriggeringAuraToMintedToken) uses); no
    /// token, no reflexive trigger (guard-return). That minted token's id is threaded into each
    /// `then` effect ([`fill_reflexive_token`]) so its resolution can attach to it.
    /// ponytail: the only "you do" this recognizes today is the preceding token creation — the one
    /// pool consumer (Forum Filibuster). Generalize the condition when a card's reflexive "when you
    /// do" keys off a different action.
    ReflexiveTrigger {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },
    /// Forum Filibuster's reflexive-ability body, placed on the stack by
    /// [`ReflexiveTrigger`](Self::ReflexiveTrigger): "return up to one target Aura or Equipment
    /// card from your graveyard to the battlefield attached to that token." Its `filter`
    /// ("Aura or Equipment") drives a [`TargetSpec::CardInGraveyard`] target over the controller's
    /// own graveyard, chosen as the ability goes on the stack (CR 601.2c), declinable ("up to
    /// one"). At resolution the chosen graveyard card is returned and attached to `token` (the
    /// minted Inkling, threaded in at trigger placement). Guard-returns (CR 608.2b) if `token` no
    /// longer exists — with the host gone the Aura can't be attached, so it stays in the graveyard.
    ReturnFromGraveyardAttachedToToken {
        filter: CardFilter,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        token: Option<ObjectId>,
    },
    /// Shielded by Faith: "Whenever a creature enters, you may attach this Aura to that
    /// creature." Attaches the ability's own source (this Aura) to the entering creature
    /// (`entering`, filled from [`TriggerContext::entering`] at trigger placement — `None` in a
    /// card template). The "may" is the whole ability's `optional` flag
    /// ([`PendingChoice::MayYesNo`]), not a field here.
    AttachSelfToEntering {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        entering: Option<ObjectId>,
    },
    /// Animate Dead's "Return enchanted creature card to the battlefield under your control and
    /// attach this Aura to it": attaches the ability's own source (this Aura) to the creature
    /// this same resolution's own [`ReanimateToBattlefield`](Self::ReanimateToBattlefield) step
    /// just put onto the battlefield — read back the most recent
    /// [`Event::ReanimatedToBattlefield`] this resolution already produced, the same read-back
    /// pattern as [`AttachTriggeringAuraToMintedToken`](Self::AttachTriggeringAuraToMintedToken).
    /// No-op if no such event is in this resolution's history yet.
    AttachSelfToReanimated,
    /// Fractal Harness's ETB: "create a 0/0 green and blue Fractal creature token... and attach
    /// this Equipment to it." Attaches the ability's own source (this Equipment) to the token
    /// this same resolution's preceding [`CreateToken`](Self::CreateToken) step just minted —
    /// read back the most recent [`Event::TokenCreated`] this same resolution already produced,
    /// the same read-back idiom as [`AttachSelfToReanimated`](Self::AttachSelfToReanimated) (an
    /// Aura's reanimate-and-attach) one zone-change earlier. No-op if no token was minted this
    /// resolution (guard-return).
    AttachSelfToMintedToken,
    /// Scriv, the Obligator: "create a … Aura … token … attached to target creature an opponent
    /// controls." The mirror of [`AttachSelfToMintedToken`](Self::AttachSelfToMintedToken) —
    /// instead of attaching the ability's *source* to a minted token, it attaches the *minted Aura
    /// token* (read back as the most recent [`Event::TokenCreated`] this same resolution's
    /// preceding [`CreateToken`](Self::CreateToken) step produced) to the ability's chosen
    /// `target` (the enclosing [`Sequence`](Self::Sequence)'s shared target — a creature an
    /// opponent controls). Guard-returns if the token wasn't an Aura (CR 303 — only an Aura
    /// attaches) or the target isn't a battlefield object.
    AttachMintedAuraToTarget { target: TargetSpec },
    /// Gift of Immortality's second sentence: "Return this card to the battlefield attached to
    /// that creature at the beginning of the next end step." Reads the creature this same
    /// resolution's preceding [`ReanimateDyingEnchantedCreature`](Self::ReanimateDyingEnchantedCreature)
    /// step just reanimated — the most recent [`Event::ReanimatedToBattlefield`] this resolution
    /// already produced, the same read-back pattern as
    /// [`AttachSelfToReanimated`](Self::AttachSelfToReanimated) — and schedules
    /// [`ReturnThisAuraAttachedTo`](Self::ReturnThisAuraAttachedTo) (CR 603.7) against it via
    /// [`Event::DelayedTriggerScheduled`] at [`Step::End`]. No-op if no such event is in this
    /// resolution's history yet (the enchanted creature wasn't reanimated).
    ScheduleReturnThisAuraAttachedToReanimated,
    /// The delayed payload [`ScheduleReturnThisAuraAttachedToReanimated`](Self::ScheduleReturnThisAuraAttachedToReanimated)
    /// arms: return this Aura from the graveyard to the battlefield attached to `creature`
    /// (filled in when the delayed trigger was scheduled, never authored directly — the same
    /// synthetic-`then` shape as [`SacrificeObject`](Self::SacrificeObject)/
    /// [`ExileObject`](Self::ExileObject)). Guard-returns with no return if the Aura has since
    /// left the graveyard (moved/exiled some other way — CR 603.10a last-known information: it
    /// won't return) or `creature` no longer resolves to a battlefield permanent (destroyed
    /// before the delayed trigger fired).
    ReturnThisAuraAttachedTo {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        creature: Option<ObjectId>,
    },
    /// Screams from Within's death-return: "return this card from your graveyard to the
    /// battlefield" — unlike [`ReturnThisAuraAttachedTo`](Self::ReturnThisAuraAttachedTo)
    /// (attaches to the *same* look-back creature), this Aura's host stays dead, so the return
    /// enters unattached and pauses on [`PendingChoice::ChooseAttachHost`] (CR 303.4f) —
    /// the same choose-host pause a deployed Aura already uses — over the legal hosts on the
    /// battlefield. Guard-returns with no return if this Aura (`source`) has since left the
    /// graveyard (CR 603.10a last-known information). With no legal host, it stays unattached
    /// for the existing Aura-legality state-based action (CR 704.5m) to sweep.
    ReturnThisAuraFromGraveyardAttachedToChosenHost,
    /// Ghoulish Impetus's death-return: "return this card to the battlefield at the beginning of
    /// the next end step" — the delayed sibling of
    /// [`ReturnThisAuraFromGraveyardAttachedToChosenHost`](Self::ReturnThisAuraFromGraveyardAttachedToChosenHost),
    /// mirroring [`ScheduleReturnThisAuraAttachedToReanimated`](Self::ScheduleReturnThisAuraAttachedToReanimated)'s
    /// schedule-emit shape. Schedules that same fieldless effect against `source` at
    /// [`Step::End`] via [`Event::DelayedTriggerScheduled`]; the choose-host pause happens when
    /// the delayed trigger fires, not now.
    ScheduleReturnThisAuraFromGraveyardAttachedToChosenHost,
    /// Exile the target creature: move it from the battlefield to its owner's Exile zone.
    /// Swords to Plowshares' "its controller gains life equal to its power" rider is a separate
    /// [`GainLifeTargetController`](Self::GainLifeTargetController) step (`Amount::TargetPower`)
    /// sharing this effect's target, run *before* this step in the [`Sequence`](Self::Sequence)
    /// so the power reads while the creature is still on the battlefield. `count` is the same
    /// multi-target surface [`ReturnToHand`](Self::ReturnToHand)'s `count` is (default `{1, 1}`,
    /// every existing single-exile card unchanged) — Curse of the Swine's "exile X target
    /// creatures" is `count = { min: 1, max: 1, x_scaled: true }`. When a [`Sequence`](Self::Sequence)
    /// pairs this with a per-target rider (Curse of the Swine's "for each creature exiled this
    /// way, its controller creates a 2/2 Boar"), the multi-target expansion in `resolve_spell`
    /// re-runs the whole sequence once per chosen target, so the rider fires once per exile too.
    ExileTarget {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
    },
    /// The O-Ring pattern (CR 603.6e "linked" exile): exile the target permanent, recording the
    /// resolving ability's own source (an Aura, on the pool's two example cards) as the exile's
    /// duration. Recorded in [`Game::exiled_until_source_leaves`] via
    /// [`Event::ExiledUntilSourceLeaves`]; returned to the battlefield under its owner's control
    /// the moment the source leaves (see [`Game::check_linked_exile_returns`]).
    /// ponytail: a token targeted this way ceases to exist instead of being exiled (CR 111.7) —
    /// it's never linked, so there's nothing to return later.
    ExileUntilSourceLeaves { target: TargetSpec },
    /// Skyclave Apparition's linked exile (a sibling of [`ExileUntilSourceLeaves`](Self::ExileUntilSourceLeaves), not a
    /// fork of its state): exile the target, recording the resolving ability's own source
    /// linked to the exiled card in [`Game::exile_links`]'s `illusion_on_source_leave` list via
    /// [`Event::ExiledUntilSourceLeavesMintingIllusion`]. Unlike the O-Ring pattern the card is
    /// never returned — [`Game::check_leaves_battlefield_illusions`] instead mints the exiled
    /// card's owner an X/X blue Illusion token (X = the exiled card's mana value) the moment
    /// `source` leaves the battlefield, and the card stays exiled forever.
    /// ponytail: a token targeted this way ceases to exist instead of being exiled (CR 111.7),
    /// same as `ExileUntilSourceLeaves` — it's never linked, so nothing mints later.
    ExileTargetMintingIllusionOnLeave { target: TargetSpec },
    /// Impulse draw (CR 118.6 / 601.3e): exile the top `count` cards of the controller's library
    /// face-up and grant that controller permission to play them until end of turn. The cards stay
    /// in exile; the permission (tracked in [`Game::play_from_exile`]) expires at cleanup. Playing
    /// still obeys normal timing — a land only on your main phase with the land drop available, a
    /// spell only when you could cast it. Takes no target.
    ExileTopMayPlay {
        count: Amount,
        /// Extends the permission to "until the end of your next turn" (Atsushi, the Blazing
        /// Sky's die-trigger exile mode) instead of the plain "until end of this turn". The
        /// permission is shielded from cleanup until it arms at the controller's own next untap
        /// (`Event::PlayFromExilePermissionArmed`), mirroring how [`Event::Goaded`] tracks
        /// "until your next turn". Defaults to `false` (every other impulse-draw card).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        until_next_turn: bool,
    },
    /// Herald of Amity's ETB dig (CR 118.5 free cast, CR 701.17 exile): exile the top `count`
    /// cards of the controller's library face-up (public, unlike [`LookAtTop`](Self::LookAtTop)'s
    /// private look), pause on a choose-up-to-one over the exiled cards matching `filter`, and
    /// grant the chosen one [`Game::may_cast_from_exile_free`] permission (it stays in exile —
    /// unlike [`ExileTopMayPlay`](Self::ExileTopMayPlay)'s blanket normal-cost permission for the
    /// whole batch). Every other exiled card — declined, non-matching, or all of them if nothing
    /// is chosen — goes to the bottom of the controller's library (CR "put the rest on the
    /// bottom"). No pause at all when nothing exiled matches `filter` (a legal no-op offer,
    /// mirroring [`Game::begin_cast_exiled_with_this_free`]'s empty-pile case) — the rest-move
    /// still bottoms everything. Takes no target.
    /// ponytail: the free cast is only offered at a later priority window this turn, not
    /// mid-resolution of this effect (same approximation Quintorius, Loremaster's free-cast
    /// permission already carries). The bottomed rest is shuffled with the injected PRNG (CR
    /// "in a random order" — [`Game::bottom_exiled_dig`]). (CR 117, CR 108.3, CR 601.2c)
    ExileTopCastMatchingFree { count: u32, filter: CardFilter },
    /// Cascade (CR 702.85), placed as a triggered ability above the cascading spell when it's
    /// cast (see [`CardDef::cascade`](crate::CardDef::cascade)): reveal cards from the top of the
    /// controller's library one at a time, exiling each, until one is a **nonland** whose mana
    /// value is strictly less than `mana_value` (the cascading spell's own mana value, baked in at
    /// placement as last-known information — CR 702.85b), or the library runs out (CR 702.85c "as
    /// many as possible"). If a hit is found, pause on a may-cast-it-free choice (reusing
    /// [`ExileTopCastMatchingFree`](Self::ExileTopCastMatchingFree)'s wire shape,
    /// [`PendingChoice::ChooseExiledDigToCastFree`](crate::PendingChoice::ChooseExiledDigToCastFree)
    /// with the single hit as the only candidate); the free cast uses the same
    /// [`Game::may_cast_from_exile_free`] permission as the dig. Every exiled card not cast is put
    /// on the bottom of the library in a random order ([`Game::bottom_exiled_dig`]). Takes no
    /// target; only placed by the engine, never authored in TOML.
    /// ponytail: `mana_value` is a fixed `u32` (no pool cascade card has `{X}` in its cost, so the
    /// printed mana value is exact at placement); widen to an [`Amount`] if an `{X}`-cost cascade
    /// card is ever added. (CR 603, CR 108.3, CR 601.2c)
    Cascade { mana_value: u32 },
    /// Abstract Performance: exile the top four cards of the controller's library into one pile,
    /// then the next four into a second pile (both face-up, public — CR 701.17), an **opponent**
    /// chooses one pile (pausing on a [`PendingChoice::OpponentChoosesPile`](crate::PendingChoice::OpponentChoosesPile)),
    /// that pile goes to the controller's graveyard, and over the other pile the controller may
    /// cast up to one card free (reusing [`Game::may_cast_from_exile_free`]) with the rest going to
    /// hand ([`PendingChoice::ChooseExiledToCastFree`](crate::PendingChoice::ChooseExiledToCastFree)
    /// with `rest_to_hand`). Takes no target; only resolves via [`Game::run`] (needs the
    /// real library order and pauses).
    /// ponytail: the "face-down" first pile is modeled as an ordinary face-up exile pile — nothing
    /// in this engine observes an exiled card's face-down hidden-ness, and the mechanically
    /// meaningful part is which pile the opponent picks (CR 713 face-down cosmetics unmodeled).
    /// ponytail: "an opponent chooses" picks the next opponent in APNAP order; the controller does
    /// not get to choose *which* opponent when there are several (CR — you choose the opponent).
    /// The free cast is offered at a later priority window, not mid-resolution (same approximation
    /// the dig/free-cast family already carries). (CR 117, CR 108.3, CR 406.5)
    OpponentSplitsExilePiles,
    /// Plargg and Nassari's upkeep trigger: each player (APNAP order) exiles cards from the top of
    /// their own library until they exile a nonland card (all face-up, public), an **opponent**
    /// chooses one of the nonland cards exiled this way (pausing on a
    /// [`PendingChoice::OpponentChoosesExiledNonland`](crate::PendingChoice::OpponentChoosesExiledNonland)),
    /// and the controller may then cast up to two of the *other* exiled cards free (reusing
    /// [`Game::may_cast_from_exile_free`], via [`PendingChoice::ChooseExiledToCastFree`](crate::PendingChoice::ChooseExiledToCastFree)
    /// with `count = 2`). The picked card and any uncast cards simply stay in exile. Takes no
    /// target; only resolves via [`Game::run`] (needs the real library order and pauses).
    /// ponytail: same "an opponent chooses" APNAP-next and later-priority-window approximations as
    /// [`OpponentSplitsExilePiles`](Self::OpponentSplitsExilePiles). (CR 117, CR 108.3, CR 601.2c)
    EachPlayerExilesUntilNonlandOpponentPicks,
    /// A `Trigger::YouDiscard` payoff (CR 601 impulse play): exile the just-discarded card from
    /// the controller's graveyard and grant permission to play it until end of turn (Containment
    /// Construct's "you may exile that card from your graveyard. If you do, you may play that
    /// card this turn"). `card` — the discarded card's current graveyard-object id — is filled in
    /// from the triggering [`TriggerContext`] when the ability is placed (mirrors
    /// [`AttackerDrawsControllerCounters`](Self::AttackerDrawsControllerCounters)'s shape); it is
    /// `None` in a card template and never authored in TOML. Takes no target.
    ExileFromGraveyardMayPlay {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        card: Option<ObjectId>,
    },
    /// "Exile a card from your graveyard at random. You may play the exiled card this turn."
    /// (Advanced Reconstruction's base level). Unlike [`ExileFromGraveyardMayPlay`](Self::ExileFromGraveyardMayPlay),
    /// the card isn't handed in by a trigger context — it's chosen by the injected RNG at
    /// resolution, so this can only resolve via [`Game::run`] (`&mut self`, the only
    /// place [`Game::next_u64`] may be called), never the private mint path. Takes no target;
    /// no filter (the only consumer says "a card", any card — add a `CardFilter` axis if a second
    /// card needs one). CR 701.19 "if you can't" — an empty graveyard is a silent no-op.
    ExileRandomFromGraveyardMayPlay,
    /// A `Trigger::YouDiscard` payoff that exiles the just-discarded card into a *source-linked*
    /// exile pile instead of granting impulse-play permission (Currency Converter's "you may
    /// exile that card from your graveyard" — no "you may play it" clause, unlike
    /// [`ExileFromGraveyardMayPlay`](Self::ExileFromGraveyardMayPlay)). The card stays in exile,
    /// linked to this ability's own `source`, until [`CashOutExiledWithThis`](Self::CashOutExiledWithThis)
    /// pulls it back out (CR 400.10a "exiled with" tracking). `card` — the discarded card's
    /// current graveyard-object id — is filled in from the triggering [`TriggerContext`] when the
    /// ability is placed, the same shape as `ExileFromGraveyardMayPlay`'s `card`; `None` in a card
    /// template and never authored in TOML. Takes no target.
    ExileDiscardedWithThis {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        card: Option<ObjectId>,
    },
    /// Quintorius, Loremaster's end step: "exile target noncreature, nonland card from your
    /// graveyard" — a targeted counterpart to [`ExileDiscardedWithThis`](Self::ExileDiscardedWithThis):
    /// same source-linked pile (CR 400.10a), but the card is a chosen target rather than a
    /// just-discarded card. No fields: the target restriction (a noncreature, nonland card in the
    /// controller's own graveyard) is fixed, so [`Effect::target`] hardcodes the spec instead of
    /// storing one — flag-don't-force, only this card needs it.
    ExileTargetFromGraveyardWithThis,
    /// Renegade Bull's attack trigger: "exile up to one target instant or sorcery card from your
    /// graveyard and copy it. You may cast the copy without paying its mana cost." A targeted
    /// counterpart to [`ExileTargetFromGraveyardWithThis`](Self::ExileTargetFromGraveyardWithThis)
    /// (its `filter` is authored, not fixed, since this card's restriction is
    /// instant-or-sorcery rather than noncreature-nonland): the chosen card is exiled, then
    /// granted the free-cast permission (CR 118.5) via
    /// [`Event::CastFromExileFreePermissionGranted`] — the same plumbing
    /// [`CastExiledWithThisFree`](Self::CastExiledWithThisFree) (Quintorius) grants for its own
    /// chosen exiled card — so the controller may genuinely *cast* it (CR 601) at their next
    /// opportunity, firing real "whenever you cast" watchers (including this card's own first
    /// ability) instead of only Magecraft. `count` is `{0, 1}` ("up to one target," CR 601.2c):
    /// the target itself is declinable, unlike its fixed-single-target siblings above.
    /// ponytail: CR 707.10c's literal reading mints a copy and casts *that*; this casts the
    /// exiled card itself instead (the same "cast that card" shape `CastExiledWithThisFree`/
    /// `ExileTopCastMatchingFree` already use for their own free casts) — unobservable for this
    /// pool, since nothing here reads the resolved spell's post-resolution zone (a true copy
    /// ceases to exist there; the exiled original would go to its owner's graveyard); widen to a
    /// genuine minted-and-cast copy if a future card's post-resolution zone matters.
    ExileTargetGraveyardSpellCastFree {
        filter: CardFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
    },
    /// Surge to Victory: "Exile target instant or sorcery card from your graveyard." A targeted
    /// sibling of [`ExileTargetGraveyardSpellCastFree`](Self::ExileTargetGraveyardSpellCastFree)
    /// that exiles without minting a copy of its own: the chosen card is exiled and its object id
    /// and mana value are snapshotted onto [`Game::surge_exiled_card`] (overwritten per call,
    /// like [`Game::milled_mana_value_this_way`]) for a following [`Amount::ExiledCardManaValueThisWay`]
    /// team-pump step and [`ScheduleThisTurnCombatDamageCopy`](Self::ScheduleThisTurnCombatDamageCopy)
    /// arm step, both sharing this same resolution's [`Sequence`](Self::Sequence).
    ExileTargetGraveyardCardRecordManaValue { filter: CardFilter },
    /// Surge to Victory: "Whenever a creature you control deals combat damage to a player this
    /// turn, copy the exiled card. You may cast the copy without paying its mana cost." Arms a
    /// CR 603.7 delayed watch over the card this same resolution's
    /// [`ExileTargetGraveyardCardRecordManaValue`](Self::ExileTargetGraveyardCardRecordManaValue)
    /// step just exiled (read off [`Game::surge_exiled_card`]) — unlike
    /// [`ArmCombatDamageWatch`](Self::ArmCombatDamageWatch)'s one-shot single-creature watch, this
    /// is controller-scoped (any creature the controller controls, not one chosen target) and
    /// **repeatable** for the rest of the turn (CR "this turn", not "this combat"): each
    /// qualifying combat-damage event mints its own free copy via
    /// [`MintFreeCopyOfExiledCard`](Self::MintFreeCopyOfExiledCard), and the watch is never
    /// consumed — only cleared unconsumed at the next turn's Untap step (see
    /// [`Game::fire_combat_damage_copy_triggers`]). No fields: the pool's only consumer always
    /// reads the same resolution-scoped exiled card; no target of its own (rides the enclosing
    /// `Sequence`'s shared target, same no-target-of-its-own shape `ArmCombatDamageWatch` doc's).
    ScheduleThisTurnCombatDamageCopy,
    /// Fired by the delayed watch [`ScheduleThisTurnCombatDamageCopy`](Self::ScheduleThisTurnCombatDamageCopy)
    /// arms (Surge to Victory): mint one free copy of the already-exiled `card` onto the stack (CR
    /// 118.5), via [`Game::mint_spell_copies`] — the card left the graveyard back when the watch
    /// was armed, so this only mints, unlike [`ExileTargetGraveyardCardRecordManaValue`](Self::ExileTargetGraveyardCardRecordManaValue)'s
    /// own exile-and-snapshot step. `card` is filled in by
    /// [`Game::fire_combat_damage_copy_triggers`] when the delayed watch fires — `None` in a card
    /// template, never authored in TOML (same shape as
    /// [`ExileDiscardedWithThis`](Self::ExileDiscardedWithThis)'s own `card` field).
    MintFreeCopyOfExiledCard {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        card: Option<ObjectId>,
    },
    /// Restore Relic (Lorehold Archivist's back face): "Exile target artifact or creature card
    /// from your graveyard. Create a token that's a copy of it." A mandatory single-target
    /// counterpart to [`ExileTargetGraveyardSpellCastFree`](Self::ExileTargetGraveyardSpellCastFree):
    /// the chosen card is exiled, then one token copy of its copiable characteristics (CR 707.2)
    /// is minted onto the battlefield — [`Effect::CreateTokenCopy`]'s target-a-battlefield-permanent
    /// shape, but reading the def off a graveyard card instead.
    ExileTargetFromGraveyardCreateTokenCopy { filter: CardFilter },
    /// Feral Appetite: "{1}{G}: Exile target card from a graveyard. If a creature card is
    /// exiled this way, create a 1/1 black and green Pest creature token with 'When this token
    /// dies, you gain 1 life.'" An unrestricted counterpart to
    /// [`ExileTargetFromGraveyardWithThis`](Self::ExileTargetFromGraveyardWithThis) (no
    /// noncreature-nonland filter, any graveyard rather than just the controller's own — no
    /// authored `filter`, since the pool's only consumer targets any card): the chosen card is
    /// exiled, then `then` runs only if the just-exiled card's own printed type (CR "this way")
    /// is a creature card. `then` is `&'static [Effect]` so [`Effect`] stays `Copy`; the pool's
    /// only consumer puts a `CreateToken` step there.
    ExileTargetGraveyardCardThenIfCreature {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },
    /// Currency Converter's `{T}` cash-out: put one card from this source's exile pile (recorded
    /// via [`ExileDiscardedWithThis`](Self::ExileDiscardedWithThis) in [`Game::exiled_with`]) into
    /// its owner's graveyard, then create a Treasure if it was a land or a 2/2 creature token
    /// otherwise (CR 406.3 "owner's graveyard"). "Put a card" pauses on a
    /// [`PendingChoice::ChooseExiledWithCard`] over the pile — up to one, so an empty pile is a
    /// legal no-op activation (mirrors [`PutLandFromHand`](Self::PutLandFromHand)'s "up to one"
    /// shape). Takes no target; the source is intrinsic (`source` at resolution).
    /// ponytail: the nonland payoff is a plain colorless, subtype-less 2/2 creature token — CR
    /// wants a black Rogue, but token color/creature-subtype isn't modeled yet (the standing #10
    /// gap).
    CashOutExiledWithThis,
    /// Quintorius, Loremaster's activated ability: "Choose target card exiled with Quintorius.
    /// You may cast that card this turn without paying its mana cost." Pauses on a
    /// [`PendingChoice::ChooseExiledWithCardToCast`] over this source's exiled-with pile — "up to
    /// one," the same shape [`CashOutExiledWithThis`](Self::CashOutExiledWithThis) pauses on
    /// [`PendingChoice::ChooseExiledWithCard`] for, an empty pile being a legal no-op activation.
    /// Accepting grants [`crate::state::PlayPermissions::cast_from_exile_free`] for the chosen
    /// card (CR 118.5) instead of cashing it out — the card stays in the pile.
    /// ponytail: the card's "if that spell would be put into a graveyard, put it on the bottom of
    /// its owner's library instead" replacement rider isn't modeled (a replacement effect scoped
    /// to one cast object, #CR 128 territory) — the cast spell resolves/dies normally. (CR 602, CR 108.4, CR 601.2c)
    CastExiledWithThisFree,
    /// Return the target permanent(s) to their owners' hands (bounce). A token returned this way
    /// ceases to exist instead (CR 111.7 — it left the battlefield). `count` is how many distinct
    /// targets are chosen at cast (CR 601.2c): the default `{1, 1}` is a single mandatory target
    /// (Prismari Charm mode 2); Aether Gale's "six target" is `{6, 6}`. Each chosen target is
    /// re-checked for legality and bounced independently at resolution (CR 608.2b).
    ReturnToHand {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
    },
    /// Return the ability's own source to its owner's hand — no target, since the source is
    /// already known at resolution (Angelic Destiny's "return this card to its owner's hand").
    /// The usual home is an Aura's `EnchantedCreatureDies` trigger: by the time this resolves,
    /// the host is gone and the Aura has already been put into its owner's graveyard as a
    /// state-based action (CR 704.5m), so this finds the source wherever it now lives and moves
    /// it to hand. A no-op if the source has already left the game entirely.
    /// ponytail: a no-target self-return-from-wherever-it-is shape (#13 deferred it for lack of a
    /// consumer; Angelic Destiny is now that consumer). Assumes nothing else moves the source
    /// again before this trigger resolves — no pool card contests the graveyard in between. (CR 704, CR 303.4, CR 108.4)
    ReturnThisToHand,
    /// Guardian of Faith's ETB: "any number of other target creatures you control phase out" (CR
    /// CR 702.26). At resolution the ability's controller chooses any number (including zero) of the
    /// *other* creatures they control; each — and everything attached to it (CR 702.26g) — phases
    /// out (see [`Permanent::phased_out`](crate::state::Permanent)). Takes no fixed target: the set
    /// is chosen at resolution via [`PendingChoice::PhaseOut`](crate::PendingChoice), the same
    /// resolution-time subset choice [`Proliferate`](Self::Proliferate) uses.
    /// ponytail: targets are chosen at resolution rather than as the trigger goes on the stack (CR
    /// CR 603.3d) — a timing approximation, since no pool card responds to Guardian's specific
    /// phase-out targets. The filter is fixed to "other creatures you control" (Guardian is the
    /// only pool consumer); widen to an authored `PermanentFilter` when a second phaser needs one.
    PhaseOut,
    /// Kinetic Ooze's X≥10 rider: "double the number of +1/+1 counters on any number of other
    /// target creatures" (CR 601.2c). The multi-target sibling of the single-target
    /// [`DoubleCounters`](Self::DoubleCounters): `count` (an "any number" `{0, MAX_TARGETS}`) chosen
    /// distinct targets matching `target` (creatures, `other = true` excluding the Ooze itself), each
    /// doubled through the same [`Self::doubled_counters_event`]. As the ETB's *second* independent
    /// target clause, its targets are chosen as the trigger goes on the stack (CR 603.3d — see
    /// [`Game::place_ability_second_clause`]) and read at resolution from
    /// [`StackItem::Ability::targets_second`](crate::StackItem), so shroud/hexproof/protection are
    /// enforced and responders can react to the specific set.
    DoubleCountersOnTargetCreatures {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
    },
    /// Return the ability's own source from a graveyard to the battlefield under its owner's
    /// control (CR 603.6e — Nether Traitor's death-watch self-reanimation; CR 112.6/603.6e's
    /// activated twin — Teacher's Pest's `{B}{G}: Return this card ... to the battlefield
    /// tapped`). A no-target self-return twin of [`ReanimateToBattlefield`](Self::ReanimateToBattlefield):
    /// the source is a graveyard card, and it enters via the same ETB path as a reanimated
    /// creature. A no-op if the source has already left the graveyard (a race the pool never
    /// creates). `tapped` (default `false`) mirrors the printed "... to the battlefield tapped."
    ReturnThisFromGraveyardToBattlefield {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        tapped: bool,
    },
    /// Return every battlefield permanent matching `filter` to its owner's hand (mass bounce —
    /// Perplexing Test / Aether Gale). Each nontoken goes to *its own owner's* hand; a token
    /// ceases to exist instead (CR 111.7). The mass mirror of [`ReturnToHand`](Self::ReturnToHand);
    /// takes no target.
    ReturnAllToHand { filter: PermanentFilter },
    /// The target player puts the top `count` cards of their library into their graveyard.
    /// (Milling never triggers the draw-from-empty loss — an empty library just mills less.)
    Mill { count: Amount, target: TargetSpec },
    /// A targeted drain (Blood Artist): the target player loses `amount` life and the ability's
    /// controller gains `amount`. Uses [`TargetSpec::Player`], or [`TargetSpec::OpponentPlayer`]
    /// when `opponent` is set (Witherbloom Command mode 3 — "target opponent loses 2 life").
    DrainTarget {
        amount: i32,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        opponent: bool,
    },
    /// Exile every card in the target player's graveyard (CR 406 zone move) — Bojuka Bog's ETB,
    /// Remorseful Cleric's sacrifice ability. Fieldless: the target is intrinsic, like
    /// [`CounterTargetSpell`](Self::CounterTargetSpell). Uses [`TargetSpec::Player`].
    /// ponytail: a graveyard normally can't hold a commander (an SBA moves it straight to the
    /// command zone), so this is a plain exile move with no command-zone diversion check. (CR 704, CR 601.2c, CR 406.5)
    ExileGraveyard,
    /// Exile every card in *every* player's graveyard (CR 406 zone move) — Final Act's "Exile all
    /// graveyards" mode. The mass twin of [`ExileGraveyard`](Self::ExileGraveyard) (itself the mass
    /// twin of a targeted single-graveyard exile, mirroring how [`DestroyAll`](Self::DestroyAll)
    /// is the mass twin of [`DestroyTarget`](Self::DestroyTarget)): fieldless, no target.
    /// ponytail: same command-zone caveat as `ExileGraveyard` — no diversion check, since a
    /// graveyard can't hold a commander in this pool. (CR 704, CR 601.2c, CR 406.5)
    ExileAllGraveyards,
    /// An each-opponent drain (Zulaport Cutthroat): each opponent of the controller loses
    /// `amount` life and the controller gains a flat `amount` (not per-opponent) unless
    /// `sum_gain` is set. No target.
    EachOpponentDrain {
        amount: Amount,
        /// "You gain life equal to the life lost this way" (Exsanguinate) — the controller
        /// gains the *total* across every opponent's loss, not a flat per-opponent `amount`
        /// like Zulaport Cutthroat's "you gain 1 life". `false` (every pre-existing consumer:
        /// Zulaport Cutthroat, Silverquill Charm, Eriette of the Charmed Apple — all print a
        /// fixed gain number independent of opponent count) keeps the flat reading.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        sum_gain: bool,
    },
    /// Each opponent of the controller loses `amount` life (Dina, Soul Steeper). The
    /// lifegain-less sibling of [`EachOpponentDrain`](Self::EachOpponentDrain): a controller
    /// gain would re-trigger a "whenever you gain life" ability into a loop. No target.
    EachOpponentLosesLife { amount: Amount },
    /// Return the targeted creature card from a graveyard to its owner's hand (Raise Dead).
    /// The `target` scopes which graveyards are legal (typically your own).
    ReturnFromGraveyardToHand { target: TargetSpec },
    /// Put the targeted graveyard creature card onto the battlefield under the ability's
    /// controller's control (Reanimate/Animate Dead). Enters via the same ETB path as a cast
    /// permanent (summoning-sick, ETB triggers fire). Reanimate's "you lose life equal to its
    /// mana value" rider is a separate [`LoseLife`](Self::LoseLife) step (`Amount::TargetManaValue`)
    /// sharing this effect's target in a [`Sequence`](Self::Sequence) — `def_of` follows the
    /// target's `Object::Moved` redirect after this step re-mints it, so the mana value still
    /// reads correctly regardless of step order.
    /// ponytail: skips Animate Dead's aura body and its -1/-0 debuff (needs the reanimated
    /// card's characteristics read at resolution, plus an auto-attach — #79's attach-riders).
    ReanimateToBattlefield {
        target: TargetSpec,
        /// Whether the reanimated permanent enters with a finality counter (CR 614.12 — "if a
        /// permanent with a finality counter on it would die, exile it instead"). Excava, the
        /// Risen Past's reanimation rider; `false` for Reanimate/Sun Titan/Sevinne's, which don't
        /// grant one.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        finality: bool,
        /// An optional *indefinite* type/subtype/base-P/T/keyword set applied to the reanimated
        /// permanent as it enters (CR 611.2c — Excava's "It's a 1/1 Spirit creature with flying in
        /// addition to its other types"). `None` for a plain reanimation (Reanimate). When
        /// `Some`, an [`Event::ReanimatedCreatureBecame`] writes it onto the just-entered
        /// permanent's indefinite `set_base_pt`/`added_types`/`added_subtypes`/`granted_keywords`.
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::opt_static_reanimate_becomes")
        )]
        becomes: Option<&'static ReanimateBecomes>,
    },
    /// Put the targeted graveyard card into its owner's library — the bottom by default
    /// (Mistveil Plains's "{W}, {T}: Put target card from your graveyard on the bottom of your
    /// library"), or the top when `to_top` is set (Mystic Sanctuary's "put target instant or
    /// sorcery card from your graveyard on top of your library").
    TuckFromGraveyard {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        to_top: bool,
    },
    /// Return every card in the ability's controller's own graveyard matching `filter` to the
    /// battlefield under their control (Eiganjo Dynastorian's Replenish face: "return all
    /// enchantment cards from your graveyard to the battlefield"). Each enters via the same ETB
    /// path as [`ReanimateToBattlefield`](Self::ReanimateToBattlefield) (summoning-sick, ETB
    /// triggers fire), with no finality counter. Takes no target of its own — every matching
    /// graveyard card is affected, not a chosen one — the mass-return twin of
    /// [`ReturnAllToHand`](Self::ReturnAllToHand).
    MassReturnFromGraveyard { filter: CardFilter },
    /// Perpetual Timepiece's second ability: "Shuffle any number of target cards from your
    /// graveyard into your library" (`max = 0`, `target_player = false`), or Quandrix Command
    /// mode 3: "Target player shuffles up to three target cards from their graveyard into their
    /// library" (`max = 3`, `target_player = true`). Pauses on a
    /// [`PendingChoice::ShuffleFromGraveyard`] over the graveyard owner's whole graveyard — the
    /// controller (not the owner) picks any subset up to `max` (`0` = unbounded), each chosen
    /// card is put into its owner's library (reusing
    /// [`TuckFromGraveyard`](Self::TuckFromGraveyard)'s [`Event::TuckedToLibrary`]), then the
    /// library is shuffled (CR 701.19-style mandatory shuffle). `target_player` selects the
    /// graveyard owner: `false` is the ability's own controller (Timepiece, no target of its
    /// own); `true` reads the owner from a targeted player (Quandrix Command).
    /// ponytail: modeled as a resolution-time choice rather than a true cast/activation-time
    /// multi-target — [`Intent::ActivateAbility`] only carries a single `Option<Target>`, so a
    /// real "up to N target cards" declared at activation would need a new multi-target
    /// activation surface (its own increment); CR 601.2c's "up to N"/"any number" doesn't change
    /// which cards can end up affected based on *when* they're chosen, so the set of legal
    /// outcomes is identical either way.
    ShuffleTargetCardsFromGraveyardIntoLibrary {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        max: u32,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target_player: bool,
    },
    /// Chaos Warp: "The owner of target permanent shuffles it into their library, then reveals
    /// the top card of their library. If it's a permanent card, they put it onto the
    /// battlefield." One continuous sentence acting on the target's *owner* (not necessarily
    /// this effect's controller — see [`Game::run`]'s arm). A token tucked this way
    /// ceases to exist instead of entering a library (CR 111.7) — the same "can't exist off the
    /// battlefield" rule [`Game::sacrifice_event`]'s token check applies for a sacrifice.
    /// Deterministic given the shuffle's injected PRNG — no
    /// player choice at any point, so this only resolves via `Game::run`'s `&mut self`
    /// path (it needs the actual post-shuffle library order, not just a list of events to apply
    /// later — see [`Effect::ExileRandomFromGraveyardMayPlay`] for the same needs-`&mut self`
    /// shape).
    /// ponytail: one cohesive effect for the whole sentence, not split into a separate tuck +
    /// reveal-until-1; Chaos Warp is the only pool card that needs this exact shape today — split
    /// it only when a second card wants just the tuck half.
    ShuffleTargetPermanentIntoLibraryThenReveal { target: TargetSpec },
    /// Scry `count` (CR 701.42): the controller looks at the top `count` cards of their library,
    /// then puts any number of them on the bottom (in any order) and the rest back on top (in any
    /// order). Pauses on a [`PendingChoice::ArrangeTop`] rather than resolving to a fixed result.
    /// `count` is an [`Amount`] (not a bare `u32`) so a derived scry-X rider fits — Study Hall's
    /// "scry X, where X is the number of times your commander's been cast from the command zone."
    Scry { count: Amount },
    /// Surveil `count` (CR 701.43): like [`Scry`](Self::Scry), but the non-kept pile goes to the
    /// graveyard instead of the bottom of the library.
    Surveil { count: u32 },
    /// Look at the top `count` cards of the controller's library, select up to `up_to` of them
    /// that match `filter` into `dest`, and put the rest into `rest` (Quandrix Apprentice's
    /// magecraft: "look at the top three cards; you may reveal a land card from among them and put
    /// that card into your hand; put the rest on the bottom"). Selecting is a "may" — the
    /// controller may take fewer than `up_to`, including zero. Pauses on a
    /// [`PendingChoice::SelectFromTop`]. A short library (fewer than `count`) looks at what's there
    /// (CR 120-style "as many as possible"); an empty library is a clean no-op.
    LookAtTop {
        count: u32,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::any_card_filter"))]
        filter: CardFilter,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one_u32"))]
        up_to: u32,
        /// The mandatory floor on how many selected cards must go to `dest` (Dig Through Time's
        /// printed "put two of them into your hand", unlike Quandrix Apprentice's pure "may").
        /// Defaults to 0 (a pure "may" — every landed card before Dig Through Time). Bounded by
        /// how many cards were actually looked at (CR 120-style "as many as possible" on a short
        /// library) — see [`Game::select_from_top`].
        #[cfg_attr(feature = "card-dsl", serde(default))]
        min: u32,
        dest: TopDest,
        /// Whether a [`TopDest::Battlefield`] destination enters tapped (Armored Skyhunter's
        /// Aura/Equipment always enters untapped, so this stays `false` today; mirrors
        /// [`Effect::RevealUntil`]'s `matched_tapped`). Ignored when `dest` is
        /// [`TopDest::Hand`].
        #[cfg_attr(feature = "card-dsl", serde(default))]
        dest_tapped: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        rest: RestDest,
        /// A cap on the summed mana value of the *selected* cards (Ao, the Dawn Sky's dies mode 1:
        /// "put any number of nonland permanent cards with total mana value 4 or less … onto the
        /// battlefield"). `None` (the default — every look-at-top card before Ao) leaves the
        /// selection uncapped; `Some(n)` rejects an answer whose selected cards' summed
        /// [`CardDef::mana_value`] exceeds `n` (see [`Game::select_from_top`]). Independent of
        /// `up_to`/`min` (the count bounds) — Ao's "any number" sets `up_to = count`, `min = 0`,
        /// leaving the budget the only real bound.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        mv_budget: Option<u32>,
    },
    /// Look at the top `count` cards of the controller's library and route them one-per-slot
    /// across three fixed destinations (Expressive Iteration: "Put one of them into your hand,
    /// put one of them on the bottom of your library, and exile one of them. You may play the
    /// exiled card this turn"). `to_hand` go to hand, `to_bottom` to the library bottom,
    /// `to_exile_may_play` into exile with permission to play until end of turn (the same
    /// impulse-draw permission [`ExileTopMayPlay`](Self::ExileTopMayPlay) grants). Every slot is
    /// mandatory (unlike [`LookAtTop`](Self::LookAtTop)'s "may") — the controller assigns exactly
    /// that many of the looked-at cards to each slot, sharing none. Pauses on a
    /// [`PendingChoice::DistributeTop`].
    /// ponytail: fixed named slots, not a generic `&'static [TopDest]` list — Expressive Iteration
    /// is the only pool card that routes to three destinations at once. Grow toward a slot-list
    /// shape only when a second card needs a different destination mix.
    DistributeTop {
        count: u32,
        to_hand: u32,
        to_bottom: u32,
        to_exile_may_play: u32,
    },
    /// Reveal the top card of `defender`'s library — publicly (CR 701.30), unlike
    /// [`LookAtTop`](Self::LookAtTop)'s private look. If it matches `filter`, that player puts
    /// it into their hand; otherwise it stays on top, unchanged (Goblin Guide's attack trigger —
    /// "defending player reveals the top card of their library. If it's a land card, that player
    /// puts it into their hand"). `defender` is filled in from the attack trigger's context
    /// ([`TriggerContext::attack`]) when placed; `None` in a card template — Goblin Guide is the
    /// pool's only non-controller reveal subject so far.
    RevealTopToHand {
        filter: CardFilter,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        defender: Option<PlayerId>,
    },
    /// Open the Way (CR 701.30-style reveal, CR 120-style "as many as possible"): reveal the
    /// controller's own top cards one at a time until `count` cards match `filter`, or the
    /// library runs out. Each matching card goes to `matched_dest` (`matched_tapped` gates a
    /// `Battlefield` destination, mirroring [`SearchLibrary`](Self::SearchLibrary)'s `tapped`);
    /// every other revealed card goes to `rest_dest` — a fixed "bottom of library" today (see
    /// [`RestDest`]; widen it only when a second card needs a different rest zone). Reveals are
    /// public ([`Event::RevealedTopOfLibrary`]), unlike the private
    /// [`LookAtTop`](Self::LookAtTop). Fully deterministic given the library, so it makes no
    /// player choice.
    RevealUntil {
        filter: CardFilter,
        count: Amount,
        matched_dest: SearchDest,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        matched_tapped: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        rest_dest: RestDest,
    },
    /// Songbirds' Blessing's enchanted-creature-attacks trigger (CR 701.30/120): reveal the
    /// controller's own top cards one at a time — bottoming each non-match, the same per-card
    /// loop shape [`RevealUntil`](Self::RevealUntil) uses — until the first card matching
    /// `filter`, or the library runs out (CR 120.3 "as many as possible"). A hit pauses on a
    /// [`PendingChoice::RevealedCardToBattlefieldOrHand`](crate::PendingChoice::RevealedCardToBattlefieldOrHand)
    /// over exactly that card, left unmoved on top of the library until answered: accepting puts
    /// it onto the battlefield untapped, declining puts it into hand. A whiff (no match found
    /// before the library empties) is a legal no-op, no pause.
    /// ponytail: kept as its own effect rather than widening `RevealUntil` with a pause axis —
    /// `RevealUntil` is deliberately deterministic and makes no player choice (Open the Way's
    /// "put onto the battlefield" needs none); a routed "may" destination that *does* need a
    /// resolution-time choice earns its own small effect instead of breaking that contract.
    RevealUntilMayDeploy { filter: CardFilter },
    /// Creative Technique's reveal-until-nonland dig (CR 701.30/120), paired with a preceding
    /// [`ShuffleLibrary`](Self::ShuffleLibrary) step in the same ability's `Sequence`: reveal the
    /// controller's own top cards one at a time — bottoming each non-match, the same per-card
    /// loop shape [`RevealUntil`](Self::RevealUntil) uses — until the first card matching
    /// `filter`, or the library runs out. A hit is exiled face-up and pauses on the shared
    /// [`PendingChoice::ChooseExiledDigToCastFree`](crate::PendingChoice::ChooseExiledDigToCastFree)
    /// (reused from Herald of Amity's dig / Cascade): accepting grants the free-cast permission
    /// (CR 118.5), declining bottoms it. A whiff (an all-land library) is a legal no-op, no pause.
    /// ponytail: same reasoning as
    /// [`RevealUntilMayDeploy`](Self::RevealUntilMayDeploy) — kept separate from `RevealUntil` to
    /// preserve its no-pause contract.
    RevealUntilExileCastFree { filter: CardFilter },
    /// Shuffle the controller's own library (CR 701.19), no target — Creative Technique's
    /// "Shuffle your library, then reveal…" lead-in, run as the `[[abilities.effects]]` step
    /// ahead of [`RevealUntilExileCastFree`](Self::RevealUntilExileCastFree) in the same
    /// `Sequence`. Needs `&mut self` to draw from the injected PRNG, so it only resolves via
    /// `Game::run`, never the private mint path.
    ShuffleLibrary,
    /// Dance with Calamity's push-your-luck loop ("As many times as you choose, you may exile the
    /// top card of your library. If the total mana value of the cards exiled this way is `budget`
    /// or less, you may cast any number of spells from among those cards without paying their mana
    /// costs."). A player-driven, one-card-at-a-time exile: pauses on a
    /// [`PendingChoice::DanceExileMore`](crate::PendingChoice::DanceExileMore) before each exile,
    /// running a live tally of the exiled cards' summed [`CardDef::mana_value`]. When the caster
    /// stops (or the library empties), if the tally is `<= budget` the caster may cast any number
    /// of the exiled (nonland) cards for free (CR 118.5) — pausing on a
    /// [`PendingChoice::ChooseExiledToCastFree`](crate::PendingChoice::ChooseExiledToCastFree),
    /// `count` = the whole exiled pile, rest stays exiled; on a bust (`> budget`) nothing is
    /// offered and every exiled card stays exiled (the cards are exiled either way — a bust never
    /// returns them). Needs `&mut self` to read the live post-shuffle library order and hold the
    /// running tally across pauses, so it only resolves via `Game::run`.
    ExileTopUntilStopCastFreeUnderBudget { budget: u32 },
    /// Animist's Awakening (CR 701.30/120): reveal exactly `count` cards from the top of the
    /// controller's library — not "until N match" like [`RevealUntil`](Self::RevealUntil), the
    /// whole top `count` regardless of how many match. Every revealed card matching `filter`
    /// goes to `matched_dest` (`matched_tapped` gates a `Battlefield` destination, unless
    /// `deploy_untapped_if` holds — see below); every other revealed card goes to `rest_dest`. A
    /// short library reveals as many as possible (CR 120.3) rather than panicking. Reveals are
    /// public ([`Event::RevealedTopOfLibrary`]), fully deterministic given the library, so it
    /// makes no player choice.
    /// ponytail: "the rest on the bottom in a random order" (CR 701.19) is dropped — the engine
    /// is pure (no `rand`), so non-matching reveals go to the bottom in library order instead,
    /// the same deterministic stand-in [`RevealUntil`](Self::RevealUntil)/`select_from_top` use;
    /// upgrade to `Game::shuffle`'s injected PRNG once a resolution-time random reorder is wired
    /// through `run`'s `&mut self` path.
    RevealTopCards {
        count: Amount,
        filter: CardFilter,
        matched_dest: SearchDest,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        matched_tapped: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        rest_dest: RestDest,
        /// Deploy matched permanents untapped instead of `matched_tapped` when this condition
        /// holds (Animist's Awakening's spell mastery — "If there are two or more instant and/or
        /// sorcery cards in your graveyard, untap those lands"). CR-wise that's a second rider
        /// step after the lands enter tapped, but nothing can respond to the intermediate tapped
        /// state before the untap resolves as part of the same effect, so "enter tapped, then
        /// untap" and "enter untapped" are observably identical — this bakes the net result in
        /// directly rather than modeling a separate untap step. `None` = `matched_tapped` always
        /// applies.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        deploy_untapped_if: Option<Condition>,
    },
    /// Keen Duelist's upkeep trigger: the ability's controller and the chosen `target` opponent
    /// each reveal the top card of their library (CR 701.30, public), each loses life equal to
    /// the mana value of the *other* player's revealed card, then each puts the card they
    /// revealed into their own hand — through the same [`Event::SearchedToHand`] zone-move
    /// [`LookAtTop`](Self::LookAtTop)'s [`TopDest::Hand`] uses (not a draw — no draw-triggered
    /// ability sees it). A player whose library is empty reveals nothing, so the *other*
    /// player's life loss for that side is 0 (CR 120.3-style "as many as possible"). Uses
    /// [`TargetSpec::OpponentPlayer`].
    RevealTopAndDrainMutual,
    /// Breena, the Demagogue: the attacking player draws a card, and the ability's controller
    /// puts `counters` +1/+1 counters on a creature they control (CR: the ability's controller
    /// chooses the creature — see [`TargetSpec::CreatureYouControl`] in `default_target_spec`).
    /// The `attacker` (the drawing player) is filled in from the triggering context when the
    /// ability is placed; it is `None` in a card template.
    AttackerDrawsControllerCounters {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        attacker: Option<PlayerId>,
        counters: u32,
    },
    /// Parasitic Impetus's attack trigger: the enchanted creature's controller loses `amount`
    /// life, and this ability's controller (the Aura's controller) gains the same amount. The
    /// `attacker` (who loses the life) is filled in from the triggering context when the ability
    /// is placed; it is `None` in a card template — mirrors
    /// [`AttackerDrawsControllerCounters`](Self::AttackerDrawsControllerCounters)'s shape.
    AttackerLosesLifeYouGain {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        attacker: Option<PlayerId>,
        amount: u32,
    },
    /// Tomik, Wielder of Law's punisher: "that opponent loses `life_loss` life and you draw a
    /// card" — the attacking opponent (context) loses life; this ability's controller draws.
    /// Unlike [`AttackerLosesLifeYouGain`](Self::AttackerLosesLifeYouGain) the controller draws
    /// rather than gains life, so it's its own variant rather than a shared shape. The
    /// `attacker` (who loses the life) is filled in from the triggering context when the ability
    /// is placed; it is `None` in a card template.
    AttackerLosesLifeYouDraw {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        attacker: Option<PlayerId>,
        life_loss: u32,
    },
    /// Firemane Commando's second ability: "they draw a card if none of those creatures
    /// attacked you" — the attacking player (context), not this ability's controller, draws.
    /// The `drawer` is filled in from the triggering context when the ability is placed; it is
    /// `None` in a card template — mirrors [`AttackerLosesLifeYouDraw`](Self::AttackerLosesLifeYouDraw)'s
    /// shape, minus the life loss.
    AttackingPlayerDraws {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        drawer: Option<PlayerId>,
        count: u32,
    },
    /// Marauding Raptor: "Whenever another creature you control enters, this creature deals 2
    /// damage to it. If a Dinosaur is dealt damage this way, this creature gets +2/+0 until end
    /// of turn." Damage is aimed at the permanent that just entered (context), not a chosen
    /// target. The `entering` permanent's id is filled in from the triggering context when the
    /// ability is placed; it is `None` in a card template — mirrors
    /// [`AttackerDrawsControllerCounters`](Self::AttackerDrawsControllerCounters)'s shape.
    /// `then_if_subtype` is the optional "if a Dinosaur is dealt damage this way" gate: `then`
    /// runs only if the entering permanent's printed subtypes intersect `then_if_subtype` AND the
    /// damage actually landed (CR 119.3 "is dealt damage" — a protection/prevention shield that
    /// stops the damage also stops the rider). Empty (the default) never matches, so `then` never
    /// runs — a slice like [`AnthemStatic::subtypes`](Self::AnthemStatic) rather than a scalar
    /// `Option<&'static str>` (see that field's doc for why a bare `&'static str` defeats serde's
    /// derive), but with the opposite empty-case meaning: `AnthemStatic`'s empty means
    /// unrestricted, this empty means never. `then` is `&'static [Effect]` so [`Effect`] stays
    /// `Copy`, the same shape
    /// [`ExileTargetGraveyardCardThenIfCreature`](Self::ExileTargetGraveyardCardThenIfCreature)
    /// uses.
    DealDamageToEnteringPermanent {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        entering: Option<ObjectId>,
        amount: i32,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        then_if_subtype: &'static [&'static str],
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },
    /// A [`Trigger::EnchantedCreatureDies`] look-back payoff: reanimate the specific creature
    /// this Aura was attached to when it died (CR 603.10a last-known information — "that card"),
    /// under either this ability's own controller (`under_owner = false` — Changing Loyalty's
    /// "under your control") or the card's owner (`under_owner = true` — Gift of Immortality's
    /// "under its owner's control"). The dying card's id is filled in from the triggering context
    /// when the ability is placed; it is `None` in a card template — mirrors
    /// [`DealDamageToEnteringPermanent`](Self::DealDamageToEnteringPermanent)'s shape. Guard-
    /// returns with no reanimation if the context never filled a dying card, or if that card no
    /// longer sits in a graveyard (exiled in response — CR 603.10a: it won't return).
    ReanimateDyingEnchantedCreature {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        dying: Option<ObjectId>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        under_owner: bool,
    },
    /// A [`Trigger::CreatureYouControlDies`]-family look-back payoff (Hofri Ghostforge): exile the
    /// specific creature that just died (CR 603.10a last-known information — "exile it", the card
    /// now sitting in a graveyard), then mint a token that's a copy of it (CR 707.2 copyable
    /// values) under this ability's controller, adding `add_subtypes` on top of the copy's printed
    /// types ("except it's a Spirit in addition to its other types"). The dead creature's id is
    /// filled in from the triggering context when the ability is placed; it is `None` in a card
    /// template — mirrors [`ReanimateDyingEnchantedCreature`](Self::ReanimateDyingEnchantedCreature).
    /// Guard-returns with no mint if the context never filled a dead creature, or if that card no
    /// longer sits in a graveyard (exiled/moved in response — the "if you do" fails).
    /// ponytail: the copied `def` is the source's printed [`CardDef`], not the full CR 707.2
    ///   copyable-values snapshot (which would also capture copy-layer effects on the source). No
    ///   pool card copies a creature that is itself under a copy effect at death — grow the
    ///   snapshot from one that does.
    /// `leaves_returns_exiled`, when set, also emits an [`Event::TokenGrantedReturnExiledOnLeave`]
    /// linking the minted token to the exiled card — see [`Self::ReturnExiledCardToOwnersGraveyard`],
    /// the granted rider's payoff, and `Game::queue_token_return_exiled_trigger`, which places it.
    ExileDeadCreatureCreateCopyWithSubtype {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        dead: Option<ObjectId>,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        add_subtypes: &'static [&'static str],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        leaves_returns_exiled: bool,
    },
    /// The payload of Hofri Ghostforge's minted Spirit token's granted rider: "When this token
    /// leaves the battlefield, return the exiled card to its owner's graveyard" (CR 603.10a
    /// last-known information). Never authored in TOML — synthesized directly by
    /// `Game::queue_token_return_exiled_trigger` onto a [`Trigger::ThisPermanentLeavesBattlefield`]
    /// ability the same way [`Effect::MyriadTokenCopies`] is synthesized for Myriad, with `exiled`
    /// baked in at synthesis time from [`Game::exile_links`]'s `token_leaves_returns_exiled` link
    /// (itself recorded by an [`Event::TokenGrantedReturnExiledOnLeave`] at mint time). Guard-
    /// returns with no move if `exiled` is no longer in `Zone::Exile` (already reclaimed some
    /// other way) — the printed rider only returns a card that's still exiled.
    ReturnExiledCardToOwnersGraveyard {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        exiled: ObjectId,
    },
    /// Goad the target creature (CR 701.38): mark it goaded by this ability's controller until
    /// that controller's next turn. A goaded creature must attack each combat if able, and must
    /// attack a player other than a goader if able (both enforced in [`Game::declare_attackers`]).
    GoadTarget { target: TargetSpec },
    /// Copy the target instant or sorcery spell on the stack (Twincast). Puts a copy — a new
    /// [`Spell`] with the same `def`/`x`/`mode` but controlled by this effect's controller — on
    /// the stack above the original, so it resolves first. The copy is not cast (pays no cost)
    /// and ceases to exist rather than going to a graveyard when it resolves (CR 707.10a/CR 111.7).
    /// The copier is then offered CR 707.10c's "you may choose new targets for the copy" (see
    /// the `Effect::CopyTargetSpell` arm in `effects.rs`).
    /// ponytail: modeled as a *mandatory* re-pick (choosing the same target back is always
    /// legal), not a true optional choice — CR 707.10c also lets the copier keep a target that
    /// has since become illegal, which a forced re-pick can't express. No pool spell can make
    /// that edge observable (nothing gets priority between the copy's mint and its retarget).
    CopyTargetSpell,
    /// A storm/Gravestorm-style copy rider (CR 706.9): mint `count` copies of *this resolving
    /// spell itself* (not a chosen target), each offered the same CR 707.10c retarget choice
    /// `CopyTargetSpell` offers when the copied ability has a target. `count` is resolved once,
    /// then minted one copy at a time through [`Game::run_sequence`]'s pause/resume
    /// machinery, so each copy's retarget choice is answered before the next copy mints (see the
    /// `Effect::CopyThisSpell` arm in `effects.rs`). A spell that is itself a copy never
    /// re-triggers this rider (a copy is never cast, so CR 706.9's "when you cast this spell"
    /// doesn't fire for it).
    /// ponytail: resolves as one of this spell's own resolution effects (last in its ability's
    /// sequence), not CR 706.9's true "copy when cast, copies resolve before the original" stack
    /// ordering — no pool storm/Gravestorm card has a response between cast and resolution that
    /// would see the difference.
    CopyThisSpell {
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one_amount"))]
        count: Amount,
        /// Gate the rider on "if this spell was cast from a graveyard" (CR — Sevinne's
        /// Reclamation's flashback copy): `true` skips the mint unless the resolving spell's
        /// [`Spell::flashback`] is set. `false` (default) mints unconditionally, as every
        /// existing storm/Gravestorm consumer (Ominous Harvest, Plumb the Forbidden) already does.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        cast_from_graveyard_only: bool,
        /// Whether the rider is itself optional ("you MAY copy this spell"): pauses on a
        /// [`PendingChoice::MayYesNo`] before minting, mirroring [`MaySacrifice`](Self::MaySacrifice)/
        /// [`MayReturnFromGraveyard`](Self::MayReturnFromGraveyard)'s "declining runs nothing"
        /// resolution-time optional shape — declining mints no copy. `false` (default) mints
        /// unconditionally, as every existing storm/Gravestorm consumer already does.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        optional: bool,
    },
    /// Internal continuation step for [`CopyThisSpell`](Self::CopyThisSpell): offer one
    /// already-minted spell copy's CR 707.10c retarget choice. `copy` is a runtime object id
    /// (the copy `CopyThisSpell` just minted), never a card-template value — not meant to be
    /// authored directly in a card TOML.
    RetargetSpellCopy { copy: ObjectId },
    /// A delayed one-shot's copy payoff (CR 603.7/707.10 — Thunderclap Drake's "when you next
    /// cast an instant or sorcery spell this turn, copy it for each time you've cast your
    /// commander from the command zone this game. You may choose new targets for the copies."):
    /// mint `count` copies of the spell that fired the armed
    /// [`Effect::ScheduleNextCastTrigger`] watch — not a chosen target
    /// ([`CopyTargetSpell`](Self::CopyTargetSpell)) and not this ability's own spell
    /// ([`CopyThisSpell`](Self::CopyThisSpell)), since the copying ability is itself a separate
    /// triggered ability, not the cast spell's own resolution. `triggering_spell` is a runtime
    /// object id baked in by [`contextualize_effect`]/`fill_triggering_spell` when the watch
    /// fires (from [`TriggerContext::triggering_spell`]) — `None` in a card template. Guard-
    /// returns with no copies if the triggering spell already left the stack (countered in
    /// response, CR 603.4) before this delayed trigger resolved — the delayed trigger goes on
    /// the stack *above* the triggering spell (CR 603.3), so ordinarily it's still there.
    CopyTriggeringSpell {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        triggering_spell: Option<ObjectId>,
        count: Amount,
        /// CR 707.10c's "you may choose new targets for the copies": `true` (every current
        /// consumer) offers the same mandatory re-pick [`CopyTargetSpell`](Self::CopyTargetSpell)/
        /// [`CopyThisSpell`](Self::CopyThisSpell) already offer. `false` mints each copy keeping
        /// the triggering spell's own targets instead — CR 707.10c's declined case, not exercised
        /// by any pool card yet.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        may_choose_new_targets: bool,
    },
    /// Mirrorwing Dragon's watch payoff (CR 707.10 — "that player copies that spell for each
    /// other creature they control that the spell could target. Each copy targets a different
    /// one of those creatures."): mint one copy of `triggering_spell` per *other* creature its own
    /// controller (not this ability's controller — "that player copies") controls that the spell
    /// could legally target, each retargeted to a distinct one of those creatures. `triggering_spell`
    /// is a runtime object id baked in by [`contextualize_effect`]/`fill_triggering_spell` when
    /// [`Trigger::SpellTargetsThisOnly`](crate::Trigger::SpellTargetsThisOnly) fires — `None` in a
    /// card template. Guard-returns with no copies if the triggering spell already left the stack
    /// (countered in response, CR 603.4), same shape as [`CopyTriggeringSpell`](Self::CopyTriggeringSpell).
    /// ponytail: "could target" is read as "is a creature the spell's controller controls, other
    /// than the original target, that passes the spell's own target legality (hexproof/protection)" —
    /// exact for the pool's single-target instant/sorcery consumers. ponytail: CR 707.10's "each
    /// copy targets a different one" distinct assignment is engine-chosen (enumeration order), not
    /// offered to the player as a choice — no pool response window sees the difference (the same
    /// posture [`CopyThisSpell`](Self::CopyThisSpell) documents for its own retarget ordering).
    CopyTriggeringSpellForEachOtherCreatureYouControl {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        triggering_spell: Option<ObjectId>,
    },
    /// Unbound Flourishing's second-ability payoff for the *ability* half (CR 707.10): "copy that
    /// ability" — mint one copy of the activated ability that fired the [`Trigger::ActivateAbility`]
    /// watch, put on the stack above the original (CR 707.10c — the copy isn't "activated"),
    /// carrying its source/effect/target/`{X}` value (CR 706.10 copies the value as-is, so an
    /// already-doubled X isn't re-doubled). `triggering_ability` is the original ability's source
    /// permanent, baked in by [`contextualize_effect`]/`fill_triggering_ability` when the watch
    /// fires (from [`TriggerContext::triggering_ability`]) — `None` in a card template. Guard-
    /// returns with no copy if the triggering ability already left the stack (countered in
    /// response, CR 603.4/707.10c); ordinarily the watch's trigger sits directly above it
    /// (CR 603.3b), so it's still there.
    /// ponytail: `may_choose_new_targets` (CR 707.10c) mints the copy keeping the original's
    /// targets — the "you may choose new targets" re-pick isn't offered for the ability half. No
    /// pool card is a *targeted* `{X}`-cost activated ability, so Unbound never copies one whose
    /// target could be re-chosen; add a copy-scoped retarget pause when such a card ships.
    /// ponytail: single-consumer primitive (only Unbound Flourishing copies an ability today) —
    /// the source disambiguation ("the topmost stack ability with this source") is exact while
    /// that holds; key copies by a real ability-instance id if a second consumer needs it.
    CopyTriggeringAbility {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        triggering_ability: Option<ObjectId>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        may_choose_new_targets: bool,
    },
    /// Demonstrate (CR 702.147), placed as a triggered ability above the cast spell (see
    /// [`CardDef::demonstrate`](crate::CardDef::demonstrate)): "you may copy it. If you do, choose
    /// an opponent to also copy it. Players may choose new targets for their copies." `spell` is
    /// the cast spell's own object id, baked in at placement (like [`Cascade`](Self::Cascade)'s
    /// `mana_value`) — never authored in TOML. Resolution pauses on a
    /// [`PendingChoice::MayYesNo`](crate::PendingChoice::MayYesNo) ("copy it?"); declining copies
    /// nothing. Accepting mints one copy under the controller via
    /// [`Game::mint_spell_copies`](crate::Game::mint_spell_copies) (offering the usual CR 707.10c
    /// retarget), then pauses the controller on a
    /// [`PendingChoice::ChooseTarget`](crate::PendingChoice::ChooseTarget) to pick an opponent, who
    /// gets a second copy the same way. Takes no target of its own; only placed by the engine.
    /// ponytail: CR 707.10c/702.147a's copies are true simultaneous objects with the "choose new
    /// targets" order following APNAP; this mints and retargets the controller's copy fully before
    /// the opponent's copy exists, matching [`CopyThisSpell`]'s existing "resolves as one of this
    /// spell's own steps, not the true stack-ordering" approximation. No pool Demonstrate card has
    /// a response window between the two copies that would see the difference.
    Demonstrate { spell: ObjectId },
    /// Opal Palace's spend-to-cast rider: "If you spend this mana to cast your commander, it enters
    /// with a number of additional +1/+1 counters on it equal to the number of times it's been cast
    /// from the command zone this game." Resolves off a [`Trigger::SpendManaToCast`] that fires at
    /// cast payment (CR 601.2), so the commander spell is still on the stack; the counters can't be
    /// placed until it resolves into a permanent, so this records `(spell, count)` on
    /// [`Game::pending_enter_bonus_counters`](crate::state) for `resolve_spell` to drain as the
    /// permanent enters. `triggering_spell` is the commander spell's stack object id baked in by
    /// [`contextualize_effect`]/`fill_triggering_spell` when the trigger fires — `None` in a card
    /// template. Guard-returns if that spell already left the stack (countered in response, CR
    /// 603.4) before this trigger resolved.
    CommanderEntersWithBonusCounters {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        triggering_spell: Option<ObjectId>,
        count: Amount,
    },
    /// Counter the target spell on the stack (CR 701.5 / 405.9): remove it from the stack and put
    /// it into its owner's graveyard, so it never resolves. The classic Counterspell effect; the
    /// target ([`TargetSpec::SpellOnStack`]) is intrinsic, so no `target` field is needed.
    /// `unless_pays`, when set, is the "unless its controller pays {N}" clause (Quandrix Charm):
    /// resolution pauses on a [`PendingChoice::PayOrCounter`] for the *target spell's* controller
    /// instead of countering outright. `None` (the common case) is a plain unconditional counter.
    /// `filter` restricts which spells are legal targets (Decisive Denial's "target noncreature
    /// spell", Quandrix Command's "target artifact or enchantment spell"); defaults to
    /// [`SpellFilter::AllSpells`], the classic "counter target spell".
    CounterTargetSpell {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        unless_pays: Option<Amount>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        filter: SpellFilter,
    },
    /// Fight (CR 701.12): the ability's controller's creature and a target creature they don't
    /// control each deal damage equal to their power to the other, simultaneously. The printed
    /// card targets both creatures at cast; the engine only threads one target through a
    /// spell/ability ([`Spell::target`]/a modal mode), so [`Effect::target`] maps this to the
    /// *opponent's* creature (the cast-time target — [`TargetSpec::Permanent`] scoped to
    /// [`FilterController::Opponent`]) and the controller's own creature is chosen at
    /// *resolution* instead, via a [`PendingChoice::ChooseTarget`] pause that mirrors how a
    /// triggered ability picks its target ([`Game::place_targeted_ability`]). `enemy` carries the
    /// already-resolved opponent creature through that pause; it's always `None` in a card
    /// template and filled in by [`Game::run`].
    /// ponytail: a real fight targets both creatures at cast (CR 601.2c/601.2d) — choosing the
    /// second at resolution instead is unobservable, since no pool card can respond between the
    /// two choices; grow true simultaneous multi-targeting (#31) if one ever needs to.
    ///
    /// `ally_is_shared_target` (default `false`) is the mirror shape for a pump-then-fight card
    /// (Primal Might): the *ally* (not the enemy) is the ability's shared cast target — already
    /// chosen and pumped by a preceding [`Sequence`](Self::Sequence) step — so [`Effect::target`]
    /// returns [`TargetSpec::None`] for this variant (it defers to that earlier step) and
    /// resolution instead pauses on an *optional* [`PendingChoice::ChooseTarget`] ("fights up to
    /// one target creature you don't control") over [`FilterController::You`]'s complement. No
    /// legal enemy (or the ally itself no longer being a creature, CR 608.2b) is a guard-return —
    /// no pause, no fight, the pump still stands.
    /// ponytail: same resolution-time-second-target ponytail as above, mirrored — unobservable
    /// for the same reason (no pool card can respond between the pump and the fight).
    Fight {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        enemy: Option<Target>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        ally_is_shared_target: bool,
    },
    /// Schedule `then` as a delayed triggered ability (CR 603.7 — "at the beginning of the next
    /// end step / turn's upkeep, do X"): it goes on the stack the next time *any* step matching
    /// `fire_at` begins, not this resolution. `who` is resolved to a concrete player *now* — the
    /// ability's own controller, or the controller of this ability's shared target spell (Arcane
    /// Denial's just-countered spell, read via [`Game::controller_of`], which still works once
    /// the spell has moved to the graveyard). Takes no target of its own: `who =
    /// TargetSpellController` reads the *sequence's* shared target, the same shape as
    /// [`SearchScope::TargetController`].
    /// ponytail: `fire_at` only covers `Upkeep`/`End` — the two timings the pool actually
    /// schedules to (Arcane Denial's draws, the token-copy family's delayed sacrifice); widen the
    /// drain (`Game::fire_delayed_triggers`) if a card ever needs a third step. (CR 603, CR 111, CR 108.3)
    ScheduleAtNextUpkeep {
        who: DelayController,
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_effect"))]
        then: &'static Effect,
        /// Which step's beginning fires the delayed trigger. Defaults to `Upkeep` (CR 603.7's
        /// usual "next turn's upkeep") so every landed card omits this key unchanged.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        fire_at: Step,
    },
    /// Arm a CR 603.7 delayed *one-shot* triggered ability that fires the next time its
    /// controller casts a spell matching `filter` **this turn** (Brass Infiniscope's "When you
    /// next cast a spell with {X} in its mana cost this turn, you draw a card and gain half X
    /// life, rounded down"). Unlike [`ScheduleAtNextUpkeep`](Self::ScheduleAtNextUpkeep) — armed
    /// against a future *step* — this arms against a future *event* (a matching cast), drained
    /// by [`Game::fire_next_cast_triggers`], removed the moment it fires (CR 603.7's "next" — at
    /// most once) and cleared unconsumed at the next turn's Untap step if no matching cast
    /// happens first (CR 603.7's implicit "this turn" duration). `then`'s `Amount::X`/
    /// `Amount::HalfXRoundedDown` resolve against the *triggering spell's* chosen `{X}` (CR
    /// 603.4 last-known information), the same [`TriggerContext::cast_x`] threading
    /// [`Game::queue_cast_spell_triggers`] already uses. Always arms for the ability's own
    /// controller/source — no pool card needs Arcane Denial's "someone else's spell" shape here.
    ScheduleNextCastTrigger {
        filter: SpellFilter,
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_slice"))]
        then: &'static [Effect],
    },
    /// Arm a CR 603.7 delayed watch on the ability's own chosen target creature (Stensian
    /// Sanguinist's "Whenever that creature deals combat damage to a player this combat, this
    /// creature becomes prepared"): the ability's source becomes prepared the first time the
    /// shared `target` deals combat damage to a player, any time later this combat (fired by
    /// [`Game::fire_combat_damage_watch_triggers`]), cleared unconsumed at end of combat (CR
    /// "this combat" — [`Game::apply`]'s `Step::EndCombat` arm). Object-scoped like
    /// [`ScheduleNextCastTrigger`](Self::ScheduleNextCastTrigger) is filter-scoped, but reads no
    /// `filter` of its own — it watches the *specific creature* this same resolution's target
    /// picked, not a class of spells. No `then` field: the pool's only consumer always resolves
    /// to the same fixed [`BecomePrepared`](Self::BecomePrepared); widen this into a
    /// `then: &'static [Effect]` if a future card arms a different delayed effect this way.
    /// ponytail: CR 603.7's exact "this combat" window is approximated as "until this combat's
    /// `EndCombat` step begins" — indistinguishable for the pool (no card cares about the sliver
    /// between the last combat damage step and `EndCombat` itself).
    ArmCombatDamageWatch,
    /// Search the controller's library for up to `count` cards matching `filter`, move each to
    /// `to_zone` (`tapped` if it enters the battlefield) as it's found, then shuffle once (CR
    /// 701.19, CR 701.19f — one search finding multiple cards shuffles only after the last one).
    /// Pauses on a [`PendingChoice::SearchLibrary`], re-pausing (over the shrinking match set)
    /// after each pick until `count` is reached or the searcher fails to find. Powers tutors
    /// (`Hand`), basic-land ramp, and fetchlands (`Battlefield`); `count > 1` powers "up to N"
    /// searches (Land Tax's three basics, Springbloom Druid's two).
    /// ponytail: our model reveals nothing to opponents, and "fail to find" (choosing none) ends
    /// the whole search early — always legal (CR 701.19c allows failing to find any or all), but
    /// it can't express "decline this match, keep searching for another" separately.
    SearchLibrary {
        filter: CardFilter,
        to_zone: SearchDest,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        tapped: bool,
        /// Whose library is searched (default: the ability's own controller).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        searcher: SearchScope,
        /// The maximum number of cards this one search may find (default 1 — the common single
        /// tutor/fetch). "Up to N" (Land Tax, Cultivate) sets this above 1.
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one_u8"))]
        count: u8,
        /// Where every find *after the first* goes, if different from `to_zone` (Cultivate: "put
        /// one onto the battlefield tapped and the other into your hand" — `to_zone =
        /// Battlefield`, `overflow = Some(Hand)`). `None` (default) is today's single-destination
        /// search: every find goes to `to_zone`. `tapped` still applies only to a `Battlefield`
        /// destination — a card routed to `Hand` has no tapped concept.
        /// ponytail: only first-vs-rest is modeled (one card needs exactly that split); a real
        /// per-pick destination *list* is the generalization if a card ever wants a third
        /// destination.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        overflow: Option<SearchDest>,
    },
    /// A multi-player sacrifice edict (Deadly Brew, Promise of Loyalty, Priest of Forgotten
    /// Gods): each affected player (`scope`) loses `life_loss` life, then chooses which of their
    /// permanents matching `filter` to sacrifice — one each (or, with `keep_one`, keeps one and
    /// sacrifices the rest). The choices are made in APNAP order, each pausing on a
    /// [`PendingChoice::SacrificeEdict`]; after the last player chooses, `then` runs for the
    /// edict's controller (Priest's "add {B}{B} and draw a card"). `then` is `&'static` so
    /// [`Effect`] stays `Copy`.
    EachPlayerSacrifices {
        #[cfg_attr(feature = "card-dsl", serde(default = "de::all_players"))]
        scope: EdictScope,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        keep_one: bool,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::creature_edict"))]
        filter: PermanentFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        life_loss: i32,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },
    /// A multi-player graveyard-exile fan-out (Augusta, Order Returned: "each player exiles a card
    /// from their graveyard"): each player, in APNAP order, exiles one card from their own
    /// graveyard (mandatory when they have any), pausing on a
    /// [`PendingChoice::ExileFromGraveyard`]. How many of the exiled cards were *nonland* is
    /// snapshotted onto [`Game::nonland_cards_exiled_this_way`] for a following `Sequence` step to
    /// read via [`Amount::NonlandCardsExiledThisWay`] (Augusta's "put that many +1/+1 counters"
    /// payoff). The payoff rides in the enclosing `Sequence`, resumed once every player has
    /// answered (the same deferred-tail path a pausing sequence step uses) — so, unlike
    /// [`EachPlayerSacrifices`](Self::EachPlayerSacrifices), this carries no `follow_up` of its own.
    EachPlayerExilesFromGraveyard,
    /// "Target player exiles a card from their graveyard" (Relic of Progenitus): the *targeted*
    /// player, not the caster/activator, picks one card from their own graveyard — mandatory when
    /// non-empty, a no-op when empty. Pauses on the same [`PendingChoice::ExileFromGraveyard`]
    /// [`EachPlayerExilesFromGraveyard`](Self::EachPlayerExilesFromGraveyard) uses, with a single
    /// player and no `remaining` — the one-player special case of that fan-out. No payoff.
    TargetPlayerExilesFromGraveyard { target: TargetSpec },
    /// The caster-directed keep-one-of-each-type sweep (Tragic Arrogance: "For each player, you
    /// choose from among the permanents that player controls an artifact, a creature, an
    /// enchantment, and a planeswalker. Then each player sacrifices all other nonland permanents
    /// they control."). For each living player in APNAP order, the effect's controller (the caster)
    /// picks up to one of that player's nonland permanents of each relevant type to keep, pausing on
    /// a [`PendingChoice::CasterKeepPermanents`]; every other nonland permanent that player controls
    /// is then sacrificed by its controller (CR 701.16b). A single-purpose effect for this card — no
    /// fields, not generalized.
    /// ponytail: the pool has no planeswalker permanent type, so the "…a planeswalker" slot is
    /// unreachable — the four-type keep collapses to artifact/creature/enchantment. Same posture as
    /// the planeswalker gaps in #91/#110; add the slot when a pool card fields a planeswalker.
    CasterKeepsOneOfEachTypePerPlayer,
    /// The controller-directed per-player +1/+1 fan-out (Nils, Discipline Enforcer: "for each
    /// player, put a +1/+1 counter on up to one target creature that player controls"). For each
    /// living player in APNAP order, the effect's controller picks up to one creature that player
    /// controls and puts one +1/+1 counter on it, pausing on a
    /// [`PendingChoice::ChooseCounterTargetForPlayer`]; a player with no creature is skipped. A
    /// single-purpose effect for this card — no fields, not generalized.
    /// ponytail: "up to one target creature" is really a target chosen as the ability goes on the
    /// stack (CR 603.3d), not at resolution; no pool card responds to Nils' specific targets, so
    /// resolution-time selection is exact here (same posture as the other per-player fan-outs).
    EachPlayerControllerChoosesCounterTarget,
    /// A per-attacker counter-scaled attack tax (Nils, Discipline Enforcer: "Each creature with one
    /// or more counters on it can't attack you … unless its controller pays {X}, where X is the
    /// number of counters on that creature."). A static read by [`Game::attack_tax_owed`]: each
    /// attacker aimed at this ability's controller that carries one or more counters owes generic
    /// mana equal to its counter count. It never resolves off the stack. Fieldless target, like the
    /// other static effects.
    /// ponytail: models only "can't attack *you*" — the printed "or planeswalkers you control"
    /// clause is unobservable while attack targets are always a `PlayerId` (planeswalker defenders
    /// aren't modeled); wire the clause through when they land.
    CounterScaledAttackTax,
    /// Council's dilemma (CR 701.32) — Fateful Tempest's "Starting with you, each player votes for
    /// past or present." Each living player, in turn order starting with the effect's controller,
    /// votes for one of `options` (the ballot labels), pausing on a [`PendingChoice::CastVote`].
    /// The two tallies are accumulated onto [`Game::council_past_votes`]/[`Game::council_present_votes`]
    /// for the following `Sequence` steps to read via [`Amount::PastVotes`]/[`Amount::PresentVotes`].
    /// Like [`EachPlayerExilesFromGraveyard`](Self::EachPlayerExilesFromGraveyard), the payoff rides
    /// in the enclosing `Sequence`, resumed once every player has voted, so this carries no follow-up.
    /// ponytail: the two tallies (and the `Amount`s that read them) are hardcoded to the
    /// past/present ballot — Fateful Tempest is the pool's only council's-dilemma card. Generalize
    /// to a label→tally map when a second voting card (a different ballot, e.g. will-of-the-council)
    /// lands.
    CouncilsDilemmaVote {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_str_slice"))]
        options: &'static [&'static str],
    },
    /// "Each player creates a 0/0 green and blue Fractal creature token and puts a number of
    /// +1/+1 counters on it equal to the total power of creatures they controlled that were
    /// exiled this way." (Oversimplify): one `token` per living player, in APNAP order, with
    /// +1/+1 counters equal to that player's own share of `Game::power_exiled_this_way` — the
    /// preceding `Effect::ExileAll` step's per-controller power snapshot — routed through
    /// [`Game::counters_after_replacements`] like `CreateToken`'s `enters_with`. Unlike
    /// [`EachPlayerExilesFromGraveyard`](Self::EachPlayerExilesFromGraveyard), no player makes a
    /// choice, so this never pauses.
    EachPlayerCreatesFractalFromExiledPower {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::token_profile"))]
        token: CardDef,
    },
    /// "Then you may choose a token you control. If you do, each other token you control becomes a
    /// copy of that token" (Brudiclad, Telchor Engineer). The controller may pick one token they
    /// control ([`PendingChoice::ChooseTokenToCopy`], up-to-one, declinable); every *other* token
    /// they control then becomes a copy of it (CR 706/707.2 — an [`Event::BecameCopy`] per other
    /// token, indefinite, CR 400.7). Takes no target; pauses to choose. Declining converts nothing.
    EachOtherTokenBecomesCopyOfChosen,
    /// "Put a +1/+1 counter on this creature, then you may have this creature become a copy of an
    /// artifact or creature card from among those cards until end of turn" (Spirit of Resilience,
    /// off [`Trigger::CardsLeaveYourGraveyard`]). Places one +1/+1 counter on the ability's own
    /// source, then — if any of `cards` is an artifact or creature card — pauses on a
    /// [`PendingChoice::ChooseCopyCardFromList`] up-to-one declinable pick; the chosen card's
    /// printed def overwrites the source until end of turn ([`Event::BecameCopy`] with
    /// `until_eot: true`, CR 706/707.2). No copyable card ⇒ no pause. Takes no target.
    PutCounterThenMayBecomeCopyOfCardFromList {
        /// The cards that left the graveyard this batch (CR 603.10a last-known information), baked
        /// in by `fill_cards_left_graveyard` from [`TriggerContext::cards_left_graveyard`]; `&[]`
        /// in a card template. Only artifact/creature cards among these are legal copy sources.
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        cards: &'static [ObjectId],
    },
    /// Muddle, the Ever-Changing's magecraft ability: "Muddle becomes a copy of up to one target
    /// nonlegendary creature you control until end of turn, except it has myriad" (CR 706/707.2,
    /// CR 702.114). `target` is a real CR 601.2c target chosen when the ability goes on the
    /// stack — the ability's own `optional = true` models "up to one" (same shape Skyclave
    /// Apparition's "up to one target" uses: declining or having no legal target both fizzle it
    /// harmlessly). On resolution, the ability's source becomes a copy of the target (an
    /// until-EOT [`Event::BecameCopy`], reverted at cleanup via `Permanent::reverts_to_def_eot`)
    /// and gains [`Keyword::Myriad`] until end of turn via an until-EOT [`Event::TempBoost`] (the
    /// same "gains a keyword" shape [`Game::answer_enter_as_copy`]'s `gains_haste` rider uses).
    BecomeCopyOfTargetCreatureGainingMyriad { target: TargetSpec },
    /// The payload [`Keyword::Myriad`] resolves into when a creature carrying it attacks (CR
    /// 702.114a): "for each opponent other than the defending player, you may create a token
    /// copy that's tapped and attacking that player or a planeswalker they control. Exile the
    /// tokens at the end of combat." Never authored in TOML — synthesized by
    /// [`Game::queue_myriad_triggers`] the same way [`Effect::PumpSelfUntilEndOfTurn`] is
    /// synthesized for Prowess. `attacking_context` is always `Some` when this effect resolves —
    /// filled by `queue_myriad_triggers` at synthesis, mirroring
    /// [`CreateToken::attacking_context`](Self::CreateToken::attacking_context)'s "watch payload
    /// fill" shape.
    /// ponytail: "you may create a token copy" per opponent is modeled as mandatory, matching the
    /// pool's existing tapped-attacking-copy convention ([`Effect::CopyEachEnteredThisTurnTokenTappedAttacking`],
    /// `Game::encore`) — no pool scenario needs to decline one specific copy.
    /// ponytail: no planeswalker permanent type exists in the pool, so "that player or a
    /// planeswalker they control" narrows to the player — the same standing pool-wide
    /// combat-target limitation every attack effect carries, not specific to this card.
    MyriadTokenCopies {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        attacking_context: Option<(PlayerId, PlayerId)>,
    },
    /// "Each player discards their hand, then draws `count` cards" (Wheel of Fortune): every
    /// living player, in APNAP order, discards their *entire* hand — not a chosen subset — then
    /// draws `count`. No player makes a choice, so this never pauses, unlike
    /// [`Discard`](Self::Discard) (a card-pick choice over a partial hand).
    /// ponytail: CR-simultaneous "each player discards, then each draws" is composed as
    /// per-player discard-then-draw sequential in APNAP order — behaviorally identical (no pool
    /// card observes discard/draw interleaving across players), same posture as
    /// defacing_duskmage's "each player loses 2 life" sequential note.
    EachPlayerDiscardsHandThenDraws { count: Amount },
    /// The ability's controller may sacrifice one permanent matching `filter`; if they do, `then`
    /// runs (CR 601.2f-style resolution-time optional cost — Witherbloom Charm mode 0's "You may
    /// sacrifice a permanent. If you do, draw two cards."). Pauses on a
    /// [`PendingChoice::MaySacrifice`]; declining runs nothing. Distinct from
    /// [`EachPlayerSacrifices`](Self::EachPlayerSacrifices) (mandatory, possibly multi-player).
    MaySacrifice {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        filter: PermanentFilter,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },
    /// The ability's controller may return one card from their own graveyard matching `filter`
    /// to their hand (CR 601.2f-style resolution-time optional rider — Deadly Brew's "you may
    /// return another permanent card from your graveyard to your hand"). Pauses on a
    /// [`PendingChoice::MayReturnFromGraveyard`]; declining runs nothing. The graveyard-return
    /// twin of [`MaySacrifice`](Self::MaySacrifice).
    MayReturnFromGraveyard {
        filter: CardFilter,
        /// Gate the rider on "if you sacrificed a permanent this way" (Deadly Brew): the rider
        /// runs nothing at all — no pause — unless the ability's own controller actually
        /// sacrificed a permanent during this resolution's own
        /// [`EachPlayerSacrifices`](Self::EachPlayerSacrifices) edict (tracked by `Game`'s
        /// `sacrificed_by_edict_controller` scratch flag). Default `false` (unconditional, every
        /// other consumer's shape).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        if_you_sacrificed_this_way: bool,
    },
    /// The ability's controller may discard one card from their hand; if they do, `then` runs
    /// (CR 608.2c-style resolution-time optional sub-action, distinct from an activation/trigger
    /// cost gate — Quintorius, History Chaser's +1 "You may discard a card. If you do, draw two
    /// cards, then mill a card."). Pauses on a [`PendingChoice::MayDiscard`]; declining (or an
    /// empty hand) runs nothing. The hand-discard twin of [`MaySacrifice`](Self::MaySacrifice).
    MayDiscard {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },
    /// A player discards `count` cards (Faithless Looting's "then discard two cards", Pull from
    /// Tomorrow's "then discard a card"). Fewer than `count` in hand discards the whole hand; an
    /// empty hand is a no-op. Pauses on a [`PendingChoice::DiscardCards`] addressed to the
    /// discarding player, who picks which cards to pitch (the graveyard-bound mirror of a
    /// cleanup discard).
    /// `target_player`: `false` (default) is the ability's controller; `true` is a chosen target
    /// player instead (Prismari Command's "target player … discards two cards" — CR 111.4).
    Discard {
        count: u32,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target_player: bool,
    },
    /// The ability's controller may put a land card from their hand onto the battlefield (CR
    /// 305.9 — a "put onto the battlefield" effect, not "play a land"), tapped iff `tapped`
    /// (Eureka Moment; Zimone, Quandrix Prodigy's first activated ability). Pauses on a
    /// [`PendingChoice::PutLandFromHand`] so the controller picks which land, or declines
    /// ("up to one"); no lands in hand is a no-op. Fires an ETB like any other enter.
    /// ponytail: does not consume the once-per-turn land drop (CR 305.9's "put onto the
    /// battlefield" is a distinct action from "play a land", as printed) — the pool has no card
    /// where the two need to interact.
    PutLandFromHand {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        tapped: bool,
    },
    /// Tap the target permanent(s) (CR 701.21) — Killian, Decisive Mentor's "tap up to one
    /// target creature [and goad it]"; Magma Opus's "tap two target permanents" (a
    /// `Timing::Spell` ability, `count = {2, 2}`). Tapping an already-tapped permanent is a
    /// legal no-op. `count` is the same [`TargetCount`] multi-target surface as
    /// [`ReturnToHand`](Self::ReturnToHand)'s.
    /// ponytail: `count` only widens the cast-time multi-target pipeline
    /// ([`Game::choose_spell_targets`], `Timing::Spell` only) — a *triggered* ability's target
    /// (Killian's ETB) still goes through the single-target `ChooseTarget` path, which never
    /// reads `count`, so "up to one" there stays mandatory until triggered abilities grow their
    /// own optional/up-to-N targeting. (CR 701.38, CR 601.2c, CR 601)
    TapTarget {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
    },
    /// Untap the target permanent (CR 701.21) — Besmirch's "untap that creature" rider. The tap
    /// state already exists; this just exposes the mutation as an effect.
    UntapTarget { target: TargetSpec },
    /// Untap every battlefield permanent the ability's controller controls matching `filter`
    /// (Beledros Witherbloom's "Pay 10 life: Untap all lands you control"). No target — same
    /// implicit "you control" scoping as [`PumpCreaturesYouControlUntilEndOfTurn`](Self::PumpCreaturesYouControlUntilEndOfTurn).
    UntapAll { filter: PermanentFilter },
    /// Every player draws `count` cards (Faerie Mastermind's "{3}{U}: Each player draws a
    /// card"). No target; unlike [`DrawCards`](Self::DrawCards) (controller only) or
    /// [`TargetPlayerDraws`](Self::TargetPlayerDraws) (one chosen player), this hits the whole table.
    EachPlayerDraws { count: u32 },
    /// The target player loses `amount` life, with no matching gain (Ominous Harvest's "target
    /// player ... loses 1 life"). The controller-drain/each-opponent shapes already cover the
    /// other life-loss flavors; this is the plain single-target one. Uses [`TargetSpec::Player`].
    TargetPlayerLosesLife { amount: i32 },
    /// The ability's controller sacrifices `count` of their own permanents matching `filter`, or
    /// all of them if fewer than `count` match (CR 700.2) — Lotus Field's ETB "sacrifice two
    /// lands", Smothering Abomination's upkeep "sacrifice a creature". No target — always the
    /// controller's own board. Which matching permanents are sacrificed is the controller's own
    /// choice (CR 701.16a) when more than `count` are available; see
    /// [`Game::begin_choose_own_sacrifices`].
    SacrificeOwn { filter: PermanentFilter, count: u32 },
    /// Annihilator N (CR 702.86a — Eldrazi Conscription's granted keyword): the *defending*
    /// player sacrifices `count` permanents of their own choice (any permanent, CR 701.16a lets
    /// them pick which), or all of them if fewer than `count` are on their board. `defender` is
    /// filled in from the attack trigger's context ([`TriggerContext::attack`]) when placed;
    /// `None` in a card template. The defender-scoped twin of [`SacrificeOwn`](Self::SacrificeOwn)
    /// (always the controller's own board); shares its
    /// [`Game::begin_choose_own_sacrifices`] machinery with an unrestricted filter and the
    /// defending player standing in for the ability's controller.
    DefendingPlayerSacrifices {
        count: u8,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        defender: Option<PlayerId>,
    },
    /// Sacrifice one already-resolved object, no re-scan (CR 603.7's "sacrifice it" — Determined
    /// Iteration's populated token). `object` is filled in when the delayed trigger is scheduled
    /// (see [`Effect::CreateTokenCopy`]'s `sacrifice_at_next_end_step`), never authored directly
    /// in a card template — this variant only ever appears as a synthetic `then`.
    SacrificeObject {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        object: Option<ObjectId>,
    },
    /// A [`Trigger::ThisPermanentLeavesBattlefield`] look-back payoff (Animate Dead): "that
    /// creature's controller sacrifices it" — the creature this permanent was attached to the
    /// instant it left the battlefield (CR 603.10a last-known information). `creature` is filled
    /// in from the triggering context when the ability is placed; it is `None` in a card
    /// template — mirrors [`ReanimateDyingEnchantedCreature`](Self::ReanimateDyingEnchantedCreature).
    /// Guard-returns with no sacrifice if the context never filled a host, or if that creature no
    /// longer sits on the battlefield (it died first and the Aura fell off via its own CR 704.5m
    /// SBA, or it was bounced/exiled in response — CR 603.10a's "that permanent" fizzle). "that
    /// creature's controller sacrifices it" reads the creature's own current controller, not this
    /// ability's — the existing sacrifice choke ([`Game::sacrifice_event`]) already resolves that.
    SacrificeEnchantedCreature {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        creature: Option<ObjectId>,
    },
    /// Exile one already-resolved object, no re-scan (CR 603.7's "exile it" — Manaform
    /// Hellkite's minted Dragon Illusion token). `object` is filled in when the delayed trigger
    /// is scheduled (see [`Effect::CreateToken`]'s `exile_at_next_end_step`), never authored
    /// directly in a card template — this variant only ever appears as a synthetic `then`, the
    /// exile-flavored twin of [`SacrificeObject`](Self::SacrificeObject).
    ExileObject {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        object: Option<ObjectId>,
    },
    /// The ability's controller mills `count` cards from their own library — untargeted
    /// self-mill (Perpetual Timepiece's "{T}: Mill two cards"), unlike [`Mill`](Self::Mill)
    /// which takes a target player.
    MillSelf { count: Amount },
    /// "Exile [this card] with N time counters on it" (CR 702.62 — Rousing Refrain's printed
    /// self-exile clause): the resolving instant/sorcery exiles *itself* with `counters` time
    /// counters instead of going to the graveyard. Marks the resolving spell (see
    /// [`Game::self_exile_time_counters`](crate::Game)); the actual zone move + counter placement
    /// happen in [`Game::finish_instant_sorcery_resolution`] once every effect step has run.
    ExileSelfWithTimeCounters { counters: u32 },
    /// Run `steps` in order as one resolution, sharing this ability's target and `{X}` (Faithless
    /// Looting's "draw two cards, then discard two cards" is one ability, not two). A single-effect
    /// ability is the one-element case (the `effect = …` TOML form is sugar for `effects = […]`).
    /// A step that pauses (surveil, discard) defers the remaining steps until its choice is
    /// answered (see [`Game::run_sequence`]).
    /// ponytail: the shared target is the first step's target — the pool never sequences two
    /// targeting effects under one ability; grow per-step targets from a card that needs them.
    #[cfg_attr(feature = "card-dsl", serde(skip))]
    Sequence { steps: &'static [Effect] },
    /// "Choose one —" on a *triggered* ability (CR 700.2): the controller picks one of `modes`
    /// and only that sub-effect resolves (Atsushi's dies trigger — exile the top two you may
    /// play • create three Treasures). Pauses on a [`PendingChoice::ChooseMode`], then runs the
    /// chosen mode through the ordinary resolution pipeline (so a mode may itself pause).
    /// ponytail: models a "choose one" whose modes are **non-targeting** effects resolved with the
    /// trigger's own `source`/controller context — a mode that needs a freshly *chosen target*
    /// isn't supported (the pool's modal triggers don't need one); grow that from a card that does.
    /// The mode is picked at *resolution*, not when the ability goes on the stack (CR 603.3d) —
    /// unobservable for Atsushi since nothing responds between placement and resolution.
    ChooseOne {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_slice"))]
        modes: &'static [Effect],
    },
    /// The source permanent "becomes prepared" (soc/sos prepare DFCs): sets its
    /// [`Permanent::prepared`] status so its controller may cast a copy of its
    /// [`CardDef::back`] face (see [`Game::cast_prepared`]). No target — it's always the
    /// ability's own source. Setting an already-prepared permanent prepared is idempotent.
    BecomePrepared,
    /// A Class's "Level N" activated ability (CR 717.2 — "Gain the next level as a sorcery"):
    /// sets the source permanent's [`Permanent::level`] to `level` via [`Event::LeveledUp`]. No
    /// target — always the ability's own source. The activation gate offers this ability only
    /// while the source is at `level - 1` (each level gained exactly once), so resolution simply
    /// records the new level. `{ type = "level_up", level = N }` in TOML.
    LevelUp { level: u8 },
    /// "As ~ enters, choose a creature type" (CR 614.12/700.9-style as-enters choice —
    /// Patchwork Banner). Pauses on a [`PendingChoice::ChooseCreatureType`] for the ability's
    /// controller; the chosen type is stored on the ability's own source
    /// ([`Permanent::chosen_subtype`]), read back by a chosen-type-gated anthem
    /// ([`Effect::AnthemStatic`]'s `chosen_subtype`). No target — it's always the source. CR
    /// 614.12's choice happens "as it enters"; resolving it as the ETB trigger's own first step
    /// is faithful for this pool since nothing depends on the choice mid-enter.
    ChooseCreatureType,
    /// "As ~ enters, choose a color" (CR 614.12/700.9-style as-enters choice — Flickering Ward).
    /// Pauses on a [`PendingChoice::ChooseColor`] for the ability's controller over the five
    /// colors; the chosen color is stored on the ability's own source ([`Permanent::chosen_color`]),
    /// read back by a `protection_from_chosen_color` [`Effect::GrantToAttached`] to grant
    /// [`Keyword::ProtectionFrom`] of that color to the enchanted creature. No target — it's always
    /// the source. Resolving it as the ETB trigger's own first step is faithful for this pool since
    /// nothing depends on the choice mid-enter.
    ChooseColor,
    /// Removes one +1/+1 counter from the ability's own source (Ingenious Prodigy's "remove a
    /// +1/+1 counter from it" — a CR 608.2c effect-internal sub-action, not a CR 602 activation
    /// cost). No target. A no-op if the source has none (guarded so the count never goes
    /// negative; the enclosing ability's `Condition::SourceHasCounters` intervening-if already
    /// keeps this reachable only when at least one counter is present).
    RemoveCounterFromSelf,
    /// Grants the ability's controller [`Player::flash_permission_this_turn`] (CR 601.3a — "you
    /// may cast spells this turn as though they had flash," unfiltered — Alchemist's Refuge).
    /// No target; fieldless. Read by [`CardDef::is_instant_speed`]'s cast-timing gate; cleared at
    /// the next Untap step alongside the other per-turn player flags.
    /// ponytail: modeled as a resolved one-shot that sets a turn flag, not a continuous "as
    /// though they had flash" static — behaviorally identical here (the permission is gone at
    /// cleanup either way, and nothing reads it mid-resolution before the flag is set). (CR 702.8, CR 108.3, CR 601.2c)
    GrantFlashThisTurn,
    /// Grants the ability's controller [`Player::channel_colorless_mana_this_turn`] — "Until end
    /// of turn, any time you could activate a mana ability, you may pay 1 life. If you do, add
    /// {C}" (Yavimaya Bloomsage's Channel back face). No target; fieldless. Surfaced by
    /// [`Intent::ChannelColorlessMana`](crate::Intent::ChannelColorlessMana); cleared at the next
    /// Untap step alongside the other per-turn player flags.
    /// ponytail: the granted permission is player-scoped (no permanent source once the copy
    /// resolves), so it's offered as a standalone `Intent` rather than wired into the
    /// permanent-keyed `Game::ability_at`/`meaningful_actions` enumerator — CR 602/605's "any
    /// time you could activate a mana ability" window isn't independently gated, it's legal
    /// whenever the flag holds, same as this engine's other mana abilities. (CR 605, CR 118.4, CR 601.2c)
    GrantChannelColorlessManaThisTurn,
    /// Runs `then` only if `condition` holds, checked fresh when *this step* resolves (not an
    /// intervening-if checked at trigger-queue time) — a per-step gate inside a
    /// [`Sequence`](Self::Sequence) (Fabled Passage's "then if you control four or more lands,
    /// untap that land"; Zimone, Quandrix Prodigy's "if you control eight or more lands, draw two
    /// cards instead," modeled as an unconditional draw plus this conditional second draw).
    /// Shares the enclosing ability's controller/source/target/`{X}`.
    /// ponytail: one `Condition` per gate, no boolean combinators (AND/OR/NOT) — the pool has no
    /// card that needs composed conditions here; add one only from a real card.
    Conditional {
        condition: Condition,
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_slice"))]
        then: &'static [Effect],
    },
    /// Untaps the permanent this same ability's own library search just put onto the
    /// battlefield (Fabled Passage's "then if you control four or more lands, untap that land" —
    /// paired with [`Conditional`](Self::Conditional) as the following `Sequence` step). No
    /// target: reads back the [`Event::SearchedToBattlefield`] this resolution already produced.
    /// ponytail: addresses only "the permanent this ability's own search just found" — not a
    /// general "the last thing you found" reference; grow that from a second card that needs it.
    UntapSearchedLand,
    /// Backup's rider (CR 702.166): the ability's shared target creature "gains the following
    /// abilities until end of turn" — the source's *other* abilities (Guardian Scalelord's flying
    /// and its attack trigger). Takes no target of its own: it rides the enclosing [`Sequence`]'s
    /// shared target (the same creature the preceding `PutCounters` step counters), and grants
    /// nothing when that target is the source itself ("if that's *another* creature"). The granted
    /// set is read live off the source's [`CardDef`] (minus the granting ability), so no ability
    /// list is copied here — only the `(target, source)` link is recorded, in
    /// [`Game::abilities_granted_until_eot`], until cleanup.
    GrantSourceAbilitiesUntilEndOfTurn,
}

impl Effect {
    /// What this effect targets (most effects target nothing).
    pub(crate) fn target(self) -> TargetSpec {
        match self {
            Effect::DealDamage { target, .. }
            | Effect::PumpUntilEndOfTurn { target, .. }
            | Effect::SetBasePtTargetUntilEndOfTurn { target, .. }
            | Effect::PutCounters { target, .. }
            | Effect::DoubleCounters { target }
            | Effect::DoubleCountersOnTargetCreatures { target, .. }
            | Effect::MoveCounters { target, .. }
            | Effect::RemoveAllCountersThenDraw { target }
            | Effect::ExileTarget { target, .. }
            | Effect::ExileUntilSourceLeaves { target }
            | Effect::ExileTargetMintingIllusionOnLeave { target }
            | Effect::ReturnFromGraveyardToHand { target }
            | Effect::ReanimateToBattlefield { target, .. }
            | Effect::TuckFromGraveyard { target, .. }
            | Effect::Mill { target, .. }
            | Effect::TargetPlayerExilesFromGraveyard { target }
            | Effect::GoadTarget { target }
            | Effect::CreateTokenCopy { target, .. }
            | Effect::TapTarget { target, .. }
            | Effect::UntapTarget { target }
            | Effect::GainControlUntilEndOfTurn { target }
            | Effect::GainControl { target }
            | Effect::ShuffleTargetPermanentIntoLibraryThenReveal { target }
            | Effect::RegenerateShield { target }
            | Effect::AttachMintedAuraToTarget { target }
            | Effect::BecomeCopyOfTargetCreatureGainingMyriad { target }
            | Effect::DestroyTarget { target, .. } => target,
            Effect::ReturnToHand { target, .. } => target,
            // A sequence shares one target: the first step that needs one supplies it.
            Effect::Sequence { steps } => steps
                .iter()
                .map(|s| s.target())
                .find(|&t| t != TargetSpec::None)
                .unwrap_or(TargetSpec::None),
            // A conditional step's target (if its gated `then` needs one) is shared from the
            // enclosing sequence, same rule as `Sequence` above.
            Effect::Conditional { then, .. } => then
                .iter()
                .map(|s| s.target())
                .find(|&t| t != TargetSpec::None)
                .unwrap_or(TargetSpec::None),
            // Quintorius's end step: a fixed target restriction (see the variant doc) — no
            // TOML-authored spec to read back.
            Effect::ExileTargetFromGraveyardWithThis => TargetSpec::CardInGraveyard {
                whose: GraveyardScope::Yours,
                filter: CardFilter::NoncreatureNonland,
            },
            // Renegade Bull's attack trigger: an authored filter (instant-or-sorcery), unlike
            // its fixed-filter sibling above.
            Effect::ExileTargetGraveyardSpellCastFree { filter, .. } => {
                TargetSpec::CardInGraveyard {
                    whose: GraveyardScope::Yours,
                    filter,
                }
            }
            // Restore Relic: an authored filter (artifact-or-creature), same shape as its
            // copy-and-cast-free sibling above.
            Effect::ExileTargetFromGraveyardCreateTokenCopy { filter } => {
                TargetSpec::CardInGraveyard {
                    whose: GraveyardScope::Yours,
                    filter,
                }
            }
            // Feral Appetite: any card in any graveyard — no fixed filter (unlike its
            // noncreature-nonland sibling `ExileTargetFromGraveyardWithThis`) and no authored
            // one (unlike its instant-or-sorcery/artifact-or-creature siblings above).
            Effect::ExileTargetGraveyardCardThenIfCreature { .. } => TargetSpec::CardInGraveyard {
                whose: GraveyardScope::Any,
                filter: CardFilter::AnyCard,
            },
            // Surge to Victory: an authored filter (instant-or-sorcery), same shape as its
            // copy-and-cast-free sibling above — this one just doesn't mint the copy itself.
            Effect::ExileTargetGraveyardCardRecordManaValue { filter } => {
                TargetSpec::CardInGraveyard {
                    whose: GraveyardScope::Yours,
                    filter,
                }
            }
            // Forum Filibuster's reflexive body: "up to one target Aura or Equipment card from
            // your graveyard" (the count is "up to one" — see `target_count`).
            Effect::ReturnFromGraveyardAttachedToToken { filter, .. } => {
                TargetSpec::CardInGraveyard {
                    whose: GraveyardScope::Yours,
                    filter,
                }
            }
            Effect::CopyTargetSpell => TargetSpec::InstantOrSorcerySpellOnStack,
            Effect::CounterTargetSpell { filter, .. } => TargetSpec::SpellOnStack(filter),
            // The cast-time target is the *opponent's* creature; the controller's own creature
            // is chosen at resolution (see `Effect::Fight`'s doc comment).
            Effect::Fight {
                ally_is_shared_target: false,
                ..
            } => TargetSpec::Permanent(PermanentFilter {
                controller: FilterController::Opponent,
                ..PermanentFilter::of(TypeSet::CREATURE)
            }),
            // Primal Might's mirror shape: the ally is a *preceding* Sequence step's target
            // (the pump); this step defers to it, same rule as the no-target-of-its-own steps
            // above.
            Effect::Fight {
                ally_is_shared_target: true,
                ..
            } => TargetSpec::None,
            Effect::TargetPlayerDraws { opponent: true, .. }
            | Effect::DrainTarget { opponent: true, .. }
            | Effect::RevealTopAndDrainMutual => TargetSpec::OpponentPlayer,
            Effect::TargetPlayerDraws { opponent: false, .. }
            | Effect::DrainTarget { opponent: false, .. }
            | Effect::ExileGraveyard
            | Effect::TargetPlayerLosesLife { .. }
            | Effect::Discard {
                target_player: true,
                ..
            }
            | Effect::CreateTreasure {
                target_player: true,
                ..
            }
            | Effect::CreateToken {
                controller: TokenController::TargetPlayer,
                ..
            }
            | Effect::PutCountersEach {
                target_player: true,
                ..
            }
            | Effect::ShuffleTargetCardsFromGraveyardIntoLibrary {
                target_player: true,
                ..
            } => TargetSpec::Player,
            // Equip targets the creature to attach to (the "you control" restriction is
            // enforced when the ability is activated, not by the target spec).
            Effect::Equip => TargetSpec::Creature,
            // Breena's counter half: "a creature you control" (the drawing player is context,
            // not a target) — restricted to the ability's controller's own creatures.
            Effect::AttackerDrawsControllerCounters { .. } => TargetSpec::CreatureYouControl,
            // A mana ability targets a player only when authored to (Rousing Refrain's "target
            // opponent"); every ordinary mana source defaults to `TargetSpec::None`.
            Effect::AddMana { target, .. } => target,
            Effect::DrawCards { .. }
            | Effect::GainLife { .. }
            | Effect::CreateToken { .. }
            | Effect::CreateTreasure {
                target_player: false,
                ..
            }
            | Effect::CopyThisSpell { .. }
            | Effect::RetargetSpellCopy { .. }
            | Effect::CopyTriggeringSpell { .. }
            | Effect::CopyTriggeringSpellForEachOtherCreatureYouControl { .. }
            | Effect::CopyTriggeringAbility { .. }
            | Effect::Demonstrate { .. }
            | Effect::CommanderEntersWithBonusCounters { .. }
            | Effect::ExileTopMayPlay { .. }
            | Effect::ExileTopCastMatchingFree { .. }
            | Effect::Cascade { .. }
            | Effect::ExileFromGraveyardMayPlay { .. }
            | Effect::ExileDiscardedWithThis { .. }
            | Effect::CashOutExiledWithThis
            | Effect::CastExiledWithThisFree
            | Effect::GrantToAttached { .. }
            | Effect::SetAttachedBasePT { .. }
            | Effect::SetAttachedTypes { .. }
            | Effect::EachOpponentDrain { .. }
            | Effect::EachOpponentLosesLife { .. }
            | Effect::Scry { .. }
            | Effect::Surveil { .. }
            | Effect::LookAtTop { .. }
            | Effect::DistributeTop { .. }
            | Effect::RevealTopToHand { .. }
            | Effect::RevealUntil { .. }
            | Effect::RevealUntilMayDeploy { .. }
            | Effect::RevealUntilExileCastFree { .. }
            | Effect::ShuffleLibrary
            | Effect::ExileTopUntilStopCastFreeUnderBudget { .. }
            | Effect::RevealTopCards { .. }
            | Effect::SearchLibrary { .. }
            | Effect::ReduceSpellCost { .. }
            | Effect::CounterReplacement { .. }
            | Effect::TokenReplacement { .. }
            | Effect::LifeGainReplacement { .. }
            | Effect::CastXReplacement { .. }
            | Effect::EntersWithCounters { .. }
            | Effect::CreaturesYouControlEnterWithCounters { .. }
            | Effect::DestroyAll { .. }
            | Effect::ExileAll { .. }
            | Effect::ExileAllGraveyards
            | Effect::ReturnAllToHand { .. }
            | Effect::MassReturnFromGraveyard { .. }
            | Effect::ShuffleTargetCardsFromGraveyardIntoLibrary {
                target_player: false,
                ..
            }
            | Effect::DamageEachCreature { .. }
            | Effect::WeakenEachCreature { .. }
            | Effect::PumpCreaturesYouControlUntilEndOfTurn { .. }
            | Effect::GrantKeywordsToPermanentsYouControlUntilEndOfTurn { .. }
            | Effect::PumpOtherAttackersAttackingYourOpponents { .. }
            | Effect::EnchantedAttackerPumpAttackingOpponentElseControllerLosesLife { .. }
            | Effect::StripKeywordsFromOpponentsCreatures { .. }
            | Effect::PumpSelfUntilEndOfTurn { .. }
            | Effect::ControlAttached
            | Effect::EachPlayerSacrifices { .. }
            | Effect::EachPlayerExilesFromGraveyard
            | Effect::CasterKeepsOneOfEachTypePerPlayer
            | Effect::EachPlayerControllerChoosesCounterTarget
            | Effect::CouncilsDilemmaVote { .. }
            | Effect::OpponentSplitsExilePiles
            | Effect::EachPlayerExilesUntilNonlandOpponentPicks
            | Effect::EachPlayerCreatesFractalFromExiledPower { .. }
            | Effect::EachOtherTokenBecomesCopyOfChosen
            | Effect::PutCounterThenMayBecomeCopyOfCardFromList { .. }
            | Effect::EachPlayerDiscardsHandThenDraws { .. }
            | Effect::MaySacrifice { .. }
            | Effect::MayReturnFromGraveyard { .. }
            | Effect::MayDiscard { .. }
            | Effect::PutCountersEach { .. }
            | Effect::Proliferate { .. }
            | Effect::Discard {
                target_player: false,
                ..
            }
            | Effect::PutLandFromHand { .. }
            | Effect::UntapAll { .. }
            | Effect::EachPlayerDraws { .. }
            | Effect::SacrificeOwn { .. }
            | Effect::DefendingPlayerSacrifices { .. }
            | Effect::SacrificeObject { .. }
            | Effect::SacrificeEnchantedCreature { .. }
            | Effect::ExileObject { .. }
            | Effect::MillSelf { .. }
            | Effect::ExileSelfWithTimeCounters { .. }
            | Effect::ExileRandomFromGraveyardMayPlay
            | Effect::AnthemStatic { .. }
            | Effect::TriggerDoublingStatic { .. }
            | Effect::GrantManaAbility { .. }
            | Effect::ScheduleAtNextUpkeep { .. }
            | Effect::ScheduleNextCastTrigger { .. }
            | Effect::AttackerLosesLifeYouGain { .. }
            | Effect::AttackerLosesLifeYouDraw { .. }
            | Effect::AttackingPlayerDraws { .. }
            | Effect::DealDamageToEnteringPermanent { .. }
            | Effect::ReanimateDyingEnchantedCreature { .. }
            | Effect::ExileDeadCreatureCreateCopyWithSubtype { .. }
            | Effect::ReturnThisToHand
            // The phase-out set is chosen at resolution (a resolution-time subset choice), not a
            // fixed target on the trigger — see the variant doc.
            | Effect::PhaseOut
            | Effect::ReturnThisFromGraveyardToBattlefield { .. }
            | Effect::AttackTax { .. }
            | Effect::CounterScaledAttackTax
            | Effect::CantBeAttackedBy { .. }
            | Effect::PreventCombatDamageToYouCreatingTokens { .. }
            | Effect::PlaceVowCounters { .. }
            | Effect::LoseLife { .. }
            // A no-target-of-its-own step: reads the enclosing `Sequence`'s shared target.
            | Effect::GainLifeTargetController { .. }
            // A no-target-of-its-own step: manifests the enclosing `Sequence`'s shared target's
            // controller's top card (see the variant doc).
            | Effect::Manifest
            // Arms a watch on the enclosing `Sequence`'s shared target (the creature the
            // preceding `pump_until_end_of_turn` step just deathtouched) — no target of its own.
            | Effect::ArmCombatDamageWatch
            // Arms the this-turn combat-damage-copy watch over `Game::surge_exiled_card` (the
            // enclosing `Sequence`'s own exile step just recorded it) — no target of its own.
            | Effect::ScheduleThisTurnCombatDamageCopy
            // `card` is filled in by the delayed watch when it fires, not a chosen target.
            | Effect::MintFreeCopyOfExiledCard { .. }
            // A modal trigger's `target` is None — its modes are non-targeting (see the variant doc).
            | Effect::ChooseOne { .. }
            // "Become prepared" always affects the ability's own source, never a chosen target.
            | Effect::BecomePrepared
            // "Level N" always raises the ability's own source's level, never a chosen target.
            | Effect::LevelUp { .. }
            // The as-enters creature-type/color choices always affect the ability's own source.
            | Effect::ChooseCreatureType
            | Effect::ChooseColor
            // Removes a counter from the ability's own source, never a chosen target.
            | Effect::RemoveCounterFromSelf
            // Grants the ability's controller a permission — no chosen target.
            | Effect::GrantFlashThisTurn
            | Effect::GrantChannelColorlessManaThisTurn
            // The searched land is read back from the resolution's own events, not a target.
            | Effect::UntapSearchedLand
            // The attach address (a minted token, the triggering entering permanent, or a
            // reanimated creature) is read from trigger context / the resolution's own events,
            // not a chosen target.
            | Effect::AttachTriggeringAuraToMintedToken { .. }
            // A reflexive trigger's own steps are placed as separate abilities, each choosing its
            // own target when placed — this scheduler step takes no target of its own.
            | Effect::ReflexiveTrigger { .. }
            | Effect::AttachSelfToEntering { .. }
            | Effect::AttachSelfToReanimated
            | Effect::AttachSelfToMintedToken
            // Doubles the counters on whatever the ability's own source is attached to, not a
            // chosen target.
            | Effect::DoubleCountersOnAttachedCreature
            // The delayed return's host creature is read from trigger context / baked in at
            // schedule time, not a chosen target.
            | Effect::ScheduleReturnThisAuraAttachedToReanimated
            | Effect::ReturnThisAuraAttachedTo { .. }
            // The new host is chosen at resolution (`ChooseAttachHost`), not a cast/
            // activation target — same as `ReturnThisAuraAttachedTo` above.
            | Effect::ReturnThisAuraFromGraveyardAttachedToChosenHost
            | Effect::ScheduleReturnThisAuraFromGraveyardAttachedToChosenHost
            | Effect::NoMaximumHandSize
            // Backup's grant rides the enclosing `Sequence`'s shared target (the counter's
            // creature), never a target of its own — see the variant doc.
            | Effect::GrantSourceAbilitiesUntilEndOfTurn
            | Effect::PlayFromGraveyardOncePerTurn
            | Effect::PreventNoncombatDamageToOtherCreaturesYouControl
            // Redoubled Stormsinger enumerates matching tokens internally — no chosen target.
            | Effect::SetBasePtCreaturesYouControlUntilEndOfTurn { .. }
            // A self-animation always affects the ability's own source (Restless Spire), no target.
            | Effect::AnimateSelfUntilEndOfTurn { .. }
            | Effect::CopyEachEnteredThisTurnTokenTappedAttacking { .. }
            // Myriad enumerates opponents internally — no chosen target (see the variant doc).
            | Effect::MyriadTokenCopies { .. }
            // Hofri's granted leaves-battlefield rider bakes its exiled card in at synthesis —
            // no chosen target (see the variant doc).
            | Effect::ReturnExiledCardToOwnersGraveyard { .. } => TargetSpec::None,
        }
    }

    /// Whether an activated ability with this effect is a mana ability (CR 605):
    /// it adds mana and takes no target, so it resolves without using the stack.
    /// CR 605.3a doesn't require *only* adding mana — an ability that could add mana and does
    /// something else besides (Brass Infiniscope's `{T}: Add {C}{C}. When you next cast a spell
    /// with {X} …` arms a delayed trigger too) is still a mana ability as long as it targets
    /// nothing, so a `Sequence` counts if any of its steps does.
    pub(crate) fn is_mana_ability(self) -> bool {
        match self {
            Effect::AddMana { .. } => true,
            Effect::Sequence { steps } => steps.iter().any(|s| matches!(s, Effect::AddMana { .. })),
            _ => false,
        }
    }

    /// Whether this (mana) ability's produced credits should be recorded in
    /// [`Player::mana_provenance`](crate::state) — an [`Effect::AddMana`] with `track_provenance`
    /// set (recursing a `Sequence` like [`is_mana_ability`](Self::is_mana_ability)). Read by
    /// `Game::activate_ability` to decide whether to tag the batch it just resolved.
    pub(crate) fn tracks_mana_provenance(self) -> bool {
        match self {
            Effect::AddMana {
                track_provenance, ..
            } => track_provenance,
            Effect::Sequence { steps } => steps.iter().any(|s| s.tracks_mana_provenance()),
            _ => false,
        }
    }

    /// How many targets this effect chooses (CR 601.2c). Most targeted effects take a single
    /// mandatory target (`{1, 1}`); [`ReturnToHand`](Self::ReturnToHand) (Aether Gale's "six
    /// target"), [`DealDamage`](Self::DealDamage) (Volcanic Salvo's "up to two", Magma Opus's
    /// divided "any number"), [`TapTarget`](Self::TapTarget) (Magma Opus's "tap two"),
    /// [`ExileTarget`](Self::ExileTarget) (Curse of the Swine's "exile X target creatures"),
    /// [`DestroyTarget`](Self::DestroyTarget) (Pest Infestation's "up to X target artifacts
    /// and/or enchantments"), [`PutCounters`](Self::PutCounters) (Silkguard's "each of up to
    /// X"), and [`ExileTargetGraveyardSpellCastFree`](Self::ExileTargetGraveyardSpellCastFree)
    /// (Renegade Bull's "up to one target," `{0, 1}`) carry a real count.
    pub(crate) fn target_count(self) -> TargetCount {
        match self {
            Effect::ReturnToHand { count, .. }
            | Effect::DealDamage { count, .. }
            | Effect::TapTarget { count, .. }
            | Effect::ExileTarget { count, .. }
            | Effect::DestroyTarget { count, .. }
            | Effect::ExileTargetGraveyardSpellCastFree { count, .. } => count,
            // "return up to one target Aura or Equipment card" (CR 601.2c — a declinable target).
            Effect::ReturnFromGraveyardAttachedToToken { .. } => TargetCount {
                min: 0,
                max: 1,
                ..TargetCount::default()
            },
            Effect::PutCounters { targets, .. } | Effect::CreateTokenCopy { targets, .. } => {
                targets
            }
            Effect::DoubleCountersOnTargetCreatures { count, .. } => count,
            // A sequence shares one target (see `Effect::target`); its count comes from whichever
            // step overrode the default (Killian, Decisive Mentor's `TapTarget { count: {0, 1} }`
            // step, shared with the following untyped `GoadTarget` step).
            Effect::Sequence { steps } => steps
                .iter()
                .map(|s| s.target_count())
                .find(|&c| c != TargetCount::default())
                .unwrap_or_default(),
            _ => TargetCount::default(),
        }
    }

    /// Whether this effect reads a triggered ability's *second* independent target clause
    /// (`StackItem::Ability::targets_second`, CR 603.3d) rather than the ability's one shared
    /// first-clause target — Kinetic Ooze's X≥10 "double ... any number of other target creatures".
    /// Distinguishes a genuinely independent clause from a `Sequence` step that merely shares the
    /// one chosen target (Killian's goad), so [`Game::place_ability_second_clause`] only chooses a
    /// second set of targets for the former.
    pub(crate) fn reads_second_target_clause(self) -> bool {
        matches!(self, Effect::DoubleCountersOnTargetCreatures { .. })
    }

    /// Whether this effect still does something with *no* chosen target — itself untargeted, or
    /// (for a `Sequence`) at least one of its steps is (Kinetic Ooze's X-threshold riders, which
    /// don't care what its "up to one" destroy step targeted). Used to decide whether an "up to
    /// one" ability with no target chosen (declined, or none legal — CR 601.2c/603.3c) still goes
    /// on the stack, versus dropping outright when every step needs the same declined target
    /// (Killian, Decisive Mentor's "tap up to one target creature and goad it" — goad has nothing
    /// to goad without a tapped creature, so parking it on the stack to do nothing is pure noise).
    pub(crate) fn has_target_independent_step(self) -> bool {
        match self {
            Effect::Sequence { steps } => steps.iter().any(|s| s.target() == TargetSpec::None),
            other => other.target() == TargetSpec::None,
        }
    }

    /// Thread the token a reflexive trigger's parent just minted (CR 603.3b — Forum Filibuster)
    /// into this effect, so its resolution can attach to it — the reflexive-ability analogue of
    /// [`fill_entering_permanent`]'s trigger-placement threading. One effect variant only.
    pub(crate) fn with_reflexive_token(self, token: ObjectId) -> Effect {
        match self {
            Effect::ReturnFromGraveyardAttachedToToken { filter, .. } => {
                Effect::ReturnFromGraveyardAttachedToToken {
                    filter,
                    token: Some(token),
                }
            }
            other => other,
        }
    }
}

/// A sacrifice requirement in an ability's activation cost (CR 118.9 — sacrifice as a cost).
/// The sacrifice happens as the ability is activated (before it resolves), and routes through
/// the normal death events so "when this/a creature dies" triggers fire off it.
/// ponytail: two shapes cover the pool — "Sacrifice this" (the source) and "Sacrifice N
/// creature(s)" (the activator names `count` they control, filtered by [`PermanentFilter`] —
/// its `other` axis is "another creature", CR "each other").
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SacrificeCost {
    /// No sacrifice in the cost.
    #[default]
    None,
    /// "Sacrifice this": the ability's own source is sacrificed.
    This,
    /// "Sacrifice a creature" / "Sacrifice another creature" / "Sacrifice two other creatures"
    /// (Priest of Forgotten Gods): the activator sacrifices `count` distinct permanents matching
    /// the filter that they control (control is enforced at the choke point, not by the filter's
    /// own `controller` axis — CR 701.17, you can only sacrifice what you control). Which ones
    /// are named in the activating [`Intent::ActivateAbility`] (a cost is chosen as it's paid),
    /// not via a separate [`PendingChoice`] round-trip. `count` is 1 for every plain "sacrifice a
    /// creature" spelling.
    /// ponytail: named `Creature` for the common case, but the legality check only requires *any*
    /// permanent on the battlefield — `filter`'s own `types`/`subtypes` axes decide what actually
    /// qualifies. The `"creature"` string sugar and `{ creature = {...} }` table form both force
    /// `filter.types = TypeSet::CREATURE`, so every existing creature-sac card is unaffected; the
    /// `{ permanent = {...} }` table form (Gyome's/Gilded Goose's "Sacrifice a Food") leaves
    /// `types` unforced so a non-creature permanent (an artifact) can pay it.
    Creature { filter: PermanentFilter, count: u8 },
}

/// A named non-P/T counter kind (CR 122.1) tracked on [`Permanent::kind_counters`] — distinct
/// from the +1/+1 counter path ([`Permanent::plus_counters`]), which stays untouched. Grows only
/// as real cards demand a kind (mana_bloom/astral_cornucopia's charge counters, staff_of_the_
/// storyteller's story counters).
/// ponytail: `pub`, not `pub(crate)` — it rides inside [`Effect`]/[`Event`]/[`ActivationCost`],
/// all `pub` types, so a private enum there would be E0446 (private type in public interface).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum CounterKind {
    Charge,
    Story,
    Study,
    /// A vow counter (CR 122.1 — Promise of Loyalty): a functional reminder counter marking a
    /// creature that "can't attack you or planeswalkers you control for as long as it has a vow
    /// counter on it." The protected player is recorded on [`Permanent::vow_protected`] where the
    /// counter is placed, and the restriction is read live in [`Game::declare_attackers`].
    Vow,
    /// A time counter (CR 702.62 — suspend, Rousing Refrain). Unlike the other kinds, a time
    /// counter sits on a card in *exile* (a suspended card), so it is tracked in
    /// [`Game::exile_time_counters`](crate::Game) keyed by object id rather than in
    /// [`Permanent::kind_counters`] (an exiled card is an [`Object::Card`], not a `Permanent`).
    /// ponytail: proliferate (CR 701.27) reads only `Permanent::kind_counters`, so it can't yet
    /// add a time counter to a suspended card — wire the exile store into proliferate when a pool
    /// card wants that.
    Time,
}

impl CounterKind {
    /// How many kinds [`Permanent::kind_counters`] has a slot for.
    /// ponytail: a fixed slot array sized to exactly what the pool's cards consume (charge, story,
    /// study, vow) rather than an open-ended map — `Permanent` must stay `Copy`, so no
    /// `Vec`/`HashMap`. Grow this (and add the matching variant) when a future card needs
    /// another named kind, or swap to a leaked `&'static [(CounterKind, u8)]`
    /// slice if the kind set ever needs to be open-ended.
    pub(crate) const COUNT: usize = 5;

    /// Every kind, for enumerating "each kind present" (proliferate, move/remove-all-counters).
    pub(crate) const ALL: [CounterKind; Self::COUNT] = [
        CounterKind::Charge,
        CounterKind::Story,
        CounterKind::Study,
        CounterKind::Vow,
        CounterKind::Time,
    ];
}

/// An *activated* ability an Aura grants its enchanted host (Fallen Ideal's "Sacrifice a
/// creature: This creature gets +2/+1 until end of turn."), carried by
/// [`Effect::GrantToAttached`]. The non-mana twin of [`Effect::GrantManaAbility`]'s inline
/// `cost`/`mana`: surfaced on the host by [`Game::granted_attachment_abilities`] and synthesized
/// into an [`Ability`] by [`Game::ability_at`], never resolved off the stack itself. Read live off
/// the attachment scan, so it disappears the instant the Aura leaves. `effects` resolve against the
/// host as the ability's own source (so `pump_self_until_end_of_turn` pumps the host).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(deny_unknown_fields, rename_all = "snake_case")
)]
pub struct GrantedAbility {
    /// The granted ability's activation cost, spelled as the same flat fields an inline
    /// [`ActivationCost`] uses (Fallen Ideal: `sacrifice = { creature = {} }`, no mana).
    #[cfg_attr(feature = "card-dsl", serde(default))]
    pub cost: ActivationCost,
    /// The granted ability's effect(s), leaked to `'static` like every other nested effect slice
    /// ([`Effect::Sequence`]'s `steps`). A one-effect grant is used as-is; multiple are run as a
    /// [`Sequence`](Effect::Sequence).
    #[cfg_attr(
        feature = "card-dsl",
        serde(default, deserialize_with = "de::static_slice")
    )]
    pub effects: &'static [Effect],
}

/// The indefinite characteristics set an [`Effect::ReanimateToBattlefield`] with a `becomes`
/// rider applies to the permanent it reanimates (CR 611.2c — Excava, the Risen Past's "It's a 1/1
/// Spirit creature with flying in addition to its other types"): `add_types`/`add_subtypes` are
/// unioned onto the reanimated object (CR 613.4), base P/T is SET to `base_power`/`base_toughness`
/// (CR 613.3(7b)), and `keywords` are added — all for as long as it stays on the battlefield.
/// Written onto the permanent's indefinite fields by [`Event::ReanimatedCreatureBecame`], the
/// as-long-as-on-battlefield twin of [`AnimateSelfUntilEndOfTurn`](Effect::AnimateSelfUntilEndOfTurn).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(deny_unknown_fields, rename_all = "snake_case")
)]
pub struct ReanimateBecomes {
    #[cfg_attr(feature = "card-dsl", serde(default))]
    pub add_types: TypeSet,
    #[cfg_attr(
        feature = "card-dsl",
        serde(default, deserialize_with = "de::static_str_slice")
    )]
    pub add_subtypes: &'static [&'static str],
    pub base_power: i32,
    pub base_toughness: i32,
    #[cfg_attr(
        feature = "card-dsl",
        serde(default, deserialize_with = "de::static_slice")
    )]
    pub keywords: &'static [Keyword],
}

/// The cost to activate an ability: tapping the permanent, paying mana, and/or a sacrifice.
///
/// A plain [`Ability`] spells its `Timing::Activated` cost as flat fields alongside the ability
/// itself (see `de.rs`'s `Ability::deserialize`), so this type's own `Deserialize` below is only
/// exercised where an `ActivationCost` is nested *inside* another value — currently
/// [`Effect::GrantManaAbility`]'s `cost` field, spelled as a `[…cost]` table with these same
/// field names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(default, deny_unknown_fields, rename_all = "snake_case")
)]
pub struct ActivationCost {
    pub taps_self: bool,
    pub mana: Cost,
    pub sacrifice: SacrificeCost,
    /// Life paid as part of the cost (CR 118.4 — fetchlands' "Pay 1 life", War Room's "pay life
    /// equal to the number of colors in your commander's color identity"). Paid on activation; a
    /// player who can't pay this much life can't activate the ability (CR 119.4). Resolved via
    /// [`Game::resolve_amount`] with `x = 0` and no target — same rule as any other ability
    /// amount (CR: an activated ability carries no `{X}`).
    pub pay_life: Amount,
    /// +1/+1 counters removed from the ability's own source as part of the cost (CR 118 "remove
    /// a counter" cost — Steelbane Hydra's "Remove a +1/+1 counter from this creature"). Paid on
    /// activation as a negative [`Event::CountersPlaced`]; a source without this many counters
    /// can't activate the ability (CR 602.2b — an uncompletable cost makes activation illegal).
    pub remove_counters: u8,
    /// Which counter kind [`remove_counters`](Self::remove_counters) removes: `None` (the
    /// default) is the +1/+1 path above; `Some(kind)` makes the cost remove that many of
    /// `kind`'s counters instead (staff_of_the_storyteller's "remove a story counter", mana_
    /// bloom's "remove a charge counter") — gated the same way (CR 602.2b: fewer than
    /// `remove_counters` of that kind on the source makes activation illegal).
    pub remove_counters_kind: Option<CounterKind>,
    /// Damage the source deals to the activating player as a rider on the ability's effect
    /// (painlands' and the Talismans' "This land/artifact deals 1 damage to you" on their colored
    /// mode; 0 for none). Unlike [`pay_life`](Self::pay_life) this is *not* a cost — it never gates
    /// activation, and a player at 1 life may still tap a painland for its colored mana. Applied
    /// as the ability resolves; a mana ability resolves the instant it's activated, so the two are
    /// indistinguishable in this pool.
    /// ponytail: modeled as plain life loss (CR: damage to a player) — no lifelink/prevention
    /// source in the pool cares about the damage/life-loss distinction on these riders. (CR 605, CR 120.3, CR 118.7)
    pub self_damage: u8,
    /// A loyalty ability's loyalty cost (CR 606): `Some(+N/0/−N)` marks the ability as a
    /// planeswalker loyalty ability (sorcery-speed, once per turn, `−N` needs loyalty ≥ N; the
    /// loyalty change is paid on activation). `None` for any ordinary activated ability.
    pub loyalty: Option<i32>,
    /// An activation restriction: "Activate only once each turn" (Beledros Witherbloom's untap
    /// ability). CR 602.2b — an unmet activation restriction makes the activation illegal.
    /// Keyed by (source object, ability index) in [`Game::once_each_turn_activated`]; `false` for
    /// an ability with no such cap.
    pub once_each_turn: bool,
    /// An activation timing restriction: "Activate only as a sorcery" (CR 602.5b — Ozolith, the
    /// Shattered Spire's counter ability). Checked against the same "any time you could cast a
    /// sorcery" predicate spells use ([`Game::can_take_sorcery_speed_action`]). Independent of a
    /// loyalty ability's own built-in sorcery-speed timing ([`loyalty`](Self::loyalty)) — this
    /// flag is for ordinary (non-loyalty) activated abilities.
    pub sorcery_speed: bool,
    /// "Return this to its owner's hand" as part of the cost (CR 118 — Rootha, Mercurial
    /// Artist's "{2}, Return Rootha to its owner's hand: …"). Paid on activation as a self-bounce
    /// (a token ceases to exist instead, CR 111.7); the source is always payable since an
    /// activated ability only exists on a live battlefield permanent, so this never gates
    /// activation on its own.
    pub return_self: bool,
    /// "Mill a card" as part of the cost (CR 701.13/118 — Millikin's "{T}, Mill a card: Add
    /// {C}."): the activator mills this many of their own cards to activate. `0` for none.
    /// Paid on activation; a library with fewer than this many cards can't pay it (CR 602.2b —
    /// an uncompletable cost makes activation illegal).
    pub mill_self: u8,
    /// "Exile this artifact"/"exile this permanent" as part of the cost (CR 118 — Perpetual
    /// Timepiece's "{2}, Exile this artifact: …"). Paid on activation via the same
    /// [`Event::MovedToExile`] path a targeted exile effect uses; a token ceases to exist
    /// instead (CR 111.7) — the same fork [`return_self`](Self::return_self) takes. Like
    /// `return_self`, this never gates activation on its own (the source is always payable) and
    /// self-limits future activations by taking the permanent off the battlefield.
    pub exile_self: bool,
}

/// An intervening-if condition on a triggered ability (CR 603.4): checked once, *when the
/// ability would trigger*. If it doesn't hold, the ability never goes on the stack.
/// ponytail: the CR 603.4 *second* check (re-evaluated as the ability resolves) is skipped — a
/// single placement-time check is all the pool's cards need; add the re-check when one relies on
/// the condition becoming false between trigger and resolution. Must stay `Copy` ([`CardDef`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "type", rename_all = "snake_case")
)]
pub enum Condition {
    /// "if you control `count` or more creatures" (Leonin Vanguard).
    /// ponytail: counts creatures only — the one object kind the pool needs; add a `kind`
    /// discriminator (permanents, artifacts, …) when a real card counts something else.
    YouControlAtLeastCreatures { count: u32 },
    /// Breena: "if that opponent has more life than another of your opponents." Reads the
    /// triggering context's attacked opponent; needs the controller to have ≥2 opponents.
    AttackedOpponentHasMoreLifeThanAnotherOpponent,
    /// "if you control `count` or more lands whose type line carries any of `subtypes`"
    /// (Clifftop Retreat: a Mountain or a Plains, `count = 1`; Mystic Sanctuary: three or more
    /// *other* Islands, `count = 3` — the land being checked hasn't entered the battlefield yet
    /// when this runs, so "other" falls out for free; Emeria: seven or more Plains, `count = 7`).
    ControlsLandsWithSubtype {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_str_slice"))]
        subtypes: &'static [&'static str],
        count: u32,
    },
    /// "if you control `count` or more basic lands" (Eclipsed Steppe and its tango-land cousins).
    ControlsBasicLands { count: u32 },
    /// "if your opponents control `count` or more lands, combined" (the turbulent_* slowland
    /// cycle).
    OpponentsControlLands { count: u32 },
    /// "if you have a card with any of `subtypes` in hand" — the reveal lands (Vineglimmer Snarl
    /// and siblings) actually offer a choice whether to reveal, but revealing is strictly better
    /// (an untapped land vs. a tapped one) with no cost or downside, so there's no real decision
    /// to pause play for.
    /// ponytail: modeled as an automatic hand scan rather than a genuine reveal choice — a
    /// rational player always reveals when they can. Add a real choice if a future card makes
    /// concealment matter (an opponent reacting to what's revealed).
    HandHasLandWithSubtype {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_str_slice"))]
        subtypes: &'static [&'static str],
    },
    /// "if an opponent controls more lands than you" (Land Tax; Claim Jumper's follow-up) —
    /// holds when some living opponent controls strictly more lands than the controller does.
    /// Paired with a trigger whose [`TriggerContext::entering`] carries a specific permanent
    /// (Archaeomancer's Map's landfall, CR "if *that player* controls more lands than you"):
    /// narrows to that permanent's controller specifically, rather than scanning every opponent
    /// (see [`Game::condition_holds`]).
    OpponentControlsMoreLands,
    /// "if you control `at_least` or more lands" — both an intervening-if and an *activation*
    /// restriction (Temple of the False God: "Activate only if you control five or more lands";
    /// checked in [`Game::ability_activation_gate`]).
    /// ponytail: only the `at_least` bound has a pool consumer; add an `at_most` sibling when a
    /// "if you control N or fewer lands" card appears.
    YouControlLands { at_least: u32 },
    /// "if you gained life this turn" (Witch of the Moors). Reads the controller's turn-scoped
    /// life-gain tally (`Player::life_gained_this_turn`).
    YouGainedLifeThisTurn,
    /// "if a modified creature died under your control this turn" (Intermediate Chirography's
    /// Level 3 — CR 700.4/701.29). Reads the controller's turn-scoped
    /// `Player::modified_creature_died_this_turn` flag.
    ModifiedCreatureDiedThisTurn,
    /// "if a card left your graveyard this turn" (Relic Retriever, Primary Research). Reads a
    /// turn-scoped flag (`Player::card_left_graveyard_this_turn`) set whenever a card moves out of
    /// the controller's graveyard.
    CardLeftYourGraveyardThisTurn,
    /// "you've cast an instant or sorcery spell this turn" (Hall of Oracles's counter ability's
    /// activation restriction). Reads a turn-scoped flag (`Player::instant_or_sorcery_cast_this_turn`)
    /// set whenever the controller casts an instant or sorcery.
    CastInstantOrSorceryThisTurn,
    /// "if you control no permanent whose printed subtypes intersect `subtypes`" (Ophiomancer's
    /// "if you control no Snakes"; Pest Rescuer's "if you don't control a Pest creature token" —
    /// with [`TokenFilter::Token`] + [`TypeSet::CREATURE`] so a nontoken Pest like Gorma, or a
    /// noncreature Pest token, does not suppress the trigger).
    YouControlNoSubtype {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_str_slice"))]
        subtypes: &'static [&'static str],
        /// Token-ness restriction (default any). Pest Rescuer sets this to [`TokenFilter::Token`].
        #[cfg_attr(feature = "card-dsl", serde(default))]
        token: TokenFilter,
        /// Required card types (empty = any). Pest Rescuer sets this to [`TypeSet::CREATURE`].
        #[cfg_attr(feature = "card-dsl", serde(default))]
        types: TypeSet,
    },
    /// "if you control no creatures with `keyword`" (Jadar, Ghoulcaller of Nephalia's "if you
    /// control no creatures with decayed") — the effective-keyword sibling of
    /// [`YouControlNoSubtype`](Self::YouControlNoSubtype): scans the controller's battlefield
    /// creatures for `keyword` in their effective set (a granted/temp decayed counts too), not
    /// just a printed subtype string.
    YouControlNoCreatureWithKeyword { keyword: Keyword },
    /// "as long as this creature has `at_least` or more +1/+1 counters on it" (CR 702 counters;
    /// Primordial Hydra's trample gate). Source-object-based — reads the object's own
    /// `Permanent::plus_counters`, not a `TriggerContext` field.
    SourceHasCounters { at_least: u32 },
    /// "if this permanent has no `kind` counters on it" (CR 702 counters; mana_bloom's upkeep
    /// self-bounce: "if this enchantment has no charge counters on it, return it to its owner's
    /// hand"). Source-object-based like [`SourceHasCounters`](Self::SourceHasCounters) above.
    /// ponytail: only the "== 0" inversion this increment's cards need, not a general
    /// `at_least`/`at_most` on a named kind; grow a `SourceHasCountersOfKind { kind, at_least }`
    /// sibling (mirroring `SourceHasCounters`) if a future card needs "at least N story/charge
    /// counters" as an intervening-if instead of exactly zero.
    SourceHasNoCountersOfKind { kind: CounterKind },
    /// "if you control `at_least` or more `color` permanents" (Mistveil Plains's "activate only
    /// if you control two or more white permanents") — an activation restriction, checked in
    /// [`Game::ability_activation_gate`]. Counts the controller's battlefield permanents whose
    /// [`Game::colors_of`] includes `color`.
    YouControlColorPermanents { color: Color, at_least: u32 },
    /// "when this land enters untapped" (Mystic Sanctuary's ETB intervening-if): whether the
    /// permanent that fired this ability is not tapped right now. Reads the object's own
    /// `Permanent::tapped`, which is set at creation from `Game::enters_tapped` (CR 614.13's own
    /// gate, evaluated before the permanent exists — see that fn), so this reads correctly with
    /// no re-derivation. Source-object-based like [`SourceHasCounters`](Self::SourceHasCounters):
    /// `TriggerContext` carries no source id, so `Game::queue_trigger_group` special-cases it
    /// directly against its own `source` parameter rather than through `condition_holds`.
    ThisPermanentEnteredUntapped,
    /// "if that spell's mana value is `at_least` or greater" (Prismari Pianist's "if that
    /// spell's mana value is 5 or greater, create three of those tokens instead") — a
    /// `Trigger::CastSpell` (magecraft) intervening-if, read off `TriggerContext::cast_mana_value`.
    /// [`condition_holds`](Game::condition_holds) gives this an honest live arm, but its only
    /// consumer (Prismari Pianist) never reaches it: the DSL always wraps this condition in an
    /// [`Effect::Conditional`] baked to its `then`/no-op at trigger placement (CR 603.4 — the
    /// triggering spell's mana value is already locked when the trigger goes on the stack), same
    /// as [`fill_cast_mana_value`] rewrites `Amount::TriggeringSpellManaValue`.
    TriggeringSpellManaValueAtLeast { at_least: u8 },
    /// "as long as you have the city's blessing" (CR 702.131, Ascend — tendershoot_dryad's
    /// Saproling anthem). Reads the controller's sticky `Player::has_citys_blessing` flag, set
    /// by a state-based action ([`Game::check_state_based_actions`]) once they control ten or
    /// more permanents (CR 702.131b) and never cleared.
    YouHaveCitysBlessing,
    /// "if a player has `at_most` or fewer cards in hand" (naktamun_lorespinner's "if a player
    /// has one or fewer cards in hand") — an existential over *every* seated player (any player,
    /// including opponents), not just the controller; holds as soon as one living player's hand
    /// is small enough. The hellbent/hand-size sibling of the board-state conditions above.
    AnyPlayerHandSizeAtMost { at_most: u32 },
    /// "if there are `count` or more instant and/or sorcery cards in your graveyard" (Animist's
    /// Awakening's spell mastery — CR intervening-if, checked fresh as the ability resolves).
    /// Counts the same way [`Amount::InstantOrSorceryCardsInYourGraveyard`] does (any
    /// [`CardKind::Spell`] card in the controller's graveyard).
    InstantOrSorceryCardsInYourGraveyardAtLeast { count: u32 },
    /// "if there are `count` or more artifact and/or creature cards in your graveyard" (Lorehold
    /// Archivist's prepare trigger). Counts the controller's graveyard cards whose kind is
    /// [`CardKind::Artifact`] or [`CardKind::Creature`] — the intervening-if sibling of
    /// [`InstantOrSorceryCardsInYourGraveyardAtLeast`](Self::InstantOrSorceryCardsInYourGraveyardAtLeast).
    ArtifactOrCreatureCardsInYourGraveyardAtLeast { count: u32 },
    /// "if that creature has power `at_least` or greater" (Yavimaya Bloomsage's "Then if that
    /// creature has power 7 or greater, this creature becomes prepared") — reads the *resolving
    /// effect's own chosen target's* power, not a `TriggerContext` field like every other arm
    /// here. `TriggerContext` carries no target, so (like `SourceHasCounters`/
    /// `ThisPermanentEnteredUntapped` above) this is unreachable through the ordinary
    /// `condition_holds` path; the [`Effect::Conditional`] resolve site special-cases it directly
    /// against the shared `target` before falling through — see `Game::run`.
    TargetPowerAtLeast { at_least: u32 },
    /// "as long as an opponent has `at_most` or less life" (Bloodghast's conditional haste) — an
    /// existential over the ability controller's opponents (CR 104.3a): holds when any living
    /// opponent's life is `at_most` or lower. Evaluated live, so a life change across the
    /// boundary flips a gated anthem on or off ([`Game::condition_holds`]).
    AnOpponentHasLifeAtMost { at_most: u32 },
    /// "if X is `at_least` or more" (CR 601.2b; Kinetic Ooze's "If X is 5 or more, you draw a
    /// card") — reads the source permanent's *locked cast* `{X}` ([`Game::ability_source_x`],
    /// backed by [`Permanent::entered_with_x`]), not any live board state (e.g. counters that
    /// happen to equal X at ETB but can change before the ability resolves). Source-object-based
    /// like [`SourceHasCounters`](Self::SourceHasCounters): `TriggerContext` carries no source
    /// id, so the [`Effect::Conditional`] resolve site special-cases it directly against its own
    /// `source` parameter, mirroring [`TargetPowerAtLeast`](Self::TargetPowerAtLeast) above.
    SourceEnteredWithXAtLeast { at_least: u32 },
    /// A composed AND of every arm in `conditions` (CR 603.4 — Zimone, All-Questioning's "if a
    /// land entered the battlefield under your control this turn *and* you control a prime
    /// number of lands" is two intervening-ifs on one trigger, and `Ability::condition` is a
    /// single slot). Holds iff every element holds. `&'static` keeps `Condition` `Copy`, leaked
    /// at deserialize like any other `&'static [T]` card field.
    All {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_slice"))]
        conditions: &'static [Condition],
    },
    /// "if a land entered the battlefield under your control this turn" (Zimone, All-Questioning
    /// — CR landfall's own "enters," not "played," so a fetched or token land counts too).
    /// Reads the controller's turn-scoped `Player::land_entered_under_your_control_this_turn`
    /// flag, set at the permanent-enters choke and reset each untap.
    LandEnteredUnderYourControlThisTurn,
    /// "if you control a prime number of lands" (Zimone, All-Questioning). Reads
    /// [`Game::lands_controlled`] through a small trial-division primality test.
    YouControlPrimeNumberOfLands,
    /// "during your turn" (Restless Spire's animated form: "During your turn, this creature has
    /// first strike") — holds iff the controller is the active player right now (CR "your turn" =
    /// the turn in which you're the active player). Reads [`Game::active_player`] against
    /// `ctx.controller`, re-evaluated live like every other static-anthem gate here — flips off
    /// the instant the turn passes to someone else, not just at cleanup.
    DuringYourTurn,
}

/// Whether `sacrifices` is a legal answer to a sacrifice edict over `options`: every id a
/// distinct one of the options, and the right count — all-but-one kept for `keep_one`, otherwise
/// exactly one. (The caller only prompts when there's a real choice, so `options` is non-empty
/// and, for `keep_one`, holds at least two.)
pub(crate) fn valid_sacrifice_choice(
    sacrifices: &[ObjectId],
    options: &[ObjectId],
    keep_one: bool,
) -> bool {
    if sacrifices.iter().any(|id| !options.contains(id)) {
        return false;
    }
    let no_duplicates = sacrifices
        .iter()
        .enumerate()
        .all(|(i, id)| !sacrifices[..i].contains(id));
    let required = if keep_one { options.len() - 1 } else { 1 };
    no_duplicates && sacrifices.len() == required
}

/// Whether `players` is a legal answer to a "choose any number of target players" pause
/// ([`PendingChoice::ChooseTargetPlayers`](super::PendingChoice::ChooseTargetPlayers) — CR
/// 601.2c/608.2b): every entry a distinct one of `legal`, and the count within `[min, max]`.
pub(crate) fn valid_target_player_choice(
    players: &[PlayerId],
    legal: &[PlayerId],
    min: u8,
    max: u8,
) -> bool {
    if players.iter().any(|p| !legal.contains(p)) {
        return false;
    }
    let no_duplicates = players
        .iter()
        .enumerate()
        .all(|(i, p)| !players[..i].contains(p));
    no_duplicates && (min as usize..=max as usize).contains(&players.len())
}

/// Unwrap a creature-targeting effect's chosen target to its object id. These effects only
/// accept `TargetSpec::Creature`, so a resolved target is always an object (never a player).
pub(crate) fn expect_object_target(target: Option<Target>, what: &str) -> ObjectId {
    match target {
        Some(Target::Object(id)) => id,
        other => panic!("{what} resolves with a chosen creature target, got {other:?}"),
    }
}

/// Fill a watch-others effect's context-dependent fields from the triggering context. Breena's
/// composite effect and Parasitic Impetus's drain both need the attacking (enchanted creature's)
/// controller baked in at placement; every other effect passes through unchanged.
pub(crate) fn contextualize_effect(effect: Effect, ctx: TriggerContext) -> Effect {
    // CR 603.10a last-known information: a Dies trigger's `Amount::SourcePower`/
    // `Amount::PerCounterOnSource` reads must resolve to the source's pre-death snapshot, not
    // its now-graveyard-card values (which read 0) — rewrite before the rest of this function's
    // context fills, which don't touch `Amount`.
    let effect = match ctx.dying_source_stats {
        Some((power, counters)) => fill_dying_source_amounts(effect, power, counters),
        None => effect,
    };
    // Same CR 603.10a last-known-information shape, one step over: an `AnEnchantedCreatureDies`
    // watch's `Amount::AurasYouControlledAttachedToDyingCreature` reads the pre-move attachment
    // count baked in at placement.
    let effect = match ctx.auras_you_controlled_attached_to_dying_creature {
        Some(count) => fill_auras_attached_to_dying_creature(effect, count),
        None => effect,
    };
    // CR 603.4/202.3: a `CastSpell` (magecraft) trigger's `Amount::TriggeringSpellManaValue`
    // reads and `Condition::TriggeringSpellManaValueAtLeast` gates both resolve against the
    // triggering spell's mana value, locked in when the trigger goes on the stack — same
    // last-known-information shape as `dying_source_stats` above, one step earlier in this fn.
    let effect = match ctx.cast_mana_value {
        Some(mv) => fill_cast_mana_value(effect, mv),
        None => effect,
    };
    // CR 601.2h: a `CastSpell` trigger's `Amount::TriggeringSpellManaSpent` reads resolve against
    // the mana actually spent to cast the triggering spell, locked in when the trigger goes on
    // the stack — same last-known-information shape as `cast_mana_value` above, one step over.
    let effect = match ctx.cast_mana_spent {
        Some(spent) => fill_cast_mana_spent(effect, spent),
        None => effect,
    };
    // CR 603.4: a `YouCastThis` self-cast trigger's `Amount::X`/`Amount::HalfXRoundedDown` reads
    // resolve against the triggering spell's chosen `{X}`, locked in when the trigger goes on the
    // stack — same last-known-information shape as `cast_mana_value` above.
    let effect = match ctx.cast_x {
        Some(x) => fill_cast_x(effect, x),
        None => effect,
    };
    // CR 510.2/603.10a: a `DealsCombatDamageToPlayer` trigger's reanimation target bound resolves
    // against the damage the source just dealt, locked in when the trigger goes on the stack —
    // same last-known-information shape as `dying_source_stats` above.
    let effect = match ctx.combat_damage {
        Some(damage) => fill_combat_damage(effect, damage),
        None => effect,
    };
    // CR 510.2/603.10a: an `Attacks` trigger's reanimation target bound resolves against the
    // attacker's power, locked in when the trigger goes on the stack — same last-known-information
    // shape as `combat_damage` above.
    let effect = match ctx.source_power {
        Some(power) => fill_source_power(effect, power),
        None => effect,
    };
    // A `YouDiscard` trigger's payoff needs the just-discarded card, not the attack tuple below —
    // guarded separately since it doesn't fit the `ctx.attack`-keyed match.
    if let Some(discarded) = ctx.discarded {
        match effect {
            Effect::ExileFromGraveyardMayPlay { .. } => {
                return Effect::ExileFromGraveyardMayPlay {
                    card: Some(discarded),
                };
            }
            Effect::ExileDiscardedWithThis { .. } => {
                return Effect::ExileDiscardedWithThis {
                    card: Some(discarded),
                };
            }
            _ => {}
        }
    }
    // A `PermanentEnters`/`PermanentEntersIncludingThis` trigger's payoff needs the entering
    // permanent's id, not the attack tuple below — guarded separately for the same reason as
    // `discarded` above. Recurses into a `Sequence` (see `fill_entering_permanent`) so a
    // multi-step ability (Ajani's Chosen's create-then-attach) shares the one entering id.
    let effect = match ctx.entering {
        Some(entering) => fill_entering_permanent(effect, entering),
        None => effect,
    };
    // An `EnchantedCreatureDies` trigger's look-back reanimation payoff needs the dying host's
    // id, not the attack tuple below — guarded separately for the same reason as `entering`
    // above.
    let effect = match ctx.dying_enchanted_creature {
        Some(dying) => fill_dying_enchanted_creature(effect, dying),
        None => effect,
    };
    // A `ThisPermanentLeavesBattlefield` trigger's sacrifice payoff needs the host this
    // permanent was attached to when it left (Animate Dead), not the attack tuple below —
    // guarded separately for the same reason as `dying_enchanted_creature` above.
    let effect = match ctx.left_battlefield_host {
        Some(host) => fill_left_battlefield_host(effect, host),
        None => effect,
    };
    // A `CreatureYouControlDies`-family watch's exile-and-copy payoff needs the dead creature's
    // id (Hofri Ghostforge's "exile it ... create a token that's a copy of that creature"), not
    // the attack tuple below — guarded separately for the same reason as the look-backs above.
    let effect = match ctx.dead_creature {
        Some(dead) => fill_dead_creature(effect, dead),
        None => effect,
    };
    // A `CardsLeaveYourGraveyard` payoff that becomes a copy of one of those cards (Spirit of
    // Resilience) needs the batch's card ids baked in — guarded separately for the same reason as
    // the look-backs above. `&[]` for every other trigger, so this is a no-op elsewhere.
    let effect = if ctx.cards_left_graveyard.is_empty() {
        effect
    } else {
        fill_cards_left_graveyard(effect, ctx.cards_left_graveyard)
    };
    // A delayed one-shot's copy payoff (Thunderclap Drake) needs the spell that fired the
    // armed watch, not the attack tuple below — guarded separately for the same reason as
    // `entering`/`dying_enchanted_creature` above.
    let effect = match ctx.triggering_spell {
        Some(spell) => fill_triggering_spell(effect, spell),
        None => effect,
    };
    // An `ActivateAbility` watch's copy payoff (Unbound Flourishing) needs the source of the
    // activated ability it copies, baked in when the watch fires — same last-known-information
    // shape as `triggering_spell` above, one step over.
    let effect = match ctx.triggering_ability {
        Some(source) => fill_triggering_ability(effect, source),
        None => effect,
    };
    match (effect, ctx.attack) {
        (Effect::AttackerDrawsControllerCounters { counters, .. }, Some((attacker, _attacked))) => {
            Effect::AttackerDrawsControllerCounters {
                attacker: Some(attacker),
                counters,
            }
        }
        (Effect::AttackerLosesLifeYouGain { amount, .. }, Some((attacker, _attacked))) => {
            Effect::AttackerLosesLifeYouGain {
                attacker: Some(attacker),
                amount,
            }
        }
        (Effect::AttackerLosesLifeYouDraw { life_loss, .. }, Some((attacker, _attacked))) => {
            Effect::AttackerLosesLifeYouDraw {
                attacker: Some(attacker),
                life_loss,
            }
        }
        (Effect::AttackingPlayerDraws { count, .. }, Some((attacker, _attacked))) => {
            Effect::AttackingPlayerDraws {
                drawer: Some(attacker),
                count,
            }
        }
        // Goblin Guide: the *defending* player (the attack's second element) reveals, not the
        // attacker.
        (Effect::RevealTopToHand { filter, .. }, Some((_attacker, defender))) => {
            Effect::RevealTopToHand {
                filter,
                defender: Some(defender),
            }
        }
        // Annihilator: the *defending* player sacrifices, not the attacker.
        (Effect::DefendingPlayerSacrifices { count, .. }, Some((_attacker, defender))) => {
            Effect::DefendingPlayerSacrifices {
                count,
                defender: Some(defender),
            }
        }
        // Combat Calligrapher: "that attacking player creates a tapped … token … that's
        // attacking that opponent" — bake the (attacker, attacked) pair so the token mints
        // under the attacker and enters tapped and attacking it, per CR 508.4.
        (
            Effect::CreateToken {
                token,
                count,
                controller,
                enters_with,
                set_base_pt,
                exile_at_next_end_step,
                enters_tapped_and_attacking: true,
                must_attack_defender,
                ..
            },
            Some(attack),
        ) => Effect::CreateToken {
            token,
            count,
            controller,
            enters_with,
            set_base_pt,
            exile_at_next_end_step,
            enters_tapped_and_attacking: true,
            attacking_context: Some(attack),
            must_attack_defender,
        },
        // Redoubled Stormsinger: "Whenever this creature attacks..." — bake the same
        // (attacker, defender) pair so the minted copies enter tapped and attacking the
        // defender Redoubled itself is attacking.
        (Effect::CopyEachEnteredThisTurnTokenTappedAttacking { .. }, Some(attack)) => {
            Effect::CopyEachEnteredThisTurnTokenTappedAttacking {
                attacking_context: Some(attack),
            }
        }
        _ => effect,
    }
}

/// Rewrite a `PermanentEnters`/`PermanentEntersIncludingThis` trigger's entering-permanent
/// placeholders (Marauding Raptor's damage target, Ajani's Chosen's attach target, Shielded by
/// Faith's re-attach target) to the entering permanent's id. Recurses into a [`Effect::Sequence`]
/// so a multi-step ability (create-then-attach) shares the one id across every step, mirroring
/// [`fill_dying_source_amounts`] below; every other effect passes through unchanged.
fn fill_entering_permanent(effect: Effect, entering: ObjectId) -> Effect {
    match effect {
        Effect::DealDamageToEnteringPermanent {
            amount,
            then_if_subtype,
            then,
            ..
        } => Effect::DealDamageToEnteringPermanent {
            entering: Some(entering),
            amount,
            then_if_subtype,
            then,
        },
        Effect::AttachTriggeringAuraToMintedToken { .. } => {
            Effect::AttachTriggeringAuraToMintedToken {
                entering: Some(entering),
            }
        }
        Effect::AttachSelfToEntering { .. } => Effect::AttachSelfToEntering {
            entering: Some(entering),
        },
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_entering_permanent(step, entering))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}

/// Rewrite an `EnchantedCreatureDies` trigger's look-back reanimation placeholder (Changing
/// Loyalty, Gift of Immortality) to the dying host's id — mirrors
/// [`fill_entering_permanent`] above, one effect variant only (flag-don't-force: no other pool
/// card reads this context field yet).
fn fill_dying_enchanted_creature(effect: Effect, dying: ObjectId) -> Effect {
    match effect {
        Effect::ReanimateDyingEnchantedCreature { under_owner, .. } => {
            Effect::ReanimateDyingEnchantedCreature {
                dying: Some(dying),
                under_owner,
            }
        }
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_dying_enchanted_creature(step, dying))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}

/// Rewrite a [`Trigger::ThisPermanentLeavesBattlefield`] look-back sacrifice placeholder (Animate
/// Dead) to the host's id — mirrors [`fill_dying_enchanted_creature`] above, one effect variant
/// only (flag-don't-force: no other pool card reads this context field yet).
fn fill_left_battlefield_host(effect: Effect, host: ObjectId) -> Effect {
    match effect {
        Effect::SacrificeEnchantedCreature { .. } => Effect::SacrificeEnchantedCreature {
            creature: Some(host),
        },
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_left_battlefield_host(step, host))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}

/// Rewrite a [`Trigger::CreatureYouControlDies`]-family watch's exile-and-copy placeholder (Hofri
/// Ghostforge) to the dead creature's id — mirrors [`fill_dying_enchanted_creature`] above, one
/// effect variant only (flag-don't-force: no other pool card reads this context field yet).
fn fill_dead_creature(effect: Effect, dead: ObjectId) -> Effect {
    match effect {
        Effect::ExileDeadCreatureCreateCopyWithSubtype {
            add_subtypes,
            leaves_returns_exiled,
            ..
        } => Effect::ExileDeadCreatureCreateCopyWithSubtype {
            dead: Some(dead),
            add_subtypes,
            leaves_returns_exiled,
        },
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_dead_creature(step, dead))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}

/// Rewrite a [`TriggerContext::cards_left_graveyard`]-reading effect placeholder to the batch's
/// card ids: [`Effect::PutCounterThenMayBecomeCopyOfCardFromList`] (Spirit of Resilience, off
/// [`Trigger::CardsLeaveYourGraveyard`]) — mirrors [`fill_dead_creature`] above. `cards` is the
/// already-leaked `&'static` slice off the trigger context, so no re-leak here.
fn fill_cards_left_graveyard(effect: Effect, cards: &'static [ObjectId]) -> Effect {
    match effect {
        Effect::PutCounterThenMayBecomeCopyOfCardFromList { .. } => {
            Effect::PutCounterThenMayBecomeCopyOfCardFromList { cards }
        }
        other => other,
    }
}

/// Rewrite a [`TriggerContext::triggering_ability`]-reading effect placeholder to the activated
/// ability's source that fired the watch: [`Effect::CopyTriggeringAbility`] (Unbound Flourishing,
/// off [`Trigger::ActivateAbility`]) — the ability half's twin of [`fill_triggering_spell`].
fn fill_triggering_ability(effect: Effect, source: ObjectId) -> Effect {
    match effect {
        Effect::CopyTriggeringAbility {
            may_choose_new_targets,
            ..
        } => Effect::CopyTriggeringAbility {
            triggering_ability: Some(source),
            may_choose_new_targets,
        },
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_triggering_ability(step, source))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}

/// Rewrite a [`TriggerContext::triggering_spell`]-reading effect placeholder to the spell that
/// fired the watch: [`Effect::CopyTriggeringSpell`] (Thunderclap Drake, off [`Trigger::CastSpell`]),
/// [`Effect::CommanderEntersWithBonusCounters`] (Opal Palace, off [`Trigger::SpendManaToCast`]), and
/// [`Effect::CopyTriggeringSpellForEachOtherCreatureYouControl`] (Mirrorwing Dragon, off
/// [`Trigger::SpellTargetsThisOnly`]) — mirrors [`fill_dying_enchanted_creature`] above.
fn fill_triggering_spell(effect: Effect, spell: ObjectId) -> Effect {
    match effect {
        Effect::CopyTriggeringSpell {
            count,
            may_choose_new_targets,
            ..
        } => Effect::CopyTriggeringSpell {
            triggering_spell: Some(spell),
            count,
            may_choose_new_targets,
        },
        Effect::CommanderEntersWithBonusCounters { count, .. } => {
            Effect::CommanderEntersWithBonusCounters {
                triggering_spell: Some(spell),
                count,
            }
        }
        Effect::CopyTriggeringSpellForEachOtherCreatureYouControl { .. } => {
            Effect::CopyTriggeringSpellForEachOtherCreatureYouControl {
                triggering_spell: Some(spell),
            }
        }
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_triggering_spell(step, spell))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}

/// Rewrite every `Amount` field the trigger-context walkers touch through `f`, recursing into a
/// [`Effect::Sequence`] so a multi-step ability shares one context snapshot across every step. The
/// arm set is the union of what the pool's context-filled effects need (flag-don't-force: add an
/// arm here when a real card first needs its `Amount` field context-filled).
fn map_effect_amounts(effect: Effect, f: &impl Fn(Amount) -> Amount) -> Effect {
    match effect {
        Effect::GainLife { amount } => Effect::GainLife { amount: f(amount) },
        Effect::DrawCards { count } => Effect::DrawCards { count: f(count) },
        Effect::PutCounters {
            count,
            target,
            targets,
            kind,
            divided,
        } => Effect::PutCounters {
            count: f(count),
            target,
            targets,
            kind,
            divided,
        },
        Effect::CreateToken {
            token,
            count,
            controller,
            enters_with,
            set_base_pt,
            exile_at_next_end_step,
            enters_tapped_and_attacking,
            attacking_context,
            must_attack_defender,
        } => Effect::CreateToken {
            token,
            count: f(count),
            controller,
            enters_with: f(enters_with),
            set_base_pt: set_base_pt.map(f),
            exile_at_next_end_step,
            enters_tapped_and_attacking,
            attacking_context,
            must_attack_defender,
        },
        Effect::CreateTreasure {
            count,
            target_player,
            tapped,
        } => Effect::CreateTreasure {
            count: f(count),
            target_player,
            tapped,
        },
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps.iter().map(|&s| map_effect_amounts(s, f)).collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}

/// Rewrite a Dies trigger's `Amount::SourcePower`/`Amount::PerCounterOnSource` placeholders to
/// the dying source's CR 603.10a last-known-information snapshot (Lifeblood Hydra's "gain life
/// and draw cards equal to its power", Hangarback Walker's "create a Thopter for each +1/+1
/// counter") — mirrors [`contextualize_sacrifice_effect`]'s `Fixed` rewrite, one event earlier.
/// Recurses into a [`Effect::Sequence`] so a multi-step Dies ability shares the one snapshot
/// across every step; every other effect passes through unchanged.
fn fill_dying_source_amounts(effect: Effect, power: i32, counters: i32) -> Effect {
    map_effect_amounts(effect, &|amount| match amount {
        Amount::SourcePower => Amount::Fixed(power),
        Amount::PerCounterOnSource => Amount::Fixed(counters),
        other => other,
    })
}

/// Rewrite an `AnEnchantedCreatureDies` watch's `Amount::AurasYouControlledAttachedToDyingCreature`
/// placeholders to the watcher's controller's CR 603.10a last-known-information count (Hateful
/// Eidolon's "draw a card for each Aura you controlled that was attached to it") — mirrors
/// [`fill_dying_source_amounts`] above, one Amount variant only (flag-don't-force: no other pool
/// card reads this count).
fn fill_auras_attached_to_dying_creature(effect: Effect, auras: u32) -> Effect {
    map_effect_amounts(effect, &|amount| match amount {
        Amount::AurasYouControlledAttachedToDyingCreature => Amount::Fixed(auras as i32),
        other => other,
    })
}

/// Rewrite a `CastSpell` (magecraft) trigger's `Amount::TriggeringSpellManaValue` placeholders to
/// the triggering spell's mana value (Renegade Bull's "+X/+0 … where X is that spell's mana
/// value"), and resolve its `Condition::TriggeringSpellManaValueAtLeast` gates against the same
/// value right now (Prismari Pianist's "if that spell's mana value is 5 or greater, create three
/// of those tokens instead") — CR 603.4: the triggering spell's mana value is locked in when the
/// trigger goes on the stack, so baking the branch here (rather than leaving it for a live
/// intervening-if, which has no `TriggerContext` to read at general resolution time) is faithful.
/// Recurses into a [`Effect::Sequence`] so a multi-step ability shares the one mana value across
/// every step, mirroring [`fill_dying_source_amounts`] above; every other effect passes through
/// unchanged.
fn fill_cast_mana_value(effect: Effect, mv: u32) -> Effect {
    let fill = |amount: Amount| match amount {
        Amount::TriggeringSpellManaValue => Amount::Fixed(mv as i32),
        other => other,
    };
    match effect {
        Effect::PumpUntilEndOfTurn {
            power,
            toughness,
            target,
            keywords,
        } => Effect::PumpUntilEndOfTurn {
            power: fill(power),
            toughness: fill(toughness),
            target,
            keywords,
        },
        Effect::CreateToken {
            token,
            count,
            controller,
            enters_with,
            set_base_pt,
            exile_at_next_end_step,
            enters_tapped_and_attacking,
            attacking_context,
            must_attack_defender,
        } => Effect::CreateToken {
            token,
            count: fill(count),
            controller,
            enters_with: fill(enters_with),
            set_base_pt: set_base_pt.map(fill),
            exile_at_next_end_step,
            enters_tapped_and_attacking,
            attacking_context,
            must_attack_defender,
        },
        Effect::Conditional {
            condition: Condition::TriggeringSpellManaValueAtLeast { at_least },
            then,
        } => {
            if mv < u32::from(at_least) {
                return Effect::Sequence { steps: &[] };
            }
            let filled: Vec<Effect> = then
                .iter()
                .map(|&step| fill_cast_mana_value(step, mv))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_cast_mana_value(step, mv))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}

/// Rewrite a `CastSpell` trigger's `Amount::TriggeringSpellManaSpent` placeholder to the mana
/// actually spent on the triggering spell (CR 601.2h) — Manaform Hellkite's "X is the amount of
/// mana spent to cast that spell," the mana-*spent* sibling of [`fill_cast_mana_value`] above
/// (which reads mana *value* instead). Reuses the generic [`map_effect_amounts`] walker rather
/// than duplicating [`fill_cast_mana_value`]'s bespoke match: this placeholder needs no
/// `Condition`-gate rewrite (unlike `fill_cast_mana_value`'s `TriggeringSpellManaValueAtLeast`
/// handling), so every effect shape it can appear in is already covered there.
fn fill_cast_mana_spent(effect: Effect, spent: u32) -> Effect {
    map_effect_amounts(effect, &|amount| match amount {
        Amount::TriggeringSpellManaSpent => Amount::Fixed(spent as i32),
        other => other,
    })
}

/// Rewrite a [`Trigger::YouCastThis`] self-cast trigger's `Amount::X`/`Amount::HalfXRoundedDown`
/// placeholders to the triggering spell's chosen `{X}` (Hydroid Krasis's "you gain half X life
/// and draw half X cards, rounded down") — a triggered ability's own resolution otherwise reads
/// `x = 0` (only a spell carries an `{X}` choice), so this is the only way the value reaches the
/// effect. Recurses into a [`Effect::Sequence`] so a multi-step ability shares the one `{X}`
/// across every step, mirroring [`fill_cast_mana_value`] above.
fn fill_cast_x(effect: Effect, x: u32) -> Effect {
    map_effect_amounts(effect, &|amount| match amount {
        Amount::X => Amount::Fixed(x as i32),
        Amount::HalfX => Amount::Fixed(x.div_ceil(2) as i32),
        Amount::HalfXRoundedDown => Amount::Fixed((x / 2) as i32),
        other => other,
    })
}

/// Rewrite a `DealsCombatDamageToPlayer` trigger's
/// [`CardFilter::CreatureWithManaValueAtMostCombatDamage`] placeholder to a resolved
/// [`CardFilter::CreatureWithManaValueAtMost`] bounded by the damage the source just dealt
/// (Venerable Warsinger's "mana value X or less … where X is the amount of damage this creature
/// dealt to that player") — CR 510.2/603.10a last-known information, locked in when the trigger
/// goes on the stack, same shape as [`fill_dying_source_amounts`] above. Recurses into a
/// [`Effect::Sequence`] so a multi-step ability shares the one damage amount across every step,
/// mirroring [`fill_entering_permanent`]; every other effect passes through unchanged.
fn fill_combat_damage(effect: Effect, damage: i32) -> Effect {
    match effect {
        Effect::ReanimateToBattlefield {
            target:
                TargetSpec::CardInGraveyard {
                    whose,
                    filter: CardFilter::CreatureWithManaValueAtMostCombatDamage,
                },
            finality,
            becomes,
        } => Effect::ReanimateToBattlefield {
            target: TargetSpec::CardInGraveyard {
                whose,
                filter: CardFilter::CreatureWithManaValueAtMost(damage.max(0) as u8),
            },
            finality,
            becomes,
        },
        // Primo, the Unbounded's "Put a number of +1/+1 counters on it equal to the damage
        // dealt": only `enters_with` (the minted Fractal's counter count) is a placeholder here —
        // every other `CreateToken` field passes through unchanged.
        Effect::CreateToken {
            enters_with: Amount::CombatDamageDealt,
            token,
            count,
            controller,
            set_base_pt,
            exile_at_next_end_step,
            enters_tapped_and_attacking,
            attacking_context,
            must_attack_defender,
        } => Effect::CreateToken {
            token,
            count,
            controller,
            enters_with: Amount::Fixed(damage.max(0)),
            set_base_pt,
            exile_at_next_end_step,
            enters_tapped_and_attacking,
            attacking_context,
            must_attack_defender,
        },
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_combat_damage(step, damage))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}

/// Fill an `Attacks` trigger's [`CardFilter::NonlandPermanentWithManaValueAtMostSourcePower`]
/// placeholder with the attacker's power, read at trigger placement (CR 510.2/603.10a last-known
/// information — Guardian Scalelord's "where X is this creature's power"). A negative power floors
/// at 0. Every other effect passes through unchanged; recurses into a [`Effect::Sequence`] like
/// [`fill_combat_damage`].
fn fill_source_power(effect: Effect, power: i32) -> Effect {
    match effect {
        Effect::ReanimateToBattlefield {
            target:
                TargetSpec::CardInGraveyard {
                    whose,
                    filter: CardFilter::NonlandPermanentWithManaValueAtMostSourcePower,
                },
            finality,
            becomes,
        } => Effect::ReanimateToBattlefield {
            target: TargetSpec::CardInGraveyard {
                whose,
                filter: CardFilter::NonlandPermanentWithManaValueAtMost(power.max(0) as u8),
            },
            finality,
            becomes,
        },
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_source_power(step, power))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}

/// Fill an activated ability's [`Amount::SacrificedCreaturePower`] placeholders with the power
/// of the creature just paid as this ability's sacrifice cost (Dina, Soul Steeper's "+X/+0";
/// Dina, Essence Brewer's "gain X life and put X counters"), read *before* the sacrifice — the
/// creature is gone by the time the ability resolves, so [`Amount::SourcePower`] can't reach it
/// from the stack. Recurses one level into a [`Effect::Sequence`] so a combo ability (Dina,
/// Essence Brewer's "gain X life *and* put X counters") shares one recorded value across both
/// steps; every other effect passes through unchanged. Called at [`Game::activate_ability`],
/// mirroring how [`contextualize_effect`] fills a triggered ability's context at placement.
pub(crate) fn contextualize_sacrifice_effect(effect: Effect, power: i32) -> Effect {
    let fill = |amount: Amount| match amount {
        Amount::SacrificedCreaturePower => Amount::Fixed(power),
        other => other,
    };
    match effect {
        Effect::PumpSelfUntilEndOfTurn {
            power: p,
            toughness,
        } => Effect::PumpSelfUntilEndOfTurn {
            power: fill(p),
            toughness: fill(toughness),
        },
        Effect::GainLife { amount } => Effect::GainLife {
            amount: fill(amount),
        },
        Effect::PutCounters {
            count,
            target,
            targets,
            kind,
            divided,
        } => Effect::PutCounters {
            count: fill(count),
            target,
            targets,
            kind,
            divided,
        },
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| contextualize_sacrifice_effect(step, power))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}
