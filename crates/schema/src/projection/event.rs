//! [`engine::Event`] → [`crate::event::VisibleEvent`] projection with per-viewer redaction.

use crate::event::VisibleEvent;
use crate::intent::WireTarget;
use crate::projection::privacy::redact_private;

/// A mana kind as a wire code: 0-4 = WUBRG, 5 = colorless `{C}`, 6 = any color,
/// 7 = either of two colors (a dual's credit), 8 = a restricted color-set credit
/// ([`engine::Mana::OfColors`] — Fellwar Stone / Exotic Orchard).
/// ponytail: *which* colors a dual/restricted-set credit spans isn't surfaced, nor is a
/// spend-restricted ([`engine::Mana::Restricted`]) credit's restriction — shown as its `base`
/// kind's plain code instead. The client renders no pool detail beyond WUBRG/colorless/any
/// counts; widen this when it does.
fn mana_code(mana: engine::Mana) -> u8 {
    use engine::Mana;
    match mana {
        Mana::Color(c) => c.index() as u8,
        Mana::Colorless => 5,
        Mana::Any => 6,
        Mana::Either(_, _) => 7,
        Mana::OfColors(_) => 8,
        Mana::Restricted { base, .. } => match base {
            engine::RestrictedManaBase::Color(c) => c.index() as u8,
            engine::RestrictedManaBase::Colorless => 5,
            engine::RestrictedManaBase::Any => 6,
        },
    }
}

/// Project a canonical engine event into what `viewer` is allowed to see.
/// `viewer` is `Some(seat)` for a player, `None` for a spectator — the owner-gated
/// branches below then simply never fire for a spectator.
pub(crate) fn project_event(
    event: &engine::Event,
    viewer: Option<engine::PlayerId>,
) -> VisibleEvent {
    use engine::Event;
    match *event {
        Event::SpellCast {
            spell,
            from,
            controller,
            target,
            // ponytail: the chosen {X} isn't surfaced on the wire (the client reads the resolved
            // amounts off the effect events); add an `x` to VisibleEvent::SpellCast when the UI
            // wants to show "cast for X=3." Likewise, the chosen modal modes aren't surfaced — the
            // client reads which modes resolved off the effect events.
            x: _,
            modes: _,
            flashback,
            escape,
            // ponytail: the sacrifice count isn't surfaced on the wire either, same reasoning as
            // `x`/`modes` above — no UI reads it yet (it feeds a copy-per-sacrifice rider that
            // doesn't exist). Add it when that UI does.
            sacrifice_count: _,
            // ponytail: whether the spell was kicked isn't surfaced on the wire either, same
            // reasoning as `sacrifice_count` above — no UI reads it yet. Add it when a kicker
            // card's UI wants to show "kicked."
            kicked: _,
            // ponytail: whether the spell was bought back (CR 702.27) isn't surfaced on the wire
            // either, same reasoning as `sacrifice_count`/`kicked` above — no UI reads it yet (the
            // client sees the resulting `ReturnedToHand` event instead). Add it when a buyback
            // card's UI wants to badge the cast itself as "bought back."
            bought_back: _,
            // ponytail: the declared Strive count isn't surfaced on the wire either, same
            // reasoning as `sacrifice_count`/`kicked` above — no UI reads it yet. Add it when a
            // Strive card's UI wants to show "cast for N targets."
            strive_count: _,
            // ponytail: the declared Replicate count isn't surfaced on the wire either, same
            // reasoning as `sacrifice_count`/`kicked`/`strive_count` above — no UI reads it yet.
            // Add it when a Replicate card's UI wants to show "cast with N copies."
            replicate_count: _,
            // ponytail: whether the spell was bestowed (CR 702.103) isn't surfaced on the wire
            // either, same reasoning as `sacrifice_count`/`kicked` above — no UI reads it yet. Add
            // a `bestowed` to VisibleEvent::SpellCast when the UI wants to show "bestowed."
            bestowed: _,
            // ponytail: whether the spell was cast face down (CR 702.37) isn't surfaced on the
            // wire either — the client reads face-down status off the resulting permanent's
            // redacted catalog entry, not this event. Add a `face_down` here when a UI wants to
            // badge the stack item itself as a morph cast.
            face_down: _,
            // ponytail: whether the face-down cast was Illusionary Mask's (CR 615 `masked`) isn't
            // surfaced on the wire — the client reads face-down status off the redacted catalog
            // entry, and the reveal arrives as its own `TurnedFaceUp` event. Add a `masked` here
            // only if a UI wants to badge the stack item itself.
            masked: _,
            // ponytail: whether the spell was cast for evoke (CR 702.74a) isn't surfaced on the
            // wire either, same reasoning as `bestowed`/`face_down` above — no UI reads it yet
            // (the client reads the resulting sacrifice off its own event). Add an `evoked` here
            // when a UI wants to badge the stack item itself as an evoke cast.
            evoked: _,
            // ponytail: the colors spent to cast (CR 106.9) aren't surfaced on the wire — the
            // mana you spent isn't secret, but no UI reads it yet (it feeds an ETB self-sacrifice
            // condition the client just sees the resulting event for). Add it here if a UI ever
            // wants to badge the cast itself with which colors funded it.
            spent_colors: _,
        } => VisibleEvent::SpellCast {
            spell,
            from,
            controller: controller.0,
            target: target.map(WireTarget::of),
            flashback,
            escape,
        },
        Event::SpellTargetsChosen { spell, targets, .. } => VisibleEvent::SpellTargetsChosen {
            spell,
            targets: targets.iter().map(WireTarget::of).collect(),
        },
        Event::PreparedChanged { object, prepared } => {
            VisibleEvent::PreparedChanged { object, prepared }
        }
        Event::LeveledUp { source, level } => VisibleEvent::LeveledUp {
            object: source,
            level,
        },
        Event::PhasedOut { object } => VisibleEvent::PhasedOut { object },
        Event::PhasedIn { object } => VisibleEvent::PhasedIn { object },
        Event::CreatureTypeChosen { object, subtype } => VisibleEvent::CreatureTypeChosen {
            object,
            subtype: subtype.to_string(),
        },
        Event::ColorChosen { object, color } => VisibleEvent::ColorChosen {
            object,
            color: color.index() as u8,
        },
        Event::PreparedSpellCast {
            spell,
            source,
            controller,
            target,
            x,
        } => VisibleEvent::PreparedSpellCast {
            spell,
            source,
            controller: controller.0,
            target: target.map(WireTarget::of),
            x,
        },
        Event::AdventureSpellCast {
            spell,
            source,
            controller,
            target,
            x,
        } => VisibleEvent::AdventureSpellCast {
            spell,
            source,
            controller: controller.0,
            target: target.map(WireTarget::of),
            x,
        },
        Event::StepBegan {
            step,
            active_player,
        } => VisibleEvent::StepBegan {
            step: step as u8,
            active_player: active_player.0,
        },
        Event::TriggeredAbilityOnStack {
            controller,
            source,
            target,
            ..
        } => VisibleEvent::TriggeredAbilityOnStack {
            controller: controller.0,
            source,
            target: target.map(WireTarget::of),
        },
        Event::AbilityResolved { source } => VisibleEvent::AbilityResolved { source },
        Event::AbilityCountered { source } => VisibleEvent::AbilityCountered { source },
        Event::LandPlayed {
            permanent,
            from,
            player,
        } => VisibleEvent::LandPlayed {
            permanent,
            from,
            player: player.0,
        },
        Event::Tapped { object } => VisibleEvent::Tapped { object },
        Event::Untapped { object } => VisibleEvent::Untapped { object },
        Event::RegenerationShieldCreated { object } => {
            VisibleEvent::RegenerationShieldCreated { object }
        }
        Event::Regenerated { object } => VisibleEvent::Regenerated { object },
        Event::RegenerationShieldsExpired { object } => {
            VisibleEvent::RegenerationShieldsExpired { object }
        }
        Event::LostSummoningSickness { object } => VisibleEvent::LostSummoningSickness { object },
        Event::CountersPlaced {
            object,
            count,
            source_name: _,
        } => VisibleEvent::CountersPlaced { object, count },
        Event::KindCountersPlaced {
            object,
            kind,
            count,
        } => VisibleEvent::KindCountersPlaced {
            object,
            counter_kind: kind as u8,
            count,
        },
        Event::LoyaltyChanged { object, amount } => VisibleEvent::LoyaltyChanged { object, amount },
        Event::LoyaltyActivated { object, active } => {
            VisibleEvent::LoyaltyActivated { object, active }
        }
        Event::AbilityActivatedThisTurn {
            object,
            ability_index,
        } => VisibleEvent::AbilityActivatedThisTurn {
            object,
            ability_index,
        },
        Event::TriggeredAbilityThisTurn { source } => {
            VisibleEvent::TriggeredAbilityThisTurn { source }
        }
        Event::AttachedTo { object, host } => VisibleEvent::AttachedTo { object, host },
        // ponytail: the granted keywords aren't threaded onto the wire event — the client's
        // per-object power/toughness/keyword state already comes from a fresh snapshot each
        // delta, so the log only needs the P/T half. Add a `keywords` field here if the log UI
        // ever needs to say *why* a creature is temporarily unblockable/indestructible.
        Event::TempBoost {
            object,
            power,
            toughness,
            keywords: _,
            source_name: _,
        } => VisibleEvent::TempBoost {
            object,
            power,
            toughness,
        },
        Event::TempBoostsEnded { object } => VisibleEvent::TempBoostsEnded { object },
        Event::BasePtSetUntilEndOfTurn {
            object,
            power,
            toughness,
        } => VisibleEvent::BasePtSetUntilEndOfTurn {
            object,
            power,
            toughness,
        },
        // ponytail: the added types/subtypes/colors aren't threaded onto the wire event, same
        // rationale as `KeywordsStripped` below — the client's per-object type state comes from a
        // fresh snapshot.
        Event::TypesAddedUntilEndOfTurn {
            object,
            types: _,
            subtypes: _,
            colors: _,
        } => VisibleEvent::TypesAddedUntilEndOfTurn { object },
        // ponytail: the set characteristics aren't threaded onto the wire event, same rationale as
        // `TypesAddedUntilEndOfTurn` above — the client's per-object state comes from a fresh
        // snapshot each delta.
        Event::ReanimatedCreatureBecame { object, .. } => {
            VisibleEvent::ReanimatedCreatureBecame { object }
        }
        // ponytail: the added subtypes aren't threaded onto the wire event, same rationale as
        // `ReanimatedCreatureBecame` above — the client's per-object state comes from a fresh
        // snapshot each delta.
        Event::AddedSubtypes { object, .. } => VisibleEvent::AddedSubtypes { object },
        // ponytail: the copied def isn't threaded onto the wire event, same rationale as
        // `AddedSubtypes` above — the client's per-object state comes from a fresh snapshot each
        // delta.
        Event::BecameCopy { object, .. } => VisibleEvent::BecameCopy { object },
        // ponytail: the stripped keywords aren't threaded onto the wire event, same rationale as
        // `TempBoost` above — the client's per-object keyword state comes from a fresh snapshot
        // each delta.
        Event::KeywordsStripped {
            object,
            keywords: _,
        } => VisibleEvent::KeywordsStripped { object },
        Event::ControlGainedUntilEndOfTurn {
            object,
            controller,
            source_name: _,
        } => VisibleEvent::ControlGainedUntilEndOfTurn {
            object,
            controller: controller.0,
        },
        Event::ControlEndedUntilEndOfTurn { object } => {
            VisibleEvent::ControlEndedUntilEndOfTurn { object }
        }
        Event::AbilitiesGranted { target, source } => {
            VisibleEvent::AbilitiesGranted { target, source }
        }
        Event::GrantedAbilitiesEnded => VisibleEvent::GrantedAbilitiesEnded,
        Event::ControlGained { object, controller } => VisibleEvent::ControlGained {
            object,
            controller: controller.0,
        },
        Event::ConditionedControlGained {
            object,
            controller,
            condition: _,
        } => VisibleEvent::ConditionedControlGained {
            object,
            controller: controller.0,
        },
        Event::ConditionedControlEnded { object } => {
            VisibleEvent::ConditionedControlEnded { object }
        }
        Event::AttackerDeclared { object, defender } => VisibleEvent::AttackerDeclared {
            object,
            defender: defender.0,
        },
        Event::TokenEnteredAttacking { token, defender } => VisibleEvent::TokenEnteredAttacking {
            token,
            defender: defender.0,
        },
        Event::Goaded {
            object,
            by,
            source_name: _,
        } => VisibleEvent::Goaded { object, by: by.0 },
        Event::GoadCleared { by } => VisibleEvent::GoadCleared { by: by.0 },
        Event::VowCountersPlaced { object, protected } => VisibleEvent::VowCountersPlaced {
            object,
            protected: protected.0,
        },
        Event::TimeCountersPlaced { card, count } => {
            VisibleEvent::TimeCountersPlaced { card, count }
        }
        Event::TimeCountersRemoved { card } => VisibleEvent::TimeCountersRemoved { card },
        Event::MustAttackDeclared { object, defender } => VisibleEvent::MustAttackDeclared {
            object,
            defender: defender.0,
        },
        Event::DelayedTriggerScheduled {
            controller, source, ..
        } => VisibleEvent::DelayedTriggerScheduled {
            controller: controller.0,
            source,
        },
        Event::DelayedTriggersFired { .. } => VisibleEvent::DelayedTriggersFired,
        Event::NextCastTriggerArmed {
            controller, source, ..
        } => VisibleEvent::NextCastTriggerArmed {
            controller: controller.0,
            source,
        },
        Event::NextCastTriggerConsumed { controller, source } => {
            VisibleEvent::NextCastTriggerConsumed {
                controller: controller.0,
                source,
            }
        }
        Event::CombatDamageWatchArmed {
            controller,
            source,
            watched,
        } => VisibleEvent::CombatDamageWatchArmed {
            controller: controller.0,
            source,
            watched,
        },
        Event::CombatDamageWatchConsumed { controller, source } => {
            VisibleEvent::CombatDamageWatchConsumed {
                controller: controller.0,
                source,
            }
        }
        Event::CombatDamageCopyArmed {
            controller,
            source,
            card,
        } => VisibleEvent::CombatDamageCopyArmed {
            controller: controller.0,
            source,
            card,
        },
        Event::ExiledFromLibraryMayPlay {
            player,
            card,
            from,
            until_next_turn,
        } => VisibleEvent::ExiledFromLibraryMayPlay {
            player: player.0,
            card,
            from,
            until_next_turn,
        },
        Event::ExiledFromLibraryToChooseCastFree {
            player,
            card,
            from,
            face_down: _, // per-viewer redaction lives in the `state.objects` snapshot, not here.
        } => VisibleEvent::ExiledFromLibraryToChooseCastFree {
            player: player.0,
            card,
            from,
        },
        Event::PlayFromExilePermissionArmed { card } => {
            VisibleEvent::PlayFromExilePermissionArmed { card }
        }
        Event::PlayFromExileEnded => VisibleEvent::PlayFromExileEnded,
        Event::ExiledFromGraveyardMayPlay { player, card, from } => {
            VisibleEvent::ExiledFromGraveyardMayPlay {
                player: player.0,
                card,
                from,
            }
        }
        Event::Discarded {
            card, from, player, ..
        } => VisibleEvent::Discarded {
            card,
            from,
            player: player.0,
        },
        Event::BlockerDeclared { blocker, attacker } => {
            VisibleEvent::BlockerDeclared { blocker, attacker }
        }
        Event::CombatDamageDivided {
            attacker,
            assignment,
        } => VisibleEvent::CombatDamageDivided {
            attacker,
            assignment: assignment.pairs(),
        },
        Event::SpellDamageDivided {
            spell,
            assignment,
            players,
        } => VisibleEvent::SpellDamageDivided {
            spell,
            assignment: assignment.pairs(),
            players: players
                .into_iter()
                .flatten()
                .map(|(p, amt)| (p.0, amt))
                .collect(),
        },
        Event::SpellCountersDivided { spell, assignment } => VisibleEvent::SpellCountersDivided {
            spell,
            assignment: assignment.pairs(),
        },
        Event::DeathtouchMarked { object } => VisibleEvent::DeathtouchMarked { object },
        Event::CombatCleared => VisibleEvent::CombatCleared,
        Event::CommanderCastFromCommandZone { player } => {
            VisibleEvent::CommanderCastFromCommandZone { player: player.0 }
        }
        Event::FlashPermissionGranted { player } => {
            VisibleEvent::FlashPermissionGranted { player: player.0 }
        }
        Event::ChannelColorlessManaGranted { player } => {
            VisibleEvent::ChannelColorlessManaGranted { player: player.0 }
        }
        Event::CommanderDamageDealt {
            source,
            player,
            amount,
        } => VisibleEvent::CommanderDamageDealt {
            source,
            player: player.0,
            amount,
        },
        Event::CombatDamageDealtToPlayer {
            source,
            player,
            amount,
        } => VisibleEvent::CombatDamageDealtToPlayer {
            source,
            player: player.0,
            amount,
        },
        Event::DamageDealtToPlayer {
            source,
            player,
            amount,
        } => VisibleEvent::DamageDealtToPlayer {
            source,
            player: player.0,
            amount,
        },
        Event::CombatDamagePrevented { player, amount } => VisibleEvent::CombatDamagePrevented {
            player: player.0,
            amount,
        },
        Event::MovedToCommandZone { card, from } => VisibleEvent::MovedToCommandZone { card, from },
        Event::ManaEmptied { player, .. } => VisibleEvent::ManaEmptied { player: player.0 },
        Event::DamageCleared { object } => VisibleEvent::DamageCleared { object },
        Event::ManaAdded {
            player,
            mana,
            amount,
            ..
        } => VisibleEvent::ManaAdded {
            player: player.0,
            mana: mana_code(mana),
            amount,
        },
        Event::ManaSpent { player, mana } => VisibleEvent::ManaSpent {
            player: player.0,
            mana: mana.colored.to_vec(),
        },
        Event::PriorityPassed { player } => VisibleEvent::PriorityPassed { player: player.0 },
        Event::ReanimatedToBattlefield {
            permanent,
            from,
            controller,
            finality,
            tapped,
        } => VisibleEvent::ReanimatedToBattlefield {
            permanent,
            from,
            controller: controller.0,
            finality,
            tapped,
        },
        Event::PermanentEntered { permanent, from } => {
            VisibleEvent::PermanentEntered { permanent, from }
        }
        Event::TokenCreated {
            token, controller, ..
        } => VisibleEvent::TokenCreated {
            token,
            controller: controller.0,
        },
        Event::TokenCeasedToExist { token, .. } => VisibleEvent::TokenCeasedToExist { token },
        Event::SpellCopied {
            copy,
            original,
            controller,
        } => VisibleEvent::SpellCopied {
            copy,
            original,
            controller: controller.0,
        },
        Event::SpellCeasedToExist { spell } => VisibleEvent::SpellCeasedToExist { spell },
        Event::DamageMarked {
            object,
            amount,
            source,
        } => VisibleEvent::DamageMarked {
            object,
            amount,
            source,
        },
        Event::MovedToGraveyard { card, from } => VisibleEvent::MovedToGraveyard { card, from },
        Event::MovedToExile { card, from } => VisibleEvent::MovedToExile { card, from },
        Event::ExiledOnAdventure { card, from, owner } => VisibleEvent::ExiledOnAdventure {
            card,
            from,
            owner: owner.0,
        },
        Event::ExiledUntilSourceLeaves { source, object } => {
            VisibleEvent::ExiledUntilSourceLeaves { source, object }
        }
        Event::ExiledUntilSourceLeavesMintingIllusion { source, object } => {
            VisibleEvent::ExiledUntilSourceLeavesMintingIllusion { source, object }
        }
        Event::LeavesIllusionMinted { source, object } => {
            VisibleEvent::LeavesIllusionMinted { source, object }
        }
        Event::TokenGrantedReturnExiledOnLeave { token, exiled } => {
            VisibleEvent::TokenGrantedReturnExiledOnLeave { token, exiled }
        }
        Event::ReturnedExiledCardToGraveyard { card, from } => {
            VisibleEvent::ReturnedExiledCardToGraveyard { card, from }
        }
        Event::ExiledWithSource { source, object } => {
            VisibleEvent::ExiledWithSource { source, object }
        }
        Event::CardExiledWithSourceLeftExile { source, object } => {
            VisibleEvent::CardExiledWithSourceLeftExile { source, object }
        }
        Event::ReturnedFromLinkedExile {
            permanent,
            from,
            controller,
            source,
        } => VisibleEvent::ReturnedFromLinkedExile {
            permanent,
            from,
            controller: controller.0,
            source,
        },
        Event::FlickeredToBattlefield {
            permanent,
            from,
            controller,
        } => VisibleEvent::FlickeredToBattlefield {
            permanent,
            from,
            controller: controller.0,
        },
        Event::ReturnedToHand { card, from } => VisibleEvent::ReturnedToHand { card, from },
        Event::TuckedToLibrary { card, from, to_top } => {
            VisibleEvent::TuckedToLibrary { card, from, to_top }
        }
        Event::LibraryShuffled { player } => VisibleEvent::LibraryShuffled { player: player.0 },
        // A reveal is public (CR 701.30) — every viewer, including a spectator, sees it.
        Event::RevealedTopOfLibrary { player, card, def } => VisibleEvent::RevealedTopOfLibrary {
            player: player.0,
            card,
            def: def.name.to_string(),
        },
        Event::PutOnBottomOfLibrary { player, card } => VisibleEvent::PutOnBottomOfLibrary {
            player: player.0,
            card,
        },
        Event::SearchedToHand {
            player,
            object,
            from,
            card,
        } => VisibleEvent::SearchedToHand {
            player: player.0,
            object,
            from: redact_private(player, viewer, from),
            card: redact_private(player, viewer, card.name.to_string()),
        },
        Event::SearchedToBattlefield {
            permanent,
            from,
            controller,
            tapped,
        } => VisibleEvent::SearchedToBattlefield {
            permanent,
            from,
            controller: controller.0,
            tapped,
        },
        // The manifested card's identity is private (it came off the library) — drop `from`, the
        // anonymous 2/2 is projected via `ObjectView::face_down`.
        Event::Manifested {
            permanent,
            controller,
            ..
        } => VisibleEvent::Manifested {
            permanent,
            controller: controller.0,
        },
        Event::TurnedFaceUp { permanent } => VisibleEvent::TurnedFaceUp { permanent },
        Event::PutOntoBattlefieldFromHand {
            permanent,
            from,
            controller,
            tapped,
        } => VisibleEvent::PutOntoBattlefieldFromHand {
            permanent,
            from,
            controller: controller.0,
            tapped,
        },
        Event::Milled { player, card, from } => VisibleEvent::Milled {
            player: player.0,
            card,
            from,
        },
        Event::LifeChanged {
            player,
            amount,
            source,
        } => VisibleEvent::LifeChanged {
            player: player.0,
            amount,
            source,
        },
        Event::DrewFromEmptyLibrary { player } => {
            VisibleEvent::DrewFromEmptyLibrary { player: player.0 }
        }
        Event::PlayerLost { player } => VisibleEvent::PlayerLost { player: player.0 },
        Event::CitysBlessingGained { player } => {
            VisibleEvent::CitysBlessingGained { player: player.0 }
        }
        Event::CardDrawn {
            player,
            object,
            from,
            card,
        } => VisibleEvent::CardDrawn {
            player: player.0,
            object,
            from: redact_private(player, viewer, from),
            card: redact_private(player, viewer, card.name.to_string()),
        },
        Event::Sacrificed { object, by, .. } => VisibleEvent::Sacrificed { object, by: by.0 },
        Event::CastFromExileFreePermissionGranted { card, player } => {
            VisibleEvent::CastFromExileFreePermissionGranted {
                card,
                player: player.0,
            }
        }
        Event::CastFromExileFreeBottomsLibraryOnLeave { card } => {
            VisibleEvent::CastFromExileFreeBottomsLibraryOnLeave { card }
        }
        Event::CastFromExileFreeEnded => VisibleEvent::CastFromExileFreeEnded,
    }
}
