use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)
)]
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
        /// `Some(step)` postpones the destruction to a CR 603.7 delayed triggered ability at that
        /// step, over the object this ability *already* chose (Stone Giant's "Destroy that
        /// creature at the beginning of the next end step") — the chosen id is baked into a
        /// [`DestroyEffect::ThatCreature`] payload at resolution, so nothing is re-targeted when
        /// it fires. `None` (default) destroys as this effect resolves. Same schedule-or-do-it-now
        /// shape as [`ZoneEffect::FlickerTarget`]'s `return_at`.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        at: Option<Step>,
        /// Berserk's "destroy that creature *if it attacked this turn*" and Nettling Imp's
        /// mirror-image "*if it didn't attack this turn*" — carried into the scheduled
        /// [`DestroyEffect::ThatCreature`] payload and checked when that fires, never here: the
        /// creature can still be declared an attacker after this effect resolves (a main-phase
        /// Berserk, an Imp activated before attackers), so a check at scheduling time would read
        /// the wrong turn. Only meaningful with `at`; [`AttackRider::Ignore`] (the default)
        /// destroys unconditionally.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        attack_rider: AttackRider,
    },

    /// "Destroy *that creature*" over an id baked in when the ability was placed or scheduled,
    /// never a chosen target: the creature a look-back trigger just damaged (Stinkweed Imp), or the
    /// one a `Target { at: Some(_) }` activation threw (Stone Giant). `None` means no context ever
    /// filled it in — nothing to destroy.
    ThatCreature {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        creature: Option<ObjectId>,
        /// The did-it-attack rider, carried down from [`DestroyEffect::Target`]'s own
        /// `attack_rider` when the delayed ability was scheduled: it reads
        /// [`Permanent::attacked_this_turn`] as this fires. [`AttackRider::Ignore`] for every
        /// other filler.
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        attack_rider: AttackRider,
        /// "Destroy that creature **at end of combat**" (Cockatrice): `Some(step)` postpones once
        /// more, re-scheduling this same already-filled payload as a CR 603.7 delayed ability at
        /// that step — the id is baked in, so nothing is re-chosen when it fires. `None` (the
        /// default, and what the re-scheduled copy carries) destroys as this resolves, which is
        /// Stinkweed Imp's shape. Same schedule-or-do-it-now knob as [`Self::Target::at`].
        #[cfg_attr(feature = "card-dsl", serde(default))]
        at: Option<Step>,
    },
}

/// Whether a scheduled destruction asks about the creature's attack this turn (CR 508.1) — the
/// two halves of one question, so one value rather than a pair of bools that must never both be
/// set. Read only when the delayed ability actually fires, never when it is scheduled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum AttackRider {
    /// Doesn't ask (Stone Giant's thrown creature, Stinkweed Imp's, Cockatrice's) — destroy.
    #[default]
    Ignore,
    /// "…if it attacked this turn" (Berserk).
    OnlyIfItAttacked,
    /// "…if it didn't attack this turn" (Nettling Imp).
    OnlyIfItDidnt,
}
