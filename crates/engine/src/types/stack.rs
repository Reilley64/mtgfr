use super::*;

/// A player's requested action. Fed to [`Game::submit`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    /// Move a spell from hand onto the stack. `x` is the value chosen for a `{X}` cost
    /// (CR 601.2b); it is 0 (and ignored) for a spell with no `{X}`.
    ///
    /// A **non-modal** spell uses `target` for its single target (`None` if it takes none) and
    /// leaves `modes` empty. A **modal** spell (CR 700.2) leaves `target` `None` and lists its
    /// chosen modes in `modes` as `(printed-mode index, that mode's target)` — exactly
    /// `modal_choose` distinct in-range modes, each with its own target where the mode needs one.
    Cast {
        player: PlayerId,
        object: ObjectId,
        target: Option<Target>,
        x: u32,
        modes: Vec<(usize, Option<Target>)>,
        /// Hand cards chosen to pay the spell's additional discard cost (CR 601.2f;
        /// [`Cost::additional`]) — empty for a spell with none.
        discard_cost: Vec<ObjectId>,
        /// Graveyard cards named to pay a delve or escape graveyard-exile cost, mirroring
        /// `discard_cost`'s shape: delve (CR 702.66) exiles a player-chosen number of the
        /// caster's own graveyard cards to reduce the generic cost by {1} each; escape (CR
        /// 702.19) requires exiling exactly [`CardDef::escape`]'s `exile` *other* graveyard
        /// cards as a fixed additional cost. Empty for a spell with neither.
        graveyard_exile: Vec<ObjectId>,
        /// Permanents chosen to pay the spell's additional sacrifice cost
        /// ([`AdditionalCost::sacrifice`]) — empty for a spell with none, or to decline an
        /// optional one.
        sacrifice_cost: Vec<ObjectId>,
        /// Whether the caster paid the spell's kicker cost ([`AdditionalCost::kicker`] — CR
        /// 702.33d), folding it into the mana paid alongside the printed cost. `false` (the
        /// default — decline) for a spell with no kicker. Recorded on the resulting
        /// [`Spell::kicked`].
        kicked: bool,
        /// Whether the caster paid the spell's buyback cost ([`AdditionalCost::buyback`] — CR
        /// 702.27c), folding it into the mana paid alongside the printed cost, mirroring
        /// `kicked`'s own opt-in shape. `false` (the default — decline) for a spell with no
        /// buyback. Recorded on the resulting [`Spell::bought_back`].
        bought_back: bool,
        /// Whether the caster is casting the spell for its evoke cost (CR 702.74a —
        /// [`CardDef::evoke`]), charged instead of the printed cost. `false` (the default) for a
        /// spell with no evoke, or to cast it normally. Recorded on the resulting
        /// [`Spell::evoked`]; the resulting permanent is sacrificed the instant it enters (see
        /// [`Permanent::evoked`]).
        evoked: bool,
        /// The caster's declared Strive target count ([`AdditionalCost::strive`] — CR 601.2c/
        /// 601.2f/702.42), settled before the stack since Strive's total cost depends on it (see
        /// [`AdditionalCost::strive`]'s own doc). 0 (the default) for a spell with no Strive, or
        /// "choose zero targets, pay the base cost." Folded into the mana paid by
        /// [`Game::cast_cost`] and recorded on the resulting [`Spell::strive_count`], read back by
        /// [`TargetCount::strive_scaled`]'s cast-time target-count substitution.
        strive_count: u8,
        /// How many times the caster paid the spell's Replicate cost ([`AdditionalCost::replicate`]
        /// — CR 702.108), settled before the stack for the same reason as `strive_count` above
        /// (its total cost depends on the declared count). 0 (the default) for a spell with no
        /// Replicate, or "pay it zero times." Folded into the mana paid by [`Game::cast_cost`] and
        /// recorded on the resulting [`Spell::replicate_count`], read by the cast choke to mint
        /// that many copies (CR 702.108b).
        replicate_count: u8,
    },
    /// Play a land from hand (a special action — no stack, once per turn).
    PlayLand { player: PlayerId, object: ObjectId },
    /// Activate a hand card's Cycling ability (CR 702.29a — "{N}, Discard this card: Draw a
    /// card."), functioning from the hand rather than a permanent's activated ability.
    Cycle { player: PlayerId, card: ObjectId },
    /// Activate a hand card's [`CardDef::hand_ability`] (CR 113.6/602.5e — a hand-activated,
    /// discard-this-card ability with an authored payload, e.g. Magma Opus's "{U/R}{U/R},
    /// Discard this card: Create a Treasure token."). The general sibling of [`Self::Cycle`]
    /// for a card whose from-hand ability isn't cycling's fixed draw-1. See
    /// [`Game::activate_hand_ability`].
    ActivateHandAbility { player: PlayerId, card: ObjectId },
    /// Suspend a hand card (CR 702.62): pay its [`CardDef::suspend`] cost and exile it with N time
    /// counters, instead of casting it. A special action from the hand, like [`Self::Cycle`].
    Suspend { player: PlayerId, card: ObjectId },
    /// Encore a graveyard card (CR 702.140): pay its [`CardDef::encore`] mana cost and exile it
    /// from the graveyard to create a must-attack haste token copy per opponent. A sorcery-speed
    /// special action from the graveyard, like [`Self::Suspend`]. See [`Game::encore`].
    Encore { player: PlayerId, card: ObjectId },
    /// Turn a face-down manifested permanent face up (CR 701.34e): pay its hidden creature card's
    /// mana cost to reveal it. A special action (no stack), like [`Self::Suspend`].
    TurnFaceUp {
        player: PlayerId,
        permanent: ObjectId,
    },
    /// Cast a copy of a prepared permanent's back-face spell (soc/sos prepare DFCs). `source` is
    /// the prepared permanent (its [`CardDef::back`] is the spell); `target` is the back face's
    /// chosen target (e.g. Pack a Punch's creature); `x` is the back face's chosen `{X}` (0 for a
    /// non-`{X}` back face, same CR 601.2b default `Intent::Cast`'s `x` uses — Braingeyser).
    /// Casting pays the back face's mana cost and unprepares the source. See
    /// [`Game::cast_prepared`].
    CastPrepared {
        player: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    },
    /// Cast the adventure half of an adventure card from hand (CR 715 — soc/sos). `source` is the
    /// card in `player`'s hand (its [`CardDef::adventure`] is the instant/sorcery spell); `target`
    /// is the adventure spell's chosen target (e.g. Petty Theft's nonland permanent); `x` is its
    /// chosen `{X}` (0 for a non-`{X}` adventure — same CR 601.2b default `Intent::Cast` uses). On
    /// resolution the card is exiled "on an adventure" and its owner may cast the creature half
    /// from exile later. See [`Game::cast_adventure`].
    CastAdventure {
        player: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    },
    /// Cast a permanent (enchantment) creature card for its bestow cost (CR 702.103 — Eidolon of
    /// Countless Battles). `object` is the card in `player`'s hand (its [`CardDef::bestow`] is the
    /// alternative cost); `target` is the creature it will enchant (bestow grants "enchant
    /// creature", CR 702.103c). Casting pays the bestow cost and puts the card on the stack as a
    /// bestowed Aura spell; on resolution it enters attached to `target`. See [`Game::cast_bestow`].
    CastBestow {
        player: PlayerId,
        object: ObjectId,
        target: Option<Target>,
    },
    /// Cast a hand card face down as a 2/2 creature for {3} (CR 702.37b — morph). `card` is the
    /// card in `player`'s hand (its [`CardDef::morph`] makes it eligible). Casting pays a flat
    /// generic {3} and puts it on the stack as a face-down creature spell; on resolution it
    /// enters as a face-down 2/2 permanent (CR 708.2). See [`Game::cast_face_down`].
    CastFaceDown { player: PlayerId, card: ObjectId },
    /// Tap a land for one mana (a mana ability — doesn't use the stack).
    TapForMana { player: PlayerId, object: ObjectId },
    /// Pay 1 life to add {C} under a Channel-style temporary grant (a mana ability — doesn't use
    /// the stack). Legal only while [`Player::channel_colorless_mana_this_turn`] holds (Yavimaya
    /// Bloomsage's Channel back face). See [`Game::channel_colorless_mana`](crate::Game::channel_colorless_mana).
    ChannelColorlessMana { player: PlayerId },
    /// Activate a permanent's activated ability, paying its cost. `sacrifice` names the
    /// creature(s) to sacrifice for a "Sacrifice N creature(s)" cost (ignored for other costs,
    /// and must have exactly the cost's `count` entries); a "Sacrifice this" cost sacrifices
    /// `object` itself.
    ActivateAbility {
        player: PlayerId,
        object: ObjectId,
        ability_index: usize,
        target: Option<Target>,
        sacrifice: Vec<ObjectId>,
        /// The chosen `{X}` for an activation cost that contains `{X}` (CR 107.3 — paid once per
        /// `{X}` symbol); `0` for an ability with no `{X}` in its cost.
        x: u32,
    },
    /// The active player declares attackers, each attacking a chosen defending player
    /// (CR 508.1 — a creature picks its own defender; you may split across opponents).
    DeclareAttackers {
        player: PlayerId,
        /// (attacking creature, the player it attacks).
        attackers: Vec<(ObjectId, PlayerId)>,
    },
    /// The defending player declares blocks as (blocker, attacker) pairs.
    DeclareBlockers {
        player: PlayerId,
        blocks: Vec<(ObjectId, ObjectId)>,
    },
    /// Answer a pending ordering choice with a permutation of the offered items.
    ChooseOrder { player: PlayerId, order: Vec<usize> },
    /// Answer a [`PendingChoice::ChooseTarget`] with the chosen target(s).
    ChooseTargets {
        player: PlayerId,
        targets: Vec<Target>,
    },
    /// Answer a [`PendingChoice::ChooseTargetPlayers`]: `players` are the chosen "any number of
    /// target players" (CR 601.2c/608.2b — a subset, possibly empty, of the offered `legal` set).
    ChooseTargetPlayers {
        player: PlayerId,
        players: Vec<PlayerId>,
    },
    /// Answer a [`PendingChoice::MayYesNo`]: accept or decline an optional trigger.
    AnswerMay { player: PlayerId, yes: bool },
    /// Answer a [`PendingChoice::DeclineUntap`] (CR 502.2 — Rubinia Soulsinger's "you may choose
    /// not to untap"): `keep_tapped` are the offered permanents the player leaves tapped; every
    /// other offered permanent untaps.
    DeclineUntap {
        player: PlayerId,
        keep_tapped: Vec<ObjectId>,
    },
    /// Answer a [`PendingChoice::PayCost`]: pay the cost (getting the effect) or decline.
    PayOptionalCost { player: PlayerId, pay: bool },
    /// Answer a [`PendingChoice::PayCost`] whose `cost` carries a chosen `{X}` (CR 107.3 — Decree
    /// of Justice's cycling rider "you may pay {X}. If you do, create X 1/1 white Soldier
    /// creature tokens."): pay `cost.with_x(x)` and thread `x` onto the placed ability so its own
    /// `Amount::X` reads it, or decline (`x` ignored). A distinct variant from
    /// [`Self::PayOptionalCost`] rather than widening its `x`-less shape — no other `PayCost`
    /// answerer in the pool needs an `{X}`, and this keeps every existing plain pay/decline call
    /// site untouched. See [`Game::pay_optional_cost_with_x`](crate::Game::pay_optional_cost_with_x).
    PayOptionalCostX { player: PlayerId, pay: bool, x: u32 },
    /// Answer a [`PendingChoice::AssignCombatDamage`] with `(blocker, amount)` assignments.
    AssignDamage {
        player: PlayerId,
        assignment: Vec<(ObjectId, i32)>,
    },
    /// Answer a [`PendingChoice::DivideSpellDamage`] with `(target, amount)` assignments (CR
    /// 601.2d). Distinct from [`Self::AssignDamage`] (combat, always blockers): a divided-damage
    /// spell's "any number of targets" may include a *player*, which [`Self::AssignDamage`]'s
    /// `ObjectId` wire can't name — so this keys by [`Target`].
    DivideSpellDamage {
        player: PlayerId,
        assignment: Vec<(Target, i32)>,
    },
    /// Answer a [`PendingChoice::ArrangeTop`] (scry/surveil): `top` cards go back on top of the
    /// library in this order, `bottom` cards to the library bottom (scry) or graveyard (surveil).
    /// Their union must be a partition of the shown cards.
    ArrangeTop {
        player: PlayerId,
        top: Vec<ObjectId>,
        bottom: Vec<ObjectId>,
    },
    /// Answer a [`PendingChoice::SelectFromTop`]: `cards` are the looked-at cards to select into
    /// the choice's destination (up to `up_to`, each matching the choice's filter); every
    /// non-selected looked-at card goes to the choice's rest zone. An empty `cards` declines.
    SelectFromTop {
        player: PlayerId,
        cards: Vec<ObjectId>,
    },
    /// Answer a [`PendingChoice::DistributeTop`]: `to_hand`/`to_bottom`/`to_exile_may_play` each
    /// list the looked-at cards routed to that slot — disjoint, and each must match the choice's
    /// slot size exactly.
    DistributeTop {
        player: PlayerId,
        to_hand: Vec<ObjectId>,
        to_bottom: Vec<ObjectId>,
        to_exile_may_play: Vec<ObjectId>,
    },
    /// Answer a [`PendingChoice::ShuffleFromGraveyard`]: `cards` are the graveyard cards this
    /// player shuffles into their library (any subset of the offered `candidates`, including
    /// none or all).
    ShuffleFromGraveyard {
        player: PlayerId,
        cards: Vec<ObjectId>,
    },
    /// Answer a [`PendingChoice::SearchLibrary`]: `choice` is the found card (one of the offered
    /// matches), or `None` to fail to find. Either way the library is then shuffled.
    SearchLibrary {
        player: PlayerId,
        choice: Option<ObjectId>,
    },
    /// Answer a [`PendingChoice::SacrificeEdict`]: `sacrifices` are the permanents this player
    /// gives up — one of their `options`, or (with `keep_one`) all but the one they keep.
    ChooseSacrifices {
        player: PlayerId,
        sacrifices: Vec<ObjectId>,
    },
    /// Answer a [`PendingChoice::DiscardToHandSize`]: the `cards` this player discards from hand
    /// at cleanup to reach the hand-size limit (exactly `count` of them, chosen by the player).
    Discard {
        player: PlayerId,
        cards: Vec<ObjectId>,
    },
    /// Answer a [`PendingChoice::PutLandFromHand`]: `choice` is the hand land put onto the
    /// battlefield (one of the offered candidates), or `None` to decline.
    PutLandFromHand {
        player: PlayerId,
        choice: Option<ObjectId>,
    },
    /// Answer a [`PendingChoice::CastCreatureFaceDown`]: `choice` is the hand creature card cast
    /// face down as a 2/2 (one of the offered candidates), or `None` to decline (Illusionary Mask).
    CastCreatureFaceDown {
        player: PlayerId,
        choice: Option<ObjectId>,
    },
    /// Answer a [`PendingChoice::SacrificeUnlessReturnLand`]: `land` is the offered non-Lair land
    /// returned to its owner's hand (keeping the source), or `None` to decline and sacrifice the
    /// source instead.
    ReturnLandOrSacrifice {
        player: PlayerId,
        land: Option<ObjectId>,
    },
    /// Answer a [`PendingChoice::ChooseExiledWithCard`]: `choice` is the exiled-with card put
    /// into its owner's graveyard (one of the offered candidates), or `None` to decline.
    ChooseExiledWithCard {
        player: PlayerId,
        choice: Option<ObjectId>,
    },
    /// Answer a [`PendingChoice::ChooseExiledWithCardToCast`]: `choice` is the exiled-with card
    /// granted the free-cast permission (one of the offered candidates), or `None` to decline.
    ChooseExiledWithCardToCast {
        player: PlayerId,
        choice: Option<ObjectId>,
    },
    /// Answer a [`PendingChoice::ChooseExiledDigToCastFree`]: `choice` is the just-exiled card
    /// granted the free-cast permission (one of the offered candidates), or `None` to decline.
    ChooseExiledDigToCastFree {
        player: PlayerId,
        choice: Option<ObjectId>,
    },
    /// Answer a [`PendingChoice::OpponentChoosesPile`] (Abstract Performance): `pile` is `0` for
    /// the first pile or `1` for the second — the pile this opponent puts into the controller's
    /// graveyard.
    ChooseOpponentPile { player: PlayerId, pile: u8 },
    /// Answer a [`PendingChoice::RevealedCardToBattlefieldOrHand`]: `choice = Some(card)` puts
    /// the revealed card onto the battlefield untapped (`card` must match the pending choice's
    /// own card); `None` puts it into hand instead (Songbirds' Blessing's "You may put that card
    /// onto the battlefield. If you don't, put it into your hand").
    RevealedCardToBattlefieldOrHand {
        player: PlayerId,
        choice: Option<ObjectId>,
    },
    /// Answer a [`PendingChoice::ChooseMode`]: `mode` is the index of the chosen mode of a
    /// "choose one" triggered ability ([`Effect::ChooseOne`]).
    ChooseMode { player: PlayerId, mode: usize },
    /// Answer a [`PendingChoice::ChooseTriggerModes`]: `modes` are `(printed-mode index, that
    /// mode's Player target)` pairs — the modal-*triggered*-ability twin of [`Intent::Cast`]'s
    /// `modes` field, same shape. Empty declines the whole "may" ability.
    ChooseTriggerModes {
        player: PlayerId,
        modes: Vec<(usize, Option<Target>)>,
    },
    /// Answer a [`PendingChoice::ChooseManaColor`] (CR 106.4's "add N mana of any one color"):
    /// `color` is the one color all of the pending amount is added as.
    ChooseManaColor { player: PlayerId, color: Color },
    /// Answer a [`PendingChoice::ChooseCreatureType`] (CR 614.12/700.9-style "as ~ enters,
    /// choose a creature type" — Patchwork Banner): `subtype` names the chosen creature type,
    /// one of the pending choice's offered `options`.
    ChooseCreatureType { player: PlayerId, subtype: String },
    /// Answer a [`PendingChoice::ChooseColor`] (CR 614.12/700.9-style "as ~ enters, choose a
    /// color" — Flickering Ward): `color` is the chosen color, stored on the source permanent.
    ChooseColor { player: PlayerId, color: Color },
    /// Answer a [`PendingChoice::ChooseAttachHost`]: `host` is the chosen creature the deployed
    /// Aura or Equipment attaches to (must be one of the choice's own `candidates`). `None` only
    /// legal when the choice is `optional` (a deployed Equipment declining to attach, CR
    /// 301.5c) — a mandatory Aura host (CR 303.4f) rejects `None`.
    ChooseAttachHost {
        player: PlayerId,
        host: Option<ObjectId>,
    },
    /// Answer a [`PendingChoice::ChooseCopyTarget`]: `copy = Some(creature)` has the entering
    /// permanent enter as a copy of that creature (one of the choice's `candidates`); `None`
    /// declines the "you may" and it enters as its printed self (CR 706/707.2 — Altered Ego,
    /// Cursed Mirror).
    ChooseCopyTarget {
        player: PlayerId,
        copy: Option<ObjectId>,
    },
    /// Answer a [`PendingChoice::ChooseCounteredSpellDestination`] (Hinder's CR 701.5b rider):
    /// `top` puts the countered card on top of its owner's library, `false` on the bottom.
    ChooseTopOrBottom { player: PlayerId, top: bool },
    /// Decline to act; pass priority.
    PassPriority { player: PlayerId },
    /// Leave the game (CR 104.3a). Legal at any time, with or without priority, and even while the
    /// engine is waiting on this player's own choice — a seat that has quit must never be something
    /// the other three wait on.
    Concede { player: PlayerId },
    /// Take one of the engine's stored legal actions by its `id` (see [`Game::legal_actions`]),
    /// carrying the chosen parameters the stored [`MeaningfulAction`] can't know. Resolution
    /// looks `id` up in the stored list and dispatches to the very same handler the matching
    /// concrete intent would (`Cast`/`PlayLand`/`ActivateAbility`/`DeclareAttackers`/
    /// `DeclareBlockers`) — this is an additional entry point, not a replacement. Which
    /// parameters matter depends on the resolved action's kind: `target`/`x`/`modes` for a cast,
    /// `target`/`sacrifice` for an activation, `attackers`/`blocks` for a combat declaration; the
    /// rest are ignored. An unknown `id`, or one whose stored `player` isn't the submitter,
    /// rejects with [`Reject::UnknownAction`].
    TakeAction {
        player: PlayerId,
        id: u64,
        target: Option<Target>,
        x: u32,
        modes: Vec<(usize, Option<Target>)>,
        sacrifice: Vec<ObjectId>,
        /// Hand cards paying an additional discard cost (CR 601.2f); empty when the cast has none.
        discard_cost: Vec<ObjectId>,
        /// Graveyard cards paying delve or escape exile (CR 702.66 / 702.19); empty when neither.
        graveyard_exile: Vec<ObjectId>,
        attackers: Vec<(ObjectId, PlayerId)>,
        blocks: Vec<(ObjectId, ObjectId)>,
    },
}

impl Intent {
    /// Every object id this intent references (for an up-front existence check).
    pub(crate) fn object_ids(&self) -> Vec<ObjectId> {
        use std::iter::once;
        match self {
            Intent::Cast {
                object,
                target,
                modes,
                discard_cost,
                graveyard_exile,
                sacrifice_cost,
                ..
            } => once(*object)
                .chain(target.and_then(Target::object_id))
                .chain(
                    modes
                        .iter()
                        .filter_map(|(_, t)| t.and_then(Target::object_id)),
                )
                .chain(discard_cost.iter().copied())
                .chain(graveyard_exile.iter().copied())
                .chain(sacrifice_cost.iter().copied())
                .collect(),
            Intent::ActivateAbility {
                object,
                target,
                sacrifice,
                ..
            } => once(*object)
                .chain(target.and_then(Target::object_id))
                .chain(sacrifice.iter().copied())
                .collect(),
            Intent::PlayLand { object, .. } | Intent::TapForMana { object, .. } => vec![*object],
            Intent::ChannelColorlessMana { .. } => Vec::new(),
            Intent::Cycle { card, .. }
            | Intent::ActivateHandAbility { card, .. }
            | Intent::Suspend { card, .. }
            | Intent::Encore { card, .. }
            | Intent::CastFaceDown { card, .. } => vec![*card],
            Intent::TurnFaceUp { permanent, .. } => vec![*permanent],
            Intent::CastPrepared { source, target, .. }
            | Intent::CastAdventure { source, target, .. } => once(*source)
                .chain(target.and_then(Target::object_id))
                .collect(),
            Intent::CastBestow { object, target, .. } => once(*object)
                .chain(target.and_then(Target::object_id))
                .collect(),
            Intent::DeclareAttackers { attackers, .. } => {
                attackers.iter().map(|&(a, _)| a).collect()
            }
            Intent::DeclareBlockers { blocks, .. } => {
                blocks.iter().flat_map(|&(b, a)| [b, a]).collect()
            }
            Intent::ChooseTargets { targets, .. } => {
                targets.iter().filter_map(|t| t.object_id()).collect()
            }
            Intent::AssignDamage { assignment, .. } => assignment.iter().map(|&(b, _)| b).collect(),
            Intent::DivideSpellDamage { assignment, .. } => assignment
                .iter()
                .filter_map(|&(t, _)| t.object_id())
                .collect(),
            Intent::ArrangeTop { top, bottom, .. } => top.iter().chain(bottom).copied().collect(),
            Intent::SelectFromTop { cards, .. } | Intent::ShuffleFromGraveyard { cards, .. } => {
                cards.clone()
            }
            Intent::DistributeTop {
                to_hand,
                to_bottom,
                to_exile_may_play,
                ..
            } => to_hand
                .iter()
                .chain(to_bottom)
                .chain(to_exile_may_play)
                .copied()
                .collect(),
            Intent::SearchLibrary { choice, .. }
            | Intent::PutLandFromHand { choice, .. }
            | Intent::CastCreatureFaceDown { choice, .. }
            | Intent::ChooseExiledWithCard { choice, .. }
            | Intent::ChooseExiledWithCardToCast { choice, .. }
            | Intent::ChooseExiledDigToCastFree { choice, .. }
            | Intent::RevealedCardToBattlefieldOrHand { choice, .. } => {
                choice.iter().copied().collect()
            }
            Intent::ReturnLandOrSacrifice { land, .. } => land.iter().copied().collect(),
            Intent::ChooseSacrifices { sacrifices, .. } => sacrifices.clone(),
            Intent::Discard { cards, .. } => cards.clone(),
            Intent::DeclineUntap { keep_tapped, .. } => keep_tapped.clone(),
            Intent::ChooseAttachHost { host, .. } => host.iter().copied().collect(),
            Intent::ChooseCopyTarget { copy, .. } => copy.iter().copied().collect(),
            // The carried params reference real object ids (the action's own object is looked
            // up from the stored list); range-check them so a bad id can't panic the engine.
            Intent::TakeAction {
                target,
                modes,
                sacrifice,
                discard_cost,
                graveyard_exile,
                attackers,
                blocks,
                ..
            } => target
                .and_then(Target::object_id)
                .into_iter()
                .chain(
                    modes
                        .iter()
                        .filter_map(|(_, t)| t.and_then(Target::object_id)),
                )
                .chain(sacrifice.iter().copied())
                .chain(discard_cost.iter().copied())
                .chain(graveyard_exile.iter().copied())
                .chain(attackers.iter().map(|&(a, _)| a))
                .chain(blocks.iter().flat_map(|&(b, a)| [b, a]))
                .collect(),
            Intent::ChooseTriggerModes { modes, .. } => modes
                .iter()
                .filter_map(|(_, t)| t.and_then(Target::object_id))
                .collect(),
            Intent::ChooseOrder { .. }
            | Intent::ChooseTargetPlayers { .. }
            | Intent::AnswerMay { .. }
            | Intent::PayOptionalCost { .. }
            | Intent::PayOptionalCostX { .. }
            | Intent::ChooseMode { .. }
            | Intent::ChooseOpponentPile { .. }
            | Intent::ChooseManaColor { .. }
            | Intent::ChooseCreatureType { .. }
            | Intent::ChooseColor { .. }
            | Intent::ChooseTopOrBottom { .. }
            | Intent::PassPriority { .. }
            | Intent::Concede { .. } => Vec::new(),
        }
    }

    /// The player taking this action (every intent names its actor).
    pub fn actor(&self) -> PlayerId {
        match self {
            Intent::Cast { player, .. }
            | Intent::PlayLand { player, .. }
            | Intent::Cycle { player, .. }
            | Intent::ActivateHandAbility { player, .. }
            | Intent::Suspend { player, .. }
            | Intent::Encore { player, .. }
            | Intent::TurnFaceUp { player, .. }
            | Intent::CastPrepared { player, .. }
            | Intent::CastAdventure { player, .. }
            | Intent::CastBestow { player, .. }
            | Intent::CastFaceDown { player, .. }
            | Intent::TapForMana { player, .. }
            | Intent::ChannelColorlessMana { player, .. }
            | Intent::ActivateAbility { player, .. }
            | Intent::DeclareAttackers { player, .. }
            | Intent::DeclareBlockers { player, .. }
            | Intent::ChooseOrder { player, .. }
            | Intent::ChooseTargets { player, .. }
            | Intent::ChooseTargetPlayers { player, .. }
            | Intent::AnswerMay { player, .. }
            | Intent::PayOptionalCost { player, .. }
            | Intent::PayOptionalCostX { player, .. }
            | Intent::AssignDamage { player, .. }
            | Intent::DivideSpellDamage { player, .. }
            | Intent::DeclineUntap { player, .. }
            | Intent::ArrangeTop { player, .. }
            | Intent::SelectFromTop { player, .. }
            | Intent::DistributeTop { player, .. }
            | Intent::ShuffleFromGraveyard { player, .. }
            | Intent::SearchLibrary { player, .. }
            | Intent::ChooseSacrifices { player, .. }
            | Intent::Discard { player, .. }
            | Intent::PutLandFromHand { player, .. }
            | Intent::CastCreatureFaceDown { player, .. }
            | Intent::ReturnLandOrSacrifice { player, .. }
            | Intent::ChooseExiledWithCard { player, .. }
            | Intent::ChooseExiledWithCardToCast { player, .. }
            | Intent::ChooseExiledDigToCastFree { player, .. }
            | Intent::ChooseOpponentPile { player, .. }
            | Intent::RevealedCardToBattlefieldOrHand { player, .. }
            | Intent::ChooseMode { player, .. }
            | Intent::ChooseTriggerModes { player, .. }
            | Intent::ChooseManaColor { player, .. }
            | Intent::ChooseCreatureType { player, .. }
            | Intent::ChooseColor { player, .. }
            | Intent::ChooseCopyTarget { player, .. }
            | Intent::ChooseAttachHost { player, .. }
            | Intent::ChooseTopOrBottom { player, .. }
            | Intent::TakeAction { player, .. }
            | Intent::PassPriority { player }
            | Intent::Concede { player } => *player,
        }
    }

    /// Whether this intent answers a pending choice (see `Game::submit`'s choice gate, which
    /// only lets an answer intent from the choice's player through while one is pending).
    ///
    /// This must stay in sync with the `PendingChoice` answer handlers below `submit`'s choice
    /// gate. Deliberately exhaustive with **no wildcard arm**: adding a new `Intent` variant
    /// breaks this match at compile time, forcing the author to decide whether it answers a
    /// choice instead of silently falling through to `Reject::ChoicePending`.
    pub(crate) fn is_choice_answer(&self) -> bool {
        match self {
            Intent::ChooseOrder { .. }
            | Intent::ChooseTargets { .. }
            | Intent::ChooseTargetPlayers { .. }
            | Intent::AnswerMay { .. }
            | Intent::PayOptionalCost { .. }
            | Intent::PayOptionalCostX { .. }
            | Intent::AssignDamage { .. }
            | Intent::DivideSpellDamage { .. }
            | Intent::DeclineUntap { .. }
            | Intent::ArrangeTop { .. }
            | Intent::SelectFromTop { .. }
            | Intent::DistributeTop { .. }
            | Intent::ShuffleFromGraveyard { .. }
            | Intent::SearchLibrary { .. }
            | Intent::ChooseSacrifices { .. }
            | Intent::Discard { .. }
            | Intent::PutLandFromHand { .. }
            | Intent::CastCreatureFaceDown { .. }
            | Intent::ReturnLandOrSacrifice { .. }
            | Intent::ChooseExiledWithCard { .. }
            | Intent::ChooseExiledWithCardToCast { .. }
            | Intent::ChooseExiledDigToCastFree { .. }
            | Intent::ChooseOpponentPile { .. }
            | Intent::RevealedCardToBattlefieldOrHand { .. }
            | Intent::ChooseMode { .. }
            | Intent::ChooseTriggerModes { .. }
            | Intent::ChooseManaColor { .. }
            | Intent::ChooseCreatureType { .. }
            | Intent::ChooseColor { .. }
            | Intent::ChooseCopyTarget { .. }
            | Intent::ChooseAttachHost { .. }
            | Intent::ChooseTopOrBottom { .. } => true,
            Intent::Cast { .. }
            | Intent::PlayLand { .. }
            | Intent::Cycle { .. }
            | Intent::ActivateHandAbility { .. }
            | Intent::Suspend { .. }
            | Intent::Encore { .. }
            | Intent::TurnFaceUp { .. }
            | Intent::CastPrepared { .. }
            | Intent::CastAdventure { .. }
            | Intent::CastBestow { .. }
            | Intent::CastFaceDown { .. }
            | Intent::TapForMana { .. }
            | Intent::ChannelColorlessMana { .. }
            | Intent::ActivateAbility { .. }
            | Intent::DeclareAttackers { .. }
            | Intent::DeclareBlockers { .. }
            | Intent::TakeAction { .. }
            | Intent::PassPriority { .. }
            | Intent::Concede { .. } => false,
        }
    }
}

/// A decision the engine is waiting on. While one is pending, only the matching
/// [`Intent::ChooseOrder`] from `player` is legal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingChoice {
    /// `player` must order their simultaneously-triggered abilities (put on the stack
    /// in the chosen order). The order is a permutation of `0..effects.len()`.
    OrderTriggers {
        player: PlayerId,
        source: ObjectId,
        effects: Vec<Effect>,
    },
    /// `player` must choose the target for a triggered ability (`source`'s `effect`) before it
    /// goes on the stack. Answered by [`Intent::ChooseTargets`]. `legal` lists the valid targets.
    /// `optional` (from the effect's own [`TargetCount::min`] being 0 — Killian, Decisive
    /// Mentor's "tap up to one target creature") lets `targets` be empty to decline: the ability
    /// is dropped rather than placed with no target (CR "up to one" — choosing zero is legal).
    ChooseTarget {
        player: PlayerId,
        source: ObjectId,
        effect: Effect,
        legal: Vec<Target>,
        optional: bool,
    },
    /// `player` (the caster) must choose the targets for the multi-target `spell` on the stack
    /// (CR 601.2c): between `min` and `max` distinct targets drawn from `legal`. Answered by
    /// [`Intent::ChooseTargets`]. Distinct from [`Self::ChooseTarget`] (a single triggered-ability
    /// target chosen before it hits the stack); this records N targets onto an already-cast spell.
    ChooseSpellTargets {
        player: PlayerId,
        spell: ObjectId,
        min: u8,
        max: u8,
        legal: Vec<Target>,
        /// Which independent target clause this pause fills (CR 601.2c — all a spell's targets are
        /// chosen at once, in printed order). `0` for the usual single multi-target clause; `1` for
        /// a second one (Magma Opus's tap clause), chained after clause 0 is answered.
        clause: u8,
    },
    /// `player` may decline or accept an optional triggered ability (`source`'s `effect`).
    /// Answered by [`Intent::AnswerMay`].
    MayYesNo {
        player: PlayerId,
        source: ObjectId,
        effect: Effect,
    },
    /// `player` (the active player at their untap step) may choose not to untap each of
    /// `permanents` — the permanents they control that carry [`CardDef::may_choose_not_to_untap`]
    /// (CR 502.2 — Rubinia Soulsinger). Raised as a turn-based-action pause; answered by
    /// [`Intent::DeclineUntap`], which untaps every offered permanent the player didn't keep tapped
    /// and resumes the interrupted untap step.
    DeclineUntap {
        player: PlayerId,
        permanents: Vec<ObjectId>,
    },
    /// `player` may pay `cost` to get an optional triggered ability (`source`'s `effect`).
    /// Answered by [`Intent::PayOptionalCost`].
    PayCost {
        player: PlayerId,
        source: ObjectId,
        cost: Cost,
        effect: Effect,
    },
    /// `player` (the target spell's controller) may pay `cost` to save `spell` from being
    /// countered (CR 701.5c-style "unless" clause — [`Effect::CounterTargetSpell`]'s
    /// `unless_pays`). Paying leaves `spell` on the stack to resolve normally; declining counters
    /// it. Answered by [`Intent::PayOptionalCost`], the mirror-image of [`PendingChoice::PayCost`]
    /// (there, declining means nothing happens; here, declining means the counter happens).
    PayOrCounter {
        player: PlayerId,
        cost: Cost,
        spell: ObjectId,
    },
    /// `player` (the triggering opponent) may pay `cost` to stop `controller`'s optional draw —
    /// Rhystic Study's "unless that player pays {1}" ([`Effect::MayDrawUnlessPays`]). Only raised
    /// after `controller` accepts the preceding [`Self::MayYesNo`] pause (the real card's ruling:
    /// declining to draw never even offers a pay window). Paying leaves `controller`'s hand
    /// untouched; declining draws them a card. Answered by [`Intent::PayOptionalCost`], the same
    /// "declining does something" polarity as [`Self::PayOrCounter`], but the something is a
    /// draw rather than a counter.
    PayOrControllerDraws {
        player: PlayerId,
        controller: PlayerId,
        cost: Cost,
    },
    /// `player` (Hinder's controller) must choose the top or bottom of `spell`'s owner's library
    /// (CR 701.5b — [`Effect::CounterTargetSpell`]'s `countered_dest` rider): `spell` is already
    /// countered — still a live [`crate::Object::Spell`] on the stack until this answers — and
    /// this choice picks where it goes instead of into the graveyard. Answered by
    /// [`Intent::ChooseTopOrBottom`].
    ChooseCounteredSpellDestination { player: PlayerId, spell: ObjectId },
    /// `player` (the permanent's controller) may pay `cost` (its printed Echo cost) to keep
    /// `source`, or decline and sacrifice it (CR 702.31c/d — "sacrifice it unless you pay its
    /// echo cost"). Answered by [`Intent::PayOptionalCost`], the permanent-scoped twin of
    /// [`Self::PayOrCounter`] — same "declining does something" polarity (there, countering the
    /// spell; here, sacrificing the source).
    PayEchoOrSacrifice {
        player: PlayerId,
        source: ObjectId,
        cost: Cost,
    },
    /// `player` (`source`'s controller) may pay `cost` to keep `source`, or decline and sacrifice
    /// it — Rupture Spire's own ETB triggered ability (CR 603.3b), NOT Echo, though it shares
    /// [`Self::PayEchoOrSacrifice`]'s pay-or-sacrifice polarity and its
    /// [`Intent::PayOptionalCost`] answer shape. Kept as its own variant (rather than reused)
    /// because it's a real triggered ability firing once at ETB, not Echo's own upkeep-scoped
    /// keyword (CR 702.31) — conflating the two would misname what's happening on the stack.
    SacrificeUnlessPay {
        player: PlayerId,
        source: ObjectId,
        cost: Cost,
    },
    /// `player` (`source`'s controller) must return one of `candidates` (their own non-Lair
    /// lands) to its owner's hand to keep `source`, or decline and sacrifice it — Treva's Ruins'
    /// own ETB triggered ability. The land-bounce twin of [`Self::SacrificeUnlessPay`]; answered
    /// by [`Intent::ReturnLandOrSacrifice`]. `candidates` are public battlefield permanents.
    SacrificeUnlessReturnLand {
        player: PlayerId,
        source: ObjectId,
        candidates: Vec<ObjectId>,
    },
    /// `player` (the attacker's controller) must divide `attacker`'s combat damage among its
    /// `blockers`. Answered by [`Intent::AssignDamage`].
    AssignCombatDamage {
        player: PlayerId,
        attacker: ObjectId,
        blockers: Vec<ObjectId>,
    },
    /// `player` (the caster) must divide a divided-damage `spell`'s `total` among its already-
    /// chosen `targets` (CR 601.2d — Magma Opus's "4 damage divided as you choose among any number
    /// of targets"), each getting at least one point. `targets` are [`Target`]s, not bare object
    /// ids: "any number of targets" admits *players* alongside creatures. Answered by
    /// [`Intent::DivideSpellDamage`], its own `Target`-keyed wire (unlike combat's object-only
    /// [`Self::AssignCombatDamage`]).
    DivideSpellDamage {
        player: PlayerId,
        spell: ObjectId,
        targets: Vec<Target>,
        total: i32,
    },
    /// `player` (the caster) must divide a divided-counters `spell`'s `total` among its already-
    /// chosen `targets` (CR 601.2d — Grove's Bounty's "Distribute X +1/+1 counters among any
    /// number of target creatures you control"), each getting at least one counter. Answered by
    /// [`Intent::AssignDamage`] — the same wire shape [`Self::DivideSpellDamage`] reuses, branched
    /// on which of the three divide-choices is pending (see `Game::submit`).
    DivideCounters {
        player: PlayerId,
        spell: ObjectId,
        targets: Vec<ObjectId>,
        total: i32,
    },
    /// `player` must distribute a *move*-counters effect's +1/+1 counters, currently on `from`,
    /// across any number of `legal` destinations (CR 601.2d — Forgotten Ancient's "move any
    /// number of +1/+1 counters from this creature onto other creatures"). Answered by
    /// [`Intent::AssignDamage`], the same wire shape [`Self::DivideCounters`] reuses (branched on
    /// which of the divide-choices is pending — see `Game::submit`). Unlike
    /// [`Self::DivideCounters`]'s fixed spell total that every chosen target must share, `cap` is
    /// only an upper bound (`from`'s live +1/+1 count): fewer than `legal.len()` destinations may
    /// be chosen, and the assigned total may be anywhere from zero up to `cap` ("any number").
    DivideMovedCounters {
        player: PlayerId,
        from: ObjectId,
        legal: Vec<ObjectId>,
        cap: i32,
    },
    /// `player` looks at the top `cards` of their library (a scry/surveil) and must split them
    /// into a kept pile (back on top, in the answered order) and a bottom pile — put on the
    /// library bottom (scry) or into the graveyard when `to_graveyard` (surveil). Answered by
    /// [`Intent::ArrangeTop`].
    ArrangeTop {
        player: PlayerId,
        cards: Vec<ObjectId>,
        to_graveyard: bool,
    },
    /// `player` looked at the top `cards` of their library ([`Effect::LookAtTop`]) and may select
    /// up to `up_to` of them that match `filter` into `dest`; every non-selected card goes to
    /// `rest`. Answered by [`Intent::SelectFromTop`]. The looked-at ids are private to `player`
    /// (like [`ArrangeTop`](Self::ArrangeTop)).
    SelectFromTop {
        player: PlayerId,
        cards: Vec<ObjectId>,
        filter: CardFilter,
        up_to: u32,
        /// See [`Effect::LookAtTop::min`].
        min: u32,
        dest: TopDest,
        /// See [`Effect::LookAtTop::dest_tapped`].
        dest_tapped: bool,
        rest: RestDest,
        /// See [`Effect::LookAtTop::mv_budget`].
        mv_budget: Option<u32>,
    },
    /// Dance with Calamity's push-your-luck pause ([`Effect::ExileTopUntilStopCastFreeUnderBudget`]):
    /// `player` (the caster) may exile another top-of-library card or stop. `exiled` are the cards
    /// exiled so far (public — exile-zone) and `total_mv` their summed mana value. Answered by
    /// [`Intent::AnswerMay`] (reusing the yes/no wire shape — `yes` exiles one more, `no` stops).
    DanceExileMore {
        player: PlayerId,
        source: ObjectId,
        exiled: Vec<ObjectId>,
        total_mv: u32,
        /// The bust threshold ([`Effect::ExileTopUntilStopCastFreeUnderBudget::budget`], 13 for
        /// Dance with Calamity), carried across the pauses so the payoff knows the gate.
        budget: u32,
    },
    /// `player` looked at the top `cards` of their library ([`Effect::DistributeTop`]) and must
    /// route exactly `to_hand` of them to hand, `to_bottom` to the library bottom, and
    /// `to_exile_may_play` into exile with permission to play this turn — one card per slot,
    /// sharing none. Answered by [`Intent::DistributeTop`]. The looked-at ids are private to
    /// `player` (like [`SelectFromTop`](Self::SelectFromTop)).
    DistributeTop {
        player: PlayerId,
        cards: Vec<ObjectId>,
        to_hand: u32,
        to_bottom: u32,
        to_exile_may_play: u32,
    },
    /// `player` may choose any number of `options` (every permanent on the battlefield that
    /// currently has a counter, any controller — CR 701.27) to proliferate: each chosen one
    /// gets another counter of every kind already on it. Answered by
    /// [`Intent::ChooseSacrifices`] (reusing its "any subset of the offered set" wire shape —
    /// unlike [`SacrificeEdict`](Self::SacrificeEdict)/[`MaySacrifice`](Self::MaySacrifice)'s
    /// exactly-one, an empty answer here is a legal "proliferate nothing"). `remaining` is how
    /// many more times [`Effect::Proliferate`]'s `times` still has to run after this one; a
    /// nonzero `remaining` re-pauses on a fresh `Proliferate` choice once this one's counters
    /// are placed, mirroring [`SearchLibrary`](Self::SearchLibrary)'s re-pause chaining.
    Proliferate {
        player: PlayerId,
        source: ObjectId,
        options: Vec<ObjectId>,
        remaining: u8,
    },
    /// `player` (Guardian of Faith's controller) may choose any number of `options` — the *other*
    /// creatures they control — to phase out (CR 702.26; [`Effect::PhaseOut`]). Answered by
    /// [`Intent::ChooseSacrifices`] (reusing its "any subset of the offered set" wire shape, like
    /// [`Proliferate`](Self::Proliferate) — an empty answer is a legal "phase out nothing," CR
    /// "any number ... target"). Each chosen creature, and everything attached to it, phases out.
    PhaseOut {
        player: PlayerId,
        source: ObjectId,
        options: Vec<ObjectId>,
    },
    /// `player` must choose a triggered ability's *second* independent target clause (CR 603.3d —
    /// Kinetic Ooze's X≥10 "double ... any number of other target creatures") before it goes on the
    /// stack: between `min` and `max` distinct targets from `legal` (CR 601.2c). The ability's
    /// `effect` (a `Sequence`) and its already-chosen first-clause `target` are carried so the
    /// assembled ability — both clauses — is pushed once answered. Answered by
    /// [`Intent::ChooseTargets`]. Distinct from [`Self::ChooseSpellTargets`] (a spell already on the
    /// stack) and [`Self::ChooseTarget`] (a single first-clause target).
    ChooseAbilityTargets {
        player: PlayerId,
        source: ObjectId,
        effect: Effect,
        target: Option<Target>,
        min: u8,
        max: u8,
        legal: Vec<Target>,
    },
    /// `player` (the ability's controller — always the answerer, even when `owner` is a
    /// different player) may shuffle up to `max` (`0` = unbounded) of `candidates` (cards in
    /// `owner`'s graveyard) into `owner`'s library
    /// ([`Effect::ShuffleTargetCardsFromGraveyardIntoLibrary`] — Perpetual Timepiece has
    /// `owner == player`; Quandrix Command mode 3 targets a player, so `owner` may differ).
    /// Answered by [`Intent::ShuffleFromGraveyard`]. The graveyard is a public zone, so
    /// `candidates` are public (unlike [`SelectFromTop`](Self::SelectFromTop)'s library cards).
    ShuffleFromGraveyard {
        player: PlayerId,
        owner: PlayerId,
        source: ObjectId,
        candidates: Vec<ObjectId>,
        max: u32,
    },
    /// `player` is searching their library and must pick one of `matches` to move to `dest`
    /// (`tapped` if it enters the battlefield), or none ("fail to find", CR 701.19c). Answered by
    /// [`Intent::SearchLibrary`]. The matching ids are private to the searcher. `remaining` is
    /// how many more cards (including this pick) the search may still find; a found pick with
    /// `remaining > 1` and matches left re-pauses on a fresh [`PendingChoice::SearchLibrary`]
    /// (over `matches` minus the pick) instead of shuffling, so an "up to N" search (Land Tax)
    /// shuffles only once, after the last pick (CR 701.19f). The library is shuffled once the
    /// search ends: `remaining` hits 0, no matches are left, or the searcher fails to find.
    /// `overflow`, carried unchanged from [`Effect::SearchLibrary::overflow`], is where every
    /// find *after the first* goes instead of `dest` (Cultivate).
    SearchLibrary {
        player: PlayerId,
        matches: Vec<ObjectId>,
        dest: SearchDest,
        tapped: bool,
        remaining: u8,
        overflow: Option<SearchDest>,
    },
    /// `player` (the trigger's controller) must choose one of `modes` for an [`Effect::ChooseOne`]
    /// "choose one" triggered ability, resolving at the point the ability resolves. Answered by
    /// [`Intent::ChooseMode`]; the chosen mode is run with the trigger's `source`/`target`/`x`
    /// context. Mode labels for the wire come from `modes.len()` / each mode's [`Effect::label`].
    ChooseMode {
        player: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
        modes: &'static [Effect],
    },
    /// `player` may choose `choose` distinct modes of a modal *triggered* ability (`source`, CR
    /// 700.2's "choose two" extended to a trigger's own modes), each mode paired with its own
    /// Player target — pairwise distinct across the chosen modes (Shadrix Silverquill's
    /// begin-combat "you may choose two. Each mode must target a different player."). `modes` are
    /// the card's printed modes matching the fired trigger, in printed order (the same indexing
    /// [`nth_mode`] uses for a modal spell's modes). Answered by [`Intent::ChooseTriggerModes`].
    /// `optional`: an empty selection is legal only when set (CR "you may") and drops the whole
    /// ability.
    /// ponytail: the pairwise-distinct-target rule is enforced unconditionally by
    /// [`Game::answer_choose_trigger_modes`] — a Shadrix-specific cross-mode restriction, not a
    /// general modal-trigger axis; no other pool card needs a different cross-mode rule yet.
    ChooseTriggerModes {
        player: PlayerId,
        source: ObjectId,
        modes: Vec<Effect>,
        choose: u8,
        optional: bool,
    },
    /// `player` must choose which of the permanents they control (`options`) to sacrifice to a
    /// multi-player edict ([`Effect::EachPlayerSacrifices`]): one of them, or — with `keep_one` —
    /// all but one they keep. Answered by [`Intent::ChooseSacrifices`]. `remaining` are the
    /// still-to-choose players (APNAP order) after this one; once they're all done, `follow_up`
    /// runs for `controller`. `filter` re-derives each remaining player's options.
    SacrificeEdict {
        player: PlayerId,
        options: Vec<ObjectId>,
        keep_one: bool,
        filter: PermanentFilter,
        remaining: Vec<PlayerId>,
        controller: PlayerId,
        source: ObjectId,
        follow_up: &'static [Effect],
    },
    /// `player` (a [`EdictScope::TargetedPlayers`](super::EdictScope::TargetedPlayers) edict's
    /// controller, e.g. Priest of Forgotten Gods) chooses "any number of target players" (CR
    /// 601.2c/608.2b — zero is legal) from `legal` (every living player): between `min` (0) and
    /// `max` (`legal.len()`). Answered by [`Intent::ChooseTargetPlayers`]. The chosen set becomes
    /// the edict's affected players — `keep_one`/`filter`/`life_loss`/`then` are the edict's own
    /// fields, carried through so [`Game::choose_target_players`] can run the same per-player
    /// sacrifice fan-out [`Game::begin_sacrifice_edict`] runs for `AllPlayers`/`EachOpponent`.
    ChooseTargetPlayers {
        player: PlayerId,
        source: ObjectId,
        legal: Vec<PlayerId>,
        min: u8,
        max: u8,
        keep_one: bool,
        filter: PermanentFilter,
        life_loss: i32,
        then: &'static [Effect],
    },
    /// `player` must exile one of `options` (a card in their own graveyard) to a multi-player
    /// graveyard-exile fan-out ([`Effect::EachPlayerExilesFromGraveyard`] — Augusta, Order
    /// Returned). Mandatory (exactly one, when they have any). Answered by
    /// [`Intent::ChooseSacrifices`], reusing its "name the chosen card" wire shape. `remaining`
    /// are the still-to-choose players (APNAP order) after this one; the graveyard is a public
    /// zone, so `options` are public (no redaction). No `follow_up`: the reflexive payoff rides in
    /// the enclosing `Sequence`, resumed once every player has answered.
    ExileFromGraveyard {
        player: PlayerId,
        source: ObjectId,
        options: Vec<ObjectId>,
        remaining: Vec<PlayerId>,
    },
    /// `caster` (Tragic Arrogance's controller) chooses which of `target_player`'s nonland
    /// permanents (`options`) to keep — up to one of each type (artifact, creature, enchantment;
    /// a planeswalker slot is unreachable, see [`Effect::CasterKeepsOneOfEachTypePerPlayer`]).
    /// Answered by [`Intent::ChooseSacrifices`], reusing its "name the chosen set" wire shape (the
    /// set is the *kept* ids). All of `target_player`'s other nonland permanents are then sacrificed
    /// (CR 701.16b). `remaining` are the still-to-choose players (APNAP order) after this one; the
    /// battlefield is public, so `options` are public. The answering seat is `caster`, not
    /// `target_player` — unlike [`Self::SacrificeEdict`], where each affected player chooses their own.
    CasterKeepPermanents {
        caster: PlayerId,
        source: ObjectId,
        target_player: PlayerId,
        options: Vec<ObjectId>,
        remaining: Vec<PlayerId>,
    },
    /// `chooser` (Nils' controller) puts a +1/+1 counter on up to one of `target_player`'s creatures
    /// (`options`), or declines (CR 603.3d "up to one" — Nils, Discipline Enforcer's per-player
    /// end-step fan-out). Answered by [`Intent::ChooseSacrifices`], reusing its "name the chosen set"
    /// wire shape (0 or 1 id). `remaining` are the still-to-choose players (APNAP order) after this
    /// one; the battlefield is public, so `options` are public. The answering seat is `chooser`, not
    /// `target_player` — like [`Self::CasterKeepPermanents`].
    ChooseCounterTargetForPlayer {
        chooser: PlayerId,
        source: ObjectId,
        target_player: PlayerId,
        options: Vec<ObjectId>,
        remaining: Vec<PlayerId>,
    },
    /// `player` casts a council's-dilemma vote (CR 701.32 — Fateful Tempest's "Starting with you,
    /// each player votes for past or present"): choose one of `options` (the ballot labels).
    /// Answered by [`Intent::ChooseMode`], reusing its "pick one labeled index" wire shape — the
    /// index into `options`. `remaining` are the still-to-vote players (turn order from the caster)
    /// after this one. Votes are public; the payoff rides in the enclosing `Sequence`, resumed once
    /// every player has voted (like [`Self::ExileFromGraveyard`]).
    CastVote {
        player: PlayerId,
        source: ObjectId,
        options: &'static [&'static str],
        remaining: Vec<PlayerId>,
    },
    /// `player` may sacrifice one of `options` (a permanent they control); if they do, `then`
    /// resolves ([`Effect::MaySacrifice`] — Witherbloom Charm mode 0's "You may sacrifice a
    /// permanent. If you do, draw two cards."). Answered by [`Intent::ChooseSacrifices`]: an
    /// empty list declines, one entry pays. Distinct from [`Self::SacrificeEdict`] (mandatory,
    /// possibly multi-player): this is one player's own resolution-time optional cost.
    MaySacrifice {
        player: PlayerId,
        source: ObjectId,
        options: Vec<ObjectId>,
        then: &'static [Effect],
    },
    /// `player` may return one of `options` (a graveyard card they own) to their hand, or decline
    /// ([`Effect::MayReturnFromGraveyard`] — Deadly Brew's "you may return another permanent card
    /// from your graveyard to your hand"). Answered by [`Intent::ChooseSacrifices`] (reusing its
    /// "empty list declines, one entry picks" wire shape): an empty list declines, one entry
    /// returns that card. The graveyard-return twin of [`Self::MaySacrifice`].
    MayReturnFromGraveyard {
        player: PlayerId,
        source: ObjectId,
        options: Vec<ObjectId>,
    },
    /// `player` may discard one of `options` (a card in their own hand); if they do, `then`
    /// resolves ([`Effect::MayDiscard`] — Quintorius, History Chaser's +1 "You may discard a
    /// card. If you do, draw two cards, then mill a card."). Answered by
    /// [`Intent::ChooseSacrifices`] (reusing its "empty list declines, one entry picks" wire
    /// shape, like [`Self::MayReturnFromGraveyard`]): an empty list declines, one entry
    /// discards that card. The hand-discard twin of [`Self::MaySacrifice`].
    MayDiscard {
        player: PlayerId,
        source: ObjectId,
        options: Vec<ObjectId>,
        then: &'static [Effect],
    },
    /// `player` must discard down to the hand-size limit at cleanup (CR 514.3): choose exactly
    /// `count` of `hand` (their whole hand, kept for stable display/validation) to discard.
    /// Answered by [`Intent::Discard`].
    DiscardToHandSize {
        player: PlayerId,
        hand: Vec<ObjectId>,
        count: usize,
    },
    /// `player` must discard exactly `count` cards to an [`Effect::Discard`] (a card-draw's
    /// rummage half, Faithless Looting): choose `count` of `hand` (their whole hand, kept for
    /// stable validation) to put into the graveyard. Answered by [`Intent::Discard`], like a
    /// cleanup discard — but resuming the resolving ability's sequence rather than a step change.
    DiscardCards {
        player: PlayerId,
        hand: Vec<ObjectId>,
        count: usize,
    },
    /// `player` may put one of `candidates` (their hand's land cards) onto the battlefield
    /// (`tapped` if it enters tapped), or decline ("up to one" — CR 305.9 special action, an
    /// [`Effect::PutLandFromHand`] resolving). Answered by [`Intent::PutLandFromHand`].
    PutLandFromHand {
        player: PlayerId,
        tapped: bool,
        candidates: Vec<ObjectId>,
    },
    /// `player` may cast one of `candidates` — the creature cards in their hand whose mana value
    /// is at most the `{X}` paid — face down as a 2/2 creature spell without paying its mana cost,
    /// or decline ("you may" — [`Effect::CastCreatureFaceDown`], Illusionary Mask, resolving).
    /// Answered by [`Intent::CastCreatureFaceDown`]. The candidates are hand cards, so private.
    CastCreatureFaceDown {
        player: PlayerId,
        candidates: Vec<ObjectId>,
    },
    /// `player` must choose up to one of `candidates` — the cards exiled with `source`
    /// ([`Game::exiled_with`]) — to put into its owner's graveyard, or decline
    /// ([`Effect::CashOutExiledWithThis`]'s "put a card exiled with this" resolving). Answered by
    /// [`Intent::ChooseExiledWithCard`]. The candidates are exile-zone cards, so public.
    ChooseExiledWithCard {
        player: PlayerId,
        source: ObjectId,
        candidates: Vec<ObjectId>,
    },
    /// `player` must choose up to one of `candidates` — the cards exiled with `source` — to
    /// grant the free-cast permission (CR 118.5), or decline ([`Effect::CastExiledWithThisFree`]'s
    /// "choose target card exiled with Quintorius" resolving). Answered by
    /// [`Intent::ChooseExiledWithCardToCast`]. The candidates are exile-zone cards, so public.
    ChooseExiledWithCardToCast {
        player: PlayerId,
        source: ObjectId,
        candidates: Vec<ObjectId>,
    },
    /// `player` must choose up to one of `candidates` — the cards among `source`'s just-exiled
    /// dig batch (`exiled`) that match the effect's filter — to grant the free-cast permission
    /// (CR 118.5), or decline (Herald of Amity's "exile the top eight … you may cast an Aura
    /// spell from among them without paying its mana cost" resolving). Answered by
    /// [`Intent::ChooseExiledDigToCastFree`]. Answering (either way) also puts every other card
    /// in `exiled` on the bottom of the library (CR "put the rest on the bottom") — unlike
    /// [`ChooseExiledWithCardToCast`](Self::ChooseExiledWithCardToCast), whose non-chosen
    /// candidates simply stay in their pile. The candidates are exile-zone cards, so public.
    ChooseExiledDigToCastFree {
        player: PlayerId,
        source: ObjectId,
        candidates: Vec<ObjectId>,
        exiled: Vec<ObjectId>,
    },
    /// `player` (an **opponent** of `controller`, not the ability's controller) must choose one of
    /// two exile piles (`pile_a`/`pile_b`, both public — exile-zone) — Abstract Performance's "an
    /// opponent chooses one of those piles". Answered by [`Intent::ChooseOpponentPile`]. The chosen
    /// pile goes to `controller`'s graveyard; the other pile is then offered to `controller` on a
    /// [`Self::ChooseExiledToCastFree`] (`rest_to_hand`). The addressee is an opponent, so
    /// [`Game::submit`]'s "only the pending choice's player may answer" gate makes only that
    /// opponent able to answer.
    OpponentChoosesPile {
        player: PlayerId,
        controller: PlayerId,
        source: ObjectId,
        pile_a: Vec<ObjectId>,
        pile_b: Vec<ObjectId>,
    },
    /// `player` (an **opponent** of `controller`) must choose one of `nonlands` — the nonland cards
    /// exiled by [`Effect::EachPlayerExilesUntilNonlandOpponentPicks`] (all public — exile-zone),
    /// Plargg and Nassari's "an opponent chooses a nonland card exiled this way". Answered by
    /// [`Intent::ChooseExiledWithCard`] (reusing its "name the chosen card" wire shape; the pick is
    /// mandatory, so `None` is illegal here). The chosen card stays exiled; the *other* exiled
    /// cards (`exiled` minus the pick, castable ones) are then offered to `controller` on a
    /// [`Self::ChooseExiledToCastFree`] (`count = 2`).
    OpponentChoosesExiledNonland {
        player: PlayerId,
        controller: PlayerId,
        source: ObjectId,
        nonlands: Vec<ObjectId>,
        exiled: Vec<ObjectId>,
    },
    /// `player` (the ability's controller) may choose up to `count` of `candidates` — castable
    /// cards among the exile pile `exiled` — to grant the free-cast permission (CR 118.5). Answered
    /// by [`Intent::ChooseSacrifices`] (reusing its "name the set" wire shape): an empty set
    /// declines, up to `count` entries grant permission. After answering, the cards in `exiled`
    /// that weren't chosen go to `player`'s **hand** if `rest_to_hand` (Abstract Performance's "put
    /// the rest into your hand"), else stay exiled (Plargg and Nassari's uncast cards). The
    /// controller-facing free-cast half of the opponent-chooses cards; distinct from
    /// [`Self::ChooseExiledDigToCastFree`] (single pick, rest bottomed).
    ChooseExiledToCastFree {
        player: PlayerId,
        source: ObjectId,
        candidates: Vec<ObjectId>,
        exiled: Vec<ObjectId>,
        count: u8,
        rest_to_hand: bool,
    },
    /// `player` may put `card` (the library card [`Effect::RevealUntilMayDeploy`] just revealed
    /// and stopped on, left unmoved on top of the library) onto the battlefield, or decline and
    /// put it into hand instead (Songbirds' Blessing). Answered by
    /// [`Intent::RevealedCardToBattlefieldOrHand`]. `card` was already publicly revealed
    /// ([`Event::RevealedTopOfLibrary`]), so it is public.
    RevealedCardToBattlefieldOrHand { player: PlayerId, card: ObjectId },
    /// `player` must sacrifice exactly `count` of `options` (their own permanents matching
    /// `filter`) — a forced sacrifice cost/effect the affected player directs (CR 701.16a: "the
    /// permanents' controller chooses which ones" — Lotus Field's ETB "sacrifice two lands",
    /// Smothering Abomination's upkeep "sacrifice a creature"). Unlike
    /// [`MaySacrifice`](Self::MaySacrifice), this is mandatory — declining isn't legal. Only
    /// raised when `options` outnumbers `count` (a real choice); with `count` or fewer legal
    /// permanents, [`Game::begin_choose_own_sacrifices`] sacrifices all of them immediately
    /// instead of pausing (CR 700.2's "as many as possible"). Answered by
    /// [`Intent::ChooseSacrifices`], reusing its "name the sacrificed set" wire shape.
    ChooseOwnSacrifices {
        player: PlayerId,
        source: ObjectId,
        filter: PermanentFilter,
        count: u32,
        options: Vec<ObjectId>,
    },
    /// `player` (a Devour N creature's controller) may sacrifice any subset of `options` (the
    /// other creatures they control) as `source` enters (CR 702.82 — "you may sacrifice any
    /// number of creatures"); `source` then gains `multiplier × count` +1/+1 counters, routed
    /// through [`Game::counters_after_replacements`] so CR 614 doublers apply. Answered by
    /// [`Intent::ChooseSacrifices`], reusing its "name the sacrificed set" wire shape: an empty
    /// list declines (0 counters, always legal). Unlike [`Self::ChooseOwnSacrifices`] the count
    /// is a free subset, not a fixed number.
    Devour {
        player: PlayerId,
        source: ObjectId,
        multiplier: u32,
        options: Vec<ObjectId>,
    },
    /// `player` (a mana ability's controller) must name one color; `amount` credits of that
    /// color are added to their pool (CR 106.4's "add N mana of any one color" —
    /// [`Effect::AddMana`]'s `single_color`: Lotus Field, Kami of Whispered Hopes). Answered by
    /// [`Intent::ChooseManaColor`]. Unlike every other pausing effect, this is raised from a mana
    /// ability's own immediate resolution ([`Game::activate_ability`]), not from the stack — CR
    /// 605.3a exempts mana abilities from the stack, not from choices made while they resolve.
    ChooseManaColor {
        player: PlayerId,
        source: ObjectId,
        amount: u8,
    },
    /// `player` (an as-enters permanent's controller) must name a creature type for `source`
    /// (CR 614.12/700.9-style "as ~ enters, choose a creature type" — Patchwork Banner's
    /// [`Effect::ChooseCreatureType`]). `options` is [`CREATURE_TYPES`], the pool's known
    /// creature-type table. Answered by [`Intent::ChooseCreatureType`], which sets `source`'s
    /// [`Permanent::chosen_subtype`].
    ChooseCreatureType {
        player: PlayerId,
        source: ObjectId,
        options: &'static [&'static str],
    },
    /// `player` (an as-enters permanent's controller) must name a color for `source` (CR
    /// 614.12/700.9-style "as ~ enters, choose a color" — Flickering Ward's
    /// [`Effect::ChooseColor`]). The candidate list is the fixed five colors ([`Color::ALL`]), so
    /// unlike [`Self::ChooseCreatureType`] no `options` slice is carried. Answered by
    /// [`Intent::ChooseColor`], which sets `source`'s [`Permanent::chosen_color`].
    ChooseColor { player: PlayerId, source: ObjectId },
    /// `player` (an entering permanent's controller) may have `source` enter as a copy of one of
    /// `candidates` (every other object of the marker's [`EnterAsCopy::of`] type on the
    /// battlefield — CR 706/707.2: a creature for Altered Ego/Cursed Mirror, an enchantment
    /// (including an Aura) for Copy Enchantment). Answered by [`Intent::ChooseCopyTarget`]:
    /// `Some(object)` copies it (overwriting
    /// `source`'s `def` and applying the [`EnterAsCopy`] riders carried here), `None` declines
    /// ("you may" — `source` stays its printed self). Only raised when at least one candidate
    /// exists ([`Game::begin_enter_as_copy`]). `until_eot`/`extra_counters`/`gains_haste` are the
    /// copied-from marker's riders, carried so the answer handler can apply them.
    ChooseCopyTarget {
        player: PlayerId,
        source: ObjectId,
        candidates: Vec<ObjectId>,
        until_eot: bool,
        extra_counters: Amount,
        gains_haste: bool,
    },
    /// `player` may choose one of `candidates` — the tokens they control ("you may choose a token
    /// you control" — Brudiclad, Telchor Engineer, [`Effect::EachOtherTokenBecomesCopyOfChosen`]).
    /// Answered by [`Intent::ChooseCopyTarget`] (reused — the answer is also "one optional chosen
    /// object"): `Some(token)` has every *other* token `player` controls become a copy of it (an
    /// indefinite [`Event::BecameCopy`] per other token, CR 706/707.2), `None` declines the "you
    /// may" and converts nothing. Only raised when `player` controls at least one token
    /// ([`Game::begin_each_other_token_becomes_copy`]).
    ChooseTokenToCopy {
        player: PlayerId,
        source: ObjectId,
        candidates: Vec<ObjectId>,
    },
    /// `player` may choose one of `candidates` — the artifact/creature cards that left their
    /// graveyard this batch ("you may have this creature become a copy of an artifact or creature
    /// card from among those cards" — Spirit of Resilience,
    /// [`Effect::PutCounterThenMayBecomeCopyOfCardFromList`]). Answered by
    /// [`Intent::ChooseCopyTarget`] (reused — the answer is also "one optional chosen object"):
    /// `Some(card)` has `source` become a copy of it until end of turn (an [`Event::BecameCopy`]
    /// with `until_eot: true`, CR 706/707.2), `None` declines the "you may". Only raised when at
    /// least one artifact/creature card left ([`Game::begin_put_counter_then_may_become_copy`]).
    ChooseCopyCardFromList {
        player: PlayerId,
        source: ObjectId,
        candidates: Vec<ObjectId>,
    },
    /// `player` (the deployed attachment's controller) must choose a host among `candidates` for
    /// `attachment` — an Aura or Equipment permanent that entered the battlefield without being
    /// cast (CR 303.4f — Songbirds' Blessing's "you may put that card onto the battlefield,"
    /// Armored Skyhunter's "you may put an Aura or Equipment card from among them onto the
    /// battlefield"). For an Aura, `optional` is `false` — the host is a legal `enchant` target
    /// and the choice is mandatory once raised (CR 303.4f); a hostless Aura instead hits the
    /// existing Aura-legality state-based action (CR 704.5m) and goes to the graveyard, unpaused.
    /// For Equipment, `optional` is `true` — the host is any creature `player` controls (CR
    /// 301.5c "you may attach it to a creature you control") and declining leaves the Equipment
    /// unattached, which is legal. Only raised when at least one legal host exists
    /// ([`Game::maybe_pause_attach_deployed_aura`]). Answered by [`Intent::ChooseAttachHost`].
    ChooseAttachHost {
        player: PlayerId,
        attachment: ObjectId,
        candidates: Vec<ObjectId>,
        optional: bool,
    },
}

/// Every creature type printed on a creature card in the pool, offered as the candidate list
/// for an as-enters "choose a creature type" choice ([`PendingChoice::ChooseCreatureType`]).
/// ponytail: the pool's own creature types, not the CR 205.3m full type list (which is much
/// longer and includes types no pool card uses) — widen this when a card needs a type not yet
/// printed on anything here.
pub(crate) const CREATURE_TYPES: &[&str] = &[
    "Advisor",
    "Aetherborn",
    "Ally",
    "Angel",
    "Archon",
    "Artificer",
    "Bard",
    "Bear",
    "Beast",
    "Bird",
    "Cat",
    "Cleric",
    "Construct",
    "Demon",
    "Dinosaur",
    "Djinn",
    "Dog",
    "Dragon",
    "Drake",
    "Druid",
    "Dryad",
    "Dwarf",
    "Efreet",
    "Elder",
    "Eldrazi",
    "Elemental",
    "Elephant",
    "Elf",
    "Faerie",
    "Fox",
    "Fractal",
    "Frog",
    "Fungus",
    "Giant",
    "Goblin",
    "Golem",
    "Griffin",
    "Horror",
    "Horse",
    "Human",
    "Hydra",
    "Incarnation",
    "Inkling",
    "Insect",
    "Jackal",
    "Jellyfish",
    "Knight",
    "Kor",
    "Lizard",
    "Mercenary",
    "Merfolk",
    "Monk",
    "Monkey",
    "Myr",
    "Ooze",
    "Orc",
    "Otter",
    "Ox",
    "Pest",
    "Phyrexian",
    "Rabbit",
    "Rat",
    "Rogue",
    "Scout",
    "Shaman",
    "Shapeshifter",
    "Skeleton",
    "Snake",
    "Soldier",
    "Sorcerer",
    "Spirit",
    "Troll",
    "Turtle",
    "Vampire",
    "Vedalken",
    "Warlock",
    "Warrior",
    "Wizard",
    "Wolf",
    "Wurm",
    "Zombie",
];

impl PendingChoice {
    pub(crate) fn player(&self) -> PlayerId {
        match self {
            PendingChoice::OrderTriggers { player, .. }
            | PendingChoice::ChooseTarget { player, .. }
            | PendingChoice::ChooseSpellTargets { player, .. }
            | PendingChoice::ChooseAbilityTargets { player, .. }
            | PendingChoice::MayYesNo { player, .. }
            | PendingChoice::DeclineUntap { player, .. }
            | PendingChoice::PayCost { player, .. }
            | PendingChoice::PayOrCounter { player, .. }
            | PendingChoice::PayOrControllerDraws { player, .. }
            | PendingChoice::ChooseCounteredSpellDestination { player, .. }
            | PendingChoice::PayEchoOrSacrifice { player, .. }
            | PendingChoice::SacrificeUnlessPay { player, .. }
            | PendingChoice::SacrificeUnlessReturnLand { player, .. }
            | PendingChoice::AssignCombatDamage { player, .. }
            | PendingChoice::DivideSpellDamage { player, .. }
            | PendingChoice::DivideCounters { player, .. }
            | PendingChoice::DivideMovedCounters { player, .. }
            | PendingChoice::ArrangeTop { player, .. }
            | PendingChoice::SelectFromTop { player, .. }
            | PendingChoice::DistributeTop { player, .. }
            | PendingChoice::Proliferate { player, .. }
            | PendingChoice::PhaseOut { player, .. }
            | PendingChoice::ShuffleFromGraveyard { player, .. }
            | PendingChoice::SearchLibrary { player, .. }
            | PendingChoice::SacrificeEdict { player, .. }
            | PendingChoice::ChooseTargetPlayers { player, .. }
            | PendingChoice::ExileFromGraveyard { player, .. }
            | PendingChoice::CastVote { player, .. }
            | PendingChoice::MaySacrifice { player, .. }
            | PendingChoice::MayReturnFromGraveyard { player, .. }
            | PendingChoice::MayDiscard { player, .. }
            | PendingChoice::DiscardToHandSize { player, .. }
            | PendingChoice::DiscardCards { player, .. }
            | PendingChoice::PutLandFromHand { player, .. }
            | PendingChoice::CastCreatureFaceDown { player, .. }
            | PendingChoice::ChooseMode { player, .. }
            | PendingChoice::ChooseTriggerModes { player, .. }
            | PendingChoice::ChooseExiledWithCard { player, .. }
            | PendingChoice::ChooseExiledWithCardToCast { player, .. }
            | PendingChoice::ChooseExiledDigToCastFree { player, .. }
            | PendingChoice::DanceExileMore { player, .. }
            | PendingChoice::OpponentChoosesPile { player, .. }
            | PendingChoice::OpponentChoosesExiledNonland { player, .. }
            | PendingChoice::ChooseExiledToCastFree { player, .. }
            | PendingChoice::RevealedCardToBattlefieldOrHand { player, .. }
            | PendingChoice::ChooseOwnSacrifices { player, .. }
            | PendingChoice::Devour { player, .. }
            | PendingChoice::ChooseManaColor { player, .. }
            | PendingChoice::ChooseCreatureType { player, .. }
            | PendingChoice::ChooseColor { player, .. }
            | PendingChoice::ChooseCopyTarget { player, .. }
            | PendingChoice::ChooseTokenToCopy { player, .. }
            | PendingChoice::ChooseCopyCardFromList { player, .. }
            | PendingChoice::ChooseAttachHost { player, .. } => *player,
            // The answering seat is the caster, not the target player whose board is being pruned.
            PendingChoice::CasterKeepPermanents { caster, .. } => *caster,
            // The chooser (Nils' controller) answers, not the player whose creature is countered.
            PendingChoice::ChooseCounterTargetForPlayer { chooser, .. } => *chooser,
        }
    }

    /// The number of items to permute — meaningful only for the ordering choices, which are
    /// the only ones answered by [`Intent::ChooseOrder`].
    pub(crate) fn len(&self) -> usize {
        match self {
            PendingChoice::OrderTriggers { effects, .. } => effects.len(),
            _ => 0,
        }
    }
}

/// Transient per-combat state: who is attacking, the declared blocks, and the attacker's
/// chosen combat-damage division for each multi-blocked attacker. Reset at end of combat.
#[derive(Debug, Clone, Default)]
pub(crate) struct CombatState {
    pub(crate) attackers: Vec<ObjectId>,
    /// Each attacker → the player it's attacking (its defending player).
    pub(crate) attack_targets: Vec<(ObjectId, PlayerId)>,
    /// (blocker, attacker) pairs.
    pub(crate) blocks: Vec<(ObjectId, ObjectId)>,
    /// Attacker → how its combat damage is divided among its blockers (multi-block only).
    /// Set via [`Event::CombatDamageDivided`].
    pub(crate) damage: Vec<(ObjectId, Vec<(ObjectId, i32)>)>,
    /// Whether the active player has finished declaring attackers this combat — so they
    /// aren't stopped again by auto-pass after a (possibly zero-attacker) declaration.
    /// ponytail: transient combat bookkeeping set directly, not event-sourced (see the note
    /// at `Game::apply`'s doc comment) — the resulting attacks/blocks are what get event-sourced.
    pub(crate) attackers_declared: bool,
    /// Attacked players who have already declared their blocks this combat (each declares once).
    pub(crate) blocked_by: Vec<PlayerId>,
}

/// One group of abilities that triggered simultaneously from a single source.
#[derive(Debug, Clone)]
pub(crate) struct TriggerGroup {
    pub(crate) controller: PlayerId,
    pub(crate) source: ObjectId,
    pub(crate) abilities: Vec<Ability>,
    /// Whether [`Game::place_pending_triggers`] has already run trigger-doubling (CR 603.3c —
    /// Harmonic Prodigy / Veyran) over this group. Set once the group (and any duplicate copies it
    /// spawns) have been considered, so a re-entrant placement pass — after a pending choice
    /// answered mid-batch re-runs the whole pipeline — doesn't double the same trigger again. A
    /// freshly queued group is `false`; a doubler-spawned copy is minted `true` (it must not
    /// itself be re-doubled).
    pub(crate) expanded: bool,
}

/// An item waiting to resolve on the stack: a cast spell, or a triggered ability.
// ponytail: Effect is ~CR 957B and this enum is Copy (CardDef: Copy invariant); boxing the large (CR 707)
// variant would break Copy. Size is acceptable; revisit only if Effect itself shrinks.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StackItem {
    Spell(ObjectId),
    Ability {
        controller: PlayerId,
        source: ObjectId,
        effect: Effect,
        /// The chosen target of the ability's first target clause, if it targets.
        target: Option<Target>,
        /// The chosen targets of a *second* independent target clause (CR 603.3d — Kinetic Ooze's
        /// X≥10 "double ... any number of other target creatures"), chosen as the trigger went on
        /// the stack. Empty for the ubiquitous single-clause ability. Read at resolution by
        /// [`Effect::DoubleCountersOnTargetCreatures`].
        targets_second: TargetList,
        /// The chosen `{X}` for an activated ability whose cost contains `{X}` (or a copy of one,
        /// CR 707.10c); `0` for every triggered ability. Read at resolution for `Amount::X`.
        x: u32,
    },
}

/// A public, read-only view of one stack item, for rendering the stack. Mirrors
/// [`StackItem`] (which is internal). Ordering follows the stack: index 0 is the
/// bottom, the last element is the top (resolves first).
// ponytail: Effect is ~957B and this enum is Copy (CardDef: Copy invariant); boxing the large
// variant would break Copy. Size is acceptable; revisit only if Effect itself shrinks.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackEntry {
    /// A cast spell waiting to resolve, identified by its stack-object id.
    Spell(ObjectId),
    /// A triggered/activated ability waiting to resolve.
    Ability {
        controller: PlayerId,
        source: ObjectId,
        effect: Effect,
        target: Option<Target>,
    },
}

/// A canonical, full-information record of something that happened. The *only* thing
/// that mutates game state (via [`Game::apply`]). The engine is audience-unaware; any
/// per-viewer redaction happens outside the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// A card was cast: it left `from` (hand/command) and became the spell `spell` on the stack.
    SpellCast {
        spell: ObjectId,
        from: ObjectId,
        controller: PlayerId,
        target: Option<Target>,
        /// The chosen `{X}` value (0 for a spell with no `{X}`).
        x: u32,
        /// A modal spell's chosen modes (empty selection for a non-modal spell); see [`Intent::Cast`].
        modes: Modes,
        /// Whether this was a flashback cast (CR 702.34 — from the graveyard for its flashback
        /// cost); the resolved spell is exiled instead of graveyard-bound.
        flashback: bool,
        /// Whether this was an escape cast (CR 702.19 — from the graveyard for its escape cost,
        /// exiling other graveyard cards as an additional cost); see [`Spell::escape`].
        escape: bool,
        /// How many permanents were sacrificed to pay [`AdditionalCost::sacrifice`] (0 for a
        /// spell with no such cost, or a decline); see [`Spell::sacrifice_count`].
        sacrifice_count: u8,
        /// Whether the caster paid the spell's kicker cost (CR 702.33d); see [`Spell::kicked`].
        kicked: bool,
        /// Whether the caster paid the spell's buyback cost (CR 702.27c); see
        /// [`Spell::bought_back`].
        bought_back: bool,
        /// The caster's declared Strive target count (CR 702.42), 0 for a spell with no Strive;
        /// see [`Spell::strive_count`].
        strive_count: u8,
        /// How many times the caster paid Replicate (CR 702.108), 0 for a spell with no Replicate;
        /// see [`Spell::replicate_count`].
        replicate_count: u8,
        /// Whether this was a bestow cast (CR 702.103 — for [`CardDef::bestow`], as an Aura spell);
        /// see [`Spell::bestowed`]. `false` for an ordinary cast.
        bestowed: bool,
        /// Whether this was a face-down morph cast (CR 702.37b — [`Intent::CastFaceDown`]); see
        /// [`Spell::face_down`]. `false` for an ordinary face-up cast.
        face_down: bool,
        /// Whether this was an evoke cast (CR 702.74a — for [`CardDef::evoke`]); see
        /// [`Spell::evoked`]. `false` for an ordinary cast.
        evoked: bool,
        /// The colors of mana actually spent to pay this cast's cost (CR 106.9); see
        /// [`Spell::spent_colors`].
        spent_colors: [bool; Color::COUNT],
    },
    /// A multi-target spell's chosen targets (CR 601.2c) were recorded onto `spell` — either
    /// auto-filled at cast (when the choice was forced) or answered via [`Intent::ChooseTargets`].
    /// A single-target spell never emits this: its lone target rides on [`Self::SpellCast`].
    /// `clause` selects which independent target clause these fill: `0` → [`Spell::targets`], `1` →
    /// [`Spell::targets_second`] (Magma Opus's tap clause).
    SpellTargetsChosen {
        spell: ObjectId,
        targets: TargetList,
        clause: u8,
    },
    /// A spell on the stack was copied (Twincast): `original` (still on the stack) is copied to a
    /// new spell object `copy` on top of it, controlled by `controller`. The copy has the same
    /// copiable characteristics/`x`/`mode` and (per this engine) the same target as the original.
    SpellCopied {
        copy: ObjectId,
        original: ObjectId,
        controller: PlayerId,
    },
    /// A spell *copy* finished resolving and ceased to exist (CR 707.10a / CR 111.7) — it leaves
    /// the stack without becoming a graveyard card. Distinct from [`Self::MovedToGraveyard`],
    /// which a cast instant/sorcery uses.
    SpellCeasedToExist { spell: ObjectId },
    /// A prepared permanent's "prepared" status changed (soc/sos prepare DFCs): set `true` by
    /// [`Effect::BecomePrepared`], cleared (`false`) when its back-face copy is cast
    /// ([`Game::cast_prepared`]). Flips [`Permanent::prepared`].
    PreparedChanged { object: ObjectId, prepared: bool },
    /// A Class permanent gained a level (CR 717.2 — [`Effect::LevelUp`]): sets `source`'s
    /// [`Permanent::level`] to `level`. Public battlefield status, like [`Self::PreparedChanged`].
    LeveledUp { source: ObjectId, level: u8 },
    /// `object` phased out (CR 702.26 — Guardian of Faith's [`Effect::PhaseOut`]): sets
    /// [`Permanent::phased_out`] on it and on everything attached to it (CR 702.26g), so it's
    /// treated as though it doesn't exist until its controller's next turn.
    PhasedOut { object: ObjectId },
    /// `object` phased in (CR 702.26f — at the start of its controller's untap step, before
    /// untapping): clears [`Permanent::phased_out`] on it and on everything attached to it.
    PhasedIn { object: ObjectId },
    /// An as-enters "choose a creature type" choice was answered (CR 614.12/700.9-style —
    /// Patchwork Banner's [`Effect::ChooseCreatureType`]). Sets `object`'s
    /// [`Permanent::chosen_subtype`].
    CreatureTypeChosen {
        object: ObjectId,
        subtype: &'static str,
    },
    /// An as-enters "choose a color" choice was answered (CR 614.12/700.9-style — Flickering
    /// Ward's [`Effect::ChooseColor`]). Sets `object`'s [`Permanent::chosen_color`].
    ColorChosen { object: ObjectId, color: Color },
    /// A copy of a prepared permanent's back-face spell was put on the stack (soc/sos prepare
    /// DFCs). `source` is the prepared permanent (its [`CardDef::back`] is the spell def);
    /// `spell` is the freshly-minted spell object, controlled by `controller`, targeting `target`.
    /// Distinct from [`Self::SpellCast`] because the spell comes from a battlefield permanent's
    /// back face, not a card in a zone — so it creates a copy that ceases to exist on resolve
    /// (no graveyard card), like a [`Self::SpellCopied`] copy.
    PreparedSpellCast {
        spell: ObjectId,
        source: ObjectId,
        controller: PlayerId,
        target: Option<Target>,
        /// The back face's chosen `{X}` (0 for a non-`{X}` back face) — Braingeyser.
        x: u32,
    },
    /// The adventure half of an adventure card (CR 715) was cast from hand and put on the stack.
    /// `source` is the hand card (its [`CardDef::adventure`] is the spell def); `spell` is the
    /// freshly-minted spell object, controlled by `controller`, targeting `target` with `x`.
    /// Distinct from [`Self::SpellCast`] because the spell's characteristics come from the card's
    /// adventure face, not its main face — so on resolution the card is exiled "on an adventure"
    /// (see [`Self::ExiledOnAdventure`]) rather than put into the graveyard.
    AdventureSpellCast {
        spell: ObjectId,
        source: ObjectId,
        controller: PlayerId,
        target: Option<Target>,
        /// The adventure spell's chosen `{X}` (0 for a non-`{X}` adventure — Grove's Bounty).
        x: u32,
    },
    /// A triggered or activated ability was put on the stack. `x` is the chosen `{X}` for an
    /// activated ability whose cost contains `{X}` (CR 107.3), or a copy of such an ability
    /// (CR 707.10c); `0` for every triggered ability (abilities carry no `{X}` of their own).
    TriggeredAbilityOnStack {
        controller: PlayerId,
        source: ObjectId,
        effect: Effect,
        target: Option<Target>,
        /// A second independent target clause's chosen targets (CR 603.3d — Kinetic Ooze's X≥10
        /// doubling rider); empty for a single-clause ability.
        targets_second: TargetList,
        x: u32,
    },
    /// The top ability of the stack finished resolving and left the stack.
    AbilityResolved { source: ObjectId },
    /// A new step began (also carries the active player, which changes each turn).
    StepBegan { step: Step, active_player: PlayerId },
    /// A land `from` was played from hand and became the permanent `permanent`.
    LandPlayed {
        permanent: ObjectId,
        from: ObjectId,
        player: PlayerId,
    },
    /// A permanent became tapped.
    Tapped { object: ObjectId },
    /// A permanent became untapped (e.g. by the untap step).
    Untapped { object: ObjectId },
    /// A regeneration shield was granted to a permanent (CR 701.15b — [`Effect::RegenerateShield`]).
    /// Increments [`Permanent::regeneration_shields`].
    RegenerationShieldCreated { object: ObjectId },
    /// A permanent was regenerated instead of destroyed (CR 701.15b): one regeneration shield is
    /// consumed, the permanent is tapped, removed from combat, and all its marked damage is
    /// healed. Replaces the graveyard move a destroy would otherwise emit.
    Regenerated { object: ObjectId },
    /// A permanent's regeneration shields expired at cleanup (CR 701.15b's "this turn"). Resets
    /// [`Permanent::regeneration_shields`] to 0.
    RegenerationShieldsExpired { object: ObjectId },
    /// A permanent shed summoning sickness (its controller's untap step).
    LostSummoningSickness { object: ObjectId },
    /// `count` +1/+1 counters were placed on a permanent as a single step — the post-replacement
    /// total (CR 614; see [`Game::counters_after_replacements`]). Modeling the whole placement as
    /// one event (rather than `count` separate ones) is what lets replacement effects see and grow
    /// the quantity before it lands.
    CountersPlaced {
        object: ObjectId,
        count: i32,
        source_name: &'static str,
    },
    /// `count` of `kind`'s named non-P/T counters were placed on (positive) or removed from
    /// (negative) a permanent — the [`CounterKind`]-keyed sibling of
    /// [`CountersPlaced`](Self::CountersPlaced) above. No replacement pipeline reads this path
    /// (see [`Effect::EntersWithCounters`]'s doc): a named kind is placed as the raw amount.
    KindCountersPlaced {
        object: ObjectId,
        kind: CounterKind,
        count: i32,
    },
    /// A planeswalker's loyalty changed by `amount` (a loyalty ability's cost: +N / 0 / −N).
    LoyaltyChanged { object: ObjectId, amount: i32 },
    /// A planeswalker's once-per-turn loyalty-ability flag was set (`active = true`, when a loyalty
    /// ability is activated) or cleared (`active = false`, at its controller's untap). CR 606.3.
    LoyaltyActivated { object: ObjectId, active: bool },
    /// A `once_each_turn`-capped activated ability was activated (CR 602.2b). Recorded so
    /// [`Game::ability_activation_gate`] can reject a second activation of the same
    /// (source, ability index) this turn; the tally clears at the start of every turn.
    /// ponytail: keyed by (object id, ability index), so a freshly re-cast permanent (a new
    /// object id) starts with a clean cap — correct, since a new object is a new game object. (CR 602, CR 601, CR 113)
    AbilityActivatedThisTurn {
        object: ObjectId,
        ability_index: usize,
    },
    /// A `once_each_turn`-capped *triggered* ability (Morbid Opportunist, Tocasia's Welcome) was
    /// placed on the stack (CR: "this ability triggers only once each turn" — counted at
    /// placement, not resolution). Recorded so [`Game::place_pending_triggers`] can drop a later
    /// placement from the same source this turn; the tally clears at the start of every turn.
    /// ponytail: keyed by source object id alone, not (source, ability index) like
    /// [`AbilityActivatedThisTurn`](Event::AbilityActivatedThisTurn) — no pool card puts two
    /// once-each-turn *triggered* abilities on one permanent. Widen to a pair if one ever does.
    TriggeredAbilityThisTurn { source: ObjectId },
    /// An Aura/Equipment's attachment changed: `object` is now attached to `host`
    /// (`None` = became unattached, e.g. its Equipment's host left the battlefield).
    AttachedTo {
        object: ObjectId,
        host: Option<ObjectId>,
    },
    /// A permanent received an until-end-of-turn power/toughness boost and/or keyword grant.
    TempBoost {
        object: ObjectId,
        power: i32,
        toughness: i32,
        keywords: &'static [Keyword],
        source_name: &'static str,
    },
    /// A permanent's until-end-of-turn boosts wore off (cleanup).
    TempBoostsEnded { object: ObjectId },
    /// A permanent's base power/toughness was SET until end of turn (CR 613.3(7b) — Biomass
    /// Mutation, Quandrix Charm's "has base power and toughness X/X until end of turn"), stored on
    /// [`Permanent::base_pt_set_eot`] and cleared alongside the temp boosts at
    /// [`Event::TempBoostsEnded`].
    BasePtSetUntilEndOfTurn {
        object: ObjectId,
        power: i32,
        toughness: i32,
    },
    /// A permanent gained card types + creature subtypes + colors until end of turn (CR 613.4 —
    /// Restless Spire's self-animation adds Creature + Elemental + blue/red to a noncreature land).
    /// Stored on [`Permanent::added_types_eot`]/[`Permanent::added_subtypes_eot`]/
    /// [`Permanent::added_colors_eot`], cleared alongside the base P/T set at
    /// [`Event::TempBoostsEnded`]. Public battlefield status, like `BasePtSetUntilEndOfTurn`.
    TypesAddedUntilEndOfTurn {
        object: ObjectId,
        types: TypeSet,
        subtypes: &'static [&'static str],
        colors: &'static [Color],
    },
    /// A just-reanimated permanent took on an *indefinite* set of characteristics (CR 611.2c —
    /// Excava, the Risen Past's "It's a 1/1 Spirit creature with flying in addition to its other
    /// types"): base P/T SET to `base_power`/`base_toughness`, `add_types`/`add_subtypes` added,
    /// `keywords` granted. Written to the permanent's indefinite
    /// `set_base_pt`/`added_types`/`added_subtypes`/`granted_keywords` and **not** cleared at
    /// cleanup — it resets only when the object leaves the battlefield (CR 400.7). Public
    /// battlefield status, like `BasePtSetUntilEndOfTurn`.
    ReanimatedCreatureBecame {
        object: ObjectId,
        add_types: TypeSet,
        add_subtypes: &'static [&'static str],
        base_power: i32,
        base_toughness: i32,
        keywords: &'static [Keyword],
    },
    /// A permanent gained an *indefinite* set of creature subtypes (CR 613.4 subtype layer —
    /// Hofri Ghostforge's minted copy "it's a Spirit in addition to its other types"), added on
    /// top of its printed subtypes and **not** cleared at cleanup (resets with the object per CR
    /// 400.7). A narrow subtype-only sibling of `ReanimatedCreatureBecame` that leaves base P/T
    /// untouched. Written to [`Permanent::added_subtypes`], unioned by `Game::effective_subtypes`.
    /// Public battlefield status, like `BasePtSetUntilEndOfTurn`.
    AddedSubtypes {
        object: ObjectId,
        subtypes: &'static [&'static str],
    },
    /// `object` became a copy of another creature as it entered (CR 706/707.2 — Altered Ego,
    /// Cursed Mirror): its `def` is overwritten with the copied creature's copyable `def`. When
    /// `until_eot`, the original `def` is stashed on [`Permanent::reverts_to_def_eot`] first and
    /// restored at cleanup ([`Event::TempBoostsEnded`], CR 514.2); otherwise the copy is
    /// indefinite (resets only when the object leaves the battlefield, CR 400.7). A copy is public
    /// battlefield status — the projected object's name/types change accordingly.
    BecameCopy {
        object: ObjectId,
        def: CardDef,
        until_eot: bool,
    },
    /// A permanent lost `keywords` until end of turn and can't have them, unioned onto
    /// [`Permanent::temp_lost_keywords`] (arcane_lighthouse's strip — see
    /// [`Effect::StripKeywordsFromOpponentsCreatures`]). Cleared alongside `temp_keywords` at
    /// [`Event::TempBoostsEnded`].
    KeywordsStripped {
        object: ObjectId,
        keywords: &'static [Keyword],
    },
    /// A one-shot control-changing effect (CR 720) took effect: `object` is now controlled by
    /// `controller` until end of turn (Besmirch). Read back by [`Game::controller_of`] via
    /// [`Game::control_overrides`], reverted by [`Event::ControlEndedUntilEndOfTurn`] at cleanup.
    ControlGainedUntilEndOfTurn {
        object: ObjectId,
        controller: PlayerId,
        source_name: &'static str,
    },
    /// An until-end-of-turn control override on `object` ended (cleanup, CR 514.2); control
    /// reverts to the owner (or an active `ControlAttached` Aura, if still attached).
    ControlEndedUntilEndOfTurn { object: ObjectId },
    /// `target` gained `source`'s other abilities until end of turn (CR 702.166 Backup — Guardian
    /// Scalelord's "it gains the following abilities until end of turn"). Recorded in
    /// [`Game::abilities_granted_until_eot`]; reverted at cleanup by
    /// [`Event::GrantedAbilitiesEnded`]. Public battlefield state.
    AbilitiesGranted { target: ObjectId, source: ObjectId },
    /// Every until-end-of-turn ability grant ended (cleanup, CR 514.2 / 702.166) — clears
    /// [`Game::abilities_granted_until_eot`]. Public battlefield state.
    GrantedAbilitiesEnded,
    /// A permanent control change with no stated duration (CR 720 — Entrancing Melody):
    /// `object` is now controlled by `controller`, with no cleanup reversion. Read back by
    /// [`Game::controller_of`] via [`Game::permanent_control_overrides`].
    ControlGained {
        object: ObjectId,
        controller: PlayerId,
    },
    /// A condition-scoped control-changing effect (CR 611.2b — Rubinia Soulsinger's "for as long
    /// as you control Rubinia and Rubinia remains tapped") took effect: `object` is now controlled
    /// by `controller` while `condition` holds. Read back by [`Game::controller_of`] via
    /// [`Game::conditioned_control_overrides`]; reverted automatically by
    /// [`Event::ConditionedControlEnded`] the moment the condition fails (a state-based check).
    ConditionedControlGained {
        object: ObjectId,
        controller: PlayerId,
        condition: crate::ControlCondition,
    },
    /// A condition-scoped control override on `object` ended because its condition no longer holds
    /// (the source untapped, left the battlefield, or changed controller — CR 611.2b); control
    /// reverts to the owner (or an active `ControlAttached` Aura, if still attached).
    ConditionedControlEnded { object: ObjectId },
    /// A creature was declared as an attacker, attacking `defender`.
    AttackerDeclared {
        object: ObjectId,
        defender: PlayerId,
    },
    /// A token was put onto the battlefield already tapped and attacking `defender` (Combat
    /// Calligrapher's minted Inkling), *not* via the declare-attackers step. CR 508.4: such a
    /// token was never declared as an attacker, so — unlike [`Event::AttackerDeclared`] — this
    /// event carries no trigger-scan arm and does not re-fire watch-attack triggers
    /// (`Trigger::Attacks` / `Trigger::PlayerAttacksYourOpponent`).
    TokenEnteredAttacking { token: ObjectId, defender: PlayerId },
    /// The creature `object` was goaded by player `by` (CR 701.38), until `by`'s next turn.
    Goaded {
        object: ObjectId,
        by: PlayerId,
        source_name: &'static str,
    },
    /// Every goad done by player `by` ended (the start of `by`'s turn — CR 701.38b).
    GoadCleared { by: PlayerId },
    /// A vow counter (CR 122.1 — Promise of Loyalty) was placed on `object`, marking `protected`
    /// as the player it "can't attack … for as long as it has a vow counter on it." Places one
    /// [`CounterKind::Vow`] counter and records `protected` on [`Permanent::vow_protected`].
    VowCountersPlaced {
        object: ObjectId,
        protected: PlayerId,
    },
    /// `count` time counters (CR 702.62 — suspend) were placed on the exiled card `card` as it
    /// was suspended (Rousing Refrain's self-exile, or a suspend cast). Recorded in
    /// [`Game::exile_time_counters`](crate::Game) keyed by the exile object.
    TimeCountersPlaced { card: ObjectId, count: u32 },
    /// One time counter was removed from the suspended card `card` (CR 702.62d — the upkeep
    /// turn-based tick). Decrements its [`Game::exile_time_counters`](crate::Game) entry.
    TimeCountersRemoved { card: ObjectId },
    /// The creature `object` must attack `defender` this turn if able (CR 508.1a "attacks … if
    /// able" — Furygale Flocking's minted tokens). Cleared at the next turn boundary, like goad.
    MustAttackDeclared {
        object: ObjectId,
        defender: PlayerId,
    },
    /// A CR 603.7 delayed triggered ability was scheduled by [`Effect::ScheduleAtNextUpkeep`]:
    /// `controller` will perform `effect` the next time a step matching `fire_at` begins.
    /// `source` is the scheduling ability's own source object, reused (still addressable via the
    /// [`Object::Moved`] chain even once it's left the stack) as the delayed ability's source
    /// when it fires.
    DelayedTriggerScheduled {
        controller: PlayerId,
        source: ObjectId,
        fire_at: Step,
        effect: Effect,
    },
    /// Every delayed trigger scheduled for `fire_at` fired at once (CR 603.7, drained in full the
    /// first time a step matching `fire_at` begins after scheduling).
    DelayedTriggersFired { fire_at: Step },
    /// A CR 603.7 delayed *one-shot* was armed by [`Effect::ScheduleNextCastTrigger`]:
    /// `controller` will perform `then` the next time they cast a spell matching `filter` this
    /// turn. `source` is the arming ability's own source object, reused as the delayed one-shot's
    /// source when it fires — same shape as `DelayedTriggerScheduled` above, event-armed rather
    /// than step-armed. See [`Game::fire_next_cast_triggers`].
    NextCastTriggerArmed {
        controller: PlayerId,
        source: ObjectId,
        filter: SpellFilter,
        then: &'static [Effect],
    },
    /// A [`NextCastTriggerArmed`](Self::NextCastTriggerArmed) watch fired (its filter matched a
    /// cast) and is removed — CR 603.7's "next" is at most once. An unconsumed watch's "this
    /// turn" expiry is a silent clear at the next turn's Untap step instead ([`Game::apply`]'s
    /// `Step::Untap` arm, mirroring how `spells_cast_this_turn` resets) — not an event of its own.
    NextCastTriggerConsumed {
        controller: PlayerId,
        source: ObjectId,
    },
    /// A CR 603.7 delayed watch was armed by [`Effect::ArmCombatDamageWatch`] (Stensian
    /// Sanguinist): `source` will become prepared the first time `watched` deals combat damage
    /// to a player, any time later this combat. `controller` is the arming ability's controller
    /// — same shape as [`NextCastTriggerArmed`](Self::NextCastTriggerArmed), object-scoped
    /// rather than filter-scoped. See [`Game::fire_combat_damage_watch_triggers`].
    CombatDamageWatchArmed {
        controller: PlayerId,
        source: ObjectId,
        watched: ObjectId,
    },
    /// A [`CombatDamageWatchArmed`](Self::CombatDamageWatchArmed) watch fired (`watched` dealt
    /// combat damage to a player) and is removed — CR 603.7's "this combat" is at most once. An
    /// unconsumed watch's "this combat" expiry is a silent clear at end of combat instead
    /// ([`Game::apply`]'s `Step::EndCombat` arm, mirroring `NextCastTriggerConsumed`'s own
    /// silent-Untap-clear note), not an event of its own.
    CombatDamageWatchConsumed {
        controller: PlayerId,
        source: ObjectId,
    },
    /// A CR 603.7 *repeatable* delayed watch was armed by
    /// [`Effect::ScheduleThisTurnCombatDamageCopy`] (Surge to Victory): every time a creature
    /// `controller` controls deals combat damage to a player for the rest of this turn, mint a
    /// free copy of `card` (an instant/sorcery card already exiled by this same resolution).
    /// `source` is the arming ability's own source (the resolving spell) — same shape as
    /// [`CombatDamageWatchArmed`](Self::CombatDamageWatchArmed), but controller-scoped rather
    /// than watching one chosen creature, and never removed on fire (CR "this turn" repeats,
    /// unlike `CombatDamageWatchArmed`'s "this combat" one-shot): an unconsumed watch's "this
    /// turn" expiry is a silent clear at the next turn's Untap step instead ([`Game::apply`]'s
    /// `Step::Untap` arm, mirroring `NextCastTriggerArmed`'s own silent-Untap-clear note). See
    /// [`Game::fire_combat_damage_copy_triggers`].
    CombatDamageCopyArmed {
        controller: PlayerId,
        source: ObjectId,
        card: ObjectId,
    },
    /// Impulse draw (CR 118.6): the top library card `from` was exiled face-up as the card `card`,
    /// and `player` may play it until end of turn (or until the end of `player`'s next turn, if
    /// `until_next_turn` — Atsushi, the Blazing Sky's die-trigger mode).
    ExiledFromLibraryMayPlay {
        player: PlayerId,
        card: ObjectId,
        from: ObjectId,
        until_next_turn: bool,
    },
    /// Herald of Amity's dig (CR 118.5 / 701.17): the top library card `from` was exiled — face-up
    /// unless `face_down` (Abstract Performance's first pile, CR 701.9 "exile a card face down" —
    /// hidden from everyone but its owner, see [`Card::face_down`]), as the card `card`, with no
    /// play permission attached (unlike [`ExiledFromLibraryMayPlay`](Self::ExiledFromLibraryMayPlay))
    /// — a choose-up-to-one over the batch decides afterward which one, if any, gets
    /// [`Event::CastFromExileFreePermissionGranted`].
    ExiledFromLibraryToChooseCastFree {
        player: PlayerId,
        card: ObjectId,
        from: ObjectId,
        face_down: bool,
    },
    /// An extended (`until_next_turn`) impulse-draw permission's shield expired: `card`'s
    /// controller's own next turn has begun (untap), so it now clears like a normal permission at
    /// this turn's cleanup — the arming half of [`Event::Goaded`]'s "until your next turn" idiom.
    PlayFromExilePermissionArmed { card: ObjectId },
    /// All impulse-draw play-until-end-of-turn permissions that aren't still shielded by
    /// `until_next_turn` expired (cleanup).
    PlayFromExileEnded,
    /// A creature was declared blocking `attacker`.
    BlockerDeclared {
        blocker: ObjectId,
        attacker: ObjectId,
    },
    /// A multi-blocked attacker's combat damage was divided among its blockers (the damage
    /// itself is dealt separately, in the combat-damage step).
    CombatDamageDivided {
        attacker: ObjectId,
        assignment: DamageAssignment,
    },
    /// A divided-damage spell's total was split among its chosen targets (CR 601.2d — see
    /// [`Effect::DealDamage`]'s `divided` field). Object shares are recorded on
    /// [`Spell::damage_division`], player shares on [`Spell::damage_division_players`] ("any number
    /// of targets" admits players); the damage itself is dealt separately, when each target's step
    /// resolves.
    SpellDamageDivided {
        spell: ObjectId,
        assignment: DamageAssignment,
        players: [Option<(PlayerId, i32)>; MAX_TARGETS],
    },
    /// A divided-counters spell's total was split among its chosen targets (CR 601.2d — see
    /// [`Effect::PutCounters`]'s `divided` field). Recorded on [`Spell::counter_division`]; the
    /// counters themselves are placed separately, when each target's step resolves.
    SpellCountersDivided {
        spell: ObjectId,
        assignment: DamageAssignment,
    },
    /// A permanent was dealt damage by a deathtouch source (lethal via SBA).
    DeathtouchMarked { object: ObjectId },
    /// Combat ended; the combat state is cleared.
    CombatCleared,
    /// A commander was cast from the command zone (raises that player's commander tax).
    CommanderCastFromCommandZone { player: PlayerId },
    /// `player` may cast spells this turn as though they had flash (CR 601.3a — Alchemist's
    /// Refuge). Sets [`Player::flash_permission_this_turn`]; cleared at the next Untap step.
    FlashPermissionGranted { player: PlayerId },
    /// `player` may, at mana-ability timing, pay 1 life to add {C} for the rest of the turn (CR
    /// 605 — Yavimaya Bloomsage's Channel). Sets
    /// [`Player::channel_colorless_mana_this_turn`]; cleared at the next Untap step.
    ChannelColorlessManaGranted { player: PlayerId },
    /// Combat damage dealt to a player by a commander, tracked per source.
    CommanderDamageDealt {
        source: ObjectId,
        player: PlayerId,
        amount: i32,
    },
    /// A creature dealt combat damage to a player (CR 510.2) — a marker distinct from the
    /// `LifeChanged` it accompanies in [`Game::damage_player`], since non-combat life loss
    /// (drain, pay-life) also emits `LifeChanged` but must not fire a
    /// [`Trigger::DealsCombatDamageToPlayer`] watch. See [`Game::queue_combat_damage_triggers`].
    CombatDamageDealtToPlayer {
        source: ObjectId,
        player: PlayerId,
        amount: i32,
    },
    /// `amount` combat damage that would have been dealt to `player` was prevented by a
    /// [`combat_damage_prevention_shields`](crate::state::CombatExtras::combat_damage_prevention_shields)
    /// entry (Inkshield, CR 615) — a marker replacing the `LifeChanged`/commander-damage this
    /// combat damage would otherwise have caused. The Inkling mints it drives ride in accompanying
    /// [`Self::TokenCreated`] events. Public — combat damage (and its prevention) is announced.
    CombatDamagePrevented { player: PlayerId, amount: i32 },
    /// `from` left play and became the command-zone card `card` (commander replacement).
    MovedToCommandZone { card: ObjectId, from: ObjectId },
    /// A player's mana pool emptied (a step or phase ended).
    ManaEmptied {
        player: PlayerId,
        /// Whether this boundary is the turn actually ending (CR 514.2 cleanup) rather than a
        /// mid-turn step/phase change — "until end of turn" persistent mana (CR 500.4 exception,
        /// [`Event::ManaAdded`]'s `persist`) survives every boundary except this one.
        end_of_turn: bool,
    },
    /// Marked damage was removed from a permanent (the cleanup step).
    DamageCleared { object: ObjectId },
    /// Mana was added to a player's pool (e.g. by tapping a land): `amount` of one `mana` kind.
    ManaAdded {
        player: PlayerId,
        mana: Mana,
        amount: u8,
        /// "Until end of turn, you don't lose this mana as steps and phases end" (CR 500.4
        /// exception; Rousing Refrain) — `true` mirrors this credit into
        /// [`Player::persistent_mana`](crate::state) so it survives mid-turn [`Event::ManaEmptied`]
        /// boundaries. `false` for every ordinary mana source.
        persist: bool,
    },
    /// Mana was removed from a player's pool to pay a cost (the exact multiset spent).
    ManaSpent { player: PlayerId, mana: ManaPool },
    /// A player passed priority.
    PriorityPassed { player: PlayerId },
    /// The spell `from` resolved and entered the battlefield as the permanent `permanent`.
    PermanentEntered { permanent: ObjectId, from: ObjectId },
    /// A graveyard card `from` was put onto the battlefield under `controller`'s control as the
    /// permanent `permanent` (reanimation). Mirrors [`Self::PermanentEntered`] but its source is
    /// a graveyard `Card` (not a stack spell) and it carries the new controller — which can differ
    /// from the card's owner. Fires ETB triggers just like a normal enter.
    ReanimatedToBattlefield {
        permanent: ObjectId,
        from: ObjectId,
        controller: PlayerId,
        /// Whether the entering permanent gets a finality counter (CR 614.12 — a permanent with
        /// one that would die is exiled instead), mirroring the triggering
        /// [`Effect::ReanimateToBattlefield`]'s `finality` field.
        finality: bool,
        /// Whether the entering permanent is tapped, mirroring
        /// [`Effect::ReturnThisFromGraveyardToBattlefield`]'s `tapped` field (Teacher's Pest's
        /// "... to the battlefield tapped").
        tapped: bool,
    },
    /// A token entered the battlefield under `controller`'s control as object `token`.
    /// Unlike [`Self::PermanentEntered`] it has no source card — its characteristics come
    /// from the inline `def`.
    TokenCreated {
        token: ObjectId,
        controller: PlayerId,
        def: CardDef,
    },
    /// A token left the battlefield and ceased to exist (CR 111.7) — a state-based action.
    /// Carries the token's `controller`/`def` (as [`Self::TokenCreated`] does) so its
    /// "when this dies" trigger can still be built after the arena slot is gone.
    TokenCeasedToExist {
        token: ObjectId,
        controller: PlayerId,
        def: CardDef,
    },
    /// Damage was marked on a permanent. `source` is what dealt it (a spell/ability/attacker),
    /// carried for the game log; `None` for engine-internal adjustments.
    DamageMarked {
        object: ObjectId,
        amount: i32,
        source: Option<ObjectId>,
    },
    /// `from` was moved to the graveyard (by resolution or an SBA) as the card `card`.
    MovedToGraveyard { card: ObjectId, from: ObjectId },
    /// `from` left the battlefield to its owner's Exile zone as the card `card`. Emitters should
    /// go through `Game::exile_or_command`, which redirects a commander to the command zone
    /// instead (CR 903.9b) rather than emitting this event directly.
    MovedToExile { card: ObjectId, from: ObjectId },
    /// An adventure spell (CR 715) finished resolving: its card `from` (the spell on the stack) is
    /// exiled "on an adventure" as the card `card`, carrying the *creature* front face (recorded in
    /// [`PlayPermissions::adventure_fronts`](crate::state::PlayPermissions)) rather than the spent
    /// adventure face. An open-ended cast-from-exile permission is granted so the owner may cast the
    /// creature half from exile later at normal cost (CR 715.3d). Distinct from
    /// [`Self::MovedToExile`], which restores the spell's own def.
    ExiledOnAdventure {
        card: ObjectId,
        from: ObjectId,
        owner: PlayerId,
    },
    /// The O-Ring pattern (CR 603.6e): `object` (an exile-zone card, minted by a preceding
    /// [`Event::MovedToExile`]/[`Event::MovedToCommandZone`] in the same batch) was exiled "until
    /// `source` leaves the battlefield" — recorded in [`Game::exiled_until_source_leaves`] so
    /// [`Game::check_linked_exile_returns`] can return it once `source` does.
    ExiledUntilSourceLeaves { source: ObjectId, object: ObjectId },
    /// Skyclave Apparition's linked exile (a sibling of [`Self::ExiledUntilSourceLeaves`]):
    /// `object` (an exile-zone card, minted by a preceding [`Self::MovedToExile`]/
    /// [`Self::MovedToCommandZone`] in the same batch) was exiled linked to `source` — recorded
    /// in [`Game::exile_links`]'s `illusion_on_source_leave` so
    /// [`Game::check_leaves_battlefield_illusions`] can mint the exiled card's owner an Illusion
    /// once `source` leaves the battlefield. The card is never returned (contrast
    /// `ExiledUntilSourceLeaves`'s `ReturnedFromLinkedExile`).
    ExiledUntilSourceLeavesMintingIllusion { source: ObjectId, object: ObjectId },
    /// The other half of Skyclave's pattern: `source`'s linked exile finished minting its
    /// Illusion (see [`Event::TokenCreated`] emitted alongside this one at the same call site) —
    /// drops the now-spent `(source, object)` entry from `illusion_on_source_leave` so it fires
    /// exactly once (unlike the O-Ring return, the exiled card never leaves `Zone::Exile`, so
    /// there's no zone-change guard to stop a re-fire on the next sweep).
    LeavesIllusionMinted { source: ObjectId, object: ObjectId },
    /// Hofri Ghostforge's minted Spirit token: `token` (minted alongside this event by the same
    /// [`Effect::ExileDeadCreatureCreateCopyWithSubtype`] resolution, `leaves_returns_exiled`
    /// set) gains a granted "When this token leaves the battlefield, return the exiled card to
    /// its owner's graveyard" rider baking in `exiled` — recorded in [`Game::exile_links`]'s
    /// `token_leaves_returns_exiled` so [`Game::queue_token_return_exiled_trigger`] can place a
    /// real [`Trigger::ThisPermanentLeavesBattlefield`] triggered ability for `token` once it
    /// leaves, unlike [`Self::ExiledUntilSourceLeavesMintingIllusion`]'s SBA-style departure
    /// sweep.
    TokenGrantedReturnExiledOnLeave { token: ObjectId, exiled: ObjectId },
    /// The granted rider's payoff (Hofri Ghostforge): `from` (an exile-zone card) arrives in its
    /// owner's graveyard as the card `card`. Deliberately its own event, not
    /// [`Self::MovedToGraveyard`] — CR 700.4 "died" is specifically "put into a graveyard *from
    /// the battlefield*", and `from` here is an exile card, not a battlefield permanent; reusing
    /// `MovedToGraveyard` would falsely re-fire `Trigger::Dies`/creature-you-control-dies watchers
    /// (including the minting Hofri's own) off a card that never died. Mirrors [`Self::Milled`]'s
    /// same reasoning for a library-to-graveyard arrival.
    ReturnedExiledCardToGraveyard { card: ObjectId, from: ObjectId },
    /// `object` (an exile-zone card, minted by a preceding [`Self::MovedToExile`]/
    /// [`Event::MovedToCommandZone`] in the same batch) was exiled "with" `source` (CR 400.10a),
    /// linking it to that ability's own source — recorded in [`Game::exiled_with`], a sibling pile
    /// to `exiled_until_source_leaves` that's drained by a *different* ability of `source`, not by
    /// `source` leaving the battlefield. Currency Converter's discard-trigger payoff.
    ExiledWithSource { source: ObjectId, object: ObjectId },
    /// The other half: `source`'s cash-out ability pulled `object` back out of the pile recorded
    /// by [`Event::ExiledWithSource`] — the linked exile ended by choice, not by `source` leaving.
    /// The actual zone move (to `object`'s owner's graveyard) is a separate event emitted
    /// alongside this one at the same call site.
    CardExiledWithSourceLeftExile { source: ObjectId, object: ObjectId },
    /// Quintorius, Loremaster's activated ability granted `player` permission to cast `card`
    /// (a card in its exiled-with pile) this turn without paying its mana cost (CR 118.5),
    /// recorded in [`crate::state::PlayPermissions::cast_from_exile_free`]. The card stays in
    /// exile — this doesn't touch [`Event::ExiledWithSource`]'s pile.
    CastFromExileFreePermissionGranted { card: ObjectId, player: PlayerId },
    /// Quintorius, Loremaster's activated ability also grants `card` a one-shot replacement
    /// (CR 614.6): "If that spell would be put into a graveyard, put it on the bottom of its
    /// owner's library instead." Recorded alongside
    /// [`Self::CastFromExileFreePermissionGranted`] in
    /// [`crate::state::PlayPermissions::stack_object_bottoms_library_on_leave`], read by the two
    /// stack-to-graveyard chokes ([`Game::finish_instant_sorcery_resolution`],
    /// [`Game::counter_spell`]). Scoped to Quintorius alone — the other
    /// [`Self::CastFromExileFreePermissionGranted`] emitters (Herald of Amity, Dance with
    /// Calamity) never emit this sibling event.
    CastFromExileFreeBottomsLibraryOnLeave { card: ObjectId },
    /// Every active free-cast-from-exile permission expired (cleanup, CR 118.5 "this turn").
    CastFromExileFreeEnded,
    /// The other half of the O-Ring pattern: `source`'s linked exile ended (it left the
    /// battlefield), so the card it exiled — `from` — returns to the battlefield as the fresh
    /// permanent `permanent`, under its owner's control (`controller`), per CR 603.6e. Fires ETB
    /// triggers like any other entry. Emitted by [`Game::check_linked_exile_returns`], swept
    /// alongside state-based actions rather than placed on the stack — the return isn't a
    /// triggered ability (it can't be responded to).
    ReturnedFromLinkedExile {
        permanent: ObjectId,
        from: ObjectId,
        controller: PlayerId,
        source: ObjectId,
    },
    /// A flicker (CR 400.7 — a new object, [`Effect::FlickerTarget`]/
    /// [`Effect::ReturnFlickeredCard`]): the exiled card `from` returns to the battlefield as the
    /// fresh permanent `permanent`, under its owner's control (`controller`) — same shape as
    /// [`Self::ReturnedFromLinkedExile`], but unconditioned on any other permanent leaving. Fires
    /// ETB triggers like any other entry.
    FlickeredToBattlefield {
        permanent: ObjectId,
        from: ObjectId,
        controller: PlayerId,
    },
    /// `from` was returned from the battlefield to its owner's hand as the card `card` (bounce).
    ReturnedToHand { card: ObjectId, from: ObjectId },
    /// `from` was put into its owner's library as the new library card `card` — the bottom
    /// (Mistveil Plains's graveyard tuck, Chaos Warp's battlefield tuck, Quintorius, Loremaster's
    /// [`Self::CastFromExileFreeBottomsLibraryOnLeave`] redirect off the stack), or the top when
    /// `to_top` is set (Mystic Sanctuary's).
    TuckedToLibrary {
        card: ObjectId,
        from: ObjectId,
        to_top: bool,
    },
    /// `player`'s library was shuffled ([`Effect::ShuffleTargetCardsFromGraveyardIntoLibrary`]'s
    /// mandatory shuffle after cards enter it — CR 701.19-style). The resulting order is a hidden
    /// zone's contents, so this event carries no order information, just that a shuffle happened.
    LibraryShuffled { player: PlayerId },
    /// The top card of `player`'s library was revealed (CR 701.30) — public to every player,
    /// unlike a private look ([`Effect::LookAtTop`]). `card` stays where it is (still the top of
    /// the library); a reveal is not itself a zone change, so a later event moves the card if
    /// one does (Goblin Guide's [`Effect::RevealTopToHand`], Keen Duelist's
    /// [`Effect::RevealTopAndDrainMutual`]).
    RevealedTopOfLibrary {
        player: PlayerId,
        card: ObjectId,
        def: CardDef,
    },
    /// A previously-revealed card ([`Self::RevealedTopOfLibrary`]) went to the bottom of its own
    /// owner's library (Open the Way's non-matching reveals, CR 701.30-adjacent). Not a zone
    /// change — the card stays in its library, just reordered — so `card` keeps its object
    /// identity (CR 400.7 mints a new object only on a zone change).
    PutOnBottomOfLibrary { player: PlayerId, card: ObjectId },
    /// A found library card `from` was put into `player`'s hand as the new hand object `object`
    /// (a tutor resolving). Mirrors [`Self::CardDrawn`] — it carries the card's `def` so the
    /// redaction layer can hide the identity from everyone but the searcher.
    SearchedToHand {
        player: PlayerId,
        object: ObjectId,
        from: ObjectId,
        card: CardDef,
    },
    /// A found library card `from` was put onto the battlefield under `controller`'s control as the
    /// permanent `permanent` (ramp / fetchland resolving), `tapped` if it enters tapped. Fires ETB
    /// triggers like any other enter.
    SearchedToBattlefield {
        permanent: ObjectId,
        from: ObjectId,
        controller: PlayerId,
        tapped: bool,
    },
    /// A library card `from` was manifested (CR 701.34) — put onto the battlefield face down as
    /// the 2/2 permanent `permanent` under `controller`'s control ([`Permanent::face_down`]). The
    /// moved card's identity is private (it comes off the private library), so the `VisibleEvent`
    /// counterpart drops `from` the way `Sacrificed`/`TokenCeasedToExist` drop hidden fields.
    Manifested {
        permanent: ObjectId,
        from: ObjectId,
        controller: PlayerId,
    },
    /// A face-down permanent was turned face up (CR 701.34e — the turn-face-up special action):
    /// its [`Permanent::face_down`] flag is cleared, revealing its real card. Public — the reveal
    /// is public game information (CR 707.9a).
    TurnedFaceUp { permanent: ObjectId },
    /// A hand land card `from` was put onto the battlefield under `controller`'s control as the
    /// permanent `permanent` (CR 305.9 "put onto the battlefield" — Eureka Moment, Zimone), tapped
    /// iff `tapped`. Distinct from [`Self::LandPlayed`]: this is a special action/effect, not
    /// "playing a land" — it doesn't touch the once-per-turn land drop. Fires ETB triggers like
    /// any other enter (see [`Game::enqueue_triggers`]).
    PutOntoBattlefieldFromHand {
        permanent: ObjectId,
        from: ObjectId,
        controller: PlayerId,
        tapped: bool,
    },
    /// `player` milled the library card `from` into their graveyard as the card `card`.
    Milled {
        player: PlayerId,
        card: ObjectId,
        from: ObjectId,
    },
    /// A player's life total changed by `amount` (negative = lost life). `source` is what
    /// caused it (an attacker, a life-gain effect) for the log; `None` for setup adjustments.
    LifeChanged {
        player: PlayerId,
        amount: i32,
        source: Option<ObjectId>,
    },
    /// A player tried to draw from an empty library; they lose on the next SBA sweep.
    DrewFromEmptyLibrary { player: PlayerId },
    /// A player lost the game (a state-based action; e.g. life <= 0).
    PlayerLost { player: PlayerId },
    /// `player` got the city's blessing (CR 702.131 Ascend) — a state-based action fired once
    /// they control ten or more permanents. Fully public: sticky for the rest of the game.
    CitysBlessingGained { player: PlayerId },
    /// A player drew a card. Full information (canonical); the card's identity is
    /// private and gets hidden from other players by the redaction layer
    /// (`schema::to_visible_event`), not here — this event stays canonical/unredacted
    /// so replays and the server's own state stay fully informed.
    CardDrawn {
        player: PlayerId,
        /// The new hand-object id.
        object: ObjectId,
        /// The library-object id it came from.
        from: ObjectId,
        card: CardDef,
    },
    /// A permanent was sacrificed (CR 701.20): `by` is the player who sacrificed it, `def` its
    /// card definition. Emitted alongside the graveyard/command-zone/vanish event a sacrifice
    /// always produces ([`Game::sacrifice_event`]) so `Trigger::YouSacrifice`/
    /// `Trigger::AnyPlayerSacrifices` can watch for a *sacrifice* specifically, as distinct from
    /// any other death. `def` is carried directly (rather than re-read at trigger-scan time)
    /// because a sacrificed token's arena slot is already gone by then (like
    /// [`Self::TokenCeasedToExist`]).
    Sacrificed {
        object: ObjectId,
        by: PlayerId,
        def: CardDef,
    },
    /// A card was discarded (CR 701.8): `card` is its new graveyard-object id (the same id
    /// `MovedToGraveyard.card` mints), `from` the hand-object id, `player` who discarded it,
    /// `def` its card definition. Emitted alongside the `MovedToGraveyard` a discard always
    /// produces (an effect discard, a discard-cost payment, or the cleanup hand-size trim) so
    /// `Trigger::YouDiscard` can watch for a *discard* specifically, as distinct from any other
    /// graveyard arrival — mirrors [`Self::Sacrificed`] riding alongside its own zone-change
    /// event. `def` is carried directly (not re-read at trigger-scan time) for the same reason
    /// `Sacrificed` carries it.
    /// ponytail: a marker riding the graveyard move, not a new zone transition.
    Discarded {
        card: ObjectId,
        from: ObjectId,
        def: CardDef,
        player: PlayerId,
    },
    /// A `Trigger::YouDiscard` payoff (CR 601 impulse play): the graveyard card `from` was
    /// exiled face-up as the card `card`, and `player` may play it until end of turn. Public —
    /// the card is exiled face-up, like [`Self::ExiledFromLibraryMayPlay`], which this mirrors
    /// (graveyard source instead of library).
    ExiledFromGraveyardMayPlay {
        player: PlayerId,
        card: ObjectId,
        from: ObjectId,
    },
}

/// Why an [`Intent`] was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reject {
    /// The object isn't in a zone/state from which this action is legal.
    NotCastable,
    /// The acting player does not currently hold priority.
    NotYourPriority,
    /// The player's mana pool can't cover the spell's cost.
    CannotPayCost,
    /// The object can't produce mana this way (not an untapped land under control).
    CannotProduceMana,
    /// The ability can't be activated (no such ability, sick, tapped, or unaffordable).
    CannotActivate,
    /// An attacker/blocker declaration was illegal (wrong step, actor, or an illegal block).
    IllegalDeclaration,
    /// The chosen target is missing or illegal for what's being cast.
    IllegalTarget,
    /// The chosen mode of a modal spell is missing, out of range, or given for a non-modal
    /// spell (CR 700.2 — a modal spell chooses exactly one of its modes at cast).
    IllegalMode,
    /// The action can't be taken at this time (e.g. a sorcery-speed spell mid-combat).
    WrongTiming,
    /// The engine is waiting on a pending choice; resolve it before acting.
    ChoicePending,
    /// The answer to a pending choice was malformed (not a valid permutation, etc.).
    IllegalChoice,
    /// The intent referenced an object id that doesn't exist (an out-of-range id from an
    /// untrusted client). Rejected up front so a bad id can't panic the engine.
    UnknownObject,
    /// An [`Intent::TakeAction`] named an action id absent from the stored legal-action list
    /// (unknown, stale, or another player's action). Every refresh mints fresh ids, so a stale
    /// id is impossible-by-construction to mistake for a live one — it simply isn't found.
    UnknownAction,
}

/// One *meaningful action* — a play worth stopping priority for (ADR 0007). Enumerated by
/// [`Game::meaningful_actions`]; see it for the deliberate scoping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeaningfulAction {
    /// Play `card` as this turn's land drop, from `zone` (hand, or exile with permission).
    PlayLand { card: ObjectId, zone: Zone },
    /// Cast `card` from `zone` (hand, the command zone, or exile with permission). The zone
    /// lets a client bucket the action by origin (Hand / Command / Exile).
    Cast { card: ObjectId, zone: Zone },
    /// Activate the non-mana activated ability at index `ability` on `source`.
    Activate { source: ObjectId, ability: usize },
    /// Cycle `card` from hand (CR 702.29a): pay its cycling cost, discard it, draw one.
    Cycle { card: ObjectId },
    /// Activate `card`'s [`CardDef::hand_ability`] (CR 113.6/602.5e): pay its cost, discard it,
    /// run its authored effects. The general sibling of [`Self::Cycle`].
    ActivateHandAbility { card: ObjectId },
    /// Suspend `card` from hand (CR 702.62): pay its suspend cost, exile it with time counters.
    Suspend { card: ObjectId },
    /// Encore `card` from the graveyard (CR 702.140): pay its encore mana cost, exile it, and mint
    /// a must-attack haste token copy per opponent. A sorcery-speed special action.
    Encore { card: ObjectId },
    /// Turn the face-down manifested `permanent` face up (CR 701.34e): pay its hidden creature
    /// card's mana cost. A special action (no stack), offered only while its hidden card is a
    /// creature and its controller can pay the cost.
    TurnFaceUp { permanent: ObjectId },
    /// Cast a prepared permanent's back-face spell (soc/sos prepare DFCs).
    CastPrepared { source: ObjectId },
    /// Cast `card` from hand face down as a 2/2 for {3} (CR 702.37b — morph). Offered only for a
    /// hand card whose [`CardDef::morph`] is `Some` and whose controller can pay the {3}.
    CastFaceDown { card: ObjectId },
    /// Declare attackers: the player has a creature able to attack this combat.
    DeclareAttackers,
    /// Declare blocks: the player has a creature able to block an attacker this combat.
    DeclareBlockers,
}

/// One entry in the engine's stored per-player legal-action list ([`Game::legal_actions`]).
/// `kind` reuses [`MeaningfulAction`]; `id` is unique per game and monotonic. An action that
/// survives a state change keeps its id across the refresh (so a client's held id stays valid
/// while the action remains legal); an action that vanishes retires its id forever, so a dead
/// id can never collide with a live one (it simply won't be found — see [`Intent::TakeAction`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegalAction {
    pub id: u64,
    pub player: PlayerId,
    pub kind: MeaningfulAction,
}

/// Whether `top` and `bottom` together are exactly the cards in `cards` (a valid split for a
/// scry/surveil answer): every shown card assigned once, nothing extra.
pub(crate) fn is_partition(top: &[ObjectId], bottom: &[ObjectId], cards: &[ObjectId]) -> bool {
    if top.len() + bottom.len() != cards.len() {
        return false;
    }
    let mut combined: Vec<ObjectId> = top.iter().chain(bottom).copied().collect();
    let mut expected = cards.to_vec();
    combined.sort_unstable();
    expected.sort_unstable();
    combined == expected
}

/// How many distinct targets an effect chooses (CR 601.2c): between `min` and `max`, inclusive.
/// The default `{1, 1}` is the ubiquitous single mandatory target, so every existing effect is
/// untouched. `count = N` in TOML is sugar for `{N, N}` (an exact "N target"); an explicit
/// `{ min, max }` spells "up to"/"one or two" ranges (see `de::TargetCount`).
/// ponytail: scalar `u8`s, so `CardDef` stays `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetCount {
    pub min: u8,
    pub max: u8,
    /// When `true`, `min`/`max` are placeholders substituted at cast time by the spell's own
    /// chosen `{X}` (CR 601.2b — X is fixed before targets are chosen): `Game::choose_spell_targets`
    /// reads the spell's `x` and overrides the effective count, per the rule on that field's own
    /// doc. `{ min: 0, max: 0, x_scaled: true }` is "up to X target(s)" (Silkguard, "up to X" —
    /// fully declinable); `{ min: 1, max: 1, x_scaled: true }` is "exactly X target(s)" (Curse of
    /// the Swine's "exile X target creatures" — X could be 0, but the caster can't decline once
    /// X is chosen positive since `min` only zeroes out when the printed `min` is 0). Defaults to
    /// `false` (every other multi-target effect keeps a fixed count). Parsed by the hand-written
    /// `Deserialize` impl in `de.rs` (not a derive), so no `serde` attribute belongs here.
    pub x_scaled: bool,
    /// The sibling of [`Self::x_scaled`] for a card whose X is never chosen as `{X}` but is
    /// instead *defined* by an additional cost (CR 601.2b/601.2f) — Immoral Bargain's "As an
    /// additional cost to cast this spell, sacrifice X creatures. Destroy X target nonland
    /// permanents." When `true`, `min`/`max` are placeholders `Game::choose_spell_targets`
    /// substitutes at cast time with [`Game::spell_sacrifice_count`] (always "exactly X", unlike
    /// `x_scaled`'s declinable "up to X" case — no pool card sacrifice-scales an optional count).
    /// Defaults to `false`. Parsed by the hand-written `Deserialize` impl in `de.rs`.
    pub sacrifice_scaled: bool,
    /// Strive's own sibling of [`Self::sacrifice_scaled`] (CR 601.2c/601.2f/702.42) — Twinflame's
    /// "Choose any number of target creatures you control" paired with "This spell costs {2}{R}
    /// more to cast for each target beyond the first." Unlike `sacrifice_scaled` (whose X is the
    /// count of permanents already paid as a cost), Strive's target count is a bare number the
    /// caster commits to *before* the stack (CR 601.2c precedes 601.2f) — carried on
    /// [`crate::Intent::Cast`] and recorded as [`crate::types::card::Spell::strive_count`] (read via
    /// [`crate::Game::spell_strive_count`]). When `true`, `min`/`max` are placeholders
    /// [`Game::choose_spell_targets`](crate::Game::choose_spell_targets) substitutes at cast time
    /// with that declared count (always "exactly N," like `sacrifice_scaled`'s "exactly X").
    /// Defaults to `false`. Parsed by the hand-written `Deserialize` impl in `de.rs`.
    pub strive_scaled: bool,
}

impl Default for TargetCount {
    fn default() -> Self {
        TargetCount {
            min: 1,
            max: 1,
            x_scaled: false,
            sacrifice_scaled: false,
            strive_scaled: false,
        }
    }
}

impl TargetCount {
    /// Whether this is the ubiquitous single-mandatory-target count — the fast path that keeps
    /// every existing spell on the untouched single-target plumbing. An `x_scaled`,
    /// `sacrifice_scaled`, or `strive_scaled` count is never single even when its printed
    /// `{min, max}` happens to be `{1, 1}` — its *effective* count depends on a cast-time
    /// choice/cost and must go through the multi-target machinery.
    pub(crate) fn is_single(self) -> bool {
        self == TargetCount::default()
            && !self.x_scaled
            && !self.sacrifice_scaled
            && !self.strive_scaled
    }
}

/// The most targets any multi-target spell in the pool chooses (Aether Gale's six). Bounds the
/// fixed, `Copy` array in [`TargetList`] so `Spell`/`Event` stay `Copy` (the id-indexed object
/// arena requires it), mirroring [`MAX_MODES`]. ponytail: bump when a card targets more.
pub(crate) const MAX_TARGETS: usize = 6;

/// A spell's chosen targets (CR 601.2c), in the order chosen. A single-target spell fills just
/// `[0]`; a multi-target spell (Aether Gale) fills up to [`MAX_TARGETS`]. `Copy` so `Spell`/
/// `Event::SpellCast` stay `Copy`, following the [`Modes`] fixed-array precedent. `pub` (not
/// `pub(crate)`): the `schema` crate projects the stack view via [`Self::iter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TargetList {
    targets: [Option<Target>; MAX_TARGETS],
}

impl TargetList {
    /// A one-element list (the single-target case) — or an empty list for `None`.
    pub(crate) fn single(target: Option<Target>) -> Self {
        let mut list = TargetList::default();
        list.targets[0] = target;
        list
    }

    /// Build from the targets chosen at cast. Any beyond [`MAX_TARGETS`] are dropped (the cast
    /// gate has already bounded the count to the effect's `max`, itself `<= MAX_TARGETS`).
    pub(crate) fn from_targets(chosen: &[Target]) -> Self {
        let mut list = TargetList::default();
        for (slot, &target) in list.targets.iter_mut().zip(chosen) {
            *slot = Some(target);
        }
        list
    }

    /// The first chosen target, for single-target readers and the stack snapshot.
    pub(crate) fn primary(&self) -> Option<Target> {
        self.targets.iter().copied().flatten().next()
    }

    /// The chosen targets, in order.
    pub fn iter(&self) -> impl Iterator<Item = Target> + '_ {
        self.targets.iter().copied().flatten()
    }
}

/// The most modes any modal card in the pool prints (Casualties of War's five). Bounds the
/// fixed, `Copy` per-mode array in [`Modes`] so `Spell`/`Event` stay `Copy` (the id-indexed
/// object arena requires it). ponytail: bump when a card prints more modes.
pub(crate) const MAX_MODES: usize = 5;

/// A modal spell's chosen modes (CR 700.2). Indexed by printed mode: `chosen[i] == Some(target)`
/// means mode `i` was chosen (its `target` is that mode's target, or `None` if the mode needs
/// none); `chosen[i] == None` means mode `i` wasn't chosen. Chosen modes resolve in printed
/// (index) order. All-`None` for a non-modal spell (which uses [`Spell::target`] and runs every
/// effect). `Copy` so `Spell`/`Event::SpellCast` stay `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modes {
    pub(crate) chosen: [Option<Option<Target>>; MAX_MODES],
}

impl Modes {
    /// Build from the `(printed-mode index, target)` pairs chosen at cast. The cast path validates
    /// the indices are in range and distinct before calling this.
    pub(crate) fn from_choices(choices: &[(usize, Option<Target>)]) -> Self {
        let mut modes = Modes::default();
        for &(i, target) in choices {
            modes.chosen[i] = Some(target);
        }
        modes
    }

    /// The chosen modes as `(printed-mode index, target)`, in printed (resolution) order.
    pub(crate) fn chosen(&self) -> impl Iterator<Item = (usize, Option<Target>)> + '_ {
        self.chosen
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.map(|target| (i, target)))
    }

    /// A representative target for display (the first chosen mode's target); `None` if no chosen
    /// mode targets. Used only for the stack snapshot of a modal spell.
    pub(crate) fn first_target(&self) -> Option<Target> {
        self.chosen().find_map(|(_, target)| target)
    }
}

/// The most blockers a single gang-blocked attacker's damage division needs to remember in one
/// [`Event::CombatDamageDivided`]. Bounds the fixed, `Copy` per-blocker array in
/// [`DamageAssignment`] so `Event` stays `Copy` (the id-indexed object arena requires it).
/// ponytail: bump if a pool board ever gang-blocks one attacker with more bodies than this.
pub(crate) const MAX_BLOCKERS: usize = 8;

/// How a multi-blocked attacker's combat damage is divided among its blockers (CR 510.1c).
/// `Copy` so `Event::CombatDamageDivided` stays `Copy`; see [`MAX_BLOCKERS`]. `pub` (not
/// `pub(crate)`): the `schema` crate's redaction reads it back out via [`Self::pairs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DamageAssignment {
    slots: [Option<(ObjectId, i32)>; MAX_BLOCKERS],
}

impl DamageAssignment {
    /// Build from `(blocker, amount)` pairs. Callers must have already checked
    /// `pairs.len() <= MAX_BLOCKERS`; any pair beyond the ceiling is silently dropped.
    pub(crate) fn from_pairs(pairs: &[(ObjectId, i32)]) -> Self {
        let mut assignment = DamageAssignment::default();
        for (slot, &pair) in assignment.slots.iter_mut().zip(pairs) {
            *slot = Some(pair);
        }
        assignment
    }

    /// The `(blocker, amount)` pairs, in the order they were assigned.
    pub fn pairs(&self) -> Vec<(ObjectId, i32)> {
        self.slots.into_iter().flatten().collect()
    }
}

/// The `mode`-th mode of a modal "choose one" spell (CR 700.2): its `Timing::Spell` abilities
/// are its modes, in card order. `None` if `mode` is out of range. For a non-modal card these are
/// simply its spell effects — which all run at resolution rather than being chosen among.
/// One printed mode of a modal spell, as [`Game::modes_of`] reports it. `needs_target` and an empty
/// `targets` differ: the mode wants a target but none is legal, so it can't be chosen right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeInfo {
    pub label: String,
    pub needs_target: bool,
    pub targets: Vec<Target>,
}

pub(crate) fn nth_mode(def: CardDef, mode: usize) -> Option<Ability> {
    def.abilities
        .iter()
        .copied()
        .filter(|a| matches!(a.timing, Timing::Spell))
        .nth(mode)
}

/// The `(amount, kind)` of a card's "enters with N counters" static ability (a hydra's
/// `(Amount::X, None)` for +1/+1; mana_bloom/astral_cornucopia's `(Amount::X, Some(Charge))` for
/// a named kind), if it has one. `None` for a card that enters with no counters.
pub(crate) fn enters_with_counters(def: CardDef) -> Option<(Amount, Option<CounterKind>)> {
    def.abilities
        .iter()
        .find_map(|a| match (a.timing, a.effect) {
            (Timing::Static, Effect::EntersWithCounters { amount, kind }) => Some((amount, kind)),
            _ => None,
        })
}

/// Whether `order` is a permutation of `0..len` (each index present exactly once).
pub(crate) fn is_permutation(order: &[usize], len: usize) -> bool {
    if order.len() != len {
        return false;
    }
    let mut seen = vec![false; len];
    for &i in order {
        if i >= len || seen[i] {
            return false;
        }
        seen[i] = true;
    }
    true
}
