use super::*;
#[cfg(feature = "card-dsl")]
use crate::de;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)
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
        /// Restricts to creatures controlled by a player who has made a matching per-player
        /// choice: `Some(true)`/`Some(false)` reads a two-sided as-enters choice recorded on
        /// [`Player`](crate::Player) (Archangel of Strife's "Creatures controlled by players who
        /// chose war/peace"); `None` (default) applies no such restriction.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        war_choice: Option<bool>,
    },

    AttackTax {
        amount: u8,
    },

    /// "Each opponent who cast a spell this turn can't attack with creatures" (Angelic Arbiter):
    /// a blanket per-player attack ban, unlike [`StaticEffect::CantBeAttackedBy`]'s
    /// defender-scoped filter — the gated player can't declare *any* attacker, not just ones
    /// aimed at a specific defender. Checked against `Player::spells_cast_this_turn` in
    /// `Game::declare_attackers`, and only against a static controlled by someone other than the
    /// declaring player (CR: "opponent").
    CantAttackIfCastThisTurn,

    CantBeAttackedBy {
        filter: PermanentFilter,
    },

    CantBlockFilter {
        filter: PermanentFilter,
    },

    CantCastDuringCombat,

    /// "Each opponent who attacked with a creature this turn can't cast spells" (Angelic
    /// Arbiter): the mirror of [`StaticEffect::CantAttackIfCastThisTurn`] — a blanket per-player
    /// cast ban, checked against `Player::attacked_this_turn` in `Game::cast_timing_ok`, and only
    /// against a static controlled by someone other than the casting player.
    CantCastIfAttackedThisTurn,

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
        /// "Add N mana of any one color" (CR 106.4 — Goldspan Dragon's granted Treasure ability):
        /// every credit locks to the one color the controller names, so activating pauses on
        /// [`crate::PendingChoice::ChooseManaColor`] rather than producing independent wildcards.
        /// The granted twin of [`ManaEffect::Add`]'s own `single_color`; `false` for a plain grant.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        single_color: bool,
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
        /// Consecrate Land's "can't be enchanted by other Auras": no *other* Aura may attach to
        /// this host — none can be cast targeting it, and one already there falls off (CR
        /// 704.5n). See [`Game::host_cant_be_enchanted_by`](crate::Game::host_cant_be_enchanted_by).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        cant_be_enchanted: bool,
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
        #[cfg_attr(feature = "card-dsl", serde(default))]
        all_players: bool,
    },

    LifeGainReplacement {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        plus: i32,
    },

    MustAttackEachCombat,

    NoMaximumHandSize,

    OpponentsCantSearchLibraries,

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

    ProtectionFromChosenColor,

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
        /// CR 613.4: when `true`, `add_types` are the host's *complete* card types (replacing its
        /// printed ones — Darksteel Mutation's "loses all other … card types"), not merely unioned
        /// on. Default `false` keeps the additive Angelic-Destiny behavior.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        set_types: bool,
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

    /// Sunglasses of Urza's "You may spend white mana as though it were red mana" (CR 609.4b):
    /// while this permanent's controller pays a cost, each of their `from` credits may pay a `to`
    /// pip as well as its own. Gathered by [`Game::mana_substitutions`](crate::Game) and applied
    /// to the pool by [`ManaPool::substituted`](crate::ManaPool) before the payment planners run.
    SpendManaAsThoughAnotherColor {
        from: Color,
        to: Color,
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
