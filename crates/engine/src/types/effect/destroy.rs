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
        /// Berserk's "destroy that creature *if it attacked this turn*" — carried into the
        /// scheduled [`DestroyEffect::ThatCreature`] payload and checked when that fires, never
        /// here: the creature can still be declared an attacker after this effect resolves (a
        /// main-phase Berserk), so a check at scheduling time would read the wrong turn. Only
        /// meaningful with `at`; `false` (the default) destroys unconditionally.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        only_if_it_attacked: bool,
    },

    /// "Destroy *that creature*" over an id baked in when the ability was placed or scheduled,
    /// never a chosen target: the creature a look-back trigger just damaged (Stinkweed Imp), or the
    /// one a `Target { at: Some(_) }` activation threw (Stone Giant). `None` means no context ever
    /// filled it in — nothing to destroy.
    ThatCreature {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        creature: Option<ObjectId>,
        /// Berserk's rider, carried down from [`DestroyEffect::Target::only_if_it_attacked`] when
        /// the delayed ability was scheduled: destroy only if `creature` was declared an attacker
        /// this turn ([`Permanent::attacked_this_turn`]). `false` for every other filler.
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        only_if_it_attacked: bool,
    },
}
