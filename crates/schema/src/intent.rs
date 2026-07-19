//! The client → server intent surface: [`IntentEnvelope`], its wire-form payload types, and
//! [`to_intent`], which maps a received `WireIntent` back into [`engine::Intent`].

use serde::{Deserialize, Serialize};

use crate::ObjectId;

/// A player's requested action (client → server, POST body payload).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentEnvelope {
    pub table_id: String,
    /// Client-assigned sequence number, for ordering/idempotency.
    pub client_seq: u64,
    pub intent: WireIntent,
}

/// Wire form of [`engine::Target`]: a chosen target is either a permanent (`object`) or a
/// player. A named, tagged enum so the generated TypeScript stays a discriminated union.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WireTarget {
    Object { id: ObjectId },
    Player { player: u8 },
}

impl WireTarget {
    /// From an engine target.
    pub(crate) fn of(target: engine::Target) -> WireTarget {
        match target {
            engine::Target::Object(id) => WireTarget::Object { id },
            engine::Target::Player(p) => WireTarget::Player { player: p.0 },
        }
    }

    /// Into an engine target.
    fn to_engine(self) -> engine::Target {
        match self {
            WireTarget::Object { id } => engine::Target::Object(id),
            WireTarget::Player { player } => engine::Target::Player(engine::PlayerId(player)),
        }
    }
}

/// One declared block: `blocker` blocks `attacker`. (A struct rather than a tuple so the
/// generated TypeScript/OpenAPI stays a named object.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireBlock {
    pub blocker: ObjectId,
    pub attacker: ObjectId,
}

/// One declared attack: `attacker` attacks player `defender`. (A named struct so the
/// generated TypeScript/OpenAPI stays an object, like [`WireBlock`].)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireAttack {
    pub attacker: ObjectId,
    pub defender: u8,
}

/// One combat-damage assignment: `amount` of an attacker's damage dealt to `blocker`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireDamage {
    pub blocker: ObjectId,
    pub amount: i32,
}

/// One share of a divided-damage spell's total: `amount` dealt to `target` (CR 601.2d). Keyed by
/// [`WireTarget`], not a bare object id, because "any number of targets" may include a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireSpellDamage {
    pub target: WireTarget,
    pub amount: i32,
}

/// One chosen mode of a modal spell (CR 700.2): the printed-mode `index` and that mode's
/// `target` (absent if the mode needs none). A named struct so the generated
/// TypeScript/OpenAPI stays an object, like [`WireBlock`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireModeChoice {
    pub index: u32,
    #[serde(default)]
    pub target: Option<WireTarget>,
}

/// Wire form of [`engine::Intent`] — the full player action surface. `to_intent` maps a
/// `WireIntent` back to the engine's `Intent` (the inbound counterpart of redaction).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WireIntent {
    Cast {
        player: u8,
        object: ObjectId,
        target: Option<WireTarget>,
        /// The chosen `{X}` value (absent/0 for a spell with no `{X}`).
        #[serde(default)]
        x: u32,
        /// A modal spell's chosen modes (CR 700.2), each with its own target; empty for a
        /// non-modal spell (which uses `target`). See [`engine::Intent::Cast`].
        #[serde(default)]
        modes: Vec<WireModeChoice>,
        /// Hand cards chosen to pay the spell's additional discard cost (CR 601.2f); empty for
        /// a spell with none. See [`engine::Intent::Cast`].
        #[serde(default)]
        discard_cost: Vec<ObjectId>,
        /// Graveyard cards chosen to pay a delve or escape graveyard-exile cost; empty for a
        /// spell with neither. See [`engine::Intent::Cast`].
        #[serde(default)]
        graveyard_exile: Vec<ObjectId>,
        /// Permanents chosen to pay the spell's additional sacrifice cost; empty for a spell
        /// with none, or to decline an optional one. See [`engine::Intent::Cast`].
        #[serde(default)]
        sacrifice_cost: Vec<ObjectId>,
        /// Whether the caster paid the spell's kicker cost (CR 702.33d); `false` (decline) for a
        /// spell with no kicker. See [`engine::Intent::Cast`].
        #[serde(default)]
        kicked: bool,
        /// Whether the caster paid the spell's buyback cost (CR 702.27c); `false` (decline) for a
        /// spell with no buyback. See [`engine::Intent::Cast`].
        #[serde(default)]
        bought_back: bool,
        /// Whether the caster is casting the spell for its evoke cost (CR 702.74a); `false` for a
        /// spell with no evoke, or to cast it normally. See [`engine::Intent::Cast`].
        #[serde(default)]
        evoked: bool,
        /// The caster's declared Strive target count (CR 702.42); 0 (default) for a spell with
        /// no Strive, or "choose zero targets." See [`engine::Intent::Cast`].
        #[serde(default)]
        strive_count: u8,
        /// How many times the caster paid the spell's Replicate cost (CR 702.108); 0 (default)
        /// for a spell with no Replicate, or "pay it zero times." See [`engine::Intent::Cast`].
        #[serde(default)]
        replicate_count: u8,
    },
    PlayLand {
        player: u8,
        object: ObjectId,
    },
    TapForMana {
        player: u8,
        object: ObjectId,
    },
    ActivateAbility {
        player: u8,
        object: ObjectId,
        ability_index: u32,
        target: Option<WireTarget>,
        /// The creature(s) to sacrifice for a "Sacrifice N creature(s)" cost (empty otherwise;
        /// must have exactly the cost's count of entries).
        #[serde(default)]
        sacrifice: Vec<ObjectId>,
        /// Hand cards named to pay a "discard a card" cost (empty otherwise; must have exactly
        /// the cost's `discard_cost` count of distinct entries currently in the activator's
        /// hand). See [`engine::Intent::ActivateAbility`].
        #[serde(default)]
        discard_cost: Vec<ObjectId>,
        /// The chosen `{X}` for an activation cost that contains `{X}` (Nin, the Pain Artist's
        /// `{X}{U}{R}, {T}`); 0 for an ability with no `{X}` in its cost.
        #[serde(default)]
        x: u32,
    },
    DeclareAttackers {
        player: u8,
        attackers: Vec<WireAttack>,
    },
    DeclareBlockers {
        player: u8,
        blocks: Vec<WireBlock>,
    },
    ChooseOrder {
        player: u8,
        order: Vec<u32>,
    },
    ChooseTargets {
        player: u8,
        targets: Vec<WireTarget>,
    },
    /// Answer a [`PendingChoiceView::ChooseTargetPlayers`](crate::dto::PendingChoiceView::ChooseTargetPlayers):
    /// `players` are the chosen "any number of target players" (a subset, possibly empty, of the
    /// offered legal players).
    ChooseTargetPlayers {
        player: u8,
        players: Vec<u8>,
    },
    AnswerMay {
        player: u8,
        yes: bool,
    },
    /// Answer a may-draw-up-to count choice (Arcane Denial's "may draw up to two cards"): `count`
    /// is how many cards to draw, any number `0..=max`. See [`engine::Intent::ChooseDrawCount`].
    ChooseDrawCount {
        player: u8,
        count: u8,
    },
    PayOptionalCost {
        player: u8,
        pay: bool,
    },
    AssignDamage {
        player: u8,
        assignment: Vec<WireDamage>,
    },
    /// Answer a `DivideSpellDamage` choice: how a divided-damage spell's total splits among its
    /// chosen targets (CR 601.2d — Magma Opus). Distinct from `AssignDamage` (combat, object-only)
    /// because a divided target may be a player.
    DivideSpellDamage {
        player: u8,
        assignment: Vec<WireSpellDamage>,
    },
    /// Answer a scry/surveil: `top` cards kept on top (in this order), `bottom` to the library
    /// bottom (scry) or graveyard (surveil).
    ArrangeTop {
        player: u8,
        top: Vec<ObjectId>,
        bottom: Vec<ObjectId>,
    },
    /// Answer a look-at-top-then-select: `cards` are the looked-at cards taken into the choice's
    /// destination (up to the offered maximum); the rest go to the choice's rest zone.
    SelectFromTop {
        player: u8,
        #[serde(default)]
        cards: Vec<ObjectId>,
    },
    /// Answer a distribute-top choice: `to_hand`/`to_bottom`/`to_exile_may_play` are the
    /// looked-at cards routed to each of the choice's three slots.
    DistributeTop {
        player: u8,
        #[serde(default)]
        to_hand: Vec<ObjectId>,
        #[serde(default)]
        to_bottom: Vec<ObjectId>,
        #[serde(default)]
        to_exile_may_play: Vec<ObjectId>,
    },
    /// Answer a shuffle-from-graveyard choice: `cards` are the graveyard cards shuffled into the
    /// library (any subset of the offered candidates, including none or all).
    ShuffleFromGraveyard {
        player: u8,
        #[serde(default)]
        cards: Vec<ObjectId>,
    },
    /// Answer a library search: `choice` is the found card, or absent to fail to find.
    SearchLibrary {
        player: u8,
        #[serde(default)]
        choice: Option<ObjectId>,
    },
    /// Answer a sacrifice edict: the permanents this player gives up.
    ChooseSacrifices {
        player: u8,
        sacrifices: Vec<ObjectId>,
    },
    /// Answer a cleanup discard: the cards this player discards to reach the hand-size limit.
    Discard {
        player: u8,
        cards: Vec<ObjectId>,
    },
    /// Answer a [`crate::dto::PendingChoiceView::PutFromHandOnTop`]: the ordered hand-card pick
    /// (Brainstorm) — first-named ends up on top of the library.
    PutFromHandOnTop {
        player: u8,
        cards: Vec<ObjectId>,
    },
    /// Answer an optional-untap choice (CR 502.2 — Rubinia Soulsinger): `keep_tapped` are the
    /// offered permanents this player leaves tapped; every other offered permanent untaps.
    DeclineUntap {
        player: u8,
        keep_tapped: Vec<ObjectId>,
    },
    /// Answer a dredge choice (CR 702.52): `dredger` is the chosen graveyard dredger to mill-and-
    /// return in place of the draw, or absent to decline and draw normally.
    ChooseDredge {
        player: u8,
        #[serde(default)]
        dredger: Option<ObjectId>,
    },
    /// Answer a put-a-land-from-hand choice: `choice` is the hand land put onto the battlefield,
    /// or absent to decline.
    PutLandFromHand {
        player: u8,
        #[serde(default)]
        choice: Option<ObjectId>,
    },
    /// Answer a put-a-creature-from-hand choice (Cauldron Dance): `choice` is the hand creature
    /// put onto the battlefield, or absent to decline.
    PutCreatureFromHand {
        player: u8,
        #[serde(default)]
        choice: Option<ObjectId>,
    },
    /// Answer a cast-a-hand-creature-face-down choice (Illusionary Mask): `choice` is the hand
    /// creature cast face down as a 2/2, or absent to decline.
    CastCreatureFaceDown {
        player: u8,
        #[serde(default)]
        choice: Option<ObjectId>,
    },
    /// Answer a sacrifice-unless-return-a-land choice (Treva's Ruins): `land` is the offered
    /// non-Lair land returned to its owner's hand, or absent to decline and sacrifice the source.
    ReturnLandOrSacrifice {
        player: u8,
        #[serde(default)]
        land: Option<ObjectId>,
    },
    /// Answer a choose-exiled-with-card choice: `choice` is the exiled-with card put into its
    /// owner's graveyard, or absent to decline.
    ChooseExiledWithCard {
        player: u8,
        #[serde(default)]
        choice: Option<ObjectId>,
    },
    /// Answer a choose-exiled-with-card-to-cast choice: `choice` is the exiled-with card granted
    /// the free-cast permission, or absent to decline.
    ChooseExiledWithCardToCast {
        player: u8,
        #[serde(default)]
        choice: Option<ObjectId>,
    },
    /// Answer a choose-exiled-dig-to-cast-free choice: `choice` is the exiled dig card granted
    /// the free-cast permission, or absent to decline.
    ChooseExiledDigToCastFree {
        player: u8,
        #[serde(default)]
        choice: Option<ObjectId>,
    },
    /// Answer an opponent-chooses-pile choice (Abstract Performance): `pile` is `0` or `1` — which
    /// of the two exile piles this opponent puts into the controller's graveyard.
    ChooseOpponentPile {
        player: u8,
        pile: u8,
    },
    /// Answer a revealed-card-to-battlefield-or-hand choice: `choice` is the revealed card put
    /// onto the battlefield, or absent to put it into hand instead.
    RevealedCardToBattlefieldOrHand {
        player: u8,
        #[serde(default)]
        choice: Option<ObjectId>,
    },
    /// Answer a choose-one triggered-ability mode choice: `mode` is the index of the chosen mode.
    ChooseMode {
        player: u8,
        mode: usize,
    },
    /// Answer a modal *triggered* ability's "choose N" choice: `modes` are (printed-mode index,
    /// chosen Player target) pairs — same shape as [`WireIntent::Cast`]'s `modes`. Empty declines
    /// the whole "may" ability. See [`engine::Intent::ChooseTriggerModes`].
    ChooseTriggerModes {
        player: u8,
        modes: Vec<WireModeChoice>,
    },
    /// Answer a choose-mana-color choice (CR 106.4 "add N mana of any one color"): `color` is the
    /// WUBRG index (see `engine::Color::index`) of the one color the pending amount is added as.
    ChooseManaColor {
        player: u8,
        color: u8,
    },
    /// Answer a choose-creature-type choice (CR 614.12/700.9-style "as ~ enters, choose a
    /// creature type"): `subtype` names the chosen creature type.
    ChooseCreatureType {
        player: u8,
        subtype: String,
    },
    /// Answer a choose-color choice (CR 614.12/700.9-style "as ~ enters, choose a color" —
    /// Flickering Ward): `color` is the WUBRG index (see `engine::Color::index`) of the chosen color.
    ChooseColor {
        player: u8,
        color: u8,
    },
    /// Answer a choose-attach-host choice: `host` is the chosen creature the deployed Aura (CR
    /// 303.4f) or Equipment (CR 301.5c) attaches to, or `None` to decline (Equipment only — an
    /// Aura host is mandatory). See [`engine::Intent::ChooseAttachHost`].
    ChooseAttachHost {
        player: u8,
        host: Option<ObjectId>,
    },
    /// Answer an enter-as-copy choice (CR 706/707.2 — Altered Ego, Cursed Mirror): `copy` is the
    /// chosen creature the entering permanent becomes a copy of, or `None` to decline (the "you
    /// may"). See [`engine::Intent::ChooseCopyTarget`].
    ChooseCopyTarget {
        player: u8,
        copy: Option<ObjectId>,
    },
    /// Answer a choose-countered-spell-destination choice (CR 701.5b — Hinder's rider): `top`
    /// puts the countered spell on top of its owner's library, `false` on the bottom. See
    /// [`engine::Intent::ChooseTopOrBottom`].
    ChooseTopOrBottom {
        player: u8,
        top: bool,
    },
    /// Activate a hand card's Cycling ability (CR 702.29a): pay its cycling cost, discard the
    /// card, draw one. `sacrifice` names the permanent paying a cycling sacrifice cost (CR
    /// 702.29b — Edge of Autumn's "Cycling—Sacrifice a land"); absent for a card whose cycling
    /// carries none. See [`engine::Intent::Cycle`].
    Cycle {
        player: u8,
        card: ObjectId,
        #[serde(default)]
        sacrifice: Option<ObjectId>,
    },
    /// Activate a hand card's hand-activated, discard-this-card ability (CR 113.6/602.5e —
    /// Magma Opus's "{U/R}{U/R}, Discard this card: Create a Treasure token."). See
    /// [`engine::Intent::ActivateHandAbility`].
    ActivateHandAbility {
        player: u8,
        card: ObjectId,
    },
    /// Suspend a hand card (CR 702.62): pay its suspend cost and exile it with time counters
    /// rather than casting it. See [`engine::Intent::Suspend`].
    Suspend {
        player: u8,
        card: ObjectId,
    },
    /// Encore a graveyard card (CR 702.140): pay its encore mana cost and exile it from the
    /// graveyard to mint a must-attack haste token copy per opponent. See [`engine::Intent::Encore`].
    Encore {
        player: u8,
        card: ObjectId,
    },
    /// Turn a face-down manifested permanent face up (CR 701.34e): pay its hidden creature card's
    /// mana cost to reveal it. See [`engine::Intent::TurnFaceUp`].
    TurnFaceUp {
        player: u8,
        permanent: ObjectId,
    },
    /// Cast a hand card face down as a 2/2 for {3} (CR 702.37b — morph): pay the flat {3} and put
    /// it on the stack as a face-down creature spell. See [`engine::Intent::CastFaceDown`].
    CastFaceDown {
        player: u8,
        card: ObjectId,
    },
    /// Cast a copy of a prepared permanent's back-face spell (soc/sos prepare DFCs): pay the back
    /// face's cost, put the copy on the stack targeting `target`, and unprepare `source`.
    CastPrepared {
        player: u8,
        source: ObjectId,
        #[serde(default)]
        target: Option<WireTarget>,
        /// The back face's chosen `{X}` value (absent/0 for a back face with no `{X}`). See
        /// [`engine::Intent::CastPrepared`].
        #[serde(default)]
        x: u32,
    },
    /// Cast the adventure half of an adventure card from hand (CR 715): pay the adventure face's
    /// cost, put that instant/sorcery on the stack targeting `target`. See
    /// [`engine::Intent::CastAdventure`].
    CastAdventure {
        player: u8,
        source: ObjectId,
        #[serde(default)]
        target: Option<WireTarget>,
        /// The adventure's chosen `{X}` value (absent/0 for an adventure with no `{X}`).
        #[serde(default)]
        x: u32,
    },
    /// Cast a card for its bestow cost (CR 702.103): pay the bestow cost, put it on the stack as an
    /// Aura spell with enchant creature targeting `target`. See [`engine::Intent::CastBestow`].
    CastBestow {
        player: u8,
        object: ObjectId,
        #[serde(default)]
        target: Option<WireTarget>,
    },
    PassPriority {
        player: u8,
    },
    /// Leave the game (CR 104.3a). Legal at any time — with or without priority, and even while the
    /// engine waits on this player's own choice.
    Concede {
        player: u8,
    },
    /// Take one of the viewer's own stored [`ActionView`](crate::ActionView)s by `id` — the
    /// action-list counterpart of `Cast`/`PlayLand`/`ActivateAbility`/`DeclareAttackers`/
    /// `DeclareBlockers`; see [`engine::Intent::TakeAction`] for which fields matter per action
    /// kind. Every optional/list field defaults so a client only sends what the resolved action
    /// needs.
    TakeAction {
        player: u8,
        id: u64,
        #[serde(default)]
        target: Option<WireTarget>,
        #[serde(default)]
        x: u32,
        #[serde(default)]
        modes: Vec<WireModeChoice>,
        sacrifice: Vec<ObjectId>,
        /// Hand cards paying an additional discard cost; empty when the cast has none.
        #[serde(default)]
        discard_cost: Vec<ObjectId>,
        /// Graveyard cards paying delve or escape exile; empty when neither.
        #[serde(default)]
        graveyard_exile: Vec<ObjectId>,
        #[serde(default)]
        attackers: Vec<WireAttack>,
        #[serde(default)]
        blocks: Vec<WireBlock>,
    },
}

/// Map a `WireIntent` back into the engine's [`engine::Intent`], stamping `player` from the
/// authenticated seat so clients cannot spoof another seat.
pub fn to_intent_for_seat(wire: WireIntent, seat: engine::PlayerId) -> engine::Intent {
    to_intent(with_player(wire, seat.0))
}

/// Overwrite every intent variant's `player` field with the authenticated seat.
fn with_player(wire: WireIntent, player: u8) -> WireIntent {
    use WireIntent::*;
    match wire {
        Cast {
            object,
            target,
            x,
            modes,
            discard_cost,
            graveyard_exile,
            sacrifice_cost,
            kicked,
            bought_back,
            evoked,
            strive_count,
            replicate_count,
            ..
        } => Cast {
            player,
            object,
            target,
            x,
            modes,
            discard_cost,
            graveyard_exile,
            sacrifice_cost,
            kicked,
            bought_back,
            evoked,
            strive_count,
            replicate_count,
        },
        PlayLand { object, .. } => PlayLand { player, object },
        TapForMana { object, .. } => TapForMana { player, object },
        ActivateAbility {
            object,
            ability_index,
            target,
            sacrifice,
            discard_cost,
            x,
            ..
        } => ActivateAbility {
            player,
            object,
            ability_index,
            target,
            sacrifice,
            discard_cost,
            x,
        },
        DeclareAttackers { attackers, .. } => DeclareAttackers { player, attackers },
        DeclareBlockers { blocks, .. } => DeclareBlockers { player, blocks },
        ChooseOrder { order, .. } => ChooseOrder { player, order },
        ChooseTargets { targets, .. } => ChooseTargets { player, targets },
        ChooseTargetPlayers { players, .. } => ChooseTargetPlayers { player, players },
        AnswerMay { yes, .. } => AnswerMay { player, yes },
        ChooseDrawCount { count, .. } => ChooseDrawCount { player, count },
        PayOptionalCost { pay, .. } => PayOptionalCost { player, pay },
        AssignDamage { assignment, .. } => AssignDamage { player, assignment },
        DivideSpellDamage { assignment, .. } => DivideSpellDamage { player, assignment },
        ArrangeTop { top, bottom, .. } => ArrangeTop {
            player,
            top,
            bottom,
        },
        SelectFromTop { cards, .. } => SelectFromTop { player, cards },
        SearchLibrary { choice, .. } => SearchLibrary { player, choice },
        ChooseSacrifices { sacrifices, .. } => ChooseSacrifices { player, sacrifices },
        Discard { cards, .. } => Discard { player, cards },
        PutFromHandOnTop { cards, .. } => PutFromHandOnTop { player, cards },
        DeclineUntap { keep_tapped, .. } => DeclineUntap {
            player,
            keep_tapped,
        },
        ChooseDredge { dredger, .. } => ChooseDredge { player, dredger },
        PutLandFromHand { choice, .. } => PutLandFromHand { player, choice },
        PutCreatureFromHand { choice, .. } => PutCreatureFromHand { player, choice },
        CastCreatureFaceDown { choice, .. } => CastCreatureFaceDown { player, choice },
        ReturnLandOrSacrifice { land, .. } => ReturnLandOrSacrifice { player, land },
        ChooseExiledWithCard { choice, .. } => ChooseExiledWithCard { player, choice },
        ChooseExiledWithCardToCast { choice, .. } => ChooseExiledWithCardToCast { player, choice },
        ChooseExiledDigToCastFree { choice, .. } => ChooseExiledDigToCastFree { player, choice },
        ChooseOpponentPile { pile, .. } => ChooseOpponentPile { player, pile },
        RevealedCardToBattlefieldOrHand { choice, .. } => {
            RevealedCardToBattlefieldOrHand { player, choice }
        }
        DistributeTop {
            to_hand,
            to_bottom,
            to_exile_may_play,
            ..
        } => DistributeTop {
            player,
            to_hand,
            to_bottom,
            to_exile_may_play,
        },
        ShuffleFromGraveyard { cards, .. } => ShuffleFromGraveyard { player, cards },
        ChooseManaColor { color, .. } => ChooseManaColor { player, color },
        ChooseCreatureType { subtype, .. } => ChooseCreatureType { player, subtype },
        ChooseColor { color, .. } => ChooseColor { player, color },
        ChooseAttachHost { host, .. } => ChooseAttachHost { player, host },
        ChooseCopyTarget { copy, .. } => ChooseCopyTarget { player, copy },
        ChooseTopOrBottom { top, .. } => ChooseTopOrBottom { player, top },
        ChooseMode { mode, .. } => ChooseMode { player, mode },
        ChooseTriggerModes { modes, .. } => ChooseTriggerModes { player, modes },
        Cycle {
            card, sacrifice, ..
        } => Cycle {
            player,
            card,
            sacrifice,
        },
        ActivateHandAbility { card, .. } => ActivateHandAbility { player, card },
        Suspend { card, .. } => Suspend { player, card },
        Encore { card, .. } => Encore { player, card },
        TurnFaceUp { permanent, .. } => TurnFaceUp { player, permanent },
        CastFaceDown { card, .. } => CastFaceDown { player, card },
        CastPrepared {
            source, target, x, ..
        } => CastPrepared {
            player,
            source,
            target,
            x,
        },
        CastAdventure {
            source, target, x, ..
        } => CastAdventure {
            player,
            source,
            target,
            x,
        },
        CastBestow { object, target, .. } => CastBestow {
            player,
            object,
            target,
        },
        PassPriority { .. } => PassPriority { player },
        Concede { .. } => Concede { player },
        TakeAction {
            id,
            target,
            x,
            modes,
            sacrifice,
            discard_cost,
            graveyard_exile,
            attackers,
            blocks,
            ..
        } => TakeAction {
            player,
            id,
            target,
            x,
            modes,
            sacrifice,
            discard_cost,
            graveyard_exile,
            attackers,
            blocks,
        },
    }
}

/// Map a `WireIntent` back into the engine's [`engine::Intent`].
pub fn to_intent(wire: WireIntent) -> engine::Intent {
    use engine::{Intent, PlayerId};
    match wire {
        WireIntent::Cast {
            player,
            object,
            target,
            x,
            modes,
            discard_cost,
            graveyard_exile,
            sacrifice_cost,
            kicked,
            bought_back,
            evoked,
            strive_count,
            replicate_count,
        } => Intent::Cast {
            player: PlayerId(player),
            object,
            target: target.map(WireTarget::to_engine),
            x,
            modes: modes
                .into_iter()
                .map(|m| (m.index as usize, m.target.map(WireTarget::to_engine)))
                .collect(),
            discard_cost,
            graveyard_exile,
            sacrifice_cost,
            kicked,
            bought_back,
            evoked,
            strive_count,
            replicate_count,
        },
        WireIntent::PlayLand { player, object } => Intent::PlayLand {
            player: PlayerId(player),
            object,
        },
        WireIntent::TapForMana { player, object } => Intent::TapForMana {
            player: PlayerId(player),
            object,
        },
        WireIntent::ActivateAbility {
            player,
            object,
            ability_index,
            target,
            sacrifice,
            discard_cost,
            x,
        } => Intent::ActivateAbility {
            player: PlayerId(player),
            object,
            ability_index: ability_index as usize,
            target: target.map(WireTarget::to_engine),
            sacrifice,
            discard_cost,
            x,
        },
        WireIntent::DeclareAttackers { player, attackers } => Intent::DeclareAttackers {
            player: PlayerId(player),
            attackers: attackers
                .into_iter()
                .map(|a| (a.attacker, PlayerId(a.defender)))
                .collect(),
        },
        WireIntent::DeclareBlockers { player, blocks } => Intent::DeclareBlockers {
            player: PlayerId(player),
            blocks: blocks
                .into_iter()
                .map(|b| (b.blocker, b.attacker))
                .collect(),
        },
        WireIntent::ChooseOrder { player, order } => Intent::ChooseOrder {
            player: PlayerId(player),
            order: order.into_iter().map(|i| i as usize).collect(),
        },
        WireIntent::ChooseTargets { player, targets } => Intent::ChooseTargets {
            player: PlayerId(player),
            targets: targets.into_iter().map(WireTarget::to_engine).collect(),
        },
        WireIntent::ChooseTargetPlayers { player, players } => Intent::ChooseTargetPlayers {
            player: PlayerId(player),
            players: players.into_iter().map(PlayerId).collect(),
        },
        WireIntent::AnswerMay { player, yes } => Intent::AnswerMay {
            player: PlayerId(player),
            yes,
        },
        WireIntent::ChooseDrawCount { player, count } => Intent::ChooseDrawCount {
            player: PlayerId(player),
            count,
        },
        WireIntent::PayOptionalCost { player, pay } => Intent::PayOptionalCost {
            player: PlayerId(player),
            pay,
        },
        WireIntent::AssignDamage { player, assignment } => Intent::AssignDamage {
            player: PlayerId(player),
            assignment: assignment
                .into_iter()
                .map(|d| (d.blocker, d.amount))
                .collect(),
        },
        WireIntent::DivideSpellDamage { player, assignment } => Intent::DivideSpellDamage {
            player: PlayerId(player),
            assignment: assignment
                .into_iter()
                .map(|d| (d.target.to_engine(), d.amount))
                .collect(),
        },
        WireIntent::ArrangeTop {
            player,
            top,
            bottom,
        } => Intent::ArrangeTop {
            player: PlayerId(player),
            top,
            bottom,
        },
        WireIntent::SelectFromTop { player, cards } => Intent::SelectFromTop {
            player: PlayerId(player),
            cards,
        },
        WireIntent::DistributeTop {
            player,
            to_hand,
            to_bottom,
            to_exile_may_play,
        } => Intent::DistributeTop {
            player: PlayerId(player),
            to_hand,
            to_bottom,
            to_exile_may_play,
        },
        WireIntent::ShuffleFromGraveyard { player, cards } => Intent::ShuffleFromGraveyard {
            player: PlayerId(player),
            cards,
        },
        WireIntent::SearchLibrary { player, choice } => Intent::SearchLibrary {
            player: PlayerId(player),
            choice,
        },
        WireIntent::ChooseSacrifices { player, sacrifices } => Intent::ChooseSacrifices {
            player: PlayerId(player),
            sacrifices,
        },
        WireIntent::Discard { player, cards } => Intent::Discard {
            player: PlayerId(player),
            cards,
        },
        WireIntent::PutFromHandOnTop { player, cards } => Intent::PutFromHandOnTop {
            player: PlayerId(player),
            cards,
        },
        WireIntent::DeclineUntap {
            player,
            keep_tapped,
        } => Intent::DeclineUntap {
            player: PlayerId(player),
            keep_tapped,
        },
        WireIntent::ChooseDredge { player, dredger } => Intent::ChooseDredge {
            player: PlayerId(player),
            dredger,
        },
        WireIntent::PutLandFromHand { player, choice } => Intent::PutLandFromHand {
            player: PlayerId(player),
            choice,
        },
        WireIntent::PutCreatureFromHand { player, choice } => Intent::PutCreatureFromHand {
            player: PlayerId(player),
            choice,
        },
        WireIntent::CastCreatureFaceDown { player, choice } => Intent::CastCreatureFaceDown {
            player: PlayerId(player),
            choice,
        },
        WireIntent::ReturnLandOrSacrifice { player, land } => Intent::ReturnLandOrSacrifice {
            player: PlayerId(player),
            land,
        },
        WireIntent::ChooseExiledWithCard { player, choice } => Intent::ChooseExiledWithCard {
            player: PlayerId(player),
            choice,
        },
        WireIntent::ChooseExiledWithCardToCast { player, choice } => {
            Intent::ChooseExiledWithCardToCast {
                player: PlayerId(player),
                choice,
            }
        }
        WireIntent::ChooseExiledDigToCastFree { player, choice } => {
            Intent::ChooseExiledDigToCastFree {
                player: PlayerId(player),
                choice,
            }
        }
        WireIntent::ChooseOpponentPile { player, pile } => Intent::ChooseOpponentPile {
            player: PlayerId(player),
            pile,
        },
        WireIntent::RevealedCardToBattlefieldOrHand { player, choice } => {
            Intent::RevealedCardToBattlefieldOrHand {
                player: PlayerId(player),
                choice,
            }
        }
        WireIntent::ChooseMode { player, mode } => Intent::ChooseMode {
            player: PlayerId(player),
            mode,
        },
        WireIntent::ChooseTriggerModes { player, modes } => Intent::ChooseTriggerModes {
            player: PlayerId(player),
            modes: modes
                .into_iter()
                .map(|m| (m.index as usize, m.target.map(WireTarget::to_engine)))
                .collect(),
        },
        WireIntent::ChooseManaColor { player, color } => Intent::ChooseManaColor {
            player: PlayerId(player),
            // ponytail: an out-of-range wire index (>4) clamps to White rather than panicking —
            // a trust-boundary default, not a real choice; any 5-way answer is equally legal
            // here (no game-state legality to violate), so this never lets a client do more than
            // pick an odd-but-legal color.
            color: engine::Color::ALL
                .get(color as usize)
                .copied()
                .unwrap_or(engine::Color::White),
        },
        WireIntent::ChooseCreatureType { player, subtype } => Intent::ChooseCreatureType {
            player: PlayerId(player),
            subtype,
        },
        WireIntent::ChooseColor { player, color } => Intent::ChooseColor {
            player: PlayerId(player),
            // ponytail: an out-of-range wire index (>4) clamps to White rather than panicking — a
            // trust-boundary default, not a real choice; any of the five colors is equally legal
            // here (no game-state legality to violate), same as `ChooseManaColor` above.
            color: engine::Color::ALL
                .get(color as usize)
                .copied()
                .unwrap_or(engine::Color::White),
        },
        WireIntent::ChooseAttachHost { player, host } => Intent::ChooseAttachHost {
            player: PlayerId(player),
            host,
        },
        WireIntent::ChooseCopyTarget { player, copy } => Intent::ChooseCopyTarget {
            player: PlayerId(player),
            copy,
        },
        WireIntent::ChooseTopOrBottom { player, top } => Intent::ChooseTopOrBottom {
            player: PlayerId(player),
            top,
        },
        WireIntent::Cycle {
            player,
            card,
            sacrifice,
        } => Intent::Cycle {
            player: PlayerId(player),
            card,
            sacrifice,
        },
        WireIntent::ActivateHandAbility { player, card } => Intent::ActivateHandAbility {
            player: PlayerId(player),
            card,
        },
        WireIntent::Suspend { player, card } => Intent::Suspend {
            player: PlayerId(player),
            card,
        },
        WireIntent::Encore { player, card } => Intent::Encore {
            player: PlayerId(player),
            card,
        },
        WireIntent::TurnFaceUp { player, permanent } => Intent::TurnFaceUp {
            player: PlayerId(player),
            permanent,
        },
        WireIntent::CastFaceDown { player, card } => Intent::CastFaceDown {
            player: PlayerId(player),
            card,
        },
        WireIntent::CastPrepared {
            player,
            source,
            target,
            x,
        } => Intent::CastPrepared {
            player: PlayerId(player),
            source,
            target: target.map(WireTarget::to_engine),
            x,
        },
        WireIntent::CastAdventure {
            player,
            source,
            target,
            x,
        } => Intent::CastAdventure {
            player: PlayerId(player),
            source,
            target: target.map(WireTarget::to_engine),
            x,
        },
        WireIntent::CastBestow {
            player,
            object,
            target,
        } => Intent::CastBestow {
            player: PlayerId(player),
            object,
            target: target.map(WireTarget::to_engine),
        },
        WireIntent::PassPriority { player } => Intent::PassPriority {
            player: PlayerId(player),
        },
        WireIntent::Concede { player } => Intent::Concede {
            player: PlayerId(player),
        },
        WireIntent::TakeAction {
            player,
            id,
            target,
            x,
            modes,
            sacrifice,
            discard_cost,
            graveyard_exile,
            attackers,
            blocks,
        } => Intent::TakeAction {
            player: PlayerId(player),
            id,
            target: target.map(WireTarget::to_engine),
            x,
            modes: modes
                .into_iter()
                .map(|m| (m.index as usize, m.target.map(WireTarget::to_engine)))
                .collect(),
            sacrifice,
            discard_cost,
            graveyard_exile,
            attackers: attackers
                .into_iter()
                .map(|a| (a.attacker, PlayerId(a.defender)))
                .collect(),
            blocks: blocks
                .into_iter()
                .map(|b| (b.blocker, b.attacker))
                .collect(),
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::intent::{WireBlock, WireDamage, WireIntent, WireTarget, to_intent};
    use engine::PlayerId;

    #[test]
    fn to_intent_maps_the_new_choice_intents() {
        assert_eq!(
            to_intent(WireIntent::ChooseTargets {
                player: 0,
                targets: vec![WireTarget::Object { id: 4 }],
            }),
            engine::Intent::ChooseTargets {
                player: PlayerId(0),
                targets: vec![engine::Target::Object(4)],
            },
        );
        assert_eq!(
            to_intent(WireIntent::AnswerMay {
                player: 1,
                yes: true,
            }),
            engine::Intent::AnswerMay {
                player: PlayerId(1),
                yes: true,
            },
        );
        assert_eq!(
            to_intent(WireIntent::AssignDamage {
                player: 0,
                assignment: vec![WireDamage {
                    blocker: 3,
                    amount: 2,
                }],
            }),
            engine::Intent::AssignDamage {
                player: PlayerId(0),
                assignment: vec![(3, 2)],
            },
        );
    }

    #[test]
    fn to_intent_round_trips_take_action() {
        assert_eq!(
            to_intent(WireIntent::TakeAction {
                player: 0,
                id: 42,
                target: Some(WireTarget::Object { id: 7 }),
                x: 3,
                modes: vec![],
                sacrifice: vec![],
                discard_cost: vec![],
                graveyard_exile: vec![],
                attackers: vec![],
                blocks: vec![],
            }),
            engine::Intent::TakeAction {
                player: PlayerId(0),
                id: 42,
                target: Some(engine::Target::Object(7)),
                x: 3,
                modes: vec![],
                sacrifice: vec![],
                discard_cost: vec![],
                graveyard_exile: vec![],
                attackers: vec![],
                blocks: vec![],
            },
        );
    }

    #[test]
    fn to_intent_maps_block_pairs() {
        let wire = WireIntent::DeclareBlockers {
            player: 1,
            blocks: vec![WireBlock {
                blocker: 5,
                attacker: 9,
            }],
        };
        assert_eq!(
            to_intent(wire),
            engine::Intent::DeclareBlockers {
                player: PlayerId(1),
                blocks: vec![(5, 9)],
            },
        );
    }
}
