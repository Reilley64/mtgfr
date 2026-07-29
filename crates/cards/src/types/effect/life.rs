use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum LifeEffect {
    /// "You gain 3 life", "target player gains 3 life", "its controller gains life equal to its
    /// power" — one payload against whichever seats [`PlayerSet`] names, defaulting to the
    /// controller. Every gain routes through `Game::life_gain_after_replacements`, so lifegain
    /// replacements and watchers see it (CR 614/CR 118.5).
    Gain {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        who: PlayerSet,
        amount: Amount,
    },

    /// "You lose 2 life", "each opponent loses X life", "target player loses 3 life" — the mirror
    /// of [`LifeEffect::Gain`]. Dina, Soul Steeper is a `Lose` rather than a [`LifeEffect::Drain`]
    /// precisely because she prints no lifegain half; a gain would re-trigger her own "whenever
    /// you gain life" ability into a loop.
    Lose {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        who: PlayerSet,
        amount: Amount,
    },

    /// "Target opponent loses 2 life and you gain 2 life" (Blood Artist), "each opponent loses X
    /// life and you gain X life" (Zulaport Cutthroat) — `who` loses, the controller gains. One
    /// effect rather than a [`LifeEffect::Lose`] beside a [`LifeEffect::Gain`] because the two
    /// halves are simultaneous (CR 118.9) and the gain is sized off what was actually lost.
    Drain {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        who: PlayerSet,
        amount: Amount,
        /// Exsanguinate's "you gain that much life" — the *total* lost across `who`, not each
        /// victim's share. Off by default, which is Zulaport Cutthroat's flat gain.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        sum_gain: bool,
    },

    /// Arbiter of Knollridge: "each player's life total becomes the highest life total among all
    /// players". A set, not a gain or loss of a stated amount (CR 118.5 — it resolves as a change
    /// of the difference), so it names no [`Amount`] and can't be a [`LifeEffect::Gain`].
    EachPlayerBecomesHighest,

    /// "Its owner loses half their life, rounded up" (Personal Incarnation's dies trigger). The
    /// only life loss in the pool billed to the source's *owner* rather than the ability's
    /// controller, and the only one that reads a life total to size itself — so both live in the
    /// variant rather than in an [`Amount`] and a [`PlayerSet`] nothing else would use.
    SourceOwnerLosesHalfTheirLife,
}
