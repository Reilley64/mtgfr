use super::*;
#[cfg(feature = "card-dsl")]
use crate::de;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)
)]
pub enum ChoiceEffect {
    CastCreatureFaceDown,

    CasterKeepsOneOfEachTypePerPlayer,

    ChooseColor,

    ChooseCreatureType,

    CouncilsDilemmaVote {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_str_slice"))]
        options: &'static [&'static str],
    },

    DamagingCreatureControllerMayDraw {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        drawer: Option<PlayerId>,
        count: u32,
    },

    DefendingPlayerSacrifices {
        count: u8,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        defender: Option<PlayerId>,
    },

    Discard {
        count: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target_player: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        or_one_matching: Option<CardFilter>,
        /// "Discards a card **at random**" (Hypnotic Specter, Mind Twist): nobody chooses, so
        /// this discard raises no pause at all — it resolves straight through
        /// [`Game::run_misc_choreo`](crate::Game::run_misc_choreo), picking from the discarder's
        /// hand with the engine's injected per-op RNG.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        random: bool,
        /// "**That player** discards a card at random" (Hypnotic Specter) — the discarder is
        /// whoever this ability's source just damaged, not its controller. Only meaningful under
        /// a damage watch; Looter il-Kor's same-trigger "draw a card, then discard a card" leaves
        /// it unset, because *its* discard is the controller's.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        damaged_player: bool,
        /// The player [`damaged_player`](Self::Discard::damaged_player) resolved to, baked in when
        /// the watch fired from
        /// [`TriggerContext::damage_recipient`](crate::types::trigger::TriggerContext). `None`
        /// everywhere else.
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        discarder: Option<PlayerId>,
    },

    EachOtherTokenBecomesCopyOfChosen,

    /// Archangel of Strife's "as this creature enters, each player chooses war or peace." A
    /// fan-out over every living player in APNAP order (CR 101.4 — no "starting with you"
    /// wording, unlike `CouncilsDilemmaVote`), reusing the council's-dilemma `CastVote` pause
    /// with `["war", "peace"]` as its ballot. Carries no payoff of its own: `answer_vote` writes
    /// each answer straight to that player's own
    /// [`Player::war_choices`](crate::Player::war_choices) against the asking permanent instead of
    /// tallying, for that permanent's own `war_choice`-gated anthems (§7) to read live.
    EachPlayerChoosesWarOrPeace,

    EachPlayerControllerChoosesCounterTarget,

    EachPlayerCreatesFractalFromExiledPower {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::token_profile"))]
        token: CardDef,
    },

    EachPlayerDiscardsHandThenDraws {
        count: Amount,
    },

    /// Timetwister's "Each player shuffles their hand and graveyard into their library, then draws
    /// seven cards." The recycling sibling of
    /// [`EachPlayerDiscardsHandThenDraws`](Self::EachPlayerDiscardsHandThenDraws): same APNAP
    /// fan-out and same redraw, but the old cards are tucked back into the library and shuffled
    /// instead of discarded, so no card reaches a graveyard and `you_discard` triggers stay quiet.
    /// A separate variant rather than a flag on the discard one — the zones moved, the zone moved
    /// *to*, and the triggers fired all differ.
    EachPlayerShufflesHandAndGraveyardThenDraws {
        count: Amount,
    },

    /// Each other player discards a card of their choice (Syphon Mind, "Each other player discards
    /// a card"). A fan-out over the opponents in APNAP order — a player with an empty hand is
    /// skipped — tallying [`ResolutionFrame::cards_discarded_this_way`](crate::resolution::ResolutionFrame)
    /// so a following `then`/Sequence step can read it (Syphon Mind's "You draw a card for each
    /// card discarded this way").
    EachOpponentDiscards,

    /// The effect's controller discards their whole hand (Malfegor's "discard your hand"). A
    /// choiceless whole-hand discard (the discard sibling of
    /// [`EachPlayerDiscardsHandThenDraws`](Self::EachPlayerDiscardsHandThenDraws), scoped to the
    /// controller), emitting the normal [`Event::Discarded`](crate::Event) markers so
    /// `Trigger::YouDiscard` fires and setting
    /// [`ResolutionFrame::cards_discarded_this_way`](crate::resolution::ResolutionFrame) to the
    /// hand's size so a following Sequence step reads it (Malfegor's "for each card discarded this
    /// way").
    // ponytail: choiceless, so it sits a little oddly under ChoiceEffect. Only Malfegor needs it
    // today — promote to its own Effect::Discard(DiscardEffect) family (mirroring Effect::Sacrifice)
    // once a second/third choiceless discard card makes the family pay for itself.
    DiscardYourHand,

    EachPlayerExilesFromGraveyard,

    EachPlayerNamesCardThenRevealsTop,

    EachPlayerSacrifices {
        #[cfg_attr(feature = "card-dsl", serde(default = "de::all_players"))]
        scope: EdictScope,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        keep_one: bool,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::creature_edict"))]
        filter: PermanentFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        life_loss: i32,
        /// How many creatures each affected player sacrifices (Malfegor's "a creature … for each
        /// card discarded this way"). Defaults to 1; a player with fewer creatures sacrifices all
        /// of them. Not used with `keep_one`.
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one_amount"))]
        count: Amount,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },

    JoinForcesPayMana,

    MayDiscard {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },

    MayDrawUnlessPays {
        cost: Amount,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        caster: Option<PlayerId>,
    },

    MayDrawUpTo {
        count: Amount,
    },

    MayDrawUpToThenOpponentMayRepeat {
        count: Amount,
    },

    /// A resolution-time optional "you may put a +1/+1 counter on a creature" (CR 601.2c —
    /// Zimone's Hypothesis' primer): pauses the controller on a
    /// [`PendingChoice::MayPutCounterOnCreature`](crate::PendingChoice) over every creature on the
    /// battlefield; picking one puts a single +1/+1 counter on it, declining does nothing. Unlike
    /// [`Self::MaySacrifice`]/[`Self::MayDiscard`] it carries no `then` — its follow-up (the mass
    /// parity bounce) runs as the next step of the enclosing `Sequence` whether or not the counter
    /// was placed, not "if you do". Non-targeted: nothing is advertised on the stack at cast.
    MayPutCounterOnCreature,

    /// A batch nonland-discard payoff (CR 701.8 — Conspiracy Theorist's "you may exile one of
    /// them from your graveyard. If you do, you may cast it this turn"): pauses the controller on
    /// a [`PendingChoice::MayExileDiscardedToPlay`](crate::PendingChoice) over `cards` (the nonland
    /// cards discarded this event, still in the graveyard), choosing one exiles it face-up with
    /// impulse-play permission until end of turn, declining does nothing. `cards` is baked in at
    /// [`Trigger::YouDiscardNonland`](crate::Trigger) placement from
    /// [`TriggerContext::discarded_nonland_cards`](crate::TriggerContext) via
    /// `contextualize_effect`'s `fill_discarded_nonland_cards` — the graveyard-return twin of
    /// [`Self::PutCounterThenMayBecomeCopyOfCardFromList`]. Non-targeted: nothing is advertised on
    /// the stack. No legal card left (a prior effect moved them) quietly does nothing (no pause).
    MayExileDiscardedNonlandMayPlay {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        cards: &'static [ObjectId],
    },

    MayReturnFromGraveyard {
        filter: CardFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        if_you_sacrificed_this_way: bool,
        /// "you return" (Witherbloom Command mode 0) rather than "you may return" (Deadly Brew,
        /// Witch of the Moors): a legal card *must* be chosen — declining is illegal (CR 700.2).
        /// No legal card in the graveyard still quietly does nothing (no pause).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        mandatory: bool,
    },

    MaySacrifice {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        filter: PermanentFilter,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },

    PhaseOut,

    Proliferate {
        times: Amount,
    },

    PutCounterThenMayBecomeCopyOfCardFromList {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        cards: &'static [ObjectId],
    },

    /// Cauldron Dance's "you may put a creature card from your hand onto the battlefield"
    /// (`subtypes` empty, `keep` false — gains haste, sacrificed at the next end step); Kaalia of
    /// the Vast's attack trigger restricts `subtypes` to Angel/Demon/Dragon, sets `keep` (no
    /// sacrifice), and — as an `attacks` trigger — threads `defender` so the put-in creature
    /// enters tapped and attacking that opponent (CR 508.4).
    PutCreatureFromHand {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        subtypes: &'static [&'static str],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        keep: bool,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        defender: Option<PlayerId>,
    },

    PutFromHandOnTop {
        count: u32,
    },

    PutLandFromHand {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        tapped: bool,
    },

    SacrificeOwn {
        filter: PermanentFilter,
        count: u32,
    },

    /// "…unless you pay `cost`" (CR 701.16's optional-cost shape): the controller is offered the
    /// payment, and declining runs `otherwise` instead. Most printings sacrifice the source
    /// (Phantasmal Forces, Rupture Spire) but the penalty is whatever the card prints — Force of
    /// Nature's is "this creature deals 8 damage to you". Always pauses: only the player can answer.
    PayOrElse {
        cost: Cost,
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_slice"))]
        otherwise: &'static [Effect],
    },

    SacrificeSelfUnlessReturnLand {
        filter: PermanentFilter,
    },

    SetOwnColorUntilEndOfTurn,

    TargetPlayerExilesFromGraveyard {
        target: TargetSpec,
    },

    TargetPlayerMayDraw {
        count: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        opponent: bool,
    },
}
