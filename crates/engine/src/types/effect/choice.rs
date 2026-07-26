use super::*;
#[cfg(feature = "card-dsl")]
use crate::de;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
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
        count: u32,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target_player: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        or_one_matching: Option<CardFilter>,
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

    MayReturnFromGraveyard {
        filter: CardFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        if_you_sacrificed_this_way: bool,
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

    SacrificeSelfUnlessPay {
        cost: Cost,
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
