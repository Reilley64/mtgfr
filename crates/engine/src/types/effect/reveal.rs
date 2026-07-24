use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
pub enum RevealEffect {
    TopAndDrainMutual,

    TopCards {
        count: Amount,
        filter: CardFilter,
        matched_dest: SearchDest,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        matched_tapped: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        rest_dest: RestDest,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        deploy_untapped_if: Option<Condition>,
    },

    TopToHand {
        filter: CardFilter,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        defender: Option<PlayerId>,
    },

    Until {
        filter: CardFilter,
        count: Amount,
        matched_dest: SearchDest,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        matched_tapped: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        rest_dest: RestDest,
    },
}
