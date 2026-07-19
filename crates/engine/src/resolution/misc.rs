//! Misc-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (ADR 0002 / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    pub(crate) fn mint_misc_family(
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
            Effect::ScheduleThisTurnCombatDamageCopy => {
                match self.resolution_frame.surge_exiled_card {
                    Some((card, _)) => vec![Event::CombatDamageCopyArmed {
                        controller,
                        source,
                        card,
                    }],
                    None => vec![],
                }
            }
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
            // Scattering Stroke's win rider (CR 603.7): schedule a delayed one-shot for the
            // controller's own next first main phase that adds {C} equal to the just-countered
            // spell's printed mana value (captured now as last-known information — the counter has
            // already moved the shared target spell to the graveyard). `Main1` firing is
            // controller-scoped in `Game::fire_delayed_triggers`, so this only fires on the
            // caster's own turn.
            Effect::ScheduleColorlessManaForCounteredSpellNextMainPhase => {
                let spell =
                    expect_object_target(target, "the countered spell for the delayed mana");
                let mana_value = self.def_of(spell).mana_value().min(u8::MAX as u32) as u8;
                vec![Event::DelayedTriggerScheduled {
                    controller,
                    source,
                    fire_at: Step::Main1,
                    effect: Effect::add_colorless(mana_value),
                }]
            }
            // Pollen Lullaby's win rider: mark every creature an opponent of the controller
            // controls so it skips that controller's next untap step (CR "creatures your opponents
            // control don't untap during their controllers' next untap steps"). Each mark is
            // consumed at that permanent's controller's next untap step (see `Game::advance_step`).
            Effect::SkipNextUntapOpponentCreatures => self
                .battlefield()
                .into_iter()
                .filter(|&id| {
                    self.is_creature_on_battlefield(id) && self.controller_of(id) != controller
                })
                .map(|object| Event::NextUntapSkipMarked { object })
                .collect(),
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

            // Nezumi Graverobber: the source permanent flips to its back face (CR 712). One-way and
            // idempotent — flipping an already-flipped or vanished source is a no-op (guard-return
            // before minting, since the apply choke reads `permanent_mut`).
            Effect::FlipSource => match self.as_permanent(source) {
                Some(p) if !p.flipped => vec![Event::Flipped { object: source }],
                _ => vec![],
            },
            _ => unreachable!("misc family mint received a non-family effect"),
        }
    }
}
