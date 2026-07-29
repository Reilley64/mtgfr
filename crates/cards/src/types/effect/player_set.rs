use super::*;

/// Which seat or seats an effect's payload lands on — "you", "target player", "each opponent".
///
/// Magic prints the same payload against a handful of recipients ("you gain 3 life", "target
/// player gains 3 life", "each opponent loses 2 life"), so the recipient is a parameter rather
/// than part of the effect's name. Resolved once by `Game::players_in`, which returns the seats in
/// turn order — CR 118.9's simultaneous life change touches them as one event batch.
///
/// A seat a player *picks* as an effect resolves is not a member of this enum — Priest of
/// Forgotten Gods' "any number of target players" is `chosen_by_controller` on the edict itself,
/// since no resolver can name those seats before someone answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(from = "PlayerSetName")
)]
#[cfg_attr(
    feature = "card-schema",
    derive(schemars::JsonSchema),
    schemars(rename_all = "snake_case")
)]
pub enum PlayerSet {
    /// The ability's controller — the unwritten default on a card that names no one ("you gain 3
    /// life", and the bare "gain 3 life" reminder shorthand).
    #[default]
    You,
    /// The chosen player target (Ominous Harvest's "target player"). Carries
    /// [`TargetSpec::Player`], so the shared targeting machinery picks and legality-checks the
    /// seat and resolution just reads it back.
    TargetPlayer,
    /// The chosen opponent target (Blood Artist's "target opponent") — the same target slot as
    /// [`PlayerSet::TargetPlayer`], but [`TargetSpec::OpponentPlayer`] narrows what may be picked.
    TargetOpponent,
    /// The controller of the chosen *object* target, not of this ability (Swords to Plowshares'
    /// "its controller gains life equal to its power", Nin's "that creature's controller draws X
    /// cards") — CR 109.4, so a stolen creature pays its thief rather than its owner.
    TargetsController,
    /// The *owner* of the chosen object target (Oblation's "its owner shuffles it … then draws two
    /// cards"), which parts ways with [`PlayerSet::TargetsController`] on a stolen permanent.
    TargetsOwner,
    /// Every opponent, in turn order.
    EachOpponent,
    /// Every living player, the controller included (Vandal's Edit's "each player loses 2 life").
    EachPlayer,
    /// Every living player *except* the chosen player target — Death by Dragons' "each player
    /// other than target player creates a 5/5 red Dragon". The target names the one seat left
    /// out, not the recipients, so this carries [`TargetSpec::Player`] like any targeted set.
    EachOtherPlayer,
    /// The attacking player, baked in when the attack trigger is placed (CR 603.10a) — Parasitic
    /// Impetus' "whenever enchanted creature attacks, its controller loses 2 life". `None` only in
    /// an unplaced card template, which never reaches resolution.
    AttackingPlayer {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        player: Option<PlayerId>,
    },
    /// The one player the trigger is about, baked in when it is placed (CR 603.10a) — Howling
    /// Mine's "at the beginning of each player's draw step, **that player** draws an additional
    /// card", Copper Tablet's "each player's upkeep … 1 damage to **that player**", Dingus Egg's
    /// "**that land's** controller". [`PlayerSet::EachPlayer`] would bill the whole table on every
    /// seat's step instead of once.
    TriggeringPlayer {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        player: Option<PlayerId>,
    },
    /// Every opponent *except* the one the trigger is about — Hydra Omnivore's "whenever this
    /// creature deals combat damage to a player, it deals that much damage to **each other
    /// opponent**", where the damaged seat rides in from the trigger's snapshot.
    EachOtherOpponent {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        damaged: Option<PlayerId>,
    },
    /// The controller of the permanent that just entered (Ankh of Mishra's "deals 2 damage to
    /// **that land's** controller") — the permanent itself rides in from the `PermanentEnters`
    /// trigger and the controller is read live, since it is still on the battlefield.
    EnteringPermanentsController {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        permanent: Option<ObjectId>,
    },
    /// The controller of the enchanted creature that just died (Creature Bond's "deals damage
    /// equal to that creature's toughness to **the creature's controller**") — snapshotted at
    /// trigger placement, because by resolution the host is a graveyard card whose `controller_of`
    /// answers its owner (CR 603.10a).
    DyingEnchantedCreaturesController {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        player: Option<PlayerId>,
    },
    /// The player the source just damaged (Hypnotic Specter's "whenever this creature deals damage
    /// to an opponent, **that player** discards a card at random") — the same trigger snapshot
    /// [`PlayerSet::EachOtherOpponent`] reads, naming the damaged seat itself rather than everyone
    /// else.
    DamagedPlayer {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        player: Option<PlayerId>,
    },
    /// The controller of the permanent that dealt the damage (Edric's "whenever a creature you
    /// control deals combat damage to a player, **that creature's controller** may draw a card") —
    /// snapshotted at trigger placement, since Edric's own controller and the damaging creature's
    /// part ways the moment the creature is stolen.
    DamagingPermanentsController {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        player: Option<PlayerId>,
    },
    /// One opponent (Invigorate's "an opponent gains 3 life" — CR 601.2f's alternative-cost
    /// rider names no target, so the choice is the caster's).
    ///
    /// ponytail: picked deterministically as the lowest-seated living opponent rather than
    ///   prompting. Give it a [`PendingChoice`] pause if a card ever makes the pick matter.
    AnOpponent,
}

/// The authorable spelling of a [`PlayerSet`] — every set is a bare string in TOML
/// (`who = "each_opponent"`).
///
/// [`PlayerSet::AttackingPlayer`] carries a slot the placement filler writes into, which makes it a
/// serde *struct* variant that a bare string can't reach. Deserializing through this shadow keeps
/// the TOML surface uniform instead of making one set spell itself `{ attacking_player = {} }`.
#[cfg(feature = "card-dsl")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum PlayerSetName {
    You,
    TargetPlayer,
    TargetOpponent,
    TargetsController,
    TargetsOwner,
    EachOpponent,
    EachPlayer,
    EachOtherPlayer,
    AttackingPlayer,
    TriggeringPlayer,
    EachOtherOpponent,
    EnteringPermanentsController,
    DyingEnchantedCreaturesController,
    DamagedPlayer,
    DamagingPermanentsController,
    AnOpponent,
}

#[cfg(feature = "card-dsl")]
impl From<PlayerSetName> for PlayerSet {
    fn from(name: PlayerSetName) -> Self {
        match name {
            PlayerSetName::You => PlayerSet::You,
            PlayerSetName::TargetPlayer => PlayerSet::TargetPlayer,
            PlayerSetName::TargetOpponent => PlayerSet::TargetOpponent,
            PlayerSetName::TargetsController => PlayerSet::TargetsController,
            PlayerSetName::TargetsOwner => PlayerSet::TargetsOwner,
            PlayerSetName::EachOpponent => PlayerSet::EachOpponent,
            PlayerSetName::EachPlayer => PlayerSet::EachPlayer,
            PlayerSetName::EachOtherPlayer => PlayerSet::EachOtherPlayer,
            PlayerSetName::AttackingPlayer => PlayerSet::AttackingPlayer { player: None },
            PlayerSetName::TriggeringPlayer => PlayerSet::TriggeringPlayer { player: None },
            PlayerSetName::EachOtherOpponent => PlayerSet::EachOtherOpponent { damaged: None },
            PlayerSetName::EnteringPermanentsController => {
                PlayerSet::EnteringPermanentsController { permanent: None }
            }
            PlayerSetName::DyingEnchantedCreaturesController => {
                PlayerSet::DyingEnchantedCreaturesController { player: None }
            }
            PlayerSetName::DamagedPlayer => PlayerSet::DamagedPlayer { player: None },
            PlayerSetName::DamagingPermanentsController => {
                PlayerSet::DamagingPermanentsController { player: None }
            }
            PlayerSetName::AnOpponent => PlayerSet::AnOpponent,
        }
    }
}
