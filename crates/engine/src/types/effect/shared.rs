use super::*;
#[cfg(feature = "card-dsl")]
use crate::de;

/// A numeric quantity in an effect: a fixed number, the casting spell's (or activated ability's)
/// chosen `{X}`, or a value derived from game state (a board/graveyard count, a permanent's
/// power, a turn tally). Every amount resolves through [`Game::resolve_amount`] /
/// [`Game::resolve_count`] — even the trivial `Fixed`/`X` — so a new derived variant is a single
/// match arm with no separate pure path. (CR 602, CR 403.5, CR 601)
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
    /// The targeted permanent's toughness — the toughness twin of [`TargetPower`](Self::TargetPower)
    /// (Condemn: "its controller gains life equal to its toughness"). Like `TargetPower`, this
    /// must resolve before the target leaves the battlefield (CR 613.6/603.10a last-known
    /// information) — the effect that reads it runs ahead of any move/tuck step in the sequence.
    TargetToughness,
    /// The targeted object's mana value.
    TargetManaValue,
    /// The number of +1/+1 counters on the effect's source.
    PerCounterOnSource,
    /// The number of `kind`-counters on the effect's source (astral_cornucopia's "add one mana
    /// of any color for each charge counter on this artifact") — the named-counter-kind sibling
    /// of [`PerCounterOnSource`](Self::PerCounterOnSource).
    PerCounterOfKindOnSource {
        kind: CounterKind,
    },
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
    /// [`Effect::Static(StaticEffect::GrantToAttached)`] static using it tracks the hand as it grows or shrinks.
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
    /// The number of battlefield permanents the effect's controller *owns* but does not control —
    /// i.e. that an opponent controls (Zedruu the Greathearted's "the number of permanents you own
    /// that your opponents control"). Compares [`Game::owner_of`] against [`Game::controller_of`];
    /// a permanent you own can only be controlled by you or an opponent (no teams), so
    /// owner-is-you-but-controller-isn't is exactly "an opponent controls it," counted once
    /// regardless of how many opponents there are.
    PermanentsYouOwnOpponentsControl,
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
    /// The toughness of the creature just paid as this ability's sacrifice cost — the toughness
    /// twin of [`SacrificedCreaturePower`](Self::SacrificedCreaturePower) (Miren, the Moaning
    /// Well's "You gain life equal to the sacrificed creature's toughness"). Same placeholder
    /// shape: [`contextualize_sacrifice_effect`] rewrites it to [`Fixed`](Self::Fixed) with the
    /// sacrificed creature's toughness at ability placement — resolving this variant directly is
    /// a bug (see [`Game::resolve_amount`]'s panic).
    SacrificedCreatureToughness,
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
    /// The mana value of the creature card revealed to pay the resolving spell's
    /// [`AdditionalCost::reveal_creature_from_hand`] (CR 601.2g) — Disaster Radius's "X is the
    /// revealed card's mana value." Reads [`Game::revealed_creature_mana_value`] off the effect's
    /// `source` (the resolving spell itself), the reveal-cost sibling of
    /// [`SpellSacrificeCount`](Self::SpellSacrificeCount)'s read.
    RevealedCreatureManaValue,
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
    /// How many permanents matching `filter` *this resolution's own* `Effect::Destroy(DestroyEffect::DestroyAll)` step
    /// just destroyed (CR "destroyed this way" riders) — Ceaseless Conflict's "for each nontoken
    /// creature you controlled that was destroyed this way" token count, Culling Ritual's "for
    /// each permanent destroyed this way" mana count. Resolution-scoped, not turn-scoped: reads
    /// [`ResolutionFrame::destroyed_this_way`](crate::resolution::ResolutionFrame), a snapshot [`Effect::Destroy(DestroyEffect::DestroyAll)`]'s own `run`
    /// special case overwrites (not accumulates) each time it runs, since an `Effect::Sequence`
    /// doesn't apply steps' events back to live battlefield state between steps (the destroyed
    /// permanents are already gone by the time a later step counts them). `#[serde(default)]`
    /// `filter` matches every destroyed permanent (Culling Ritual's unfiltered count).
    PermanentsDestroyedThisWay {
        filter: PermanentFilter,
    },
    /// How many *nonland* cards this resolution's own [`Effect::Choice(ChoiceEffect::EachPlayerExilesFromGraveyard)`]
    /// step just exiled across every player (Augusta, Order Returned's "put that many +1/+1
    /// counters"). Resolution-scoped like [`PermanentsDestroyedThisWay`](Self::PermanentsDestroyedThisWay):
    /// reads [`ResolutionFrame::nonland_cards_exiled_this_way`](crate::resolution::ResolutionFrame), overwritten (not accumulated) each time the
    /// fan-out begins.
    NonlandCardsExiledThisWay,
    /// How many cards this resolution's own [`Effect::Dig(DigEffect::SearchLibrary)`] step just moved to an
    /// `Exile` destination (Trench Gorger: "has base power and toughness each equal to the number
    /// of cards exiled this way"). Resolution-scoped like
    /// [`NonlandCardsExiledThisWay`](Self::NonlandCardsExiledThisWay): reads
    /// [`ResolutionFrame::cards_exiled_by_search_this_way`](crate::resolution::ResolutionFrame),
    /// reset to 0 when the search begins and incremented per pick, so a declined "may" search
    /// never reaches this read at all (the whole ability doesn't run) while a search that finds
    /// zero matches correctly reads 0.
    CardsExiledBySearchThisWay,
    /// How many "past" votes this resolution's own [`Effect::Choice(ChoiceEffect::CouncilsDilemmaVote)`] round tallied
    /// (Fateful Tempest's "mill a card for each past vote"). Reads [`ResolutionFrame::council_past_votes`](crate::resolution::ResolutionFrame), a
    /// resolution-scoped tally reset when the vote round begins — the vote-round sibling of
    /// [`NonlandCardsExiledThisWay`](Self::NonlandCardsExiledThisWay).
    /// The total mana every player paid into this resolution's own
    /// [`Effect::Choice(ChoiceEffect::JoinForcesPayMana)`] round — Collective Voyage's "X is the total amount of mana
    /// paid this way". Reads [`ResolutionFrame::join_forces_mana`](crate::resolution::ResolutionFrame),
    /// a resolution-scoped tally reset when the round begins, like [`PastVotes`](Self::PastVotes).
    ManaPaidThisWay,
    PastVotes,
    /// How many "present" votes this resolution's own [`Effect::Choice(ChoiceEffect::CouncilsDilemmaVote)`] round tallied
    /// (Fateful Tempest's "Exile the top card of your library for each present vote"). Reads
    /// [`ResolutionFrame::council_present_votes`](crate::resolution::ResolutionFrame), the present-ballot twin of [`PastVotes`](Self::PastVotes).
    PresentVotes,
    /// The total mana value of the cards this resolution's own [`Effect::Mill(MillEffect::MillSelf)`] step just
    /// milled (Fateful Tempest's "damage to each opponent equal to the total mana value of cards
    /// milled this way"). Reads [`ResolutionFrame::milled_mana_value_this_way`](crate::resolution::ResolutionFrame), snapshotted at the mill choke
    /// — resolution-scoped, like [`NonlandCardsExiledThisWay`](Self::NonlandCardsExiledThisWay).
    TotalManaValueMilledThisWay,
    /// The mana value of the card this resolution's own
    /// [`Effect::Dig(DigEffect::ExileTargetGraveyardCardRecordManaValue)`] step just exiled (Surge to Victory's
    /// "Creatures you control get +X/+0 until end of turn, where X is that card's mana value").
    /// Reads [`ResolutionFrame::surge_exiled_card`](crate::resolution::ResolutionFrame), snapshotted at the exile choke — resolution-scoped,
    /// like [`TotalManaValueMilledThisWay`](Self::TotalManaValueMilledThisWay). `0` if unset (the
    /// exile step never ran — unreachable in practice, since a fizzled target drops this whole
    /// ability before either step resolves, CR 608.2b).
    ExiledCardManaValueThisWay,
    /// The mana value of the **nonland** card this resolution's own
    /// [`Effect::Zone(ZoneEffect::ReturnFromGraveyardToHand)`] step just returned to its owner's hand (Vengeful
    /// Rebirth's "If you return a nonland card to your hand this way, Vengeful Rebirth deals
    /// damage equal to that card's mana value to any target"). Reads
    /// [`ResolutionFrame::returned_nonland_card_mana_value`](crate::resolution::ResolutionFrame),
    /// snapshotted at the return choke — resolution-scoped, like
    /// [`ExiledCardManaValueThisWay`](Self::ExiledCardManaValueThisWay). `0` when a *land* came
    /// back or the return step never ran, which is exactly the oracle's "if you return a nonland
    /// card" gate: a source that would deal 0 damage deals none at all (CR 120.8).
    ReturnedNonlandCardManaValue,
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
    /// The combat damage just dealt to a player, from either of two `DealsCombatDamageToPlayer`-
    /// family placements: the summed damage a
    /// [`Trigger::ZeroBasePowerCreaturesYouControlDealCombatDamage`] watch's whole batch of
    /// base-power-0 attackers dealt (Primo, the Unbounded's "Put a number of +1/+1 counters on it
    /// equal to the damage dealt"), or the damage a single `who = "this"`
    /// [`Trigger::DealsCombatDamageToPlayer`] source dealt on its own (Rapacious One's "create
    /// that many 0/1 … Eldrazi Spawn creature tokens"). A placeholder, like
    /// [`SacrificedCreaturePower`](Self::SacrificedCreaturePower) above:
    /// [`fill_combat_damage`] rewrites it to [`Fixed`](Self::Fixed) with the dealt damage when
    /// the watch's ability is placed on the stack — resolving this variant directly never
    /// happens (see [`Game::resolve_amount`]'s fallback).
    CombatDamageDealt,
    /// The amount of damage — combat or noncombat alike — the enchanted host of a
    /// [`Trigger::EnchantedCreatureDealsDamage`] watch just dealt (CR 609.7, Armadillo Cloak:
    /// "you gain that much life"). A placeholder, like [`CombatDamageDealt`](Self::CombatDamageDealt)
    /// above: [`fill_triggering_damage_dealt`] rewrites it to [`Fixed`](Self::Fixed) with the dealt
    /// amount when the watch's ability is placed on the stack — resolving this variant directly
    /// never happens (see [`Game::resolve_amount`]'s fallback). Distinct from `CombatDamageDealt`,
    /// which is specifically the summed *combat* damage a base-power-0 batch dealt *a player*.
    TriggeringDamageDealt,
    /// CR 702.40a's storm count: "for each spell cast before it this turn" — the game-wide tally
    /// (every player) of spells cast before this one this turn (Reaping the Graves' Storm). A
    /// placeholder, like [`TriggeringSpellManaValue`](Self::TriggeringSpellManaValue) above:
    /// `fill_spells_cast_before_this` rewrites it to [`Fixed`](Self::Fixed) with the snapshotted
    /// count when a [`Trigger::YouCastThis`] ability is placed on the stack — resolving this
    /// variant directly never happens (see [`Game::resolve_amount`]'s fallback).
    SpellsCastBeforeThisThisTurn,
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

/// Which land taps an [`Effect::Static(StaticEffect::TappedForManaBonus)`] watch reacts to (CR "whenever … is tapped
/// for mana").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum LandTapScope {
    /// Only the land this watcher (an Aura) is attached to — Fertile Ground's "enchanted land".
    #[default]
    EnchantedHost,
    /// Every land the watcher's controller taps for mana — Mirari's Wake's "whenever you tap a
    /// land for mana".
    Controller,
}

/// What color the bonus mana of an [`Effect::Static(StaticEffect::TappedForManaBonus)`] watch is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum LandTapBonusColor {
    /// "One mana of any color" — the controller names it, so the bonus pauses on a
    /// [`PendingChoice::ChooseManaColor`](crate::PendingChoice::ChooseManaColor) (Fertile Ground).
    #[default]
    AnyColor,
    /// "One mana of any type that land produced" — copied from the type the tap just produced, no
    /// pause when the tap made a single concrete credit (Mirari's Wake).
    Produced,
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
/// see [`Effect::Pump(PumpEffect::PumpUntilEndOfTurn)`]'s `keywords` field for the canonical example.
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
    Damage(DamageEffect),
    Draw(DrawEffect),
    Life(LifeEffect),
    Destroy(DestroyEffect),
    Exile(ExileEffect),
    Sacrifice(SacrificeEffect),
    Control(ControlEffect),
    Counters(CountersEffect),
    Mana(ManaEffect),
    Mill(MillEffect),
    Pump(PumpEffect),
    Reveal(RevealEffect),
    Token(TokenEffect),
    Zone(ZoneEffect),
    Copy(CopyEffect),
    Dig(DigEffect),
    Choice(ChoiceEffect),
    Static(StaticEffect),
    Misc(MiscEffect),
    Sequence {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_slice"))]
        steps: &'static [Effect],
    },
    ChooseOne {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_slice"))]
        options: &'static [Effect],
    },
    Conditional {
        condition: Condition,
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_slice"))]
        then: &'static [Effect],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        negate: bool,
    },
}


impl Effect {
    /// A plain "add `amount` {C}" mana ability, built at runtime so a delayed trigger can bake in a
    /// count known only at schedule time (Scattering Stroke's {C}-per-mana-value rider). Every other
    /// [`ManaEffect::Add`](crate::ManaEffect::Add) field takes its ordinary card default.
    pub(crate) fn add_colorless(amount: u8) -> Effect {
        Effect::Mana(ManaEffect::Add {
            mana: ManaPool::of(Mana::Colorless, amount),
            identity: 0,
            opponent_colors: 0,
            repeat: Amount::Fixed(1),
            restriction: None,
            single_color: false,
            track_provenance: false,
            target: TargetSpec::None,
            persist_until_end_of_turn: false,
            recipient: None,
        })
    }

    /// What this effect targets (most effects target nothing).
    pub(crate) fn target(self) -> TargetSpec {
        match self {
            Effect::Damage(DamageEffect::Target { target, .. })
            | Effect::Pump(PumpEffect::PumpUntilEndOfTurn { target, .. })
            | Effect::Pump(PumpEffect::SetBasePtTargetUntilEndOfTurn { target, .. })
            | Effect::Counters(CountersEffect::PutCounters { target, .. })
            | Effect::Counters(CountersEffect::DoubleCounters { target })
            | Effect::Counters(CountersEffect::DoubleCountersOnTargetCreatures { target, .. })
            | Effect::Counters(CountersEffect::MoveCounters { target, .. })
            | Effect::Counters(CountersEffect::RemoveAllCountersThenDraw { target })
            | Effect::Exile(ExileEffect::Target { target, .. })
            | Effect::Exile(ExileEffect::UntilSourceLeaves { target })
            | Effect::Exile(ExileEffect::TargetMintingIllusionOnLeave { target })
            | Effect::Zone(ZoneEffect::FlickerTarget { target, .. })
            | Effect::Zone(ZoneEffect::ReturnFromGraveyardToHand { target, .. })
            | Effect::Zone(ZoneEffect::ReanimateToBattlefield { target, .. })
            | Effect::Zone(ZoneEffect::TuckFromGraveyard { target, .. })
            | Effect::Mill(MillEffect::Mill { target, .. })
            | Effect::Choice(ChoiceEffect::TargetPlayerExilesFromGraveyard { target })
            | Effect::Control(ControlEffect::GoadTarget { target })
            | Effect::Token(TokenEffect::CreateCopy { target, .. })
            | Effect::Control(ControlEffect::TapTarget { target, .. })
            | Effect::Control(ControlEffect::UntapTarget { target, .. })
            | Effect::Control(ControlEffect::RemoveFromCombat { target })
            | Effect::Control(ControlEffect::GainControlUntilEndOfTurn { target })
            | Effect::Control(ControlEffect::ExchangeAllCreaturesUntilEndOfTurn { target })
            | Effect::Control(ControlEffect::GainControl { target })
            | Effect::Control(ControlEffect::GainControlWhile { target, .. })
            | Effect::Control(ControlEffect::TargetOpponentGainsControl { target, .. })
            | Effect::Zone(ZoneEffect::ShuffleTargetPermanentIntoLibraryThenReveal { target })
            | Effect::Zone(ZoneEffect::ShuffleTargetPermanentIntoLibrary { target })
            | Effect::Zone(ZoneEffect::TuckPermanentIntoLibrary { target, .. })
            | Effect::Control(ControlEffect::RegenerateShield { target })
            | Effect::Zone(ZoneEffect::AttachMintedAuraToTarget { target })
            | Effect::Token(TokenEffect::BecomeCopyOfTargetCreatureGainingMyriad { target })
            | Effect::Copy(CopyEffect::ChangeTargetOfTargetSpellOrAbility { target, .. })
            | Effect::Destroy(DestroyEffect::Target { target, .. }) => target,
            Effect::Zone(ZoneEffect::ReturnToHand { target, .. }) => target,
            // The first target clause is the ability's own target; the second is chosen separately
            // (see `Game::ability_second_target_clause`) and read off `targets_second`.
            Effect::Control(ControlEffect::ExchangeControl { first, .. }) => first,
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
            Effect::Mill(MillEffect::ExileTargetFromGraveyardWithThis) => TargetSpec::CardInGraveyard {
                whose: GraveyardScope::Yours,
                filter: CardFilter::NoncreatureNonland,
                other: false,
            },
            // Renegade Bull's attack trigger: an authored filter (instant-or-sorcery), unlike
            // its fixed-filter sibling above.
            Effect::Dig(DigEffect::ExileTargetGraveyardSpellCastFree { filter, .. }) => {
                TargetSpec::CardInGraveyard {
                    whose: GraveyardScope::Yours,
                    filter,
                    other: false,
                }
            }
            // Restore Relic: an authored filter (artifact-or-creature), same shape as its
            // copy-and-cast-free sibling above.
            Effect::Mill(MillEffect::ExileTargetFromGraveyardCreateTokenCopy { filter }) => {
                TargetSpec::CardInGraveyard {
                    whose: GraveyardScope::Yours,
                    filter,
                    other: false,
                }
            }
            // Feral Appetite: any card in any graveyard — no fixed filter (unlike its
            // noncreature-nonland sibling `ExileTargetFromGraveyardWithThis`) and no authored
            // one (unlike its instant-or-sorcery/artifact-or-creature siblings above).
            Effect::Zone(ZoneEffect::ExileTargetGraveyardCardThenIfCreature { .. }) => TargetSpec::CardInGraveyard {
                whose: GraveyardScope::Any,
                filter: CardFilter::AnyCard,
                other: false,
            },
            // Surge to Victory: an authored filter (instant-or-sorcery), same shape as its
            // copy-and-cast-free sibling above — this one just doesn't mint the copy itself.
            Effect::Dig(DigEffect::ExileTargetGraveyardCardRecordManaValue { filter }) => {
                TargetSpec::CardInGraveyard {
                    whose: GraveyardScope::Yours,
                    filter,
                    other: false,
                }
            }
            // Forum Filibuster's reflexive body: "up to one target Aura or Equipment card from
            // your graveyard" (the count is "up to one" — see `target_count`).
            Effect::Zone(ZoneEffect::ReturnFromGraveyardAttachedToToken { filter, .. }) => {
                TargetSpec::CardInGraveyard {
                    whose: GraveyardScope::Yours,
                    filter,
                    other: false,
                }
            }
            Effect::Copy(CopyEffect::TargetSpell) => TargetSpec::InstantOrSorcerySpellOnStack,
            Effect::Misc(MiscEffect::CounterTargetSpell { filter, .. }) => TargetSpec::SpellOnStack(filter),
            Effect::Misc(MiscEffect::CounterTargetActivatedAbility) => TargetSpec::ActivatedAbilityOnStack,
            // The cast-time target is the *opponent's* creature; the controller's own creature
            // is chosen at resolution (see `Effect::Misc(MiscEffect::Fight)`'s doc comment).
            Effect::Misc(MiscEffect::Fight {
                ally_is_shared_target: false,
                ..
            }) => TargetSpec::Permanent(PermanentFilter {
                controller: FilterController::Opponent,
                ..PermanentFilter::of(TypeSet::CREATURE)
            }),
            // Primal Might's mirror shape: the ally is a *preceding* Sequence step's target
            // (the pump); this step defers to it, same rule as the no-target-of-its-own steps
            // above.
            Effect::Misc(MiscEffect::Fight {
                ally_is_shared_target: true,
                ..
            }) => TargetSpec::None,
            Effect::Draw(DrawEffect::TargetPlayer { opponent: true, .. })
            | Effect::Life(LifeEffect::DrainTarget { opponent: true, .. })
            | Effect::Reveal(RevealEffect::TopAndDrainMutual)
            | Effect::Life(LifeEffect::TargetPlayerGains { opponent: true, .. })
            | Effect::Choice(ChoiceEffect::TargetPlayerMayDraw { opponent: true, .. })
            | Effect::Choice(ChoiceEffect::MayDrawUpToThenOpponentMayRepeat { .. })
            | Effect::Token(TokenEffect::Create {
                controller: TokenController::TargetOpponent,
                ..
            }) => TargetSpec::OpponentPlayer,
            Effect::Draw(DrawEffect::TargetPlayer { opponent: false, .. })
            | Effect::Life(LifeEffect::DrainTarget { opponent: false, .. })
            | Effect::Life(LifeEffect::TargetPlayerGains { opponent: false, .. })
            | Effect::Choice(ChoiceEffect::TargetPlayerMayDraw { opponent: false, .. })
            | Effect::Exile(ExileEffect::Graveyard)
            | Effect::Life(LifeEffect::TargetPlayerLoses { .. })
            | Effect::Choice(ChoiceEffect::Discard {
                target_player: true,
                ..
            })
            | Effect::Token(TokenEffect::CreateTreasure {
                target_player: true,
                ..
            })
            | Effect::Token(TokenEffect::Create {
                controller: TokenController::TargetPlayer,
                ..
            })
            | Effect::Token(TokenEffect::Create {
                controller: TokenController::EachOtherPlayer,
                ..
            })
            | Effect::Counters(CountersEffect::PutCountersEach {
                target_player: true,
                ..
            })
            | Effect::Dig(DigEffect::ShuffleTargetCardsFromGraveyardIntoLibrary {
                target_player: true,
                ..
            }) => TargetSpec::Player,
            // Equip targets the creature to attach to (the "you control" restriction is
            // enforced when the ability is activated, not by the target spec).
            Effect::Control(ControlEffect::Equip) => TargetSpec::Creature,
            // Breena's counter half: "a creature you control" (the drawing player is context,
            // not a target) — restricted to the ability's controller's own creatures.
            Effect::Counters(CountersEffect::AttackerDrawsControllerCounters { .. }) => TargetSpec::CreatureYouControl,
            // A mana ability targets a player only when authored to (Rousing Refrain's "target
            // opponent"); every ordinary mana source defaults to `TargetSpec::None`.
            Effect::Mana(ManaEffect::Add { target, .. }) => target,
            Effect::Draw(DrawEffect::Cards { .. })
            | Effect::Choice(ChoiceEffect::MayDrawUpTo { .. })
            | Effect::Life(LifeEffect::Gain { .. })
            | Effect::Life(LifeEffect::OpponentGains { .. })
            | Effect::Token(TokenEffect::Create { .. })
            | Effect::Token(TokenEffect::CreateTreasure {
                target_player: false,
                ..
            })
            | Effect::Copy(CopyEffect::ThisSpell { .. })
            | Effect::Copy(CopyEffect::RetargetSpellCopy { .. })
            | Effect::Copy(CopyEffect::MayPayToCopyThis { .. })
            | Effect::Copy(CopyEffect::CopyTriggeringSpell { .. })
            | Effect::Copy(CopyEffect::CopyTriggeringSpellForEachOtherCreatureYouControl { .. })
            | Effect::Copy(CopyEffect::CopyTriggeringAbility { .. })
            | Effect::Copy(CopyEffect::Demonstrate { .. })
            | Effect::Counters(CountersEffect::CommanderEntersWithBonusCounters { .. })
            | Effect::Mill(MillEffect::ExileTopMayPlay { .. })
            | Effect::Dig(DigEffect::ExileTopCastMatchingFree { .. })
            | Effect::Dig(DigEffect::Cascade { .. })
            | Effect::Mill(MillEffect::ExileFromGraveyardMayPlay { .. })
            | Effect::Mill(MillEffect::ExileDiscardedWithThis { .. })
            | Effect::Dig(DigEffect::CashOutExiledWithThis)
            | Effect::Dig(DigEffect::CastExiledWithThisFree)
            | Effect::Static(StaticEffect::GrantToAttached { .. })
            | Effect::Static(StaticEffect::SetAttachedBasePt { .. })
            | Effect::Static(StaticEffect::SetAttachedTypes { .. })
            | Effect::Life(LifeEffect::EachOpponentDrain { .. })
            | Effect::Life(LifeEffect::EachOpponentLoses { .. })
            | Effect::Life(LifeEffect::EachPlayerBecomesHighest)
            | Effect::Dig(DigEffect::Scry { .. })
            | Effect::Dig(DigEffect::Surveil { .. })
            | Effect::Dig(DigEffect::LookAtTop { .. })
            | Effect::Dig(DigEffect::DistributeTop { .. })
            | Effect::Reveal(RevealEffect::TopToHand { .. })
            | Effect::Reveal(RevealEffect::Until { .. })
            | Effect::Dig(DigEffect::RevealUntilMayDeploy { .. })
            | Effect::Dig(DigEffect::RevealUntilExileCastFree { .. })
            | Effect::Dig(DigEffect::ShuffleLibrary)
            | Effect::Dig(DigEffect::ExileTopUntilStopCastFreeUnderBudget { .. })
            | Effect::Reveal(RevealEffect::TopCards { .. })
            | Effect::Dig(DigEffect::SearchLibrary { .. })
            | Effect::Static(StaticEffect::ReduceSpellCost { .. })
            | Effect::Static(StaticEffect::CounterReplacement { .. })
            | Effect::Static(StaticEffect::TokenReplacement { .. })
            | Effect::Static(StaticEffect::LifeGainReplacement { .. })
            | Effect::Static(StaticEffect::CastXReplacement { .. })
            | Effect::Static(StaticEffect::EntersWithCounters { .. })
            | Effect::Static(StaticEffect::CreaturesYouControlEnterWithCounters { .. })
            | Effect::Destroy(DestroyEffect::All { .. })
            | Effect::Exile(ExileEffect::All { .. })
            | Effect::Exile(ExileEffect::AllGraveyards)
            | Effect::Zone(ZoneEffect::ReturnAllToHand { .. })
            | Effect::Zone(ZoneEffect::MassReturnFromGraveyard { .. })
            | Effect::Dig(DigEffect::ShuffleTargetCardsFromGraveyardIntoLibrary {
                target_player: false,
                ..
            })
            | Effect::Damage(DamageEffect::EachCreature { .. })
            | Effect::Damage(DamageEffect::EachPlayer { .. })
            | Effect::Damage(DamageEffect::EachOtherOpponent { .. })
            | Effect::Pump(PumpEffect::WeakenEachCreature { .. })
            | Effect::Pump(PumpEffect::PumpCreaturesYouControlUntilEndOfTurn { .. })
            | Effect::Pump(PumpEffect::GrantKeywordsToPermanentsYouControlUntilEndOfTurn { .. })
            | Effect::Pump(PumpEffect::PumpOtherAttackersAttackingYourOpponents { .. })
            | Effect::Pump(PumpEffect::EnchantedAttackerPumpAttackingOpponentElseControllerLosesLife { .. })
            | Effect::Pump(PumpEffect::StripKeywordsFromOpponentsCreatures { .. })
            | Effect::Pump(PumpEffect::PumpSelfUntilEndOfTurn { .. })
            | Effect::Static(StaticEffect::ControlAttached)
            | Effect::Choice(ChoiceEffect::EachPlayerSacrifices { .. })
            | Effect::Choice(ChoiceEffect::EachPlayerExilesFromGraveyard)
            | Effect::Choice(ChoiceEffect::CasterKeepsOneOfEachTypePerPlayer)
            | Effect::Choice(ChoiceEffect::EachPlayerControllerChoosesCounterTarget)
            | Effect::Choice(ChoiceEffect::CouncilsDilemmaVote { .. })
            | Effect::Choice(ChoiceEffect::JoinForcesPayMana)
            | Effect::Choice(ChoiceEffect::EachPlayerNamesCardThenRevealsTop)
            | Effect::Dig(DigEffect::OpponentSplitsExilePiles)
            | Effect::Dig(DigEffect::RevealTopSplitPiles)
            | Effect::Dig(DigEffect::RevealTopOpponentPicksOneToGraveyard { .. })
            | Effect::Dig(DigEffect::EachPlayerExilesUntilNonlandOpponentPicks)
            | Effect::Choice(ChoiceEffect::EachPlayerCreatesFractalFromExiledPower { .. })
            | Effect::Choice(ChoiceEffect::EachOtherTokenBecomesCopyOfChosen)
            | Effect::Choice(ChoiceEffect::PutCounterThenMayBecomeCopyOfCardFromList { .. })
            | Effect::Choice(ChoiceEffect::EachPlayerDiscardsHandThenDraws { .. })
            | Effect::Choice(ChoiceEffect::MaySacrifice { .. })
            | Effect::Choice(ChoiceEffect::MayReturnFromGraveyard { .. })
            | Effect::Choice(ChoiceEffect::MayDiscard { .. })
            | Effect::Choice(ChoiceEffect::MayDrawUnlessPays { .. })
            | Effect::Counters(CountersEffect::PutCountersEach { .. })
            | Effect::Choice(ChoiceEffect::Proliferate { .. })
            | Effect::Choice(ChoiceEffect::Discard {
                target_player: false,
                ..
            })
            | Effect::Choice(ChoiceEffect::PutLandFromHand { .. })
            | Effect::Choice(ChoiceEffect::PutCreatureFromHand)
            | Effect::Choice(ChoiceEffect::PutFromHandOnTop { .. })
            | Effect::Choice(ChoiceEffect::CastCreatureFaceDown)
            | Effect::Control(ControlEffect::UntapAll { .. })
            | Effect::Control(ControlEffect::GainControlAllUntilEndOfTurn { .. })
            | Effect::Draw(DrawEffect::EachPlayer { .. })
            | Effect::Choice(ChoiceEffect::SacrificeOwn { .. })
            | Effect::Choice(ChoiceEffect::DefendingPlayerSacrifices { .. })
            | Effect::Sacrifice(SacrificeEffect::Object { .. })
            | Effect::Sacrifice(SacrificeEffect::Source)
            | Effect::Sacrifice(SacrificeEffect::EnchantedCreature { .. })
            | Effect::Destroy(DestroyEffect::TriggeringDamagedCreature { .. })
            | Effect::Exile(ExileEffect::Object { .. })
            | Effect::Zone(ZoneEffect::ReturnObjectToHand { .. })
            | Effect::Zone(ZoneEffect::ExileGraveyardObjectGainLife { .. })
            | Effect::Mill(MillEffect::MillSelf { .. })
            | Effect::Zone(ZoneEffect::ExileSelfWithTimeCounters { .. })
            | Effect::Zone(ZoneEffect::TuckSelfToLibraryBottom)
            | Effect::Zone(ZoneEffect::ExileSelfOnResolve)
            | Effect::Dig(DigEffect::ExileRandomFromGraveyardMayPlay)
            | Effect::Static(StaticEffect::Anthem { .. })
            | Effect::Static(StaticEffect::KeywordAnthem { .. })
            | Effect::Static(StaticEffect::TappedForManaBonus { .. })
            | Effect::Static(StaticEffect::TriggerDoubling { .. })
            | Effect::Static(StaticEffect::GrantManaAbility { .. })
            | Effect::Misc(MiscEffect::ScheduleAtNextUpkeep { .. })
            | Effect::Misc(MiscEffect::ScheduleColorlessManaForCounteredSpellNextMainPhase)
            | Effect::Misc(MiscEffect::SkipNextUntapOpponentCreatures)
            | Effect::Misc(MiscEffect::ScheduleNextCastTrigger { .. })
            | Effect::Life(LifeEffect::AttackerLosesYouGain { .. })
            | Effect::Life(LifeEffect::AttackerLosesYouDraw { .. })
            | Effect::Draw(DrawEffect::AttackingPlayer { .. })
            | Effect::Choice(ChoiceEffect::DamagingCreatureControllerMayDraw { .. })
            | Effect::Draw(DrawEffect::EachDrawStepPlayer { .. })
            | Effect::Damage(DamageEffect::ToEnteringPermanent { .. })
            | Effect::Zone(ZoneEffect::ReanimateDyingEnchantedCreature { .. })
            | Effect::Zone(ZoneEffect::ExileDeadCreatureCreateCopyWithSubtype { .. })
            | Effect::Zone(ZoneEffect::ReturnThisToHand)
            // The phase-out set is chosen at resolution (a resolution-time subset choice), not a
            // fixed target on the trigger — see the variant doc.
            | Effect::Choice(ChoiceEffect::PhaseOut)
            | Effect::Zone(ZoneEffect::ReturnThisFromGraveyardToBattlefield { .. })
            | Effect::Static(StaticEffect::AttackTax { .. })
            | Effect::Static(StaticEffect::CounterScaledAttackTax)
            | Effect::Static(StaticEffect::CantBeAttackedBy { .. })
            // Always names the ability's own source as the required attacker — no chosen target.
            | Effect::Misc(MiscEffect::MustAttackRandomOpponent)
            | Effect::Misc(MiscEffect::PreventCombatDamageToYouCreatingTokens { .. })
            | Effect::Misc(MiscEffect::PreventAllCombatDamageThisTurn)
            | Effect::Counters(CountersEffect::PlaceVowCounters { .. })
            | Effect::Life(LifeEffect::Lose { .. })
            | Effect::Damage(DamageEffect::ToSelf { .. })
            // A no-target-of-its-own step: reads the enclosing `Sequence`'s shared target.
            | Effect::Life(LifeEffect::GainTargetController { .. })
            // Reads the enclosing `Sequence`'s shared target creature's controller; no target of
            // its own (Lash Out's win rider).
            | Effect::Damage(DamageEffect::ToTargetController { .. })
            // A no-target-of-its-own step: reads the enclosing `Sequence`'s shared target's owner
            // or controller (Oblation's "then draws two cards" rider).
            | Effect::Draw(DrawEffect::TargetOwner { .. })
            // Clash picks its opponent at resolution (CR 701.22), not via a cast/activation target.
            | Effect::Dig(DigEffect::Clash)
            // A no-target-of-its-own step: manifests the enclosing `Sequence`'s shared target's
            // controller's top card (see the variant doc).
            | Effect::Zone(ZoneEffect::Manifest)
            // Arms a watch on the enclosing `Sequence`'s shared target (the creature the
            // preceding `pump_until_end_of_turn` step just deathtouched) — no target of its own.
            | Effect::Misc(MiscEffect::ArmCombatDamageWatch)
            // Arms the this-turn combat-damage-copy watch over `ResolutionFrame::surge_exiled_card` (the
            // enclosing `Sequence`'s own exile step just recorded it) — no target of its own.
            | Effect::Misc(MiscEffect::ScheduleThisTurnCombatDamageCopy)
            // `card` is filled in by the delayed watch when it fires, not a chosen target.
            | Effect::Copy(CopyEffect::MintFreeCopyOfExiledCard { .. })
            // A modal trigger's `target` is None — its modes are non-targeting (see the variant doc).
            | Effect::ChooseOne { .. }
            // "Become prepared" always affects the ability's own source, never a chosen target.
            | Effect::Misc(MiscEffect::BecomePrepared)
            // Flipping (CR 712) always affects the ability's own source, never a chosen target.
            | Effect::Misc(MiscEffect::FlipSource)
            // "Level N" always raises the ability's own source's level, never a chosen target.
            | Effect::Counters(CountersEffect::LevelUp { .. })
            // The as-enters creature-type/color choices always affect the ability's own source.
            | Effect::Choice(ChoiceEffect::ChooseCreatureType)
            | Effect::Choice(ChoiceEffect::ChooseColor)
            | Effect::Choice(ChoiceEffect::SetOwnColorUntilEndOfTurn)
            // Removes a counter from the ability's own source, never a chosen target.
            | Effect::Counters(CountersEffect::RemoveCounterFromSelf)
            // Grants the ability's controller a permission — no chosen target.
            | Effect::Misc(MiscEffect::GrantFlashThisTurn)
            | Effect::Misc(MiscEffect::GrantChannelColorlessManaThisTurn)
            // The searched land is read back from the resolution's own events, not a target.
            | Effect::Zone(ZoneEffect::UntapSearchedLand)
            // The attach address (a minted token, the triggering entering permanent, or a
            // reanimated creature) is read from trigger context / the resolution's own events,
            // not a chosen target.
            | Effect::Zone(ZoneEffect::AttachTriggeringAuraToMintedToken { .. })
            // A reflexive trigger's own steps are placed as separate abilities, each choosing its
            // own target when placed — this scheduler step takes no target of its own.
            | Effect::Zone(ZoneEffect::ReflexiveTrigger { .. })
            | Effect::Control(ControlEffect::AttachSelfToEntering { .. })
            | Effect::Zone(ZoneEffect::AttachSelfToReanimated)
            | Effect::Zone(ZoneEffect::AttachSelfToMintedToken)
            // Doubles the counters on whatever the ability's own source is attached to, not a
            // chosen target.
            | Effect::Counters(CountersEffect::DoubleCountersOnAttachedCreature)
            // The delayed return's host creature is read from trigger context / baked in at
            // schedule time, not a chosen target.
            | Effect::Zone(ZoneEffect::ScheduleReturnThisAuraAttachedToReanimated)
            | Effect::Zone(ZoneEffect::ReturnThisAuraAttachedTo { .. })
            | Effect::Zone(ZoneEffect::ScheduleReturnReanimatedToHand)
            // The specific exiled card was already resolved when the delayed trigger was
            // scheduled — no chosen target of its own.
            | Effect::Zone(ZoneEffect::ReturnFlickeredCard { .. })
            // The new host is chosen at resolution (`ChooseAttachHost`), not a cast/
            // activation target — same as `ReturnThisAuraAttachedTo` above.
            | Effect::Zone(ZoneEffect::ReturnThisAuraFromGraveyardAttachedToChosenHost)
            | Effect::Zone(ZoneEffect::ScheduleReturnThisAuraFromGraveyardAttachedToChosenHost)
            | Effect::Static(StaticEffect::NoMaximumHandSize)
            // Backup's grant rides the enclosing `Sequence`'s shared target (the counter's
            // creature), never a target of its own — see the variant doc.
            | Effect::Control(ControlEffect::GrantSourceAbilitiesUntilEndOfTurn)
            | Effect::Static(StaticEffect::PlayFromGraveyardOncePerTurn)
            | Effect::Static(StaticEffect::PreventNoncombatDamageToOtherCreaturesYouControl)
            | Effect::Static(StaticEffect::PreventDamageToSelfRemovingCounter)
            | Effect::Static(StaticEffect::PreventCombatDamage { .. })
            // Redoubled Stormsinger enumerates matching tokens internally — no chosen target.
            | Effect::Pump(PumpEffect::SetBasePtCreaturesYouControlUntilEndOfTurn { .. })
            // A self-animation always affects the ability's own source (Restless Spire), no target.
            | Effect::Pump(PumpEffect::AnimateSelfUntilEndOfTurn { .. })
            // A self-base-P/T set always affects the ability's own source (Trench Gorger), no target.
            | Effect::Pump(PumpEffect::SetOwnBasePtFromAmount { .. })
            | Effect::Token(TokenEffect::CopyEachEnteredThisTurnTokenTappedAttacking { .. })
            // Myriad enumerates opponents internally — no chosen target (see the variant doc).
            | Effect::Token(TokenEffect::MyriadTokenCopies { .. })
            // Hofri's granted leaves-battlefield rider bakes its exiled card in at synthesis —
            // no chosen target (see the variant doc).
            | Effect::Zone(ZoneEffect::ReturnExiledCardToOwnersGraveyard { .. })
            // Both ETB sacrifice-unless arms always act on their own source — no chosen target.
            | Effect::Choice(ChoiceEffect::SacrificeSelfUnlessPay { .. })
            | Effect::Choice(ChoiceEffect::SacrificeSelfUnlessReturnLand { .. })
            // Gomazoa enumerates its own blocked creatures internally — no chosen target.
            | Effect::Zone(ZoneEffect::TuckSelfAndBlockedCreatures)
            // Homeward Path enumerates every mismatched creature on the battlefield internally
            // — no chosen target (see the variant doc).
            | Effect::Control(ControlEffect::RevertAllCreaturesToOwners) => TargetSpec::None,
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
            Effect::Mana(ManaEffect::Add { .. }) => true,
            Effect::Sequence { steps } => steps.iter().any(|s| matches!(s, Effect::Mana(ManaEffect::Add { .. }))),
            _ => false,
        }
    }

    /// Whether this (mana) ability's produced credits should be recorded in
    /// [`Player::mana_provenance`](crate::state) — an [`Effect::Mana(ManaEffect::Add)`] with `track_provenance`
    /// set (recursing a `Sequence` like [`is_mana_ability`](Self::is_mana_ability)). Read by
    /// `Game::activate_ability` to decide whether to tag the batch it just resolved.
    pub(crate) fn tracks_mana_provenance(self) -> bool {
        match self {
            Effect::Mana(ManaEffect::Add {
                track_provenance, ..
            }) => track_provenance,
            Effect::Sequence { steps } => steps.iter().any(|s| s.tracks_mana_provenance()),
            _ => false,
        }
    }

    /// How many targets this effect chooses (CR 601.2c). Most targeted effects take a single
    /// mandatory target (`{1, 1}`); [`ZoneEffect::ReturnToHand`](crate::ZoneEffect::ReturnToHand) (Aether Gale's "six
    /// target"), [`DamageEffect::Target`](crate::DamageEffect::Target) (Volcanic Salvo's "up to two", Magma Opus's
    /// divided "any number"), [`ControlEffect::TapTarget`](crate::ControlEffect::TapTarget) (Magma Opus's "tap two"),
    /// [`ExileEffect::Target`](crate::ExileEffect::Target) (Curse of the Swine's "exile X target creatures"),
    /// [`DestroyEffect::Target`](crate::DestroyEffect::Target) (Pest Infestation's "up to X target artifacts
    /// and/or enchantments"), [`CountersEffect::PutCounters`](crate::CountersEffect::PutCounters) (Silkguard's "each of up to
    /// X"), and [`DigEffect::ExileTargetGraveyardSpellCastFree`](crate::DigEffect::ExileTargetGraveyardSpellCastFree)
    /// (Renegade Bull's "up to one target," `{0, 1}`) carry a real count.
    pub(crate) fn target_count(self) -> TargetCount {
        match self {
            Effect::Zone(ZoneEffect::ReturnToHand { count, .. })
            | Effect::Zone(ZoneEffect::ReturnFromGraveyardToHand { count, .. })
            | Effect::Damage(DamageEffect::Target { count, .. })
            | Effect::Control(ControlEffect::TapTarget { count, .. })
            | Effect::Control(ControlEffect::UntapTarget { count, .. })
            | Effect::Exile(ExileEffect::Target { count, .. })
            | Effect::Destroy(DestroyEffect::Target { count, .. })
            | Effect::Dig(DigEffect::ExileTargetGraveyardSpellCastFree { count, .. }) => count,
            // "return up to one target Aura or Equipment card" (CR 601.2c — a declinable target).
            Effect::Zone(ZoneEffect::ReturnFromGraveyardAttachedToToken { .. }) => TargetCount {
                min: 0,
                max: 1,
                ..TargetCount::default()
            },
            Effect::Counters(CountersEffect::PutCounters { targets, .. }) | Effect::Token(TokenEffect::CreateCopy { targets, .. }) => {
                targets
            }
            Effect::Counters(CountersEffect::DoubleCountersOnTargetCreatures { count, .. }) => count,
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
        matches!(self, Effect::Counters(CountersEffect::DoubleCountersOnTargetCreatures { .. }))
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
            Effect::Zone(ZoneEffect::ReturnFromGraveyardAttachedToToken { filter, .. }) => {
                Effect::Zone(ZoneEffect::ReturnFromGraveyardAttachedToToken {
                    filter,
                    token: Some(token),
                })
            }
            other => other,
        }
    }
}

/// Where a countered spell goes instead of its owner's graveyard (CR 701.5b), the destination
/// rider on [`Effect::Misc(MiscEffect::CounterTargetSpell)::countered_dest`] (Hinder).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum CounteredDest {
    /// "Put that card on the top or bottom of its owner's library" — the countering ability's
    /// controller picks, via a [`PendingChoice::ChooseCounteredSpellDestination`] pause.
    LibraryTopOrBottom,
    /// "Put it on the bottom of its owner's library instead of into that player's graveyard"
    /// (Spell Crumple) — forced, unlike [`LibraryTopOrBottom`](Self::LibraryTopOrBottom); no
    /// pause, straight to the bottom.
    LibraryBottom,
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

/// A named counter kind (CR 122.1) tracked on [`Permanent::kind_counters`] — distinct from the
/// +1/+1 counter path ([`Permanent::plus_counters`]), which stays untouched, even for
/// [`MinusOneMinusOne`](Self::MinusOneMinusOne) (its P/T contribution is read straight off this
/// map by [`Game::pt_layers`](crate::Game::pt_layers), not folded into `plus_counters`). Grows
/// only as real cards demand a kind (mana_bloom/astral_cornucopia's charge counters, staff_of_the_
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
    /// A scream counter (All Hallow's Eve — CR 122.1's functional-reminder family). Mechanically a
    /// time counter with a different name: it sits on a card in *exile*, ticks down at the owner's
    /// upkeep, and drives the card's expiry payload when the last is removed.
    /// ponytail: reuses the [`Game::exile_time_counters`](crate::Game) store (keyed by object id,
    /// kind-agnostic), so a scream counter and a time counter are indistinguishable there — the
    /// distinct variant only earns the card its oracle-faithful name; split the store when a card
    /// needs both kinds on one exiled object at once.
    Scream,
    /// A -1/-1 counter (CR 121.4/122.1 — Wickerbough Elder), tracked in the same kind-keyed map as
    /// every other named counter rather than as negative [`Permanent::plus_counters`] — the two
    /// stay independently addressable so a "remove a -1/-1 counter" cost can't ever accidentally
    /// pay with a +1/+1 counter. [`Game::pt_layers`] reads this slot to subtract 1/1 per counter,
    /// mirroring `plus_counters`' own P/T contribution.
    MinusOneMinusOne,
    /// A strife counter (CR 122.1 — Crescendo of War): placed on the source itself at each
    /// upkeep, read back by an [`AnthemStatic`](Effect::Static(StaticEffect::Anthem))'s
    /// `power: Amount::PerCounterOfKindOnSource` for both its attacking and blocking clauses.
    Strife,
    /// An age counter (CR 122.1, CR 702.24a — cumulative upkeep, Jotun Grunt): placed on the
    /// source itself at its controller's upkeep, one more each time, scaling
    /// [`CardDef::cumulative_upkeep`](super::CardDef::cumulative_upkeep)'s pay-or-sacrifice cost.
    Age,
    /// A storage counter (CR 122.1 — storage lands, e.g. Fungal Reaches' "{1}, {T}: Put a
    /// storage counter on this land." / "{1}, Remove X storage counters from this land: Add X
    /// mana ..."): banked mana potential, removed later (possibly many at once, unlike the
    /// pool's other remove-a-counter costs) to fund a burst of mana.
    Storage,
}

impl CounterKind {
    /// How many kinds [`Permanent::kind_counters`] has a slot for.
    /// ponytail: a fixed slot array sized to exactly what the pool's cards consume (charge, story,
    /// study, vow) rather than an open-ended map — `Permanent` must stay `Copy`, so no
    /// `Vec`/`HashMap`. Grow this (and add the matching variant) when a future card needs
    /// another named kind, or swap to a leaked `&'static [(CounterKind, u8)]`
    /// slice if the kind set ever needs to be open-ended.
    pub(crate) const COUNT: usize = 10;

    /// Every kind, for enumerating "each kind present" (proliferate, move/remove-all-counters).
    pub(crate) const ALL: [CounterKind; Self::COUNT] = [
        CounterKind::Charge,
        CounterKind::Story,
        CounterKind::Study,
        CounterKind::Vow,
        CounterKind::Time,
        CounterKind::Scream,
        CounterKind::MinusOneMinusOne,
        CounterKind::Strife,
        CounterKind::Age,
        CounterKind::Storage,
    ];
}

/// [`CardDef::cumulative_upkeep`](super::CardDef::cumulative_upkeep)'s upkeep cost (CR 702.24):
/// paid once per age counter already on the permanent (including the one just placed this
/// upkeep) or the permanent is sacrificed.
/// ponytail: the pool's one non-mana cumulative-upkeep shape (Jotun Grunt's "put two cards from
/// a single graveyard on the bottom of their owner's library") — CR 702.24's upkeep cost may be
/// any cost at all; grow a `Mana(Cost)` sibling when a future card needs a mana-paid cumulative
/// upkeep instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "card-dsl", derive(serde::Deserialize))]
pub struct CumulativeUpkeepCost {
    /// Cards from a single graveyard put on the bottom of their owner's library, per age
    /// counter (Jotun Grunt: 2).
    pub graveyard_cards: u8,
}

/// The Pacifism-family "activated abilities can't be activated" restriction an
/// [`Effect::Static(StaticEffect::GrantToAttached)`] Aura imposes on its host (CR 605's mana-ability carve-out is the
/// only axis the pool needs — Faith's Fetters exempts mana abilities, Prison Term exempts
/// nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum AbilityRestriction {
    /// Prison Term: "Enchanted creature can't attack or block, and its activated abilities
    /// can't be activated." No activated ability of the host's may be activated, mana or not.
    #[cfg_attr(feature = "card-dsl", serde(rename = "none"))]
    NoActivatedAbilities,
    /// Faith's Fetters: "… its activated abilities can't be activated unless they're mana
    /// abilities." (CR 605.) A mana ability of the host's still activates; nothing else does.
    #[cfg_attr(feature = "card-dsl", serde(rename = "mana_only"))]
    ManaAbilitiesOnly,
}

/// An *activated* ability an Aura grants its enchanted host (Fallen Ideal's "Sacrifice a
/// creature: This creature gets +2/+1 until end of turn."), carried by
/// [`Effect::Static(StaticEffect::GrantToAttached)`]. The non-mana twin of [`Effect::Static(StaticEffect::GrantManaAbility)`]'s inline
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

/// The indefinite characteristics set an [`Effect::Zone(ZoneEffect::ReanimateToBattlefield)`] with a `becomes`
/// rider applies to the permanent it reanimates (CR 611.2c — Excava, the Risen Past's "It's a 1/1
/// Spirit creature with flying in addition to its other types"): `add_types`/`add_subtypes` are
/// unioned onto the reanimated object (CR 613.4), base P/T is SET to `base_power`/`base_toughness`
/// (CR 613.3(7b)), and `keywords` are added — all for as long as it stays on the battlefield.
/// Written onto the permanent's indefinite fields by [`Event::ReanimatedCreatureBecame`], the
/// as-long-as-on-battlefield twin of [`AnimateSelfUntilEndOfTurn`](Effect::Pump(PumpEffect::AnimateSelfUntilEndOfTurn)).
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
/// [`Effect::Static(StaticEffect::GrantManaAbility)`]'s `cost` field, spelled as a `[…cost]` table with these same
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
    /// Whether the counter-removal cost's count is a player-declared `{X}` (CR 601.2b) instead
    /// of the fixed [`remove_counters`](Self::remove_counters) (Fungal Reaches' "Remove X storage
    /// counters from this land"). `remove_counters_kind` must be `Some` alongside this — an X
    /// removal always names a kind. Gated at activation against the source's actual count of
    /// that kind (CR 602.2b — an uncompletable cost makes activation illegal); `X = 0` is always
    /// legal (CR 107.3c). `false` (every other counter-removal cost) leaves the fixed count above
    /// in force.
    pub remove_counters_x: bool,
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
    /// "Discard a card" as part of the cost (CR 602.2b/118 — Wild Mongrel's "Discard a card:
    /// Wild Mongrel gets +1/+1..."): the activating [`Intent::ActivateAbility`]'s own
    /// `discard_cost` names this many distinct hand cards to discard. `0` for none. Paid on
    /// activation via the normal discard choke (so "whenever you discard a card" watchers fire);
    /// an activator whose hand holds fewer than this many cards — or who names cards not in
    /// hand, or names the same card twice — can't pay it (CR 602.2b — an uncompletable/illegal
    /// cost makes activation illegal).
    pub discard_cost: u8,
    /// "Exile this artifact"/"exile this permanent" as part of the cost (CR 118 — Perpetual
    /// Timepiece's "{2}, Exile this artifact: …"). Paid on activation via the same
    /// [`Event::MovedToExile`] path a targeted exile effect uses; a token ceases to exist
    /// instead (CR 111.7) — the same fork [`return_self`](Self::return_self) takes. Like
    /// `return_self`, this never gates activation on its own (the source is always payable) and
    /// self-limits future activations by taking the permanent off the battlefield.
    pub exile_self: bool,
    /// "Exile N target cards from an opponent's graveyard" as an additional cost (CR 601.2c/
    /// 602.2b — Spurnmage Advocate's "Exile two target cards from an opponent's graveyard:
    /// …"). Unlike `sacrifice`/`discard_cost`'s untargeted choices, CR 601.2c treats these as
    /// real targets: the activator names them in [`Intent::ActivateAbility`]'s `target_second`
    /// (the ability's independent *second* target clause, alongside its own stack `target`),
    /// validated distinct/legal/all-from-one-opponent's-graveyard and exiled immediately in
    /// [`Game::activate_ability`] — before the ability (with its own single `target`) goes on
    /// the stack. `0` (the default) for every other pool cost.
    pub graveyard_exile_target_count: u8,
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
    /// "if an opponent controls `count` or more lands whose type line carries any of
    /// `subtypes`" (Massacre's free-cast permission: "if an opponent controls a Plains" —
    /// `subtypes = ["Plains"], count = 1`). The opponent-scoped twin of
    /// [`ControlsLandsWithSubtype`](Self::ControlsLandsWithSubtype) — holds when *some* living
    /// opponent of the controller meets the threshold individually (not summed across
    /// opponents, unlike [`OpponentsControlLands`](Self::OpponentsControlLands)).
    OpponentControlsLandsWithSubtype {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_str_slice"))]
        subtypes: &'static [&'static str],
        count: u32,
    },
    /// "if you control `count` or more basic lands" (Eclipsed Steppe and its tango-land cousins).
    ControlsBasicLands { count: u32 },
    /// "if your opponents control `count` or more lands, combined" (the turbulent_* slowland
    /// cycle).
    OpponentsControlLands { count: u32 },
    /// "if an opponent controls `at_least` or more lands" (Avatar of Fury's cost-reduction
    /// condition: "If an opponent controls seven or more lands, this spell costs {6} less to
    /// cast."). The opponent-scoped, land-count twin of
    /// [`OpponentControlsLandsWithSubtype`](Self::OpponentControlsLandsWithSubtype) — holds when
    /// *some* living opponent individually meets the threshold (not summed across opponents,
    /// unlike [`OpponentsControlLands`](Self::OpponentsControlLands)).
    AnOpponentControlsLands { at_least: u32 },
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
    YouControlLands { at_least: u32 },
    /// "if you control `at_most` or fewer lands" (Edge of Autumn's "If you control four or fewer
    /// lands, search your library for a basic land card…") — the `at_most` twin of
    /// [`YouControlLands`](Self::YouControlLands), only reachable inside `{ type = "conditional",
    /// … }` (a spell's own resolve-time check, CR 608.2h) since no pool card needs it as an
    /// intervening-if or activation restriction yet.
    YouControlLandsAtMost { at_most: u32 },
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
    /// "if [this permanent] is untapped" (Howling Mine's intervening-if) — whether the ability's own
    /// source permanent is untapped *right now*, re-read live rather than snapshotted once like
    /// [`ThisPermanentEnteredUntapped`](Self::ThisPermanentEnteredUntapped) (which asks how the
    /// permanent entered, not its current state). Source-object-based, same shape as
    /// [`SourceHasCounters`](Self::SourceHasCounters): usable both as an `[abilities.condition]`
    /// intervening-if (checked at trigger placement, [`Game::ability_condition_holds`]) *and*
    /// nested in an [`Effect::Conditional`] (checked fresh at resolution) — CR 603.4 requires
    /// *both* checks, and the pool falsifies skipping the second (Magma Opus's instant-speed "tap
    /// two target permanents" can tap the source in response, after it triggered untapped).
    SourceUntapped,
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
    /// "if there are `count` or more cards in your graveyard" (Werebear's Threshold ability
    /// word: "gets +3/+3 as long as there are seven or more cards in your graveyard") — every
    /// card in the controller's graveyard, any kind, re-checked live like the other
    /// graveyard-count siblings above.
    CardsInYourGraveyardAtLeast { count: u32 },
    /// "if there are `count` or more creature cards total in all graveyards" (Avatar of Woe's
    /// self cost reduction). Sums [`Game::graveyard_cards`] over *every* living player, not just
    /// the controller — the all-graveyards sibling of
    /// [`ArtifactOrCreatureCardsInYourGraveyardAtLeast`](Self::ArtifactOrCreatureCardsInYourGraveyardAtLeast).
    CreatureCardsInAllGraveyardsAtLeast { count: u32 },
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
    /// single slot; Massacre's free-cast permission — "if an opponent controls a Plains *and*
    /// you control a Swamp" — reuses the same combinator on [`CardDef::free_cast_if`]). Holds
    /// iff every element holds. `&'static` keeps `Condition` `Copy`, leaked at deserialize like
    /// any other `&'static [T]` card field.
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
    /// "If you win the clash" (Lash Out — CR 701.22d): reads the resolution-scoped
    /// [`Game::clash_won`](crate::Game) flag a preceding [`Effect::Dig(DigEffect::Clash)`](crate::Effect::Dig(DigEffect::Clash))
    /// step in the same resolution just set. Not a persistent board fact — it lives only for the
    /// rest of the current resolution, and is only ever read by a `conditional` step that follows
    /// a clash in the same ability, so a stale value from an earlier resolution can never be read.
    WonClash,
    /// "Then if there are no cards in that player's graveyard" (Nezumi Graverobber's flip gate —
    /// CR 608.2, evaluated as the ability resolves, after the same ability's exile step already
    /// emptied — or didn't — the targeted card's owner's graveyard). Target-based like
    /// [`TargetPowerAtLeast`](Self::TargetPowerAtLeast): `TriggerContext` carries no target, so the
    /// [`Effect::Conditional`] resolve site special-cases it directly against the shared `target`,
    /// reading the owner of that graveyard card (its owner survives the exile — the moved object
    /// still records it) and checking that owner's graveyard is now empty. Unreachable through the
    /// ordinary [`condition_holds`](Game::condition_holds) path (returns `false` there).
    TargetCardOwnerGraveyardEmpty,
    /// "if `color` was spent to cast this" (CR 106.9 — Court Hussar's "unless {W} was spent to
    /// cast it"). Source-object-based like [`SourceEnteredWithXAtLeast`](Self::SourceEnteredWithXAtLeast):
    /// `TriggerContext` carries no source id, so the [`Effect::Conditional`] resolve site
    /// special-cases it directly against its own `source` parameter, reading
    /// [`Permanent::spent_colors`].
    ColorWasSpentToCastThis { color: Color },
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
    // CR 609.7: an `EnchantedCreatureDealsDamage` trigger's `Amount::TriggeringDamageDealt` reads
    // resolve against the damage the enchanted host just dealt, locked in when the trigger goes on
    // the stack — same last-known-information shape as `dying_source_stats` above.
    let effect = match ctx.triggering_damage_dealt {
        Some(damage) => fill_triggering_damage_dealt(effect, damage),
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
    // CR 510.2/603.10a: a `DealsCombatDamageToPlayer` trigger's "its controller may draw" payoff
    // (Edric) belongs to whoever controlled the damaging creature, locked in when the trigger goes
    // on the stack — same last-known-information shape as `combat_damage` above.
    let effect = match ctx.combat_damage_source_controller {
        Some(player) => fill_combat_damage_source_controller(effect, player),
        None => effect,
    };
    // CR 510.2/603.10a: a `DealsCombatDamageToPlayer` trigger's "each other opponent" splash (Hydra
    // Omnivore) excludes whoever took the combat damage, locked in when the trigger goes on the
    // stack — same last-known-information shape as `combat_damage` above.
    let effect = match ctx.combat_damage_recipient {
        Some(player) => fill_combat_damage_recipient(effect, player),
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
            Effect::Mill(MillEffect::ExileFromGraveyardMayPlay { .. }) => {
                return Effect::Mill(MillEffect::ExileFromGraveyardMayPlay {
                    card: Some(discarded),
                });
            }
            Effect::Mill(MillEffect::ExileDiscardedWithThis { .. }) => {
                return Effect::Mill(MillEffect::ExileDiscardedWithThis {
                    card: Some(discarded),
                });
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
    // A `DealsCombatDamageToCreature` trigger's destroy payoff needs the damaged creature's id,
    // not the attack tuple below — guarded separately for the same reason as
    // `dying_enchanted_creature` above.
    let effect = match ctx.damaged_creature {
        Some(damaged) => fill_damaged_creature(effect, damaged),
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
    // A `NonlandPermanentYouControlDiesIncludingThis` watch's dynamic edict payoff (Martyr's
    // Bond's "shares a card type with it") needs the dying permanent's own last-known card types
    // baked into its `EachPlayerSacrifices` filter — guarded separately for the same reason as
    // the look-backs above.
    let effect = match ctx.dying_permanent_types {
        Some(types) => fill_dying_permanent_types(effect, types),
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
    // CR 702.40a Storm's copy count, locked in when a `Trigger::YouCastThis` ability goes on the
    // stack — same last-known-information shape as `triggering_spell` above.
    let effect = match ctx.spells_cast_before_this {
        Some(n) => fill_spells_cast_before_this(effect, n),
        None => effect,
    };
    // An `ActivateAbility` watch's copy payoff (Unbound Flourishing) needs the source of the
    // activated ability it copies, baked in when the watch fires — same last-known-information
    // shape as `triggering_spell` above, one step over.
    let effect = match ctx.triggering_ability {
        Some(source) => fill_triggering_ability(effect, source),
        None => effect,
    };
    // Rhystic Study's "unless that player pays" payoff needs the triggering opponent's identity,
    // baked in when the `CastSpell` watch fires — same last-known-information shape as
    // `triggering_spell` above.
    let effect = match ctx.triggering_caster {
        Some(caster) => fill_triggering_caster(effect, caster),
        None => effect,
    };
    // Howling Mine: `EachDrawStepPlayerDraws`'s drawer is the active player whose draw step this
    // is (context), not this ability's controller — CR "that player draws an additional card".
    // Magus of the Vineyard: `AddMana`'s recipient is that same active player, one step over —
    // CR "at the beginning of each player's first main phase … that player adds {G}{G}".
    let effect = match ctx.active_player {
        Some(active_player) => fill_add_mana_recipient(
            fill_each_draw_step_drawer(effect, active_player),
            active_player,
        ),
        None => effect,
    };
    match (effect, ctx.attack) {
        (Effect::Counters(CountersEffect::AttackerDrawsControllerCounters { counters, .. }), Some((attacker, _attacked))) => {
            Effect::Counters(CountersEffect::AttackerDrawsControllerCounters {
                attacker: Some(attacker),
                counters,
            })
        }
        (Effect::Life(LifeEffect::AttackerLosesYouGain { amount, .. }), Some((attacker, _attacked))) => {
            Effect::Life(LifeEffect::AttackerLosesYouGain {
                attacker: Some(attacker),
                amount,
            })
        }
        (Effect::Life(LifeEffect::AttackerLosesYouDraw { life_loss, .. }), Some((attacker, _attacked))) => {
            Effect::Life(LifeEffect::AttackerLosesYouDraw {
                attacker: Some(attacker),
                life_loss,
            })
        }
        (Effect::Draw(DrawEffect::AttackingPlayer { count, .. }), Some((attacker, _attacked))) => {
            Effect::Draw(DrawEffect::AttackingPlayer {
                drawer: Some(attacker),
                count,
            })
        }
        // Goblin Guide: the *defending* player (the attack's second element) reveals, not the
        // attacker.
        (Effect::Reveal(RevealEffect::TopToHand { filter, .. }), Some((_attacker, defender))) => {
            Effect::Reveal(RevealEffect::TopToHand {
                filter,
                defender: Some(defender),
            })
        }
        // Annihilator: the *defending* player sacrifices, not the attacker.
        (Effect::Choice(ChoiceEffect::DefendingPlayerSacrifices { count, .. }), Some((_attacker, defender))) => {
            Effect::Choice(ChoiceEffect::DefendingPlayerSacrifices {
                count,
                defender: Some(defender),
            })
        }
        // Combat Calligrapher: "that attacking player creates a tapped … token … that's
        // attacking that opponent" — bake the (attacker, attacked) pair so the token mints
        // under the attacker and enters tapped and attacking it, per CR 508.4.
        (
            Effect::Token(TokenEffect::Create {
                token,
                count,
                controller,
                enters_with,
                set_base_pt,
                exile_at_next_end_step,
                enters_tapped_and_attacking: true,
                must_attack_defender,
                ..
            }),
            Some(attack),
        ) => Effect::Token(TokenEffect::Create {
            token,
            count,
            controller,
            enters_with,
            set_base_pt,
            exile_at_next_end_step,
            enters_tapped_and_attacking: true,
            attacking_context: Some(attack),
            must_attack_defender,
        }),
        // Redoubled Stormsinger: "Whenever this creature attacks..." — bake the same
        // (attacker, defender) pair so the minted copies enter tapped and attacking the
        // defender Redoubled itself is attacking.
        (Effect::Token(TokenEffect::CopyEachEnteredThisTurnTokenTappedAttacking { .. }), Some(attack)) => {
            Effect::Token(TokenEffect::CopyEachEnteredThisTurnTokenTappedAttacking {
                attacking_context: Some(attack),
            })
        }
        _ => effect,
    }
}

/// Rewrite a `PermanentEnters`/`PermanentEntersIncludingThis` trigger's entering-permanent
/// placeholders (Marauding Raptor's damage target, Ajani's Chosen's attach target, Shielded by
/// Faith's re-attach target, Riku of Two Reflections' token-copy target) to the entering
/// permanent's id. Recurses into a [`Effect::Sequence`]
/// so a multi-step ability (create-then-attach) shares the one id across every step, mirroring
/// [`fill_dying_source_amounts`] below; every other effect passes through unchanged.
fn fill_entering_permanent(effect: Effect, entering: ObjectId) -> Effect {
    match effect {
        Effect::Damage(DamageEffect::ToEnteringPermanent {
            amount,
            then_if_subtype,
            then,
            ..
        }) => Effect::Damage(DamageEffect::ToEnteringPermanent {
            entering: Some(entering),
            amount,
            then_if_subtype,
            then,
        }),
        Effect::Zone(ZoneEffect::AttachTriggeringAuraToMintedToken { .. }) => {
            Effect::Zone(ZoneEffect::AttachTriggeringAuraToMintedToken {
                entering: Some(entering),
            })
        }
        Effect::Control(ControlEffect::AttachSelfToEntering { .. }) => Effect::Control(ControlEffect::AttachSelfToEntering {
            entering: Some(entering),
        }),
        Effect::Token(TokenEffect::CreateCopy {
            target,
            count,
            targets,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            haste,
            ..
        }) => Effect::Token(TokenEffect::CreateCopy {
            target,
            count,
            targets,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            haste,
            entering: Some(entering),
        }),
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
        Effect::Zone(ZoneEffect::ReanimateDyingEnchantedCreature { under_owner, .. }) => {
            Effect::Zone(ZoneEffect::ReanimateDyingEnchantedCreature {
                dying: Some(dying),
                under_owner,
            })
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

/// Rewrite a [`Trigger::DealsCombatDamageToCreature`] look-back destroy placeholder (Stinkweed
/// Imp) to the damaged creature's id — mirrors [`fill_dying_enchanted_creature`] above, one
/// effect variant only (flag-don't-force: no other pool card reads this context field yet).
fn fill_damaged_creature(effect: Effect, damaged: ObjectId) -> Effect {
    match effect {
        Effect::Destroy(DestroyEffect::TriggeringDamagedCreature { .. }) => {
            Effect::Destroy(DestroyEffect::TriggeringDamagedCreature {
                creature: Some(damaged),
            })
        }
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_damaged_creature(step, damaged))
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
        Effect::Sacrifice(SacrificeEffect::EnchantedCreature { .. }) => Effect::Sacrifice(SacrificeEffect::EnchantedCreature {
            creature: Some(host),
        }),
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
        Effect::Zone(ZoneEffect::ExileDeadCreatureCreateCopyWithSubtype {
            add_subtypes,
            leaves_returns_exiled,
            ..
        }) => Effect::Zone(ZoneEffect::ExileDeadCreatureCreateCopyWithSubtype {
            dead: Some(dead),
            add_subtypes,
            leaves_returns_exiled,
        }),
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

/// Rewrite an `EachPlayerSacrifices` edict's `shares_type_with_dying_permanent`-marked filter to
/// the dying permanent's own last-known card types (CR 603.10a — Martyr's Bond's "shares a card
/// type with it"), replacing its authored (empty) `types` with the resolved set. Mirrors
/// [`fill_dead_creature`] above, one effect variant only (flag-don't-force: no other pool card
/// reads this context field yet).
fn fill_dying_permanent_types(effect: Effect, types: TypeSet) -> Effect {
    match effect {
        Effect::Choice(ChoiceEffect::EachPlayerSacrifices {
            filter,
            scope,
            keep_one,
            life_loss,
            then,
        }) if filter.shares_type_with_dying_permanent => Effect::Choice(ChoiceEffect::EachPlayerSacrifices {
            filter: PermanentFilter { types, ..filter },
            scope,
            keep_one,
            life_loss,
            then,
        }),
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_dying_permanent_types(step, types))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}

/// Rewrite a [`TriggerContext::cards_left_graveyard`]-reading effect placeholder to the batch's
/// card ids: [`Effect::Choice(ChoiceEffect::PutCounterThenMayBecomeCopyOfCardFromList)`] (Spirit of Resilience, off
/// [`Trigger::CardsLeaveYourGraveyard`]) — mirrors [`fill_dead_creature`] above. `cards` is the
/// already-leaked `&'static` slice off the trigger context, so no re-leak here.
fn fill_cards_left_graveyard(effect: Effect, cards: &'static [ObjectId]) -> Effect {
    match effect {
        Effect::Choice(ChoiceEffect::PutCounterThenMayBecomeCopyOfCardFromList { .. }) => {
            Effect::Choice(ChoiceEffect::PutCounterThenMayBecomeCopyOfCardFromList { cards })
        }
        other => other,
    }
}

/// Rewrite a [`TriggerContext::triggering_ability`]-reading effect placeholder to the activated
/// ability's source that fired the watch: [`Effect::Copy(CopyEffect::CopyTriggeringAbility)`] (Unbound Flourishing,
/// off [`Trigger::ActivateAbility`]) — the ability half's twin of [`fill_triggering_spell`].
fn fill_triggering_ability(effect: Effect, source: ObjectId) -> Effect {
    match effect {
        Effect::Copy(CopyEffect::CopyTriggeringAbility {
            may_choose_new_targets,
            ..
        }) => Effect::Copy(CopyEffect::CopyTriggeringAbility {
            triggering_ability: Some(source),
            may_choose_new_targets,
        }),
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
/// fired the watch: [`Effect::Copy(CopyEffect::CopyTriggeringSpell)`] (Thunderclap Drake, off [`Trigger::CastSpell`]),
/// [`Effect::Counters(CountersEffect::CommanderEntersWithBonusCounters)`] (Opal Palace, off [`Trigger::SpendManaToCast`]), and
/// [`Effect::Copy(CopyEffect::CopyTriggeringSpellForEachOtherCreatureYouControl)`] (Mirrorwing Dragon, off
/// [`Trigger::SpellTargetsThisOnly`]) — mirrors [`fill_dying_enchanted_creature`] above.
fn fill_triggering_spell(effect: Effect, spell: ObjectId) -> Effect {
    match effect {
        Effect::Copy(CopyEffect::CopyTriggeringSpell {
            count,
            may_choose_new_targets,
            last_known_information,
            ..
        }) => Effect::Copy(CopyEffect::CopyTriggeringSpell {
            triggering_spell: Some(spell),
            count,
            may_choose_new_targets,
            last_known_information,
        }),
        Effect::Counters(CountersEffect::CommanderEntersWithBonusCounters { count, .. }) => {
            Effect::Counters(CountersEffect::CommanderEntersWithBonusCounters {
                triggering_spell: Some(spell),
                count,
            })
        }
        Effect::Copy(CopyEffect::CopyTriggeringSpellForEachOtherCreatureYouControl { .. }) => {
            Effect::Copy(CopyEffect::CopyTriggeringSpellForEachOtherCreatureYouControl {
                triggering_spell: Some(spell),
            })
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

/// Rewrite a [`TriggerContext::spells_cast_before_this`]-reading effect placeholder to CR
/// 702.40a's storm count: [`Effect::Copy(CopyEffect::CopyTriggeringSpell)`]'s `count` field, when it holds
/// [`Amount::SpellsCastBeforeThisThisTurn`] (Reaping the Graves' Storm, off
/// [`Trigger::YouCastThis`]) — mirrors [`fill_triggering_spell`] above, one field over.
fn fill_spells_cast_before_this(effect: Effect, n: u32) -> Effect {
    match effect {
        Effect::Copy(CopyEffect::CopyTriggeringSpell {
            triggering_spell,
            count: Amount::SpellsCastBeforeThisThisTurn,
            may_choose_new_targets,
            last_known_information,
        }) => Effect::Copy(CopyEffect::CopyTriggeringSpell {
            triggering_spell,
            count: Amount::Fixed(n as i32),
            may_choose_new_targets,
            last_known_information,
        }),
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_spells_cast_before_this(step, n))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}

/// Rewrite a [`TriggerContext::triggering_caster`]-reading effect placeholder to the player who
/// cast the spell that fired the watch: [`Effect::Choice(ChoiceEffect::MayDrawUnlessPays)`] (Rhystic Study's "unless
/// that player pays {1}") — mirrors [`fill_triggering_spell`] above, one field over.
fn fill_triggering_caster(effect: Effect, caster: PlayerId) -> Effect {
    match effect {
        Effect::Choice(ChoiceEffect::MayDrawUnlessPays { cost, .. }) => Effect::Choice(ChoiceEffect::MayDrawUnlessPays {
            cost,
            caster: Some(caster),
        }),
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_triggering_caster(step, caster))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}

/// Rewrite a [`TriggerContext::combat_damage_source_controller`]-reading effect placeholder to the
/// player who controlled the creature that dealt the combat damage:
/// [`Effect::Choice(ChoiceEffect::DamagingCreatureControllerMayDraw)`] (Edric's "its controller may draw a card") —
/// mirrors [`fill_triggering_caster`] above, one field over.
fn fill_combat_damage_source_controller(effect: Effect, player: PlayerId) -> Effect {
    match effect {
        Effect::Choice(ChoiceEffect::DamagingCreatureControllerMayDraw { count, .. }) => {
            Effect::Choice(ChoiceEffect::DamagingCreatureControllerMayDraw {
                drawer: Some(player),
                count,
            })
        }
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_combat_damage_source_controller(step, player))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}

/// Rewrite a [`TriggerContext::combat_damage_recipient`]-reading effect placeholder to the player
/// who took the combat damage: [`Effect::Damage(DamageEffect::EachOtherOpponent)`] (Hydra Omnivore's "each *other*
/// opponent") — mirrors [`fill_combat_damage_source_controller`] above, one field over.
fn fill_combat_damage_recipient(effect: Effect, player: PlayerId) -> Effect {
    match effect {
        Effect::Damage(DamageEffect::EachOtherOpponent { amount, .. }) => Effect::Damage(DamageEffect::EachOtherOpponent {
            amount,
            damaged: Some(player),
        }),
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_combat_damage_recipient(step, player))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}

/// Rewrite a [`TriggerContext::active_player`]-reading effect placeholder to the player whose
/// first main phase it is: [`Effect::Mana(ManaEffect::Add)`]'s `recipient` (Magus of the Vineyard's "that
/// player adds {G}{G}") — mirrors [`fill_each_draw_step_drawer`] below, one field over. Only
/// `Sequence` recurses (no pool `add_mana` sits inside a `Conditional`).
fn fill_add_mana_recipient(effect: Effect, active_player: PlayerId) -> Effect {
    match effect {
        Effect::Mana(ManaEffect::Add {
            mana,
            identity,
            opponent_colors,
            repeat,
            restriction,
            single_color,
            track_provenance,
            target,
            persist_until_end_of_turn,
            ..
        }) => Effect::Mana(ManaEffect::Add {
            mana,
            identity,
            opponent_colors,
            repeat,
            restriction,
            single_color,
            track_provenance,
            target,
            persist_until_end_of_turn,
            recipient: Some(active_player),
        }),
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_add_mana_recipient(step, active_player))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}

/// Rewrite a [`TriggerContext::active_player`]-reading effect placeholder to the player whose
/// draw step it is: [`Effect::Draw(DrawEffect::EachDrawStepPlayer)`] (Howling Mine's "that player draws an
/// additional card") — mirrors [`fill_triggering_caster`] above. Recurses into
/// [`Effect::Conditional`]'s `then` (not just `Sequence`, unlike its siblings) so Howling Mine's
/// CR 603.4 resolution-time re-check wrapper still gets its nested draw filled.
fn fill_each_draw_step_drawer(effect: Effect, active_player: PlayerId) -> Effect {
    match effect {
        Effect::Draw(DrawEffect::EachDrawStepPlayer { count, .. }) => Effect::Draw(DrawEffect::EachDrawStepPlayer {
            drawer: Some(active_player),
            count,
        }),
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_each_draw_step_drawer(step, active_player))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        Effect::Conditional {
            condition,
            then,
            negate,
        } => {
            let filled: Vec<Effect> = then
                .iter()
                .map(|&step| fill_each_draw_step_drawer(step, active_player))
                .collect();
            Effect::Conditional {
                condition,
                then: Box::leak(filled.into_boxed_slice()),
                negate,
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
        Effect::Life(LifeEffect::Gain { amount }) => Effect::Life(LifeEffect::Gain { amount: f(amount) }),
        Effect::Draw(DrawEffect::Cards { count }) => Effect::Draw(DrawEffect::Cards { count: f(count) }),
        Effect::Counters(CountersEffect::PutCounters {
            count,
            target,
            targets,
            kind,
            divided,
        }) => Effect::Counters(CountersEffect::PutCounters {
            count: f(count),
            target,
            targets,
            kind,
            divided,
        }),
        Effect::Token(TokenEffect::Create {
            token,
            count,
            controller,
            enters_with,
            set_base_pt,
            exile_at_next_end_step,
            enters_tapped_and_attacking,
            attacking_context,
            must_attack_defender,
        }) => Effect::Token(TokenEffect::Create {
            token,
            count: f(count),
            controller,
            enters_with: f(enters_with),
            set_base_pt: set_base_pt.map(f),
            exile_at_next_end_step,
            enters_tapped_and_attacking,
            attacking_context,
            must_attack_defender,
        }),
        Effect::Token(TokenEffect::CreateTreasure {
            count,
            target_player,
            tapped,
        }) => Effect::Token(TokenEffect::CreateTreasure {
            count: f(count),
            target_player,
            tapped,
        }),
        Effect::Damage(DamageEffect::EachOtherOpponent { amount, damaged }) => Effect::Damage(DamageEffect::EachOtherOpponent {
            amount: f(amount),
            damaged,
        }),
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

/// Rewrite an `EnchantedCreatureDealsDamage` watch's `Amount::TriggeringDamageDealt` placeholder to
/// the enchanted host's just-dealt damage (CR 609.7, Armadillo Cloak's "you gain that much life")
/// — mirrors [`fill_auras_attached_to_dying_creature`] above, one `Amount` variant only
/// (flag-don't-force: no other pool card reads this amount).
fn fill_triggering_damage_dealt(effect: Effect, damage: i32) -> Effect {
    map_effect_amounts(effect, &|amount| match amount {
        Amount::TriggeringDamageDealt => Amount::Fixed(damage),
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
        Effect::Pump(PumpEffect::PumpUntilEndOfTurn {
            power,
            toughness,
            target,
            keywords,
        }) => Effect::Pump(PumpEffect::PumpUntilEndOfTurn {
            power: fill(power),
            toughness: fill(toughness),
            target,
            keywords,
        }),
        Effect::Token(TokenEffect::Create {
            token,
            count,
            controller,
            enters_with,
            set_base_pt,
            exile_at_next_end_step,
            enters_tapped_and_attacking,
            attacking_context,
            must_attack_defender,
        }) => Effect::Token(TokenEffect::Create {
            token,
            count: fill(count),
            controller,
            enters_with: fill(enters_with),
            set_base_pt: set_base_pt.map(fill),
            exile_at_next_end_step,
            enters_tapped_and_attacking,
            attacking_context,
            must_attack_defender,
        }),
        Effect::Conditional {
            condition: Condition::TriggeringSpellManaValueAtLeast { at_least },
            then,
            negate,
        } => {
            if (mv < u32::from(at_least)) != negate {
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
/// goes on the stack, same shape as [`fill_dying_source_amounts`] above. Every other effect
/// (including `Amount::CombatDamageDealt` itself — Primo, the Unbounded's minted Fractal's
/// `enters_with` counter count, and Rapacious One's `count` of minted Eldrazi Spawn) is handled
/// by the generic [`map_effect_amounts`] walker, which also covers the [`Effect::Sequence`]
/// recursion so a multi-step ability shares the one damage amount across every step.
fn fill_combat_damage(effect: Effect, damage: i32) -> Effect {
    match effect {
        Effect::Zone(ZoneEffect::ReanimateToBattlefield {
            target:
                TargetSpec::CardInGraveyard {
                    whose,
                    filter: CardFilter::CreatureWithManaValueAtMostCombatDamage,
                    other,
                },
            finality,
            becomes,
        }) => Effect::Zone(ZoneEffect::ReanimateToBattlefield {
            target: TargetSpec::CardInGraveyard {
                whose,
                filter: CardFilter::CreatureWithManaValueAtMost(damage.max(0) as u8),
                other,
            },
            finality,
            becomes,
        }),
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| fill_combat_damage(step, damage))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => map_effect_amounts(other, &|amount| match amount {
            Amount::CombatDamageDealt => Amount::Fixed(damage.max(0)),
            other => other,
        }),
    }
}

/// Fill an `Attacks` trigger's [`CardFilter::NonlandPermanentWithManaValueAtMostSourcePower`]
/// placeholder with the attacker's power, read at trigger placement (CR 510.2/603.10a last-known
/// information — Guardian Scalelord's "where X is this creature's power"). A negative power floors
/// at 0. Every other effect passes through unchanged; recurses into a [`Effect::Sequence`] like
/// [`fill_combat_damage`].
fn fill_source_power(effect: Effect, power: i32) -> Effect {
    match effect {
        Effect::Zone(ZoneEffect::ReanimateToBattlefield {
            target:
                TargetSpec::CardInGraveyard {
                    whose,
                    filter: CardFilter::NonlandPermanentWithManaValueAtMostSourcePower,
                    other,
                },
            finality,
            becomes,
        }) => Effect::Zone(ZoneEffect::ReanimateToBattlefield {
            target: TargetSpec::CardInGraveyard {
                whose,
                filter: CardFilter::NonlandPermanentWithManaValueAtMost(power.max(0) as u8),
                other,
            },
            finality,
            becomes,
        }),
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
pub(crate) fn contextualize_sacrifice_effect(effect: Effect, power: i32, toughness: i32) -> Effect {
    let fill = |amount: Amount| match amount {
        Amount::SacrificedCreaturePower => Amount::Fixed(power),
        Amount::SacrificedCreatureToughness => Amount::Fixed(toughness),
        other => other,
    };
    match effect {
        Effect::Pump(PumpEffect::PumpSelfUntilEndOfTurn {
            power: p,
            toughness,
            keywords,
        }) => Effect::Pump(PumpEffect::PumpSelfUntilEndOfTurn {
            power: fill(p),
            toughness: fill(toughness),
            keywords,
        }),
        Effect::Life(LifeEffect::Gain { amount }) => Effect::Life(LifeEffect::Gain {
            amount: fill(amount),
        }),
        // Brion Stoutarm: "deals damage equal to the sacrificed creature's power to target
        // player or planeswalker" — the sac-power/toughness fill applies to damage amounts too.
        Effect::Damage(DamageEffect::Target {
            amount,
            target,
            count,
            divided,
        }) => Effect::Damage(DamageEffect::Target {
            amount: fill(amount),
            target,
            count,
            divided,
        }),
        Effect::Counters(CountersEffect::PutCounters {
            count,
            target,
            targets,
            kind,
            divided,
        }) => Effect::Counters(CountersEffect::PutCounters {
            count: fill(count),
            target,
            targets,
            kind,
            divided,
        }),
        Effect::Sequence { steps } => {
            let filled: Vec<Effect> = steps
                .iter()
                .map(|&step| contextualize_sacrifice_effect(step, power, toughness))
                .collect();
            Effect::Sequence {
                steps: Box::leak(filled.into_boxed_slice()),
            }
        }
        other => other,
    }
}
