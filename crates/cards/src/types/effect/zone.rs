use super::*;
#[cfg(feature = "card-dsl")]
use crate::de;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum ZoneEffect {
    AttachMintedAuraToTarget {
        target: TargetSpec,
    },

    AttachSelfToMintedToken,

    AttachSelfToReanimated,

    AttachTriggeringAuraToMintedToken {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        entering: Option<ObjectId>,
    },

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

    ExileGraveyardObjectGainLife {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        object: Option<ObjectId>,
        amount: i32,
    },

    ExileSelfOnResolve,

    ExileSelfWithTimeCounters {
        counters: u32,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        on_expiry: &'static [Effect],
    },

    ExileTargetGraveyardCardThenIfCreature {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },

    FlickerTarget {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        return_at: Option<Step>,
    },

    Manifest,

    MassReturnFromGraveyard {
        filter: CardFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        all_players: bool,
    },

    ReanimateDyingEnchantedCreature {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        dying: Option<ObjectId>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        under_owner: bool,
    },

    /// Puppet Master: "When enchanted creature dies, return **that card** to its owner's hand."
    /// The hand-side twin of [`ReanimateDyingEnchantedCreature`](Self::ReanimateDyingEnchantedCreature)
    /// — same CR 603.10a snapshot of the dying host, filled at trigger placement by
    /// `fill_dying_enchanted_creature`, since by resolution the host is a graveyard card. A
    /// no-op if it has left the graveyard by then (CR 400.7).
    ReturnDyingEnchantedCreatureToHand {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        dying: Option<ObjectId>,
    },

    /// Tariel, Reckoner of Souls: "Choose a creature card at random from target opponent's
    /// graveyard. Put that card onto the battlefield under your control." `target` is the
    /// targeted opponent (CR-real target, `TargetSpec::OpponentPlayer`); the creature card
    /// itself is picked by the injected RNG at resolution — needs `&mut self`, so this resolves
    /// via `Game::run_misc_choreo` like `ExileRandomFromGraveyardMayPlay`'s "at random" pick,
    /// not the pure `mint_zones` path `ReanimateToBattlefield` (a chosen, not random, card) uses.
    ReanimateRandomFromTargetOpponentGraveyard {
        target: TargetSpec,
    },

    ReanimateToBattlefield {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        finality: bool,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::opt_static_reanimate_becomes")
        )]
        becomes: Option<&'static ReanimateBecomes>,
    },

    ReflexiveTrigger {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },

    /// "When one or more nonland cards are exiled this way, …" (CR 603.3b — Augusta, Order
    /// Returned). The reflexive twin of [`ReflexiveTrigger`](Self::ReflexiveTrigger) gated on a
    /// count rather than a minted token: placed after the same resolution's
    /// [`EachPlayerExilesFromGraveyard`](crate::ChoiceEffect::EachPlayerExilesFromGraveyard) fan-out,
    /// it creates a reflexive triggered ability for each `then` effect **only when that fan-out
    /// exiled one or more nonland cards** — none at all when the count is zero. The count
    /// ([`ResolutionFrame::nonland_cards_exiled_this_way`](crate::resolution::ResolutionFrame)) is
    /// baked into each `then` effect's [`Amount::NonlandCardsExiledThisWay`](crate::Amount) at
    /// placement, so the follow-up reads the settled number even though it resolves in its own
    /// later frame; its target is chosen when it goes on the stack (CR 601.2c), after the fan-out.
    ReflexiveTriggerIfNonlandExiled {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },

    ReturnAllToHand {
        filter: PermanentFilter,
    },

    /// Knowledge Vault's two ways out of its own pile: "put all cards exiled with this artifact
    /// into their owner's hand" (the `{0}` cash-out) and "…into their owner's graveyard"
    /// (`to_graveyard`, the leaves-the-battlefield punishment). Empties the source's CR 400.10a
    /// "exiled with" association ([`MillEffect::ExileTopFaceDownWithThis`](crate::MillEffect)),
    /// so a second reading finds nothing — which is what makes the cash-out beat the departure
    /// trigger it sets off.
    ReturnAllExiledWithThis {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        to_graveyard: bool,
    },

    ReturnExiledCardToOwnersGraveyard {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        exiled: ObjectId,
    },

    ReturnFlickeredCard {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        exiled: Option<ObjectId>,
    },

    ReturnFromGraveyardAttachedToToken {
        filter: CardFilter,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        token: Option<ObjectId>,
    },

    ReturnFromGraveyardToHand {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
    },

    ReturnObjectToHand {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        object: Option<ObjectId>,
    },

    /// Spurnmage Advocate's "Return two target cards from an opponent's graveyard to their hand" —
    /// the plural sibling of [`Self::ReturnFromGraveyardToHand`], which returns the one target the
    /// ability shares. This is an *independent* target clause (CR 601.2c/603.3d) chosen alongside
    /// the ability's own target and read back off `targets_second` at resolution, the same shape as
    /// [`crate::CountersEffect::DoubleCountersOnTargetCreatures`].
    ReturnTargetCardsFromGraveyardToHand {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
    },

    ReturnThisAuraAttachedTo {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        creature: Option<ObjectId>,
    },

    ReturnThisAuraFromGraveyardAttachedToChosenHost {
        /// Who picks the new host. [`PlayerSet::You`] — the default, and the unwritten one on
        /// Screams from Within / Ghoulish Impetus, which say only "return this card to the
        /// battlefield" and leave CR 303.4f's choice with the Aura's own controller.
        /// `dying_enchanted_creatures_controller` is Takklemaggot's "**that creature's
        /// controller** chooses a creature that this card could enchant", baked in at trigger
        /// placement (CR 603.10a) because the host is a graveyard card by resolution.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        chosen_by: PlayerSet,
        /// Takklemaggot's "if they don't, return this card to the battlefield under your control
        /// as a **non-Aura enchantment**": with no legal host it comes back anyway, unattached
        /// and no longer an Aura, instead of staying in the graveyard the way an ordinary
        /// transferrable Aura does.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        hostless_returns_as_non_aura: bool,
    },

    ReturnThisFromGraveyardToBattlefield {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        tapped: bool,
    },

    ReturnThisToHand,

    ReturnToHand {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
    },

    ScheduleReturnReanimatedToHand,

    ScheduleReturnThisAuraAttachedToReanimated,

    ScheduleReturnThisAuraFromGraveyardAttachedToChosenHost,

    ShuffleTargetPermanentIntoLibrary {
        target: TargetSpec,
    },

    ShuffleTargetPermanentIntoLibraryThenReveal {
        target: TargetSpec,
    },

    TuckFromGraveyard {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        to_top: bool,
    },

    TuckPermanentIntoLibrary {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        to_top: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        second_from_top: bool,
    },

    TuckSelfAndBlockedCreatures,

    TuckSelfToLibraryBottom,

    UntapSearchedLand,
}
