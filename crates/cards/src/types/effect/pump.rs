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
pub enum PumpEffect {
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
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        add_colors: &'static [Color],
        /// "becomes a 3/6 Golem artifact creature **until end of combat**" (Jade Statue) — the
        /// animation is swept at the End of Combat step instead of surviving to cleanup. Strictly
        /// shorter than the default until-end-of-turn duration, so it can't be approximated by
        /// leaving it off.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        ends_at_end_of_combat: bool,
    },

    EnchantedAttackerPumpAttackingOpponentElseControllerLosesLife {
        power: i32,
        toughness: i32,
        life: u32,
    },

    /// "This Aura gains 'Enchanted creature loses flying'" (Earthbind's enters trigger, whose
    /// intervening-if already checked that the host had it): the host loses `keywords` for as long
    /// as this Aura stays attached to it, with no end-of-turn expiry. The indefinite, Aura-bound
    /// sibling of [`StripKeywordsFromOpponentsCreatures`](Self::StripKeywordsFromOpponentsCreatures)
    /// — see [`Event::AttachedKeywordsLost`](crate::Event). Nothing happens if the Aura has already
    /// fallen off (CR 704.5m).
    EnchantedCreatureLosesKeywords {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_slice"))]
        keywords: &'static [Keyword],
    },

    GrantChosenColorProtectionUntilEndOfTurn {
        target: TargetSpec,
    },

    /// The old "Radiance" keyword action's batch twin of
    /// [`GrantChosenColorProtectionUntilEndOfTurn`](Self::GrantChosenColorProtectionUntilEndOfTurn)
    /// (Bathe in Light): "Target creature and each other creature that shares a color with it
    /// gain protection from the chosen color until end of turn." Sequenced after a `choose_color`
    /// step the same way — the color lives on the ability's own `source` — but the grant lands on
    /// [`Game::radiance_batch`] of `target`, not just `target` itself.
    RadianceChosenColorProtectionUntilEndOfTurn {
        target: TargetSpec,
    },

    GrantKeywordsToPermanentsYouControlUntilEndOfTurn {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        keywords: &'static [Keyword],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        filter: PermanentFilter,
    },

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

    /// Mass pump, every controller: every creature on the battlefield matching `filter`, not just
    /// the controller's own (Bladewing the Risen: "Dragon creatures get +1/+1 until end of turn" —
    /// board-wide, unlike [`PumpCreaturesYouControlUntilEndOfTurn`](Self::PumpCreaturesYouControlUntilEndOfTurn),
    /// which hardcodes "you control" on top of `filter`'s own controller axis).
    PumpEachCreatureUntilEndOfTurn {
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

    PumpOtherAttackersAttackingYourOpponents {
        power: i32,
        toughness: i32,
    },

    PumpSelfUntilEndOfTurn {
        power: Amount,
        toughness: Amount,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        keywords: &'static [Keyword],
    },

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

    SetBasePtCreaturesYouControlUntilEndOfTurn {
        power: Amount,
        toughness: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        other: bool,
    },

    SetBasePtTargetUntilEndOfTurn {
        power: Amount,
        toughness: Amount,
        target: TargetSpec,
    },

    SetOwnBasePtFromAmount {
        amount: Amount,
    },

    StripKeywordsFromOpponentsCreatures {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        keywords: &'static [Keyword],
    },

    /// Vesuvan Doppelganger's upkeep: "you may have this creature become a copy of target
    /// creature, except it doesn't copy that creature's color and it has this ability" (CR
    /// 707.2). The *source* permanent becomes the copy, so `target` names the creature being
    /// copied, not the one being rewritten. The two exception flags mirror the
    /// [`crate::EnterAsCopy`] fields of the same name and are applied by the same
    /// `copy_with_exceptions` synthesizer, so entering as a copy and re-copying later land on
    /// exactly the same def. Indefinite (`until_eot: false`) — the printed shapeshifter never
    /// comes back.
    BecomesCopyOfTarget {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        keeps_own_color: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        keeps_own_abilities: bool,
    },

    TargetBecomesTreasure {
        target: TargetSpec,
    },

    /// "Target spell or permanent becomes black." (Deathlace and the rest of the lace cycle) — a
    /// CR 613.3c layer-5 color SET with no duration printed at all, so it lasts as long as the
    /// object does. `color` *replaces* the object's colors rather than unioning with them, which
    /// is why a laced spell stops being countered by "counter target blue spell". Targets a spell
    /// on the stack or a permanent, and registers the layer-5 SET / writes [`Spell::set_color`]
    /// through [`Event::ColorSet`]. The reminder text ("its mana symbols remain unchanged") needs
    /// no modelling — colors are read from `colors_of`, never re-derived from the pips.
    TargetBecomesColor {
        target: TargetSpec,
        color: Color,
    },

    /// "Target land becomes a Forest until this creature leaves the battlefield" (Gaea's Liege) —
    /// a CR 613.4 type change whose duration is the *source's* stay on the battlefield, the only
    /// one the pool prints. `set_subtypes` replaces the whole land-type line (CR 305.7), so a
    /// Mountain taps for `{G}` and not `{R}`. Written to the target's
    /// `Permanent::subtypes_set_while_source_remains` and read back only while the source is still
    /// a permanent; nothing needs to undo it.
    TargetBecomesSubtypesWhileSourceRemains {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_str_slice"))]
        set_subtypes: &'static [&'static str],
    },

    WeakenEachCreature {
        power: Amount,
        toughness: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        opponents_only: bool,
    },
}
