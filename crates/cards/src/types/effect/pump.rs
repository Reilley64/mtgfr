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

    /// "Target creature loses `keywords` (and every keyword in `families`)" — the targeted,
    /// CR 613.1f-layered keyword removal the Legends lands and their two creatures print:
    /// Hammerheim's "all landwalk abilities", Radjan Spirit's "flying", Tolaria's "banding and all
    /// \"bands with other\" abilities", Shelkin Brownie's "all \"bands with other\" abilities",
    /// Urborg's "first strike or swampwalk", and Elder Land Wurm's own "it loses defender".
    ///
    /// `until_end_of_turn` is the *special* case, not the default: Elder Land Wurm's blocks trigger
    /// has no duration at all, so the loss is indefinite unless the card says otherwise.
    ///
    /// `choose_one` is CR 609.4's resolution-time pick between the listed `keywords` (Urborg's
    /// "first strike **or** swampwalk"): the ability's target is locked at activation and its
    /// controller names one of the keywords when it resolves, not when it goes on the stack — which
    /// is what separates it from a printed "Choose one —" (CR 601.2b). Peeled to the mode pause
    /// before this ever reaches the pump minter.
    ///
    /// Unlike [`StripKeywordsFromOpponentsCreatures`](Self::StripKeywordsFromOpponentsCreatures)
    /// the loss is *not* "and can't have": a grant with a later timestamp beats it (CR 613.7).
    TargetLosesKeywords {
        target: TargetSpec,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        keywords: &'static [Keyword],
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        families: &'static [KeywordFamily],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        until_end_of_turn: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        choose_one: bool,
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
        /// "Gets +10/+0 **until end of combat**" (CR 511.3 — Glyph of Destruction): the shorter
        /// of the two durations a pump can print, and the same knob Jade Statue's
        /// [`AnimateSelfUntilEndOfTurn`](Self::AnimateSelfUntilEndOfTurn) spells. Defaults to
        /// `false`, which is the until-end-of-turn wording the mode is named for; `true` moves the
        /// wear-off from the cleanup step to the end of combat step.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        ends_at_end_of_combat: bool,
        /// Part Water's "**X target creatures** gain islandwalk until end of turn" (CR 601.2c) —
        /// the same target-count axis [`ControlEffect::TapTarget`](crate::ControlEffect) carries
        /// for Winter Blast's "tap X target creatures". Defaults to the single target every other
        /// pump in the pool takes.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
    },

    /// "That creature gains flying" (Cocoon) — a keyword grant with **no printed duration**, so it
    /// lasts as long as the object does (CR 400.7) rather than wearing off at cleanup. The
    /// durationless twin of [`Self::PumpUntilEndOfTurn`]'s keyword half; it grants no P/T, because
    /// nothing in the pool sets an indefinite boost this way. Distinct from an
    /// [`Effect::Static(StaticEffect::GrantToAttached)`] keyword grant, which only holds while the
    /// granting Aura is still attached — Cocoon is sacrificed by the very ability that grants this.
    GrantKeywordsIndefinitely {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_slice"))]
        keywords: &'static [Keyword],
    },

    /// "Gabriel Angelfire gains that ability until your next upkeep" — a keyword grant on the
    /// ability's own source (no target) on the one duration that ends at the *start* of an upkeep
    /// rather than at its end. Strictly shorter than Halfdane's
    /// [`SetOwnBasePtFromTargetUntilEndOfNextUpkeep`](Self::SetOwnBasePtFromTargetUntilEndOfNextUpkeep):
    /// the previous grant is already gone by the time the next upkeep's trigger resolves, which is
    /// what makes each upkeep's choice replace the last one without any explicit removal.
    ///
    /// `choose_one` is CR 609.4's resolution-time pick among the listed `keywords` ("choose flying,
    /// first strike, trample, or rampage 3"), the same shape
    /// [`TargetLosesKeywords`](Self::TargetLosesKeywords) uses for Urborg — not a printed
    /// "Choose one —" (CR 601.2b/603.3d), which a triggered ability would decide as it goes on the
    /// stack. Peeled to the mode pause before this ever reaches the pump minter.
    GrantSelfKeywordsUntilNextUpkeep {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_slice"))]
        keywords: &'static [Keyword],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        choose_one: bool,
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

    /// "Change this creature's base toughness to 1 plus the power of target creature blocking or
    /// blocked by this creature" (Sentinel), "…to 1 plus the number of creature cards in your
    /// graveyard" (Wall of Tombstones): the base-*toughness*-only, indefinite sibling of
    /// [`SetOwnBasePtFromAmount`](Self::SetOwnBasePtFromAmount). The ability's own source is what
    /// changes; `target` is only there for an `amount` that reads off a target (Sentinel's
    /// `target_power`) and is `TargetSpec::None` otherwise.
    ///
    /// Layer 7b (CR 613.3(7b)), so counters and pumps still ride above it, and — being indefinite
    /// with its own timestamp — a second activation simply outranks the first (CR 613.7).
    SetOwnBaseToughnessFromAmount {
        amount: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target: TargetSpec,
    },

    /// "Change Halfdane's base power and toughness to the power and toughness of target creature
    /// other than Halfdane until the end of your next upkeep": a layer-7b set on the ability's own
    /// source, snapshotting the target's *effective* P/T as the ability resolves (CR 613.4b), on
    /// the only duration in the pool that outlives cleanup without being indefinite.
    SetOwnBasePtFromTargetUntilEndOfNextUpkeep {
        target: TargetSpec,
    },

    /// "When this creature dies, change the base power and toughness of all creatures that dealt
    /// damage to it this turn to 0/2" (Brine Hag): a layer-7b set with no duration, on every
    /// creature the source's turn-scoped damage tally recorded as a dealer (CR 603.10a — the tally
    /// is last-known information by the time this resolves from the graveyard).
    SetBasePtCreaturesThatDamagedSourceThisTurn {
        power: i32,
        toughness: i32,
    },

    /// "Switch target creature's power and toughness until end of turn" (Transmutation): CR 613.4e,
    /// applied after every other P/T layer rather than as a base set — a switch over a −0/−2
    /// counter on a 6/4 gives a 2/6, not a 4/4.
    SwitchPtUntilEndOfTurn {
        target: TargetSpec,
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
    ///
    /// The Legends colour-wash cycle (Dwarven Song, Heaven's Gate, Sea Kings' Blessing, Sylvan
    /// Paradise, Touch of Darkness) is the same SET with the two other axes turned on: `count`
    /// carries "one or more target creatures" (CR 601.2c) and `until_end_of_turn` its printed
    /// duration. Alchor's Tomb turns on the third — a `color` of `None` is "the color of your
    /// choice", picked by the ability's controller at resolution (CR 609.3) through the shared
    /// `PendingChoice::ChooseColor` picker rather than authored here.
    /// Colorless is never a candidate: CR 105.1 says colorless is not a color.
    TargetBecomesColor {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        color: Option<Color>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        until_end_of_turn: bool,
    },

    /// Aisling Leprechaun's "that creature becomes green. (This effect lasts indefinitely.)" —
    /// [`TargetBecomesColor`](Self::TargetBecomesColor)'s no-duration layer-5 SET aimed at a block
    /// pair's other half instead of a chosen target (CR 613.3c). "That creature" is not a target
    /// (CR 115.1), so `creature` is baked in when the trigger is placed, the same slot
    /// [`DestroyEffect::ThatCreature`](crate::DestroyEffect) and
    /// [`MiscEffect::ThatCreatureCantAttackNextOwnTurn`](crate::MiscEffect) take off the same
    /// block pair.
    ThatCreatureBecomesColor {
        color: Color,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        creature: Option<ObjectId>,
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
