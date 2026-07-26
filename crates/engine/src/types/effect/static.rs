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
pub enum StaticEffect {
    Anthem {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        power: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        toughness: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        self_only: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        exclude_source: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        tokens_only: bool,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        keywords: &'static [Keyword],
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        subtypes: &'static [&'static str],
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        colors: &'static [Color],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        chosen_subtype: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        attacking_only: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        blocking_only: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        commander_only: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        has_counters: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        condition: Option<Condition>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        from_graveyard: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        all_players: bool,
    },

    AttackTax {
        amount: u8,
    },

    CantBeAttackedBy {
        filter: PermanentFilter,
    },

    CastXReplacement {
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one"))]
        times: i32,
    },

    ControlAttached,

    /// A counter-placement replacement (CR 614 — Hardened Scales, Doubling Season, Vorinclex).
    /// `add` then `times` then `halve` describe the modification; the remaining fields say which
    /// placements it sees. See [`Game::counters_after_replacements`](crate::Game).
    CounterReplacement {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        add: i32,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one"))]
        times: i32,
        /// Vorinclex's opponent-facing clause: "half that many … rounded down".
        #[cfg_attr(feature = "card-dsl", serde(default))]
        halve: bool,
        /// Benevolent Hydra's "another creature you control": never replaces its own source's
        /// counters.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        other: bool,
        /// "one or more counters" (Winding Constrictor, Vorinclex) rather than the default
        /// "one or more +1/+1 counters" (Hardened Scales, Corpsejack Menace).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        any_kind: bool,
        /// Vorinclex / Innkeeper's Talent Level 3: the clause keys off who *would put* the
        /// counters (CR 614.1), not off whose permanent receives them. `None` keys off the
        /// recipient's side instead (Doubling Season's "a permanent you control", Winding
        /// Constrictor's passive "would be put on … you control").
        #[cfg_attr(feature = "card-dsl", serde(default))]
        placer: Option<CounterPlacer>,
        /// Which recipients the replacement reaches (CR 122.1 — counters sit on permanents and on
        /// players).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        recipients: CounterRecipients,
        /// A type gate on the receiving permanent — Ozolith's "an artifact or creature you
        /// control". `None` is every permanent (Doubling Season, Vorinclex).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        filter: Option<PermanentFilter>,
    },

    CounterScaledAttackTax,

    CreaturesYouControlEnterWithCounters {
        filter: PermanentFilter,
        count: Amount,
    },

    EntersWithCounters {
        #[cfg_attr(feature = "card-dsl", serde(rename = "count"))]
        amount: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        kind: Option<CounterKind>,
    },

    GrantManaAbility {
        filter: PermanentFilter,
        cost: ActivationCost,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::mana_batch")
        )]
        mana: ManaPool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        restriction: Option<SpendRestriction>,
    },

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
        #[cfg_attr(feature = "card-dsl", serde(default))]
        goad: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        protection_from_chosen_color: bool,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::opt_static_granted_ability")
        )]
        granted_ability: Option<&'static GrantedAbility>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        cant_attack: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        cant_block: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        cant_attack_controller: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        activated_abilities: Option<AbilityRestriction>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        legendary_only: bool,
    },

    KeywordAnthem {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        keywords: &'static [Keyword],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        filter: PermanentFilter,
    },

    LifeGainReplacement {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        plus: i32,
    },

    NoMaximumHandSize,

    PlayFromGraveyardOncePerTurn,

    PreventCombatDamage {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        to_self: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        by_self: bool,
    },

    PreventDamageToSelfRemovingCounter,

    PreventDamageToSelfRemovingCountersGivingRad,

    PreventNoncombatDamageToOtherCreaturesYouControl,

    ReduceSpellCost {
        amount: Amount,
        filter: SpellFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        first_x_spell_each_turn: bool,
    },

    SetAttachedBasePt {
        power: i32,
        toughness: i32,
    },

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

    TappedForManaBonus {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        scope: LandTapScope,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        bonus_color: LandTapBonusColor,
    },

    TokenReplacement {
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one"))]
        times: i32,
    },

    TriggerDoubling {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        source_subtypes: &'static [&'static str],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        source_other: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        caused_by_instant_or_sorcery_cast: bool,
    },
}

/// Which recipients a [`StaticEffect::CounterReplacement`] reaches. Counters sit on permanents and
/// on players (CR 122.1), and a card names one or both: Hardened Scales only permanents, Winding
/// Constrictor's second ability only its controller, Vorinclex "a permanent or player".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum CounterRecipients {
    #[default]
    Permanents,
    Players,
    PermanentsAndPlayers,
}

/// Whose *placement* a [`StaticEffect::CounterReplacement`] replaces (CR 614.1) — the axis
/// Vorinclex's "if **you would put**" / "if an **opponent would put**" reads. Distinct from
/// [`CounterRecipients`] and from the effect's `filter`, which both gate the *recipient*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum CounterPlacer {
    You,
    Opponents,
}
