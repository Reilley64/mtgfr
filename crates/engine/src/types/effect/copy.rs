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
pub enum CopyEffect {
    ChangeTargetOfTargetSpellOrAbility {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        optional: bool,
    },

    TargetSpell,

    ThisSpell {
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one_amount"))]
        count: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        cast_from_graveyard_only: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        optional: bool,
    },

    CopyTriggeringAbility {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        triggering_ability: Option<ObjectId>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        may_choose_new_targets: bool,
    },

    CopyTriggeringSpell {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        triggering_spell: Option<ObjectId>,
        count: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        may_choose_new_targets: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        last_known_information: bool,
    },

    CopyTriggeringSpellForEachOtherCreatureYouControl {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        triggering_spell: Option<ObjectId>,
    },

    Demonstrate {
        spell: ObjectId,
    },

    MayPayToCopyThis {
        cost: Cost,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one_amount"))]
        count: Amount,
    },

    MintFreeCopyOfExiledCard {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        card: Option<ObjectId>,
    },

    RetargetSpellCopy {
        copy: ObjectId,
    },
}
