//! Misc-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (ADR 0002 / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    /// Mint events for the Misc Effect family, or [`None`] if `effect` is not in this family.
    pub(crate) fn try_mint_misc(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Option<Vec<Event>> {
        if !matches!(
            effect,
            Effect::ArmCombatDamageWatch
                | Effect::BecomePrepared
                | Effect::CounterTargetActivatedAbility
                | Effect::CounterTargetSpell { .. }
                | Effect::GrantChannelColorlessManaThisTurn
                | Effect::GrantFlashThisTurn
                | Effect::ScheduleAtNextUpkeep { .. }
                | Effect::ScheduleNextCastTrigger { .. }
                | Effect::ScheduleThisTurnCombatDamageCopy
        ) {
            return None;
        }
        Some(self.mint_misc_family(effect, controller, source, target, x))
    }

    fn mint_misc_family(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        _x: u32,
    ) -> Vec<Event> {
        let _source_name = self.source_name_of(source);
        match effect {
            // Kirol, History Buff: the source becomes prepared (idempotent if already prepared),
            // enabling its back-face copy cast (see `Game::cast_prepared`).
            Effect::BecomePrepared => vec![Event::PreparedChanged {
                object: source,
                prepared: true,
            }],
            // Stensian Sanguinist's attack trigger: arm a delayed watch on the just-deathtouched
            // shared target — its own source becomes prepared the first time that creature deals
            // combat damage to a player this combat (see `Game::fire_combat_damage_watch_triggers`). (CR 510, CR 120.3, CR 506)
            Effect::ArmCombatDamageWatch => {
                let watched = expect_object_target(target, "a combat-damage watch's armed target");
                vec![Event::CombatDamageWatchArmed {
                    controller,
                    source,
                    watched,
                }]
            }
            // Surge to Victory: arm the this-turn, controller-scoped, repeatable combat-damage-
            // copy watch over the card the preceding `Sequence` step just exiled. `None` (the
            // exile step never ran) is unreachable in practice — CR 608.2b already fizzles the
            // whole ability before either step resolves without a legal target — but a silent
            // no-op rather than a panic, matching this resolution's other snapshot-read arms.
            Effect::ScheduleThisTurnCombatDamageCopy => match self.surge_exiled_card {
                Some((card, _)) => vec![Event::CombatDamageCopyArmed {
                    controller,
                    source,
                    card,
                }],
                None => vec![],
            },
            // Alchemist's Refuge: "You may cast spells this turn as though they had flash." (CR 702.8, CR 601, CR 500)
            // ponytail: resolved as a one-shot turn-flag set (`Player::flash_permission_this_turn`) (CR 500)
            // rather than a continuous "as though they had flash" static — behaviorally identical (CR 702.8)
            // for this pool (gone at cleanup either way; nothing reads it mid-resolution before
            // the flag is set here).
            Effect::GrantFlashThisTurn => {
                vec![Event::FlashPermissionGranted { player: controller }]
            }
            // Yavimaya Bloomsage's Channel back face: "Until end of turn, any time you could (CR 605, CR 118.4)
            // activate a mana ability, you may pay 1 life. If you do, add {C}." Resolved as a
            // one-shot turn-flag set, mirroring `GrantFlashThisTurn` above.
            Effect::GrantChannelColorlessManaThisTurn => {
                vec![Event::ChannelColorlessManaGranted { player: controller }]
            }
            // Counter target spell (the unconditional hard-counter path — `unless_pays: Some(_)`
            // is intercepted earlier, in `run`, so this arm only ever sees `None`).
            Effect::CounterTargetSpell { .. } => {
                let original = expect_object_target(target, "a spell to counter");
                self.counter_spell(original)
            }
            // Counter target activated ability (CR 701.5c/112.7a — Azorius Guildmage). The target
            // is the ability's source id (see `TargetSpec::ActivatedAbilityOnStack`); the
            // `AbilityCountered` apply removes the topmost matching stack ability. A guard-return
            // (CR 608.2b) if it already left the stack is handled upstream by `target_still_legal`,
            // which fizzles this ability before it runs; this stays a no-op if nothing matches.
            Effect::CounterTargetActivatedAbility => {
                let source_id = expect_object_target(target, "an activated ability to counter");
                let on_stack = self.stack.iter().any(|item| {
                    matches!(item, StackItem::Ability { source, activated: true, .. } if *source == source_id)
                });
                if !on_stack {
                    return Vec::new();
                }
                vec![Event::AbilityCountered { source: source_id }]
            }
            // Schedule a CR 603.7 delayed trigger: resolve `who` to a concrete player now (the
            // effect itself doesn't fire until the matching step begins — see
            // `Game::fire_delayed_triggers`).
            Effect::ScheduleAtNextUpkeep { who, then, fire_at } => {
                let player = match who {
                    DelayController::You => controller,
                    DelayController::TargetSpellController => self.controller_of(
                        expect_object_target(target, "a delayed trigger's target-spell controller"),
                    ),
                };
                vec![Event::DelayedTriggerScheduled {
                    controller: player,
                    source,
                    fire_at,
                    effect: *then,
                }]
            }
            // Arm a CR 603.7 delayed one-shot: always the ability's own controller/source (Brass
            // Infiniscope has no "someone else's spell" wrinkle) — the watch itself doesn't fire
            // until a matching cast happens, see `Game::fire_next_cast_triggers`.
            Effect::ScheduleNextCastTrigger { filter, then } => {
                vec![Event::NextCastTriggerArmed {
                    controller,
                    source,
                    filter,
                    then,
                }]
            }

            _ => unreachable!("misc family mint received a non-family effect"),
        }
    }
}
