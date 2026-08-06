use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum ExileEffect {
    All {
        filter: PermanentFilter,
    },

    AllGraveyards,

    Graveyard,

    /// "Exile Stangg Twin when Stangg leaves the battlefield" — the exile half of
    /// [`SacrificeEffect::LinkedTwin`](crate::SacrificeEffect::LinkedTwin): exile the permanent
    /// paired with the source (`token.link_as_twin`), found by scanning the battlefield for the
    /// permanent that points back at it. A token partner ceases to exist instead (CR 111.7).
    LinkedTwin,

    Object {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        object: Option<ObjectId>,
    },

    /// "…then exile this artifact and those creature cards" (Sword of the Ages) — exile one card
    /// that paid this ability's [`SacrificeCost::ThisAndAnyNumber`](crate::SacrificeCost)
    /// activation cost, wherever that card is now (the graveyard, since a sacrifice happened as
    /// the cost was paid). Authored payload-free (`{ type = "exile", mode = "sacrificed_card" }`);
    /// [`contextualize_exiled_sacrifices`](crate::contextualize_exiled_sacrifices) expands the one
    /// authored leaf into a [`Effect::Sequence`] of these, one per card actually sacrificed, as
    /// the ability goes on the stack. Distinct from [`Object`](Self::Object), which only exiles a
    /// permanent still on the battlefield.
    SacrificedCard {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        object: Option<ObjectId>,
    },

    /// "When this creature dies, exile it" (Cyclopean Mummy) — exile the ability's own source
    /// wherever its card is *now*, which for a dies trigger is the graveyard card the battlefield
    /// permanent became (CR 603.6c: the trigger looks back at the permanent, but "it" is the new
    /// object). The exile sibling of [`SacrificeEffect::Source`](crate::SacrificeEffect::Source);
    /// unlike [`ExileEffect::Object`] it needs no engine-filled id, so a card can author it.
    /// A token source has no card left to exile and nothing happens.
    Source,

    Target {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
    },

    TargetMintingIllusionOnLeave {
        target: TargetSpec,
    },

    UntilSourceLeaves {
        target: TargetSpec,
    },
}
