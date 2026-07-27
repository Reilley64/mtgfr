//! [`engine::PendingChoice`] → [`crate::dto::PendingChoiceView`] projection.

use crate::catalog::wire_cost;
use crate::dto::{ChoiceItem, ModeView, PendingChoiceView};
use crate::intent::WireTarget;
use crate::message::{named_message, to_wire_message};
use crate::projection::privacy::private_items;
use engine::{Target, TargetSpec};

/// Union of legal cast-time targets across dig candidates that need one (Aura enchant hosts).
/// Empty when every candidate is untargeted (cascade creature hit, etc.).
fn dig_cast_targets(game: &engine::Game, candidates: &[engine::ObjectId]) -> Vec<Target> {
    let mut out = Vec::new();
    for &card in candidates {
        if game.target_spec_of(card) == TargetSpec::None {
            continue;
        }
        for target in game.legal_targets(card, None) {
            if !out.contains(&target) {
                out.push(target);
            }
        }
    }
    out
}

/// Per-snapshot context for labeling object ids and applying owner-gated privacy.
pub(crate) struct ChoiceCtx<'a> {
    game: &'a engine::Game,
    viewer: Option<engine::PlayerId>,
}

impl<'a> ChoiceCtx<'a> {
    pub(crate) fn new(game: &'a engine::Game, viewer: Option<engine::PlayerId>) -> Self {
        Self { game, viewer }
    }

    fn label_items(&self, ids: Vec<engine::ObjectId>) -> Vec<ChoiceItem> {
        ids.into_iter().map(|id| self.object_item(id)).collect()
    }

    /// An object choice item with name + default Printing (seat prints overlay later).
    fn object_item(&self, id: engine::ObjectId) -> ChoiceItem {
        let def = self.game.def_of(id);
        ChoiceItem {
            id,
            label: def.name.to_string(),
            print: def.default_print.to_string(),
            player: None,
        }
    }

    /// A player-seat choice item — no card art.
    fn player_item(&self, p: engine::PlayerId) -> ChoiceItem {
        ChoiceItem {
            id: 0,
            label: format!("Player {}", p.0 + 1),
            print: String::new(),
            player: Some(p.0),
        }
    }

    /// Label an exile pile whose cards may individually be face down (CR 701.9 — Abstract
    /// Performance's first pile, [`engine::Game::is_card_face_down`]): `owner` (the pile's
    /// controller) sees every card's real name, but any other viewer (the opponent choosing
    /// between piles, a spectator) gets an empty label for a face-down card — the id and slot
    /// still appear, so the pile's *size* stays visible, only its face-down cards' identities
    /// are hidden.
    fn label_pile(&self, owner: engine::PlayerId, ids: Vec<engine::ObjectId>) -> Vec<ChoiceItem> {
        let reveals_face_down = self.viewer == Some(owner);
        ids.into_iter()
            .map(|id| {
                let hidden = !reveals_face_down && self.game.is_card_face_down(id);
                if hidden {
                    ChoiceItem {
                        id,
                        label: String::new(),
                        print: String::new(),
                        player: None,
                    }
                } else {
                    self.object_item(id)
                }
            })
            .collect()
    }

    /// Label a player list for a choose-target-players prompt (CR "any number of target
    /// players" — Priest of Forgotten Gods), same shape [`Self::label_targets`] gives a
    /// `Target::Player`.
    fn label_players(&self, players: Vec<engine::PlayerId>) -> Vec<ChoiceItem> {
        players.into_iter().map(|p| self.player_item(p)).collect()
    }

    /// Label a target list for a choose-target prompt — objects and players.
    fn label_targets(&self, targets: Vec<engine::Target>) -> Vec<ChoiceItem> {
        targets
            .into_iter()
            .map(|t| match t {
                engine::Target::Object(id) => self.object_item(id),
                engine::Target::Player(p) => self.player_item(p),
            })
            .collect()
    }

    /// Project one engine pending choice into its wire view for this viewer.
    pub(crate) fn project(&self, pc: engine::PendingChoice) -> PendingChoiceView {
        match pc {
            engine::PendingChoice::OrderTriggers {
                player,
                source,
                effects,
            } => PendingChoiceView::OrderTriggers {
                player: player.0,
                source,
                count: effects.len() as u32,
                labels: effects
                    .iter()
                    .cloned()
                    .map(|effect| to_wire_message(effect.message()))
                    .collect(),
            },
            engine::PendingChoice::ChooseTarget {
                player,
                source,
                effect,
                legal,
                count,
                ..
            } => PendingChoiceView::ChooseTarget {
                player: player.0,
                source,
                label: effect.as_ref().map_or_else(
                    || named_message("card.name", self.game.def_of(source).name),
                    |effect| to_wire_message(effect.clone().message()),
                ),
                items: self.label_targets(legal),
                min: count.min,
                max: count.max,
            },
            engine::PendingChoice::ChooseTargetPlayers {
                player,
                source,
                legal,
                min,
                max,
                ..
            } => PendingChoiceView::ChooseTargetPlayers {
                player: player.0,
                source,
                label: named_message("card.name", self.game.def_of(source).name),
                min,
                max,
                items: self.label_players(legal),
            },
            engine::PendingChoice::MayYesNo {
                player,
                source,
                effect,
                ..
            } => PendingChoiceView::MayYesNo {
                player: player.0,
                source,
                label: to_wire_message(effect.message()),
            },
            engine::PendingChoice::MayRevealLandFromHand { player, land, .. } => {
                PendingChoiceView::MayYesNo {
                    player: player.0,
                    source: land,
                    label: crate::message::message("effect.choice_may_reveal_land_from_hand"),
                }
            }
            engine::PendingChoice::MayDrawUpTo {
                player,
                max,
                effect,
                ..
            } => PendingChoiceView::MayDrawUpTo {
                player: player.0,
                max,
                label: to_wire_message(effect.message()),
            },
            engine::PendingChoice::JoinForcesPayment { player, source, .. } => {
                PendingChoiceView::PayAnyAmountOfMana {
                    player: player.0,
                    source,
                    max: self.game.max_payable_x(player, None, |x| engine::Cost {
                        generic: x.min(u32::from(u8::MAX)) as u8,
                        ..Default::default()
                    }),
                }
            }
            engine::PendingChoice::PayCost {
                player,
                source,
                cost,
                effect,
            } => {
                let discard_count = cost.additional.discard;
                let discard_choices = (discard_count > 0).then(|| self.game.hand(player));
                PendingChoiceView::PayCost {
                    player: player.0,
                    source,
                    can_pay: self.game.can_pay_cost(player, cost),
                    cost: wire_cost(cost),
                    label: to_wire_message(effect.message()),
                    discard_count,
                    discard_choices,
                }
            }
            engine::PendingChoice::PayOrCounter {
                player,
                cost,
                spell,
            } => PendingChoiceView::PayOrCounter {
                player: player.0,
                spell,
                cost: wire_cost(cost),
            },
            engine::PendingChoice::PayOrControllerDraws {
                player,
                controller,
                cost,
            } => PendingChoiceView::PayOrControllerDraws {
                player: player.0,
                controller: controller.0,
                cost: wire_cost(cost),
            },
            engine::PendingChoice::ChooseCounteredSpellDestination { player, spell } => {
                PendingChoiceView::ChooseCounteredSpellDestination {
                    player: player.0,
                    spell,
                }
            }
            engine::PendingChoice::PayEchoOrSacrifice {
                player,
                source,
                cost,
            } => PendingChoiceView::PayEchoOrSacrifice {
                player: player.0,
                source,
                cost: wire_cost(cost),
            },
            engine::PendingChoice::PayRecoverOrExile {
                player,
                source,
                cost,
            } => PendingChoiceView::PayRecoverOrExile {
                player: player.0,
                source,
                cost: wire_cost(cost),
            },
            engine::PendingChoice::PayCumulativeUpkeepOrSacrifice {
                player,
                source,
                options,
                count,
            } => PendingChoiceView::PayCumulativeUpkeepOrSacrifice {
                player: player.0,
                source,
                items: self.label_items(options),
                count,
            },
            engine::PendingChoice::SacrificeUnlessPay {
                player,
                source,
                cost,
            } => PendingChoiceView::SacrificeUnlessPay {
                player: player.0,
                source,
                cost: wire_cost(cost),
            },
            engine::PendingChoice::PayLifeOrEntersTapped {
                player,
                source,
                life,
            } => PendingChoiceView::PayLifeOrEntersTapped {
                player: player.0,
                source,
                life,
            },
            engine::PendingChoice::SacrificeUnlessReturnLand {
                player,
                source,
                candidates,
            } => PendingChoiceView::SacrificeUnlessReturnLand {
                player: player.0,
                source,
                items: self.label_items(candidates),
            },
            engine::PendingChoice::AssignCombatDamage {
                player,
                attacker,
                blockers,
            } => PendingChoiceView::AssignCombatDamage {
                player: player.0,
                source: attacker,
                items: self.label_items(blockers),
            },
            engine::PendingChoice::DivideSpellDamage {
                player,
                spell,
                targets,
                total,
            } => PendingChoiceView::DivideSpellDamage {
                player: player.0,
                spell,
                items: self.label_targets(targets),
                total,
            },
            engine::PendingChoice::DivideCounters {
                player,
                spell,
                targets,
                total,
            } => PendingChoiceView::DivideCounters {
                player: player.0,
                spell,
                items: self.label_items(targets),
                total,
            },
            // A move-counters distribution (Forgotten Ancient) renders identically to a
            // divided-counters spell — a target→amount map up to a cap — so it reuses
            // `PendingChoiceView::DivideCounters`'s wire shape; `Game::submit` still routes the
            // answer by the distinct engine `PendingChoice` variant (see `pending::answer`).
            engine::PendingChoice::DivideMovedCounters {
                player,
                from,
                legal,
                cap,
            } => PendingChoiceView::DivideCounters {
                player: player.0,
                spell: from,
                items: self.label_items(legal),
                total: cap,
            },
            engine::PendingChoice::ArrangeTop {
                player,
                cards,
                to_graveyard,
            } => {
                let items = private_items(player, self.viewer, cards, |ids| self.label_items(ids));
                if to_graveyard {
                    PendingChoiceView::Surveil {
                        player: player.0,
                        items,
                    }
                } else {
                    PendingChoiceView::Scry {
                        player: player.0,
                        items,
                    }
                }
            }
            engine::PendingChoice::SearchLibrary {
                player, matches, ..
            } => PendingChoiceView::SearchLibrary {
                player: player.0,
                items: private_items(player, self.viewer, matches, |ids| self.label_items(ids)),
            },
            engine::PendingChoice::SelectFromTop {
                player,
                cards,
                up_to,
                ..
            } => PendingChoiceView::SelectFromTop {
                player: player.0,
                up_to,
                items: private_items(player, self.viewer, cards, |ids| self.label_items(ids)),
            },
            engine::PendingChoice::DistributeTop {
                player,
                cards,
                to_hand,
                to_bottom,
                to_exile_may_play,
            } => PendingChoiceView::DistributeTop {
                player: player.0,
                to_hand,
                to_bottom,
                to_exile_may_play,
                items: private_items(player, self.viewer, cards, |ids| self.label_items(ids)),
            },
            engine::PendingChoice::ShuffleFromGraveyard {
                player,
                owner,
                source,
                candidates,
                max,
            } => PendingChoiceView::ShuffleFromGraveyard {
                player: player.0,
                owner: owner.0,
                source,
                max,
                items: self.label_items(candidates),
            },
            engine::PendingChoice::SacrificeEdict {
                player,
                options,
                source,
                keep_one,
                ..
            } => PendingChoiceView::SacrificeEdict {
                player: player.0,
                source,
                keep_one,
                items: self.label_items(options),
            },
            engine::PendingChoice::Proliferate {
                player,
                source,
                options,
                ..
            } => PendingChoiceView::Proliferate {
                player: player.0,
                source,
                // Counter-bearing permanents and players alike (CR 701.27). Both are public
                // information — a poison total is visible to the whole table.
                items: options
                    .into_iter()
                    .map(|target| match target {
                        engine::ProliferateTarget::Permanent(id) => self.object_item(id),
                        engine::ProliferateTarget::Player(seat) => self.player_item(seat),
                    })
                    .collect(),
            },
            engine::PendingChoice::PhaseOut {
                player,
                source,
                options,
            } => PendingChoiceView::PhaseOut {
                player: player.0,
                source,
                items: self.label_items(options),
            },
            engine::PendingChoice::ChooseActivationCostTargets {
                player,
                source,
                legal,
                count,
                ..
            } => PendingChoiceView::ChooseActivationCostTargets {
                player: player.0,
                source,
                count,
                items: self.label_targets(legal),
            },
            engine::PendingChoice::MaySacrifice {
                player,
                source,
                options,
                ..
            } => PendingChoiceView::MaySacrifice {
                player: player.0,
                source,
                items: self.label_items(options),
            },
            engine::PendingChoice::ExileFromGraveyard {
                player,
                source,
                options,
                ..
            } => PendingChoiceView::ExileFromGraveyard {
                player: player.0,
                source,
                items: self.label_items(options),
            },
            engine::PendingChoice::CasterKeepPermanents {
                caster,
                source,
                target_player,
                options,
                ..
            } => PendingChoiceView::CasterKeepPermanents {
                player: caster.0,
                source,
                target_player: target_player.0,
                items: self.label_items(options),
            },
            engine::PendingChoice::ChooseCounterTargetForPlayer {
                chooser,
                source,
                target_player,
                options,
                ..
            } => PendingChoiceView::ChooseCounterTargetForPlayer {
                player: chooser.0,
                source,
                target_player: target_player.0,
                items: self.label_items(options),
            },
            // A council's-dilemma vote is a "choose one labeled option" decision — reuse the
            // ChooseMode view (labels = the ballot options), the wire twin of the engine's
            // ChooseMode-intent reuse. ponytail: split out a CastVote view if a client must render
            // votes distinctly from mode choices.
            engine::PendingChoice::CastVote {
                player,
                source,
                options,
                ..
            } => PendingChoiceView::ChooseMode {
                player: player.0,
                source,
                labels: options
                    .iter()
                    .map(|&o| named_message("choice.option", o))
                    .collect(),
            },
            engine::PendingChoice::MayReturnFromGraveyard {
                player,
                source,
                options,
                mandatory,
            } => PendingChoiceView::MayReturnFromGraveyard {
                player: player.0,
                source,
                mandatory,
                items: self.label_items(options),
            },
            engine::PendingChoice::MayExileDiscardedToPlay {
                player,
                source,
                options,
            } => PendingChoiceView::MayExileDiscardedToPlay {
                player: player.0,
                source,
                items: self.label_items(options),
            },
            engine::PendingChoice::MayDiscard {
                player,
                source,
                options,
                ..
            } => PendingChoiceView::MayDiscard {
                player: player.0,
                source,
                items: private_items(player, self.viewer, options, |ids| self.label_items(ids)),
            },
            // Zimone's Hypothesis' "+1/+1 counter on a creature" primer is still a "pick one
            // public object or decline" answer, so it reuses `ChooseCopyTarget` but flips a small
            // discriminator so the client shows counter wording instead of copy wording.
            engine::PendingChoice::MayPutCounterOnCreature {
                player,
                source,
                options,
            } => PendingChoiceView::ChooseCopyTarget {
                player: player.0,
                source,
                items: self.label_items(options),
                put_counter_on_creature: true,
            },
            engine::PendingChoice::ChooseOwnSacrifices {
                player,
                source,
                count,
                options,
                ..
            } => PendingChoiceView::ChooseOwnSacrifices {
                player: player.0,
                source,
                count,
                items: self.label_items(options),
            },
            engine::PendingChoice::Devour {
                player,
                source,
                multiplier,
                options,
            } => PendingChoiceView::Devour {
                player: player.0,
                source,
                multiplier,
                items: self.label_items(options),
            },
            engine::PendingChoice::DiscardToHandSize {
                player,
                hand,
                count,
            }
            | engine::PendingChoice::DiscardCards {
                player,
                hand,
                count,
                ..
            } => PendingChoiceView::Discard {
                player: player.0,
                count: count as u32,
                items: private_items(player, self.viewer, hand, |ids| self.label_items(ids)),
            },
            // Syphon Mind's per-opponent discard: exactly one card from a private hand — the same
            // wire shape as a `DiscardCards` of one, with the options redacted from other viewers.
            engine::PendingChoice::DiscardEdict { player, options, .. } => {
                PendingChoiceView::Discard {
                    player: player.0,
                    count: 1,
                    items: private_items(player, self.viewer, options, |ids| {
                        self.label_items(ids)
                    }),
                }
            }
            engine::PendingChoice::PutFromHandOnTop {
                player,
                hand,
                count,
            } => PendingChoiceView::PutFromHandOnTop {
                player: player.0,
                count: count as u32,
                items: private_items(player, self.viewer, hand, |ids| self.label_items(ids)),
            },
            engine::PendingChoice::DeclineUntap { player, permanents } => {
                PendingChoiceView::DeclineUntap {
                    player: player.0,
                    items: self.label_items(permanents),
                }
            }
            engine::PendingChoice::ChooseDredge {
                player, eligible, ..
            } => PendingChoiceView::ChooseDredge {
                player: player.0,
                items: self.label_items(eligible.iter().map(|(id, _)| *id).collect()),
            },
            engine::PendingChoice::PutLandFromHand {
                player, candidates, ..
            } => PendingChoiceView::PutLandFromHand {
                player: player.0,
                items: private_items(player, self.viewer, candidates, |ids| self.label_items(ids)),
            },
            engine::PendingChoice::PutCreatureFromHand {
                player, candidates, ..
            } => PendingChoiceView::PutCreatureFromHand {
                player: player.0,
                items: private_items(player, self.viewer, candidates, |ids| self.label_items(ids)),
            },
            engine::PendingChoice::CastCreatureFaceDown { player, candidates } => {
                PendingChoiceView::CastCreatureFaceDown {
                    player: player.0,
                    items: private_items(player, self.viewer, candidates, |ids| {
                        self.label_items(ids)
                    }),
                }
            }
            engine::PendingChoice::ChooseExiledWithCard {
                player,
                source,
                candidates,
            } => PendingChoiceView::ChooseExiledWithCard {
                player: player.0,
                source,
                items: self.label_items(candidates),
            },
            engine::PendingChoice::ChooseExiledWithCardToCast {
                player,
                source,
                candidates,
            } => PendingChoiceView::ChooseExiledWithCardToCast {
                player: player.0,
                source,
                items: self.label_items(candidates),
            },
            engine::PendingChoice::ChooseExiledDigToCastFree {
                player,
                source,
                candidates,
                ..
            } => {
                let cast_targets = dig_cast_targets(self.game, &candidates);
                PendingChoiceView::ChooseExiledDigToCastFree {
                    player: player.0,
                    source,
                    items: self.label_items(candidates),
                    cast_targets: self.label_targets(cast_targets),
                }
            }
            engine::PendingChoice::DanceExileMore {
                player,
                source,
                exiled,
                total_mv,
                budget,
            } => PendingChoiceView::DanceExileMore {
                player: player.0,
                source,
                total_mv,
                budget,
                items: self.label_items(exiled),
            },
            engine::PendingChoice::OpponentChoosesPile {
                player,
                controller,
                source,
                pile_a,
                pile_b,
            } => PendingChoiceView::OpponentChoosesPile {
                player: player.0,
                source,
                pile_a: self.label_pile(controller, pile_a),
                pile_b: self.label_pile(controller, pile_b),
            },
            engine::PendingChoice::OpponentChoosesExiledNonland {
                player,
                source,
                nonlands,
                ..
            } => PendingChoiceView::OpponentChoosesExiledNonland {
                player: player.0,
                source,
                items: self.label_items(nonlands),
            },
            engine::PendingChoice::ChooseSplittingOpponent {
                player,
                source,
                legal,
                ..
            } => PendingChoiceView::ChooseSplittingOpponent {
                player: player.0,
                source,
                label: named_message("card.name", self.game.def_of(source).name),
                items: self.label_players(legal),
            },
            engine::PendingChoice::PartitionRevealed {
                player,
                source,
                revealed,
                ..
            } => PendingChoiceView::PartitionRevealed {
                player: player.0,
                source,
                items: self.label_items(revealed),
            },
            engine::PendingChoice::OpponentChoosesRevealedToGraveyard {
                player,
                source,
                revealed,
                ..
            } => PendingChoiceView::OpponentChoosesRevealedToGraveyard {
                player: player.0,
                source,
                items: self.label_items(revealed),
            },
            engine::PendingChoice::ChoosePileForHand {
                player,
                source,
                pile_a,
                pile_b,
            } => PendingChoiceView::ChoosePileForHand {
                player: player.0,
                source,
                pile_a: self.label_items(pile_a),
                pile_b: self.label_items(pile_b),
            },
            engine::PendingChoice::ChooseExiledToCastFree {
                player,
                source,
                candidates,
                count,
                ..
            } => PendingChoiceView::ChooseExiledToCastFree {
                player: player.0,
                source,
                count,
                items: self.label_items(candidates),
            },
            engine::PendingChoice::RevealedCardToBattlefieldOrHand { player, card } => {
                PendingChoiceView::RevealedCardToBattlefieldOrHand {
                    player: player.0,
                    item: self.object_item(card),
                }
            }
            engine::PendingChoice::ChooseMode {
                player,
                source,
                modes,
                ..
            } => PendingChoiceView::ChooseMode {
                player: player.0,
                source,
                labels: modes
                    .iter()
                    .cloned()
                    .map(|mode| to_wire_message(mode.message()))
                    .collect(),
            },
            engine::PendingChoice::ChooseTriggerModes {
                player,
                source,
                modes,
                choose,
                optional,
            } => {
                // ponytail: every mode in the pool's only modal-trigger card (Shadrix) targets a
                // player, so `needs_target`/`targets` are unconditionally the living-player set —
                // widen if a future modal-trigger mode is targetless or targets something else.
                let targets: Vec<WireTarget> = self
                    .game
                    .legal_player_targets()
                    .into_iter()
                    .map(WireTarget::of)
                    .collect();
                PendingChoiceView::ChooseTriggerModes {
                    player: player.0,
                    source,
                    choose,
                    optional,
                    modes: modes
                        .iter()
                        .cloned()
                        .map(|effect| ModeView {
                            label: to_wire_message(effect.message()),
                            needs_target: true,
                            targets: targets.clone(),
                        })
                        .collect(),
                }
            }
            engine::PendingChoice::ChooseManaColor {
                player,
                source,
                amount,
            } => PendingChoiceView::ChooseManaColor {
                player: player.0,
                source,
                amount,
            },
            engine::PendingChoice::ChooseCreatureType {
                player,
                source,
                options,
            } => PendingChoiceView::ChooseCreatureType {
                player: player.0,
                source,
                options: options.iter().map(|s| s.to_string()).collect(),
            },
            // Same wire prompt regardless of `until_end_of_turn` — an as-enters choice
            // (Flickering Ward) and a resolution-time color-SET (Wild Mongrel) both just ask the
            // player to pick one of the five colors; the engine-internal purpose isn't surfaced.
            engine::PendingChoice::ChooseColor { player, source, .. } => {
                PendingChoiceView::ChooseColor {
                    player: player.0,
                    source,
                }
            }
            // `remaining` (who still has to name after this seat) stays engine-internal — the
            // prompt only needs to know who is naming and for which source. Nothing about the
            // chooser's library or their eventual name is projected: the reveal is a separate
            // public event.
            engine::PendingChoice::ChooseCardName { player, source, .. } => {
                PendingChoiceView::ChooseCardName {
                    player: player.0,
                    source,
                }
            }
            engine::PendingChoice::ChooseAttachHost {
                player,
                attachment,
                candidates,
                optional,
            } => PendingChoiceView::ChooseAttachHost {
                player: player.0,
                attachment,
                items: self.label_items(candidates),
                optional,
            },
            engine::PendingChoice::ChooseLegendaryKeep {
                player,
                name,
                options,
            } => PendingChoiceView::ChooseLegendaryKeep {
                player: player.0,
                name: name.to_string(),
                items: self.label_items(options),
            },
            engine::PendingChoice::ChooseCopyTarget {
                player,
                source,
                candidates,
                ..
            }
            // Brudiclad's mass token conversion projects onto the same client shape — "choose one
            // of these objects (or decline)"; the client need not distinguish it.
            | engine::PendingChoice::ChooseTokenToCopy {
                player,
                source,
                candidates,
            }
            // Spirit of Resilience's become-a-copy-from-the-graveyard-batch projects onto the same
            // client shape for the same reason.
            | engine::PendingChoice::ChooseCopyCardFromList {
                player,
                source,
                candidates,
            } => PendingChoiceView::ChooseCopyTarget {
                player: player.0,
                source,
                items: self.label_items(candidates),
                put_counter_on_creature: false,
            },
        }
    }
}

/// Project a pending choice for a seated or spectator viewer.
pub(crate) fn project_pending_choice(
    game: &engine::Game,
    viewer: Option<engine::PlayerId>,
    pc: engine::PendingChoice,
) -> PendingChoiceView {
    ChoiceCtx::new(game, viewer).project(pc)
}

#[cfg(test)]
mod coverage_tests {
    use super::project_pending_choice;
    use crate::dto::PendingChoiceView;
    use crate::test_support::def;
    use engine::{Amount, DrawEffect, Effect, Game, LifeEffect, PendingChoice, PlayerId, Target};

    const CHOOSE_ONE_MODES: &[Effect] = &[
        Effect::Draw(DrawEffect::Cards {
            count: Amount::Fixed(1),
        }),
        Effect::Life(LifeEffect::Gain {
            amount: Amount::Fixed(1),
        }),
    ];

    fn draw_effect() -> Effect {
        Effect::Draw(DrawEffect::Cards {
            count: Amount::Fixed(1),
        })
    }

    #[test]
    fn pending_choice_projection_covers_each_variant() {
        let mut game = Game::new();
        let source = game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bears"));
        let spell = game.spawn_in_hand(PlayerId(0), def("Shock"));
        let blocker = game.spawn_on_battlefield(PlayerId(1), def("Grizzly Bears"));
        let hand_card = game.spawn_in_hand(PlayerId(0), def("Forest"));

        type Case = (PendingChoice, fn(PendingChoiceView) -> bool);
        let cases: Vec<Case> = vec![
            (
                PendingChoice::OrderTriggers {
                    player: PlayerId(0),
                    source,
                    effects: vec![
                        Effect::Life(LifeEffect::Gain {
                            amount: Amount::Fixed(1),
                        }),
                        draw_effect(),
                    ],
                },
                |view| matches!(view, PendingChoiceView::OrderTriggers { count: 2, .. }),
            ),
            (
                PendingChoice::ChooseTarget {
                    player: PlayerId(0),
                    source,
                    effect: Some(draw_effect()),
                    legal: vec![Target::Object(blocker)],
                    count: engine::TargetCount::default(),
                    clause: 0,
                    target: None,
                    x: 0,
                    spent_mana: [0; 6],
                    activated: false,
                },
                |view| matches!(view, PendingChoiceView::ChooseTarget { .. }),
            ),
            (
                PendingChoice::ChooseTarget {
                    player: PlayerId(0),
                    source: spell,
                    effect: None,
                    legal: vec![Target::Object(blocker)],
                    count: engine::TargetCount::default(),
                    clause: 0,
                    target: None,
                    x: 0,
                    spent_mana: [0; 6],
                    activated: false,
                },
                |view| matches!(view, PendingChoiceView::ChooseTarget { min: 1, max: 1, .. }),
            ),
            (
                PendingChoice::MayYesNo {
                    player: PlayerId(0),
                    source,
                    effect: draw_effect(),
                    resume: engine::MayYesNoResume::Default,
                },
                |view| matches!(view, PendingChoiceView::MayYesNo { .. }),
            ),
            (
                PendingChoice::MayRevealLandFromHand {
                    player: PlayerId(0),
                    land: hand_card,
                    subtypes: &["Forest", "Island"],
                },
                |view| matches!(view, PendingChoiceView::MayYesNo { .. }),
            ),
            (
                PendingChoice::PayCost {
                    player: PlayerId(0),
                    source,
                    cost: engine::Cost::default(),
                    effect: draw_effect(),
                },
                |view| matches!(view, PendingChoiceView::PayCost { .. }),
            ),
            (
                PendingChoice::PayOrCounter {
                    player: PlayerId(0),
                    cost: engine::Cost::default(),
                    spell,
                },
                |view| matches!(view, PendingChoiceView::PayOrCounter { .. }),
            ),
            (
                PendingChoice::ChooseCounteredSpellDestination {
                    player: PlayerId(0),
                    spell,
                },
                |view| {
                    matches!(
                        view,
                        PendingChoiceView::ChooseCounteredSpellDestination { .. }
                    )
                },
            ),
            (
                PendingChoice::AssignCombatDamage {
                    player: PlayerId(0),
                    attacker: source,
                    blockers: vec![blocker],
                },
                |view| matches!(view, PendingChoiceView::AssignCombatDamage { .. }),
            ),
            (
                PendingChoice::ArrangeTop {
                    player: PlayerId(0),
                    cards: vec![hand_card],
                    to_graveyard: false,
                },
                |view| matches!(view, PendingChoiceView::Scry { .. }),
            ),
            (
                PendingChoice::ArrangeTop {
                    player: PlayerId(0),
                    cards: vec![hand_card],
                    to_graveyard: true,
                },
                |view| matches!(view, PendingChoiceView::Surveil { .. }),
            ),
            (
                PendingChoice::SearchLibrary {
                    player: PlayerId(0),
                    matches: vec![hand_card],
                    dest: engine::SearchDest::Hand,
                    tapped: false,
                    remaining: 1,
                    overflow: None,
                },
                |view| matches!(view, PendingChoiceView::SearchLibrary { .. }),
            ),
            (
                PendingChoice::SelectFromTop {
                    player: PlayerId(0),
                    cards: vec![hand_card],
                    filter: engine::CardFilter::AnyCard,
                    up_to: 1,
                    min: 0,
                    dest: engine::TopDest::Hand,
                    dest_tapped: false,
                    rest: engine::RestDest::Bottom,
                    mv_budget: None,
                },
                |view| matches!(view, PendingChoiceView::SelectFromTop { .. }),
            ),
            (
                PendingChoice::SacrificeEdict {
                    player: PlayerId(0),
                    options: vec![source],
                    keep_one: true,
                    count: 1,
                    filter: engine::PermanentFilter::default(),
                    remaining: vec![],
                    controller: PlayerId(0),
                    source,
                    follow_up: &[],
                },
                |view| {
                    matches!(
                        view,
                        PendingChoiceView::SacrificeEdict { keep_one: true, .. }
                    )
                },
            ),
            (
                PendingChoice::DiscardToHandSize {
                    player: PlayerId(0),
                    hand: vec![hand_card],
                    count: 1,
                },
                |view| matches!(view, PendingChoiceView::Discard { count: 1, .. }),
            ),
            (
                PendingChoice::DiscardCards {
                    player: PlayerId(0),
                    hand: vec![hand_card],
                    count: 2,
                    or_one_matching: None,
                },
                |view| matches!(view, PendingChoiceView::Discard { count: 2, .. }),
            ),
            (
                PendingChoice::PutLandFromHand {
                    player: PlayerId(0),
                    candidates: vec![hand_card],
                    tapped: false,
                },
                |view| matches!(view, PendingChoiceView::PutLandFromHand { .. }),
            ),
            (
                PendingChoice::PutCreatureFromHand {
                    player: PlayerId(0),
                    source,
                    candidates: vec![hand_card],
                    keep: false,
                    defender: None,
                },
                |view| matches!(view, PendingChoiceView::PutCreatureFromHand { .. }),
            ),
            (
                PendingChoice::CastCreatureFaceDown {
                    player: PlayerId(0),
                    candidates: vec![hand_card],
                },
                |view| matches!(view, PendingChoiceView::CastCreatureFaceDown { .. }),
            ),
            (
                PendingChoice::ChooseExiledWithCard {
                    player: PlayerId(0),
                    source,
                    candidates: vec![hand_card],
                },
                |view| matches!(view, PendingChoiceView::ChooseExiledWithCard { .. }),
            ),
            (
                PendingChoice::ChooseMode {
                    player: PlayerId(0),
                    source,
                    target: None,
                    x: 0,
                    modes: CHOOSE_ONE_MODES.into(),
                    at_placement: false,
                    activated: false,
                },
                |view| matches!(view, PendingChoiceView::ChooseMode { .. }),
            ),
            (
                PendingChoice::ChooseCardName {
                    player: PlayerId(0),
                    source,
                    remaining: vec![PlayerId(1)],
                },
                |view| matches!(view, PendingChoiceView::ChooseCardName { .. }),
            ),
        ];

        for (choice, check) in cases {
            let view = project_pending_choice(&game, Some(PlayerId(0)), choice);
            assert!(check(view), "unexpected projection");
        }
    }

    /// Illusionary Mask's offered hand creatures are private (CR — a hidden hand): a non-owner
    /// viewer sees the choice but not which cards, while the owner sees them.
    #[test]
    fn cast_creature_face_down_candidates_are_redacted_for_non_owner() {
        let mut game = Game::new();
        let hand_card = game.spawn_in_hand(PlayerId(0), def("Grizzly Bears"));
        let choice = PendingChoice::CastCreatureFaceDown {
            player: PlayerId(0),
            candidates: vec![hand_card],
        };

        let owner_view = project_pending_choice(&game, Some(PlayerId(0)), choice.clone());
        let PendingChoiceView::CastCreatureFaceDown { items, .. } = owner_view else {
            panic!("expected CastCreatureFaceDown");
        };
        assert_eq!(items.len(), 1, "the owner sees the offered creature");

        let opponent_view = project_pending_choice(&game, Some(PlayerId(1)), choice);
        let PendingChoiceView::CastCreatureFaceDown { items, .. } = opponent_view else {
            panic!("expected CastCreatureFaceDown");
        };
        assert!(items.is_empty(), "a non-owner sees no hand cards");
    }

    #[test]
    fn pay_cost_projects_the_paid_effect_label() {
        let mut game = Game::new();
        let source = game.spawn_on_battlefield(PlayerId(0), def("Trudge Garden"));
        let view = project_pending_choice(
            &game,
            Some(PlayerId(0)),
            PendingChoice::PayCost {
                player: PlayerId(0),
                source,
                cost: engine::Cost {
                    generic: 2,
                    ..engine::Cost::default()
                },
                effect: draw_effect(),
            },
        );
        match view {
            PendingChoiceView::PayCost { label, cost, .. } => {
                assert_eq!(label.key, "effect.draw_cards");
                assert_eq!(cost.generic, 2);
            }
            other => panic!("expected PayCost, got {other:?}"),
        }
    }

    #[test]
    fn pay_cost_reports_whether_the_player_can_actually_pay() {
        let mut game = Game::new();
        let source = game.spawn_on_battlefield(PlayerId(0), def("Trudge Garden"));
        let choice = PendingChoice::PayCost {
            player: PlayerId(0),
            source,
            cost: engine::Cost {
                generic: 2,
                ..engine::Cost::default()
            },
            effect: draw_effect(),
        };

        let broke = project_pending_choice(&game, Some(PlayerId(0)), choice.clone());
        let PendingChoiceView::PayCost { can_pay, .. } = broke else {
            panic!("expected PayCost, got {broke:?}");
        };
        assert!(!can_pay, "an empty board cannot produce {{2}}");

        game.spawn_on_battlefield(PlayerId(0), def("Mountain"));
        game.spawn_on_battlefield(PlayerId(0), def("Mountain"));
        let solvent = project_pending_choice(&game, Some(PlayerId(0)), choice);
        let PendingChoiceView::PayCost { can_pay, .. } = solvent else {
            panic!("expected PayCost");
        };
        assert!(can_pay, "two untapped lands cover {{2}}");
    }

    #[test]
    fn may_put_counter_on_creature_marks_the_reused_copy_target_view() {
        let mut game = Game::new();
        let source = game.spawn_on_battlefield(PlayerId(0), def("Zimone's Hypothesis"));
        let creature = game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bears"));
        let view = project_pending_choice(
            &game,
            Some(PlayerId(0)),
            PendingChoice::MayPutCounterOnCreature {
                player: PlayerId(0),
                source,
                options: vec![creature],
            },
        );

        let PendingChoiceView::ChooseCopyTarget {
            source: view_source,
            items,
            put_counter_on_creature,
            ..
        } = view
        else {
            panic!("expected ChooseCopyTarget");
        };
        assert_eq!(view_source, source);
        assert_eq!(
            items.len(),
            1,
            "the player still chooses from the offered creatures"
        );
        assert!(
            put_counter_on_creature,
            "the reused copy-target view must advertise counter wording"
        );
    }

    /// Herald of Amity dig-cast: an Aura candidate projects legal enchant hosts so the client
    /// can name a cast-time target with the dig answer.
    #[test]
    fn choose_exiled_dig_to_cast_free_projects_cast_targets_for_aura() {
        let mut game = Game::new();
        let host = game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bears"));
        let aura = game.spawn_in_hand(PlayerId(0), def("Spirit Mantle"));
        let source = game.spawn_on_battlefield(PlayerId(0), def("Herald of Amity"));
        let view = project_pending_choice(
            &game,
            Some(PlayerId(0)),
            PendingChoice::ChooseExiledDigToCastFree {
                player: PlayerId(0),
                source,
                candidates: vec![aura],
                exiled: vec![aura],
            },
        );

        let PendingChoiceView::ChooseExiledDigToCastFree {
            items,
            cast_targets,
            ..
        } = view
        else {
            panic!("expected ChooseExiledDigToCastFree, got {view:?}");
        };
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, aura);
        assert!(
            cast_targets.iter().any(|item| item.id == host),
            "Aura dig must project the legal host, got {cast_targets:?}"
        );
    }

    /// Cascade / untargeted dig hits: no cast-time target → empty `cast_targets`.
    #[test]
    fn choose_exiled_dig_to_cast_free_omits_cast_targets_for_untargeted() {
        let mut game = Game::new();
        let _host = game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bears"));
        let bear = game.spawn_in_hand(PlayerId(0), def("Grizzly Bears"));
        let source = game.spawn_on_battlefield(PlayerId(0), def("Herald of Amity"));
        let view = project_pending_choice(
            &game,
            Some(PlayerId(0)),
            PendingChoice::ChooseExiledDigToCastFree {
                player: PlayerId(0),
                source,
                candidates: vec![bear],
                exiled: vec![bear],
            },
        );

        let PendingChoiceView::ChooseExiledDigToCastFree { cast_targets, .. } = view else {
            panic!("expected ChooseExiledDigToCastFree, got {view:?}");
        };
        assert!(
            cast_targets.is_empty(),
            "untargeted dig must not project cast hosts, got {cast_targets:?}"
        );
    }
}
