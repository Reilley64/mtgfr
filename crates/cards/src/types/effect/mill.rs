use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum MillEffect {
    ExileDiscardedWithThis {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        card: Option<ObjectId>,
    },

    ExileFromGraveyardMayPlay {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        card: Option<ObjectId>,
    },

    ExileTargetFromGraveyardCreateTokenCopy {
        filter: CardFilter,
    },

    ExileTargetFromGraveyardWithThis,

    /// Knowledge Vault's "{2}, {T}: Exile the top card of your library face down." The library twin
    /// of [`ExileDiscardedWithThis`](Self::ExileDiscardedWithThis): the card joins the ability's own
    /// source-linked pile (CR 400.10a's "exiled with"), face down and with no permission to play it
    /// — [`ZoneEffect::ReturnAllExiledWithThis`](crate::ZoneEffect) is the only way back out.
    ExileTopFaceDownWithThis,

    ExileTopMayPlay {
        count: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        until_next_turn: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        face_down: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        free_while_source: bool,
    },

    /// "`who` mills `count` cards" — from Perpetual Timepiece's own-library mill to Tome Scour's
    /// "target player mills five". Who mills and how many are independent axes ([`PlayerSet`] and
    /// [`Amount`]), so no variant names a recipient.
    Mill {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        who: PlayerSet,
        count: Amount,
    },
}
