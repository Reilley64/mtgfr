use super::*;
#[cfg(feature = "card-dsl")]
use crate::de;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)
)]
pub enum MiscEffect {
    ArmCombatDamageWatch,

    BecomePrepared,

    CounterTargetActivatedAbility,

    CounterTargetSpell {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        unless_pays: Option<Amount>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        filter: SpellFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        countered_dest: Option<CounteredDest>,
        /// Power Sink's "if that player doesn't, they tap all lands with mana abilities they
        /// control and lose all unspent mana" — a penalty riding on the *declined* half of
        /// `unless_pays`, so it can't be an ordinary following step (those run either way).
        /// `false` (the default) for every other counter-unless-pays. Ignored without
        /// `unless_pays`.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        strips_mana_on_decline: bool,
    },

    Fight {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        enemy: Option<Target>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        ally_is_shared_target: bool,
        // Infectious Bite: "Target creature you control deals damage equal to its power to
        // target creature you don't control" is one-directional — CR 701.12/119.3 fight
        // language never appears, so only the ally→enemy damage half of the plumbing below runs.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        one_way: bool,
    },

    FlipSource,

    /// "You get an emblem with …" (CR 114.1, Garruk, Cursed Huntsman's −6): create the emblem
    /// named by `emblem` in the resolving controller's command zone. The emblem's abilities are
    /// an ordinary [`CardDef`] resolved out of `cards/data/tokens/` by Scryfall oracle id, the
    /// same registry `create_token` reads — Scryfall models emblems as token-set cards, and CR
    /// 114.5's "no characteristics other than its abilities" is expressed by giving that profile
    /// [`CardKind::Spell`], whose [`TypeSet`] is empty.
    GetEmblem {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::token_profile"))]
        emblem: CardDef,
    },

    GrantChannelColorlessManaThisTurn,

    GrantFlashThisTurn,

    MustAttackRandomOpponent,

    /// "Creatures the active player controls attack this turn if able" (CR 508.1a — Siren's
    /// Call): [`MustAttackTarget`](Self::MustAttackTarget)'s clause over a board scan instead of
    /// one chosen creature, minting the same requirement per match. The set is locked in as this
    /// resolves — a creature that arrives afterwards was never named.
    MustAttackAll {
        filter: PermanentFilter,
    },

    /// "Target creature attacks this turn if able" (CR 508.1a). `target` is the spec the
    /// choice is made against: `TargetSpec::Creature` by default (Basandra, Battle Seraph's
    /// "target creature" — anyone's), narrowed by cards that print a qualified clause (Nettling
    /// Imp's "target non-Wall creature the active player has controlled continuously since the
    /// beginning of the turn"). Authorable for the same reason [`ManaEffect::Add`]'s own `target`
    /// is: the spec is the card's wording, not the effect's fixed shape.
    MustAttackTarget {
        #[cfg_attr(feature = "card-dsl", serde(default = "de::target_creature"))]
        target: TargetSpec,
    },

    PreventAllCombatDamageThisTurn,

    /// "Prevent the next N damage that would be dealt to any target this turn" (CR 615 — Healing
    /// Salve, Samite Healer). Arms a consumable entry on
    /// [`Game::damage_prevention_shields`](crate::Game::damage_prevention_shields) worth `amount`
    /// points, spent at the two damage chokes against any damage — combat or not — unlike the
    /// all-or-nothing, combat-only shields on either side of it here.
    ///
    /// `target` left at [`TargetSpec::None`] shields the ability's controller instead of a chosen
    /// target: Conservator's "dealt to you", which takes no target at all.
    ///
    /// `amount` left at `None` is "prevent *that* damage" rather than "the next N" — the whole of
    /// the next qualifying hit, however big (the Circle of Protection cycle, Reverse Damage).
    PreventNextDamage {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        amount: Option<Amount>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target: TargetSpec,
        /// "a **black** source of your choice" (CR 105.2a) — only damage from a source of this
        /// color is prevented. [`ColorFilter::Any`] (the default) is the plain uncolored shield.
        ///
        /// ponytail: the color, but not the *choice*. A Circle's source is picked when the
        /// ability resolves (CR 609.7), which lets its controller hold the shield for the second
        /// of two black hits; this arms against whichever black source strikes first. Identical
        /// in the ordinary line — the Circle is activated in response to the damage — and the
        /// upgrade path is `PendingChoice::ChooseDamageSource`, offering every battlefield and
        /// stack object of the color.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        from_color: ColorFilter,
        /// "You gain life equal to the damage prevented this way" (Reverse Damage): the shield's
        /// controller gains whatever this shield actually ate, minted alongside the spend.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        gain_life: bool,
        /// "That source deals that damage to **you** instead" (Jade Monolith, CR 615.10) — the
        /// hit is moved onto the ability's controller rather than prevented. Both pool cards that
        /// redirect send it to the same player who armed the shield, so there is nothing to name.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        redirect_to_controller: bool,
        /// "Damage that would be dealt to **this creature**" (Personal Incarnation): the shield
        /// covers the permanent that armed it. Not a `target` — the ability targets nothing, and
        /// no [`TargetSpec`](crate::TargetSpec) can name an effect's own source.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        shield_source: bool,
    },

    PreventCombatDamageToYouCreatingTokens {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::token_profile"))]
        token: CardDef,
    },

    ScheduleAtNextUpkeep {
        who: DelayController,
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_effect"))]
        then: &'static Effect,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        fire_at: Step,
    },

    ScheduleColorlessManaForCounteredSpellNextMainPhase,

    ScheduleNextCastTrigger {
        filter: SpellFilter,
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_slice"))]
        then: &'static [Effect],
    },

    ScheduleThisTurnCombatDamageCopy,

    SkipNextUntapOpponentCreatures,

    /// "Take an extra turn after this one" (Time Walk; CR 505.6a). Queues one turn for the
    /// controller, taken as this turn ends and before the rotation moves on — see
    /// [`Game::advance_step`](crate::Game::advance_step).
    TakeExtraTurn,

    YouChooseWhichCreaturesAttack,

    YouChooseWhichCreaturesBlock,
}
