//! Boundary mappers for `common.proto`'s shared wire vocabulary.
#![allow(dead_code)]

use schema::{
    CommanderDamageView, ObjectId, WireAttack, WireBlock, WireCost, WireDamage, WireEitherMana,
    WireKind, WireManaPool, WireModeChoice, WireOfColorsMana, WireSpellDamage, WireTarget,
};

use crate::grpc::pb;

fn u8s(v: impl IntoIterator<Item = u8>) -> Vec<u32> {
    v.into_iter().map(u32::from).collect()
}

pub(crate) fn u8_trunc(v: u32) -> u8 {
    v.min(255) as u8
}

/// `[u8; 5]` from a `repeated uint32` — pads with `0` when short, ignores anything past index 4.
fn colored5(v: &[u32]) -> [u8; 5] {
    let mut out = [0u8; 5];
    for (slot, &value) in out.iter_mut().zip(v.iter()) {
        *slot = u8_trunc(value);
    }
    out
}

pub fn wire_target_to_pb(target: WireTarget) -> pb::WireTarget {
    let kind = match target {
        WireTarget::Object { id } => {
            pb::wire_target::Kind::Object(pb::wire_target::ObjectTarget { id })
        }
        WireTarget::Player { player } => {
            pb::wire_target::Kind::Player(pb::wire_target::PlayerTarget {
                player: u32::from(player),
            })
        }
    };
    pb::WireTarget { kind: Some(kind) }
}

pub fn wire_target_from_pb(target: pb::WireTarget) -> Result<WireTarget, String> {
    match target.kind {
        Some(pb::wire_target::Kind::Object(pb::wire_target::ObjectTarget { id })) => {
            Ok(WireTarget::Object { id })
        }
        Some(pb::wire_target::Kind::Player(pb::wire_target::PlayerTarget { player })) => {
            Ok(WireTarget::Player {
                player: u8_trunc(player),
            })
        }
        None => Err("missing kind".to_string()),
    }
}

/// `Option<WireTarget>` variant of [`wire_target_from_pb`], for optional target fields.
pub fn opt_wire_target_from_pb(
    target: Option<pb::WireTarget>,
) -> Result<Option<WireTarget>, String> {
    target.map(wire_target_from_pb).transpose()
}

pub fn wire_block_to_pb(block: WireBlock) -> pb::WireBlock {
    pb::WireBlock {
        blocker: block.blocker,
        attacker: block.attacker,
    }
}

pub fn wire_block_from_pb(block: pb::WireBlock) -> WireBlock {
    WireBlock {
        blocker: block.blocker,
        attacker: block.attacker,
    }
}

pub fn wire_attack_to_pb(attack: WireAttack) -> pb::WireAttack {
    pb::WireAttack {
        attacker: attack.attacker,
        defender: u32::from(attack.defender),
    }
}

pub fn wire_attack_from_pb(attack: pb::WireAttack) -> WireAttack {
    WireAttack {
        attacker: attack.attacker,
        defender: u8_trunc(attack.defender),
    }
}

pub fn wire_damage_to_pb(damage: WireDamage) -> pb::WireDamage {
    pb::WireDamage {
        blocker: damage.blocker,
        amount: damage.amount,
    }
}

pub fn wire_damage_from_pb(damage: pb::WireDamage) -> WireDamage {
    WireDamage {
        blocker: damage.blocker,
        amount: damage.amount,
    }
}

pub fn wire_spell_damage_to_pb(damage: WireSpellDamage) -> pb::WireSpellDamage {
    pb::WireSpellDamage {
        target: Some(wire_target_to_pb(damage.target)),
        amount: damage.amount,
    }
}

pub fn wire_spell_damage_from_pb(damage: pb::WireSpellDamage) -> Result<WireSpellDamage, String> {
    let target = damage.target.ok_or_else(|| "missing target".to_string())?;
    Ok(WireSpellDamage {
        target: wire_target_from_pb(target)?,
        amount: damage.amount,
    })
}

pub fn wire_mode_choice_to_pb(mode: WireModeChoice) -> pb::WireModeChoice {
    pb::WireModeChoice {
        index: mode.index,
        target: mode.target.map(wire_target_to_pb),
    }
}

pub fn wire_mode_choice_from_pb(mode: pb::WireModeChoice) -> Result<WireModeChoice, String> {
    Ok(WireModeChoice {
        index: mode.index,
        target: opt_wire_target_from_pb(mode.target)?,
    })
}

pub fn wire_cost_to_pb(cost: WireCost) -> pb::WireCost {
    pb::WireCost {
        generic: u32::from(cost.generic),
        colored: u8s(cost.colored),
        has_x: cost.has_x,
        x_symbols: u32::from(cost.x_symbols),
    }
}

pub fn wire_cost_from_pb(cost: pb::WireCost) -> WireCost {
    WireCost {
        generic: u8_trunc(cost.generic),
        colored: colored5(&cost.colored),
        has_x: cost.has_x,
        x_symbols: u8_trunc(cost.x_symbols),
    }
}

pub fn wire_kind_to_pb(kind: WireKind) -> pb::WireKind {
    use pb::wire_kind::Kind;
    let kind = match kind {
        WireKind::Creature { power, toughness } => {
            Kind::Creature(pb::wire_kind::Creature { power, toughness })
        }
        WireKind::Instant => Kind::Instant(pb::wire_kind::Instant {}),
        WireKind::Sorcery => Kind::Sorcery(pb::wire_kind::Sorcery {}),
        WireKind::Enchantment => Kind::Enchantment(pb::wire_kind::Enchantment {}),
        WireKind::Artifact => Kind::Artifact(pb::wire_kind::Artifact {}),
        WireKind::Planeswalker { loyalty } => {
            Kind::Planeswalker(pb::wire_kind::Planeswalker { loyalty })
        }
        WireKind::Land { colors } => Kind::Land(pb::wire_kind::Land {
            colors: u8s(colors),
        }),
    };
    pb::WireKind { kind: Some(kind) }
}

pub fn wire_kind_from_pb(kind: pb::WireKind) -> Result<WireKind, String> {
    use pb::wire_kind::Kind;
    match kind.kind {
        Some(Kind::Creature(pb::wire_kind::Creature { power, toughness })) => {
            Ok(WireKind::Creature { power, toughness })
        }
        Some(Kind::Instant(_)) => Ok(WireKind::Instant),
        Some(Kind::Sorcery(_)) => Ok(WireKind::Sorcery),
        Some(Kind::Enchantment(_)) => Ok(WireKind::Enchantment),
        Some(Kind::Artifact(_)) => Ok(WireKind::Artifact),
        Some(Kind::Planeswalker(pb::wire_kind::Planeswalker { loyalty })) => {
            Ok(WireKind::Planeswalker { loyalty })
        }
        Some(Kind::Land(pb::wire_kind::Land { colors })) => Ok(WireKind::Land {
            colors: colors.into_iter().map(u8_trunc).collect(),
        }),
        None => Err("missing kind".to_string()),
    }
}

pub fn wire_either_mana_to_pb(mana: WireEitherMana) -> pb::WireEitherMana {
    pb::WireEitherMana {
        a: u32::from(mana.a),
        b: u32::from(mana.b),
        amount: u32::from(mana.amount),
    }
}

pub fn wire_of_colors_mana_to_pb(mana: WireOfColorsMana) -> pb::WireOfColorsMana {
    pb::WireOfColorsMana {
        mask: u32::from(mana.mask),
        amount: u32::from(mana.amount),
    }
}

pub fn wire_mana_pool_to_pb(pool: WireManaPool) -> pb::WireManaPool {
    pb::WireManaPool {
        colored: u8s(pool.colored),
        colorless: u32::from(pool.colorless),
        any: u32::from(pool.any),
        either: pool
            .either
            .into_iter()
            .map(wire_either_mana_to_pb)
            .collect(),
        of_colors: pool
            .of_colors
            .into_iter()
            .map(wire_of_colors_mana_to_pb)
            .collect(),
    }
}

pub fn commander_damage_view_to_pb(view: CommanderDamageView) -> pb::CommanderDamageView {
    pb::CommanderDamageView {
        from: u32::from(view.from),
        amount: view.amount,
    }
}

/// A divided-damage/counters share, or a commander-damage entry keyed by object.
pub fn object_amount_to_pb((id, amount): (ObjectId, i32)) -> pb::ObjectAmount {
    pb::ObjectAmount { id, amount }
}

/// A divided-damage share dealt to a player target.
pub fn player_amount_to_pb((player, amount): (u8, i32)) -> pb::PlayerAmount {
    pb::PlayerAmount {
        player: u32::from(player),
        amount,
    }
}

/// `Some(ids)` becomes a present `ObjectIdList`; `None` stays absent (proto3 field presence for
/// a `repeated` field — see `ObjectIdList`'s doc).
pub fn object_id_list_to_pb(ids: Option<Vec<ObjectId>>) -> Option<pb::ObjectIdList> {
    ids.map(|ids| pb::ObjectIdList { ids })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_round_trips() {
        let heartbeat = pb::Heartbeat {};
        let frame = pb::StreamFrame {
            frame: Some(pb::stream_frame::Frame::Heartbeat(heartbeat)),
        };
        assert!(matches!(
            frame.frame,
            Some(pb::stream_frame::Frame::Heartbeat(_))
        ));
    }

    #[test]
    fn wire_target_round_trips_both_variants() {
        let object = WireTarget::Object { id: 7 };
        assert_eq!(
            wire_target_from_pb(wire_target_to_pb(object)).unwrap(),
            object
        );

        let player = WireTarget::Player { player: 2 };
        assert_eq!(
            wire_target_from_pb(wire_target_to_pb(player)).unwrap(),
            player
        );
    }

    #[test]
    fn wire_cost_colored_pads_and_truncates_to_five() {
        let pb = pb::WireCost {
            generic: 3,
            colored: vec![1, 2],
            has_x: true,
            x_symbols: 2,
        };
        let wire = wire_cost_from_pb(pb);
        assert_eq!(wire.colored, [1, 2, 0, 0, 0]);
        assert_eq!(wire.x_symbols, 2);

        let pb_long = pb::WireCost {
            generic: 0,
            colored: vec![1, 2, 3, 4, 5, 6, 7],
            has_x: false,
            x_symbols: 0,
        };
        assert_eq!(wire_cost_from_pb(pb_long).colored, [1, 2, 3, 4, 5]);
    }
}
