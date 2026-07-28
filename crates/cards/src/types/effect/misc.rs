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

    MustAttackTarget,

    PreventAllCombatDamageThisTurn,

    PreventCombatDamageToYouCreatingTokens {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::token_profile"))]
        #[cfg_attr(feature = "card-schema", schemars(with = "String"))]
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

    YouChooseWhichCreaturesAttack,

    YouChooseWhichCreaturesBlock,
}
