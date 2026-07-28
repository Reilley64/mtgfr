use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum DestroyEffect {
    All {
        filter: PermanentFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        cant_be_regenerated: bool,
    },

    Target {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        cant_be_regenerated: bool,
    },

    TriggeringDamagedCreature {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        creature: Option<ObjectId>,
    },
}
