//! Control-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (ADR 0002 / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    pub(crate) fn mint_control_family(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        _x: u32,
    ) -> Vec<Event> {
        let source_name = self.source_name_of(source);
        match effect {
            // Equip resolves by attaching the Equipment (the ability's source) to the chosen
            // creature, replacing any prior attachment.
            Effect::Equip => {
                let host = expect_object_target(target, "equip");
                vec![Event::AttachedTo {
                    object: source,
                    host: Some(host),
                }]
            }
            // Shielded by Faith / Prison Term: attach this Aura (the ability's source) to the
            // entering creature — moving it off any host it's already attached to (CR 704.5n
            // simply drops the old attachment once `apply` overwrites `attached_to`). `entering`
            // is filled at trigger placement; `None` only in an unplaced card template, which
            // never reaches resolution. Re-checks the Aura's own `enchant` filter against the
            // entering permanent (CR 303.4f-style legality) — a no-op if it isn't a legal host,
            // even though the "you may" was accepted (FIDELITY_BACKLOG #156).
            Effect::AttachSelfToEntering { entering } => {
                let host = entering.expect("filled in from the entering trigger at placement");
                if !self.attachment_host_legal(source, host) {
                    return Vec::new();
                }
                vec![Event::AttachedTo {
                    object: source,
                    host: Some(host),
                }]
            }
            Effect::GoadTarget { .. } => {
                let object = expect_object_target(target, "goad");
                vec![Event::Goaded {
                    object,
                    by: controller,
                    source_name,
                }]
            }
            Effect::TapTarget { .. } => {
                let object = expect_object_target(target, "tap");
                vec![Event::Tapped { object }]
            }
            Effect::RegenerateShield { .. } => {
                let object = expect_object_target(target, "a regeneration shield");
                vec![Event::RegenerationShieldCreated { object }]
            }
            Effect::UntapTarget { .. } => {
                let object = expect_object_target(target, "untap");
                vec![Event::Untapped { object }]
            }
            Effect::GainControlUntilEndOfTurn { .. } => {
                let object = expect_object_target(target, "a steal");
                vec![Event::ControlGainedUntilEndOfTurn {
                    object,
                    controller,
                    source_name,
                }]
            }
            Effect::GainControl { .. } => {
                let object = expect_object_target(target, "a permanent control change");
                vec![Event::ControlGained { object, controller }]
            }
            Effect::GainControlWhile {
                while_source_tapped,
                ..
            } => {
                let object = expect_object_target(target, "a conditioned steal");
                vec![Event::ConditionedControlGained {
                    object,
                    controller,
                    condition: crate::ControlCondition {
                        source,
                        needs_tapped: while_source_tapped,
                    },
                }]
            }
            // Backup's rider (CR 702.166): the shared target creature gains the source's other
            // abilities until end of turn — but only "if that's another creature", so the source
            // targeting itself grants nothing (the counter still landed in the preceding step).
            Effect::GrantSourceAbilitiesUntilEndOfTurn => {
                let object = expect_object_target(target, "Backup's ability grant");
                if object == source {
                    return Vec::new();
                }
                vec![Event::AbilitiesGranted {
                    target: object,
                    source,
                }]
            }
            // Beledros: untap every matching permanent the controller controls — the mass
            // mirror of UntapTarget, same "you control" scoping as PumpCreaturesYouControlUntilEndOfTurn.
            Effect::UntapAll { filter } => self
                .battlefield()
                .into_iter()
                .filter(|&id| {
                    self.controller_of(id) == controller
                        && self.permanent_matches(&filter, id, controller, Some(source))
                })
                .map(|object| Event::Untapped { object })
                .collect(),

            _ => unreachable!("control family mint received a non-family effect"),
        }
    }
}
