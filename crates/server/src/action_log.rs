//! Per-table intent traces for post-hoc debugging ("read the log").
//!
//! One TOON-tabular file per table under `./data/actions.<table_id>.toon`: header once, then
//! one indented row per submitted intent (accepted or rejected). Independent of DB persistence
//! (ADR 0021) — live games stay in-memory; these files are a local diagnostic only.
//!
//! ponytail: traces still hold full hidden info (Debug-formatted `CardDef`s) under `./data/`
//! (gitignored), so a static-file route can't serve them. Redact to `VisibleEvent`s if they
//! ever need to leave the box.

use engine::{Event, Game};
use schema::WireIntent;

use crate::session::ApplyResult;

const LOG_FIELDS: &str = "seq,player,intent,accepted,reason,step,active,priority,pending,events";

/// Directory the action traces live in (gitignored).
const LOG_DIR: &str = "data";

/// Each table traces to its own file so concurrent games don't interleave.
pub fn log_path(table_id: &str) -> String {
    format!("{LOG_DIR}/actions.{table_id}.toon")
}

/// Start a fresh trace for a table: truncate its file and write the TOON header.
pub fn start(table_id: &str) {
    let _ = std::fs::create_dir_all(LOG_DIR);
    start_at(&log_path(table_id));
}

/// Write the TOON header to an explicit path (test seam).
fn start_at(path: &str) {
    let _ = std::fs::write(path, format!("actions{{{LOG_FIELDS}}}:\n"));
}

/// Format one TOON-tabular row for an applied intent. Built while the table is still borrowed;
/// write with [`append`] *after* releasing the registry lock so disk I/O can't stall other
/// tables.
///
/// When `game` is `None` (e.g. a panicked table whose arena was cleared), turn-state columns
/// are `-` so the row still lands — panic/quarantine lines are the ones you most need.
pub fn format_row(
    seq: u64,
    player: u8,
    intent: &WireIntent,
    result: &ApplyResult,
    events: &[Event],
    game: Option<&Game>,
) -> String {
    format_labeled(seq, player, &intent_str(intent), result, events, game)
}

/// Like [`format_row`], but with an explicit intent label (yield-driven auto-advance, etc.).
pub fn format_labeled(
    seq: u64,
    player: u8,
    intent_label: &str,
    result: &ApplyResult,
    events: &[Event],
    game: Option<&Game>,
) -> String {
    let (step, active, priority, pending) = match game {
        Some(game) => (
            (game.current_step() as u8).to_string(),
            game.active_player().0.to_string(),
            game.priority_holder().0.to_string(),
            game.pending_choice()
                .map_or_else(|| "-".to_string(), |c| format!("{c:?}")),
        ),
        None => (
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
        ),
    };
    let fields = [
        seq.to_string(),
        player.to_string(),
        intent_label.to_string(),
        if result.accepted { "t" } else { "f" }.to_string(),
        result.reason.clone().unwrap_or_else(|| "-".to_string()),
        step,
        active,
        priority,
        pending,
        compact_events(events),
    ];
    fields
        .iter()
        .map(|f| toon_field(f))
        .collect::<Vec<_>>()
        .join(",")
}

/// Append a pre-formatted trace row. Blocking file I/O — call outside the registry lock.
pub fn append(table_id: &str, row: &str) {
    append_at(&log_path(table_id), row);
}

/// Append to an explicit path (test seam).
fn append_at(path: &str, row: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "  {row}");
    }
}

/// Quote a TOON tabular field only if it contains a delimiter (comma, quote, or newline).
fn toon_field(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// A compact one-token-ish string for an intent (the player is a separate column).
fn intent_str(w: &WireIntent) -> String {
    let opt = |t: &Option<schema::WireTarget>| {
        t.map_or_else(
            || "-".to_string(),
            |target| match target {
                schema::WireTarget::Object { id } => id.to_string(),
                schema::WireTarget::Player { player } => format!("P{player}"),
            },
        )
    };
    let opt_id =
        |id: &Option<schema::ObjectId>| id.map_or_else(|| "-".to_string(), |i| i.to_string());
    match w {
        WireIntent::Cast { object, target, .. } => format!("cast {object}>{}", opt(target)),
        WireIntent::PlayLand { object, .. } => format!("land {object}"),
        WireIntent::TapForMana { object, .. } => format!("tap {object}"),
        WireIntent::ActivateAbility {
            object,
            ability_index,
            target,
            ..
        } => format!("act {object}#{ability_index}>{}", opt(target)),
        WireIntent::DeclareAttackers { attackers, .. } => format!("attack {attackers:?}"),
        WireIntent::DeclareBlockers { blocks, .. } => format!(
            "block {:?}",
            blocks
                .iter()
                .map(|b| (b.blocker, b.attacker))
                .collect::<Vec<_>>()
        ),
        WireIntent::ChooseOrder { order, .. } => format!("order {order:?}"),
        WireIntent::ChooseTargets { targets, .. } => format!("targets {targets:?}"),
        WireIntent::ChooseTargetPlayers { players, .. } => format!("target-players {players:?}"),
        WireIntent::AnswerMay { yes, .. } => format!("may {yes}"),
        WireIntent::PayOptionalCost { pay, .. } => format!("pay {pay}"),
        WireIntent::AssignDamage { assignment, .. } => format!(
            "assign {:?}",
            assignment
                .iter()
                .map(|d| (d.blocker, d.amount))
                .collect::<Vec<_>>()
        ),
        WireIntent::DivideSpellDamage { assignment, .. } => format!(
            "divide-spell-damage {:?}",
            assignment
                .iter()
                .map(|d| (d.target, d.amount))
                .collect::<Vec<_>>()
        ),
        WireIntent::ArrangeTop { top, bottom, .. } => format!("arrange {top:?}/{bottom:?}"),
        WireIntent::SelectFromTop { cards, .. } => format!("select-top {cards:?}"),
        WireIntent::DistributeTop {
            to_hand,
            to_bottom,
            to_exile_may_play,
            ..
        } => format!("distribute-top {to_hand:?}/{to_bottom:?}/{to_exile_may_play:?}"),
        WireIntent::ShuffleFromGraveyard { cards, .. } => format!("shuffle-gy {cards:?}"),
        WireIntent::SearchLibrary { choice, .. } => format!("search {}", opt_id(choice)),
        WireIntent::ChooseSacrifices { sacrifices, .. } => format!("sacrifice {sacrifices:?}"),
        WireIntent::Discard { cards, .. } => format!("discard {cards:?}"),
        WireIntent::PutLandFromHand { choice, .. } => format!("put-land {}", opt_id(choice)),
        WireIntent::ChooseExiledWithCard { choice, .. } => {
            format!("choose-exiled {}", opt_id(choice))
        }
        WireIntent::ChooseExiledWithCardToCast { choice, .. } => {
            format!("choose-exiled-cast {}", opt_id(choice))
        }
        WireIntent::ChooseExiledDigToCastFree { choice, .. } => {
            format!("choose-dig-cast {}", opt_id(choice))
        }
        WireIntent::ChooseOpponentPile { pile, .. } => format!("choose-pile {pile}"),
        WireIntent::RevealedCardToBattlefieldOrHand { choice, .. } => {
            format!("revealed-to-bf-or-hand {}", opt_id(choice))
        }
        WireIntent::ChooseMode { mode, .. } => format!("mode {mode}"),
        WireIntent::ChooseTriggerModes { modes, .. } => format!("trigger-modes {modes:?}"),
        WireIntent::ChooseManaColor { color, .. } => format!("mana-color {color}"),
        WireIntent::ChooseCreatureType { subtype, .. } => format!("creature-type {subtype}"),
        WireIntent::ChooseColor { color, .. } => format!("color {color}"),
        WireIntent::ChooseAttachHost { host, .. } => format!("attach-host {}", opt_id(host)),
        WireIntent::ChooseCopyTarget { copy, .. } => format!("copy-target {}", opt_id(copy)),
        WireIntent::Cycle { card, .. } => format!("cycle {card}"),
        WireIntent::ActivateHandAbility { card, .. } => format!("activate hand ability {card}"),
        WireIntent::Suspend { card, .. } => format!("suspend {card}"),
        WireIntent::Encore { card, .. } => format!("encore {card}"),
        WireIntent::TurnFaceUp { permanent, .. } => format!("turn-face-up {permanent}"),
        WireIntent::CastPrepared { source, target, .. } => {
            format!("cast-prepared {source}>{}", opt(target))
        }
        WireIntent::CastAdventure { source, target, .. } => {
            format!("cast-adventure {source}>{}", opt(target))
        }
        WireIntent::CastBestow { object, target, .. } => {
            format!("cast-bestow {object}>{}", opt(target))
        }
        WireIntent::PassPriority { .. } => "pass".to_string(),
        WireIntent::Concede { .. } => "concede".to_string(),
        WireIntent::TakeAction { id, target, .. } => {
            format!("take {id}>{}", opt(target))
        }
    }
}

/// The events an intent produced, compacted: strip `Debug`'s field names and braces so a long
/// auto-pass chain stays short (e.g. `DamageMarked(82,1,Some(78))`), joined by `;`.
fn compact_events(events: &[Event]) -> String {
    const FIELD_PREFIXES: [&str; 20] = [
        "object: ",
        "amount: ",
        "source: ",
        "player: ",
        "controller: ",
        "spell: ",
        "from: ",
        "permanent: ",
        "card: ",
        "step: ",
        "active_player: ",
        "color: ",
        "blocker: ",
        "attacker: ",
        "effect: ",
        "target: ",
        "mana: ",
        "power: ",
        "toughness: ",
        "count: ",
    ];
    events
        .iter()
        .map(|e| {
            let mut s = format!("{e:?}");
            for p in FIELD_PREFIXES {
                s = s.replace(p, "");
            }
            s.replace(" { ", "(").replace(" }", ")").replace(", ", ",")
        })
        .collect::<Vec<_>>()
        .join(";")
}

#[cfg(test)]
mod tests {
    use super::*;
    use schema::WireIntent;

    fn rejected(reason: &str) -> ApplyResult {
        ApplyResult {
            accepted: false,
            reason: Some(reason.to_string()),
            events: Vec::new(),
        }
    }

    #[test]
    fn toon_field_quotes_only_when_needed() {
        assert_eq!(toon_field("pass"), "pass");
        assert_eq!(toon_field("a,b"), "\"a,b\"");
        assert_eq!(toon_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn intent_str_names_common_intents() {
        assert_eq!(intent_str(&WireIntent::PassPriority { player: 0 }), "pass");
        assert_eq!(
            intent_str(&WireIntent::PlayLand {
                player: 1,
                object: 42
            }),
            "land 42"
        );
        assert_eq!(intent_str(&WireIntent::Concede { player: 2 }), "concede");
    }

    #[test]
    fn engine_error_row_lands_even_without_a_live_game() {
        // Quarantine path: the table may already have dropped its Game when we format. The
        // panic row must still be written — those are the lines you need after a quarantine.
        let row = format_row(
            7,
            1,
            &WireIntent::PassPriority { player: 1 },
            &rejected("EngineError"),
            &[],
            None,
        );
        assert!(
            row.starts_with("7,1,pass,f,EngineError,-,-,-,-,"),
            "panic row without game: {row}"
        );
    }

    #[test]
    fn start_and_append_write_a_readable_trace_file() {
        let dir = std::env::temp_dir().join(format!("mtgfr-action-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("actions.TEST01.toon");
        let path = path.to_str().unwrap();

        start_at(path);
        append_at(
            path,
            &format_labeled(
                1,
                0,
                "pass",
                &ApplyResult {
                    accepted: true,
                    reason: None,
                    events: Vec::new(),
                },
                &[],
                None,
            ),
        );

        let body = std::fs::read_to_string(path).unwrap();
        assert!(
            body.starts_with("actions{seq,player,intent,"),
            "header from start_at: {body}"
        );
        assert!(
            body.contains("\n  1,0,pass,t,-,-,-,-,-,"),
            "row from append_at: {body}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
