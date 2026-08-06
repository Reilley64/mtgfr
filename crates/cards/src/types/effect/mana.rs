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
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum ManaEffect {
    Add {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::mana_batch")
        )]
        #[cfg_attr(feature = "card-schema", schemars(with = "Vec<crate::Mana>"))]
        mana: ManaPool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        identity: u8,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        opponent_colors: u8,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one_amount"))]
        repeat: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        restriction: Option<SpendRestriction>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        single_color: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        track_provenance: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        persist_until_end_of_turn: bool,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        recipient: Option<PlayerId>,
    },

    /// "That player loses all unspent mana" — the 1993 mana-burn era's leavings, still printed on
    /// Mana Short, Power Sink, and Drain Power. Empties the enclosing `Sequence`'s shared target
    /// player's pool outright, persistent "until end of turn" credits included (CR 500.4's
    /// exception is about *boundaries*; a card that says "all" means all). No target of its own —
    /// it always follows a step that already named the player.
    ///
    /// `to_you = true` is Drain Power's "and you add the mana lost this way": every credit lands
    /// in this ability's controller's pool instead of evaporating, kind for kind, so a dual land's
    /// either-credit arrives as flexible as it left.
    LoseAllUnspent {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        to_you: bool,
    },

    /// "Target player activates a mana ability of each land they control" (Drain Power). Walks
    /// that player's untapped lands and taps each for mana on their behalf — the same
    /// [`Game::tap_for_mana`] path their own click would take, so every land-tap watch
    /// ([`Effect::Static(StaticEffect::TappedForManaBonus)`], Manabarbs) fires exactly as it
    /// would have. A land with no mana ability (Maze of Ith) is skipped, as is one already tapped.
    ///
    /// ponytail: takes each land's default credit rather than offering a per-land pick, so a land
    /// with two competing mana abilities gets the first one. Every land in the 2ed pool has one
    /// mana ability, and a dual's either-credit stays undecided in the pool anyway. Raise a
    /// per-land pending choice if a card ever makes the pick matter.
    TargetPlayerTapsLandsForMana,

    /// Quarum Trench Gnomes' "{T}: If target Plains is tapped for mana, it produces colorless mana
    /// instead of white mana. (This effect lasts indefinitely.)"
    ///
    /// A durationless rewrite of what the land's *free tap* credits (CR 605.1a), registered as an
    /// indefinite modifier on the land and read back at
    /// [`Game::land_mana_credit`](crate::Game) — the one choke every read of a land's `produces`
    /// goes through, so the tap intent and the auto-tap planner both see it. Not a type change: the
    /// Plains stays a Plains, keeps every other ability it has, and dies to Flashfires still.
    ///
    /// `color` is the color that becomes colorless; a land that doesn't produce it is unaffected.
    TargetLandProducesColorlessInsteadOf {
        color: Color,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target: TargetSpec,
    },
}
