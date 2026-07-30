use super::*;
#[cfg(feature = "card-dsl")]
use crate::de;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
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

    /// "counter it" / "counter that spell" on a [`Trigger::CastSpell`] watch — Presence of the
    /// Master, Nether Void, In the Eye of Chaos, Invoke Prejudice. CR 115.1: "it" is not a target,
    /// so the spell is the one that fired the watch, baked in at trigger placement from
    /// [`TriggerContext::triggering_spell`] rather than chosen. The optional `unless_pays` rider is
    /// CR 701.5c, offered to the spell's own controller.
    CounterTriggeringSpell {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        triggering_spell: Option<ObjectId>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        unless_pays: Option<Amount>,
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
        #[cfg_attr(feature = "card-schema", schemars(with = "String"))]
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

    /// Blaze of Glory's "Target creature defending player controls can block any number of
    /// creatures this turn. It blocks each attacking creature this turn if able." Both halves of
    /// one printed sentence, so one effect: the block ceiling comes off in
    /// `CombatExtras::may_block_any_number` and the requirement goes into
    /// `CombatExtras::must_block_all`. `Game::declare_blockers` reads both, and both expire at the
    /// next Untap step.
    ///
    /// ponytail: the two halves are stored apart but only ever set together — no card in the pool
    /// prints one without the other. Give this a pair of flags the day one does.
    BlocksEachAttackerIfAble {
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
        /// "Prevent all but 1 of that damage" (Forcefield, CR 615.4) — the points that get
        /// *through*, rather than the points stopped. Set it instead of `amount`, not alongside:
        /// a keep-shield is spent outright by the hit it stood in front of.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        all_but: Option<Amount>,
        /// "An unblocked creature of your choice would deal combat damage **to you**"
        /// (Forcefield): the chosen `target` is the damage's *source*, and the shield goes up in
        /// front of this ability's controller. The ordinary reading — `target` is what the shield
        /// protects — is what every other card in the pool prints.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target_is_source: bool,
        /// "Would deal *combat* damage" (Forcefield): only damage dealt in a combat damage step
        /// is stopped. `false` (default) covers combat and noncombat alike.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        combat_only: bool,
        /// "… by **attacking creatures without flying**" (Al-abara's Carpet) — a gate on the
        /// damage's *source*, the turn-scoped twin of
        /// [`StaticEffect::PreventDamage`](crate::StaticEffect::PreventDamage)'s `source_filter`.
        /// A source that isn't a permanent never matches one.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        from_filter: Option<PermanentFilter>,
        /// "… a spell or ability that **targets that creature**" (Silhouette) — the same
        /// source/recipient relationship [`StaticEffect::PreventDamage`](crate::StaticEffect::PreventDamage)'s
        /// `source_relation` reads, on a turn-scoped shield instead of a permanent's static.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        from_relation: Option<SourceRelation>,
        /// "Prevent **all** damage … this turn" (Al-abara's Carpet, Silhouette) rather than "the
        /// next …": CR 615.6 — this shield is never used up, so it stands in front of every
        /// qualifying hit until it expires at end of turn. `false` (every other pool shield) is
        /// consumed by what it prevents.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        all_damage: bool,
    },

    /// Guardian Angel's second sentence: "Until end of turn, you may pay {1} any time you could
    /// cast an instant. If you do, prevent the next 1 damage that would be dealt to that permanent
    /// or player this turn." A repeatable, optional, priority-timed *offer* rather than a shield —
    /// nothing is prevented until someone pays, and each payment mints its own
    /// [`PreventNextDamage`](Self::PreventNextDamage)-shaped shield, so the offer can be milked all
    /// turn.
    ///
    /// Takes no target of its own: "that permanent or player" is whatever the enclosing
    /// [`Effect::Sequence`](crate::Effect::Sequence)'s first step targeted, which resolution reads
    /// off the shared target. Recorded on
    /// [`Game::standing_preventions`](crate::Game::standing_preventions) and offered as a
    /// [`MeaningfulAction::PayStandingPrevention`](crate::MeaningfulAction) — the same legal-action
    /// list an activated ability rides, so timing ("any time you could cast an instant") and the
    /// payment plumbing are already covered.
    OfferPreventionTopUp {
        /// What one payment costs — Guardian Angel's `{1}`.
        cost: Cost,
        /// Points each bought shield is worth — Guardian Angel's `1`.
        amount: i32,
    },

    PreventCombatDamageToYouCreatingTokens {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::token_profile"))]
        #[cfg_attr(feature = "card-schema", schemars(with = "String"))]
        token: CardDef,
    },

    ScheduleAtNextUpkeep {
        /// Who the delayed trigger belongs to, resolved to a concrete seat now and stored on the
        /// scheduled trigger — the ability's controller by default (Dragon Whelp's "sacrifice
        /// this creature"), `targets_controller` for the shared target spell's controller
        /// (Arcane Denial's "**its controller** may draw up to two cards"). One seat: a delayed
        /// trigger has one controller, so a multi-seat set is rejected at resolution.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        who: PlayerSet,
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

    /// "This creature can't be regenerated this turn" (Clergy of the Holy Nimbus's second
    /// ability, CR 701.15d) — sets the same suppression flag Disintegrate/Terror bundle with
    /// their own destruction, but on its own, with no damage or destroy attached. Always the
    /// ability's own source, never a target — the printed ability names "this creature", not
    /// "target creature".
    SourceCantBeRegeneratedThisTurn,

    SkipNextUntapOpponentCreatures,

    /// "Take an extra turn after this one" (Time Walk; CR 505.6a). Queues one turn for the
    /// controller, taken as this turn ends and before the rotation moves on — see
    /// [`Game::advance_step`](crate::Game::advance_step).
    TakeExtraTurn,

    /// "You lose the game" (CR 104.3b — Lich's dies trigger, and the shortfall arm of its damage
    /// tax). The effect's controller is eliminated outright, not taken to 0 life: nothing about
    /// the loss is a life total, so no prevention, protection or life-total static can hold it
    /// off.
    YouLoseTheGame,

    /// "The game is a draw" (CR 104.4 — Divine Intervention). Its own game outcome, not everybody
    /// losing: the game ends the moment this resolves with no winner and no loser
    /// ([`Game::outcome`](crate::Game::outcome) reports
    /// [`GameOutcome::Draw`](crate::GameOutcome)), so no player's `lost` flag is set and none of
    /// the CR 800.4a elimination bookkeeping runs.
    GameIsADraw,

    YouChooseWhichCreaturesAttack,

    YouChooseWhichCreaturesBlock,
}
