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
        /// `Some(step)` postpones the sweep to a CR 603.7 delayed triggered ability at that step
        /// (Siren's Call's "at the beginning of the next end step, destroy all …"). Unlike
        /// [`DestroyEffect::Target`]'s `at`, nothing is baked in when it is scheduled: the same
        /// `filter` re-runs over the battlefield when the delayed ability fires, which is the only
        /// way `did_not_attack_this_turn` can read an attack declared after the scheduling.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        at: Option<Step>,
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
        /// "At the beginning of the next end step, **if that creature was destroyed this way**,
        /// put a +1/+1 counter on the first creature" (Infinite Authority): scheduled as a second
        /// CR 603.7 delayed ability *only* on the branch where this destruction actually put the
        /// creature in a graveyard, which is what makes the intervening-if free — a regenerated,
        /// indestructible or already-gone creature never reaches it. Carried through `at`'s
        /// re-schedule so an end-of-combat destroy still gets its end-step payoff. Runs against
        /// this ability's own source, so `this` is the Aura's host.
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::opt_static_effect")
        )]
        #[cfg_attr(feature = "card-schema", schemars(with = "Option<Effect>"))]
        then: Option<&'static Effect>,
    },

    /// "Destroy all creatures that were blocked by target Wall **this turn**" (Glyph of
    /// Reincarnation, and Glyph of Doom's delayed form). Neither a board scan like
    /// [`Self::All`] nor a single chosen victim like [`Self::Target`]: the victims are read out
    /// of the Glyph cycle's turn-scoped ledger (`CombatExtras::blocked_this_turn`) through
    /// `Game::blocked_by_this_turn`, keyed by the *targeted blocker*. It has to be that ledger
    /// rather than the live block list, because both cards are cast after the combat the block
    /// happened in — CR 509.1h's "still blocked" fact dies at end of combat, and these want the
    /// rest of the turn.
    BlockedByTarget {
        /// The blocker whose ledger is read — "target Wall".
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        cant_be_regenerated: bool,
        /// `Some(step)` postpones the sweep to a CR 603.7 delayed triggered ability at that step
        /// (Glyph of Doom's "at this turn's next end of combat"). Only the *blocker* is baked in
        /// when it is scheduled (into `blocker` below) — the ledger itself is re-read when the
        /// delayed ability fires, so a block declared after this resolved is still swept, the
        /// same late-reading shape [`Self::All`]'s `at` has.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        at: Option<Step>,
        /// Glyph of Reincarnation's "For each creature that died this way, put a creature card
        /// from the graveyard of the player who controlled that creature the last time it became
        /// blocked by that Wall onto the battlefield under its owner's control." `false` (the
        /// default) is Glyph of Doom — destroy and stop.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        reincarnate: bool,
        /// The blocker this sweep already chose, baked in when `at` scheduled the delayed
        /// ability. `None` everywhere else, where the resolved `target` is read instead.
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        blocker: Option<ObjectId>,
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
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum AttackRider {
    /// Doesn't ask (Stone Giant's thrown creature, Stinkweed Imp's, Cockatrice's) — destroy.
    #[default]
    Ignore,
    /// "…if it attacked this turn" (Berserk).
    OnlyIfItAttacked,
    /// "…if it didn't attack this turn" (Nettling Imp).
    OnlyIfItDidnt,
}
