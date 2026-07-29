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
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum ChoiceEffect {
    CastCreatureFaceDown,

    CasterKeepsOneOfEachTypePerPlayer,

    /// "Change the text of target spell or permanent by replacing all instances of one `words`
    /// with another" (CR 612.1 — Magical Hack's basic land types, Sleight of Mind's color words).
    /// Resolving asks the controller twice through [`PendingChoice::ChooseCreatureType`]'s picker
    /// (the word being replaced, then its replacement) and records the answer as a [`TextSwap`] on
    /// the target — see that type for how far a swap reaches.
    ChangeText {
        words: TextWords,
        target: TargetSpec,
    },

    /// Phantasmal Terrain's "as this Aura enters, choose a basic land type." The same picker
    /// [`Self::ChooseCreatureType`] raises, narrowed to [`BASIC_LAND_TYPES`](crate::BASIC_LAND_TYPES)
    /// — the answer lands on the source's own [`Permanent::chosen_subtype`](crate::Permanent),
    /// read back by a `set_chosen_land_type`
    /// [`StaticEffect::SetAttachedTypes`](crate::StaticEffect).
    ChooseBasicLandType,

    ChooseColor,

    ChooseCreatureType,

    /// Black Vise's "as this artifact enters, choose an opponent." Reuses the shared
    /// "an opponent ..." picker
    /// ([`Game::choose_splitting_opponent`](crate::Game)) with a
    /// [`SplittingContinuation::RememberAsChosenOpponent`](crate::SplittingContinuation) tail, so
    /// a table with one opponent left collapses the pause and a table with more asks. The answer
    /// is written to the source's own [`Permanent::chosen_opponent`], read back by a
    /// [`Condition::ChosenPlayersUpkeep`] gate on its upkeep trigger.
    ChooseOpponent,

    CouncilsDilemmaVote {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_str_slice"))]
        options: &'static [&'static str],
    },

    DefendingPlayerSacrifices {
        count: u8,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        defender: Option<PlayerId>,
    },

    Discard {
        count: Amount,
        /// Who discards: the ability's controller by default (Looter il-Kor's "draw a card, then
        /// discard a card"), `target_player` for the chosen seat (Prismari Command), or
        /// `damaged_player` for whoever this ability's source just damaged (Hypnotic Specter's
        /// "*that player* discards a card at random"). A one-seat pause, so a multi-seat set is
        /// rejected at resolution — "each player discards" is
        /// [`EachPlayerDiscards`](Self::EachPlayerDiscards), which fans out and tallies.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        who: PlayerSet,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        or_one_matching: Option<CardFilter>,
        /// "Discards a card **at random**" (Hypnotic Specter, Mind Twist): nobody chooses, so
        /// this discard raises no pause at all — it resolves straight through
        /// [`Game::run_misc_choreo`](crate::Game::run_misc_choreo), picking from the discarder's
        /// hand with the engine's injected per-op RNG.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        random: bool,
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
        #[cfg_attr(feature = "card-schema", schemars(with = "String"))]
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

    /// Each player in `who` discards a card of their choice (Syphon Mind, "Each other player
    /// discards a card"). A fan-out over those players in APNAP order — a player with an empty
    /// hand is skipped — tallying [`ResolutionFrame::cards_discarded_this_way`](crate::resolution::ResolutionFrame)
    /// so a following `then`/Sequence step can read it (Syphon Mind's "You draw a card for each
    /// card discarded this way").
    EachPlayerDiscards {
        /// Whose hands the fan-out visits — `each_player` (Balance) or `each_opponent` (Syphon
        /// Mind's "each other player"). Resolved in APNAP order (CR 101.4) rather than seat
        /// order, since each seat's discard is a choice made in sequence.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        who: PlayerSet,
        /// Balance's "Players discard cards … the same way": instead of one card each, every
        /// player discards down to the smallest hand in `who` — see
        /// [`down_to_fewest`](Self::EachPlayerSacrifices::down_to_fewest).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        down_to_fewest: bool,
    },

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
        /// Who pays the edict — `each_player` (Promise of Loyalty), `each_opponent` (Martyr's
        /// Bond), or `you` alone (Lich's damage tax, a one-seat fan-out so the prompt, the count
        /// and the shortfall check stay the ones every other edict uses). Resolved in APNAP order
        /// (CR 101.4), since the sacrifices are chosen one seat at a time.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        who: PlayerSet,
        /// Priest of Forgotten Gods' "any number of target players": `who` names the legal pool
        /// and the controller picks its subset as the effect resolves, via a
        /// [`PendingChoice::ChooseTargetPlayers`](crate::PendingChoice::ChooseTargetPlayers) pause
        /// before the fan-out begins (CR 601.2c/608.2b — choosing zero is legal). A modifier
        /// rather than a member of [`PlayerSet`]: the seats aren't derivable until a player
        /// answers, so no seat resolver can name them up front.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        chosen_by_controller: bool,
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
        /// Balance's "Each player chooses a number of lands they control equal to the number of
        /// lands controlled by the player who controls the fewest, then sacrifices the rest": the
        /// number sacrificed isn't fixed but each player's own excess over the smallest matching
        /// battlefield in `who`, measured once as this effect starts. Overrides `count`; a player
        /// already at that floor is skipped entirely, the way an empty-handed seat is skipped by a
        /// discard fan-out.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        down_to_fewest: bool,
        /// Lich's "sacrifice that many nontoken permanents. If you can't, you lose the game": an
        /// affected player who controls fewer than `count` matching permanents is eliminated
        /// outright (CR 104.3b) instead of sacrificing what they have. Distinct from every other
        /// edict, where a short board just means sacrificing all of it.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        lose_game_if_short: bool,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },

    JoinForcesPayMana,

    /// Kudzu's "That land's controller may attach this Aura to a land of their choice" — the Aura
    /// moves itself, and the seat that picks its new host is the one the trigger is about rather
    /// than the Aura's own controller. Raises the same [`PendingChoice::ChooseAttachHost`] a
    /// deployed Aura raises, so the answer handler, the attachment event and the state-based
    /// orphan exemption all come from there; only the chooser and the "may" differ.
    ///
    /// Declining is a real answer, not a no-op: the Aura is left unattached and CR 704.5m sweeps
    /// it to the graveyard on the next check, which is what "may" buys you on this card.
    TriggeringPlayerMayAttachThisAuraToChosen {
        /// The hosts on offer ("a **land** of their choice"). Evaluated for the chooser, so a
        /// `FilterController::You` axis would read as *their* battlefield, not the Aura's
        /// controller's — Kudzu's own filter has no controller restriction.
        filter: PermanentFilter,
        /// Filled in at trigger placement from the permanent the trigger is about, the same slot
        /// [`Effect::Damage(DamageEffect::ToTriggeringPlayer)`](crate::DamageEffect) fills for
        /// Psychic Venom's "that land's controller".
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        player: Option<PlayerId>,
    },

    /// Power Leak's "that player may pay any amount of mana. This Aura deals 2 damage to that
    /// player. Prevent X of that damage, where X is the amount of mana that player paid this way"
    /// (CR 615). The single-seat twin of [`JoinForcesPayMana`](Self::JoinForcesPayMana): the same
    /// unbounded generic payment and the same pause, offered to the one player the trigger is
    /// about instead of to the whole table, and cashed out as a prevention shield on that same
    /// player rather than as a shared `X` a later step reads.
    ///
    /// `prevent_up_to` is the damage the following step deals, and caps the shield: "prevent X of
    /// *that* damage" can't bank the overpayment against an unrelated hit later in the turn.
    TriggeringPlayerMayPayAnyAmountToPrevent {
        prevent_up_to: Amount,
        /// Filled in at trigger placement from the enchanted permanent's controller, the same slot
        /// [`Effect::Damage(DamageEffect::ToTriggeringPlayer)`](crate::DamageEffect) fills.
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        player: Option<PlayerId>,
    },

    /// Paralyze's "that player may pay {4}. If the player does, untap the creature" (CR 603.3b).
    /// The upside twin of [`PayOrElse`](Self::PayOrElse) — paying *buys* `then` rather than
    /// dodging a penalty — and the fixed-cost twin of
    /// [`TriggeringPlayerMayPayAnyAmountToPrevent`](Self::TriggeringPlayerMayPayAnyAmountToPrevent),
    /// with which it shares its payer: the offer goes to the player the trigger is about, not to
    /// the ability's controller, which is the one thing an ability-level `[abilities.cost]` (Mana
    /// Vault's "you may pay {4}") cannot express.
    TriggeringPlayerMayPay {
        cost: Cost,
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_slice"))]
        then: &'static [Effect],
        /// Filled in at trigger placement from the enchanted permanent's controller, the same slot
        /// [`Effect::Damage(DamageEffect::ToTriggeringPlayer)`](crate::DamageEffect) fills.
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        player: Option<PlayerId>,
    },

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

    /// "`who` may draw `count` cards" — Questing Phelddagrif's "target opponent may draw a card",
    /// Edric's "that creature's controller may draw a card". The *drawer* answers, not the
    /// ability's controller (that is [`MayDrawUnlessPays`](Self::MayDrawUnlessPays)), so `who`
    /// names one seat and a multi-seat set is rejected at resolution.
    ///
    /// Distinct from [`MayDrawUpTo`](Self::MayDrawUpTo), which asks for a *number* rather than
    /// yes/no.
    MayDraw {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        who: PlayerSet,
        count: Amount,
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

    /// "You may have it block an attacking creature of your choice" (CR 601.2c — False Orders):
    /// the ability's own target, just pulled out of combat by the preceding
    /// [`ControlEffect::RemoveFromCombat`](crate::ControlEffect), is offered back to the *spell's*
    /// controller as a fresh blocker. Pauses on a
    /// [`PendingChoice::ChooseBlockTarget`](crate::PendingChoice) over the attackers that creature
    /// could legally have been declared as blocking; declining leaves it out of combat. Fieldless
    /// like [`Self::MayPutCounterOnCreature`] above — the creature is the shared target, and the
    /// candidate list is read live off combat when the choice is raised.
    MayBlockAttackerOfYourChoice,

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
        /// "…sacrifice a land of **an opponent's choice**" (Demonic Hordes). The controller still
        /// loses the permanent — only the pick moves, which is the one thing CR 701.16a's "the
        /// permanents' controller chooses" default doesn't allow. ponytail: "an opponent" is
        /// underspecified at a pod, so the next living seat in turn order answers; see the card's
        /// `approximates`.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        opponent_chooses: bool,
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

    /// "Each defending player divides all creatures without flying they control into a 'left'
    /// pile and a 'right' pile. Then, for each attacking creature you control, choose 'left' or
    /// 'right.' That creature can't be blocked this combat except by creatures with flying and
    /// creatures in a pile with the chosen label." (Raging River.) Two pauses, in that order:
    /// every defending player divides ([`PendingChoice::SplitBlockersIntoPiles`]), then the
    /// ability's controller labels each of their attacking creatures
    /// ([`PendingChoice::ChoosePileForAttacker`]). The payoff is the *unnamed* pile becoming
    /// illegal blockers for that one attacker, recorded on
    /// [`CombatState::cant_block_this_combat`](crate::CombatState).
    DefendersSplitBlockersIntoPiles,
    /// "This turn, instead of declaring blockers, each defending player chooses any number of
    /// creatures they control and divides them into a number of piles equal to the number of
    /// attacking creatures for whom that player is the defending player. … Assign each pile to a
    /// different one of those attacking creatures at random. Each creature in a pile that can
    /// block the creature that pile is assigned to does so." (Camouflage.) The defender's own
    /// [`PendingChoice::DivideBlockersIntoPiles`] is asked once per pile, and the deal that
    /// follows *replaces* their declaration — the blocks are written down here, so the
    /// declare-blockers step finds their seat already sealed.
    DefendersDivideBlockersAmongAttackers,
    /// "Look at target opponent's hand and choose a card from it. You control that player until
    /// this spell finishes resolving. The player plays that card if able." (CR 720.1 — Word of
    /// Command.) One seat answers, another seat's resources are spent: the pause addresses the
    /// spell's *controller*, and the card is then played by its own controller, from their hand,
    /// paying with their own mana ([`Game::settle_payment`](crate::Game) auto-taps only that
    /// player's sources, which is exactly the printed mana restriction). The compelled play
    /// ignores priority and timing, so a sorcery can land on your turn in response to nothing.
    /// "If able" is literal — an unaffordable or otherwise unplayable pick simply does nothing.
    ControlPlayerToPlayCardFromHand {
        target: TargetSpec,
    },

    TargetPlayerExilesFromGraveyard {
        target: TargetSpec,
    },
}
