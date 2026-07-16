//! Table game lifecycle: intent submission, yield / helpless-dwell chrome, auto-advance,
//! stack-hold scheduling, and delta packaging. HTTP handlers in `game_loop` validate seats and
//! call the three [`TableSession`] verbs; they must not poke yield / dwell fields directly.
//! `stream` stays a pure projection of `PublishedDelta` payloads.

use std::sync::Arc;

use crate::decks::Table;
use crate::{AppState, lock};
use engine::{Event, Game, Intent, PendingChoice, PlayerId, Reject};
use tokio::time::Instant;

/// How long an uncontested spell or ability visibly sits on the stack before the server
/// submits the final, resolving pass on everyone's behalf — the "let the table read the card"
/// beat. Applies per stack object, so a stack of three resolves with three beats.
pub const STACK_HOLD: std::time::Duration = std::time::Duration::from_millis(2000);
/// Extra time a helpless dwell may add on top of [`STACK_HOLD`] (hard cap = hold + this).
pub const STACK_HOLD_DWELL_EXTRA: std::time::Duration = std::time::Duration::from_millis(3000);

/// One applied intent's canonical events plus the full post-apply game, tagged with its seq,
/// plus the human-readable labels of any forced choices `auto_advance` submitted along the way
/// and the post-apply yield flags. Each subscriber builds its own frame purely (`redact` +
/// `complete_visible` for its viewer) — no re-lock, no race.
///
/// ponytail: clones the whole `Game` per intent — trivial at this scale; if it ever shows in a
/// profile, carry a canonical full-info snapshot struct instead (see ADR 0006).
/// ponytail: `yielded` is stamped per-viewer via `complete_visible` + `ViewExtras` in
/// `stream::frame_for` / the opening snapshot — can't stamp once at publish without knowing
/// every viewer.
pub struct PublishedDelta {
    pub seq: u64,
    /// Advances on every fan-out, including same-`seq` hold ticks.
    pub broadcast_seq: u64,
    pub events: Vec<Event>,
    pub game: Game,
    pub auto_actions: Vec<String>,
    pub yields: [bool; 4],
    pub turn_yields: [bool; 4],
    /// Stack-hold countdown for clients (ms); `0` when no hold is active.
    pub stack_hold_remaining_ms: u32,
}

/// Fan-out payload: `Arc` so subscribers clone a pointer, not the payload.
pub type Broadcast = Arc<PublishedDelta>;

/// What became of a table after applying an intent, decided under the lock and acted on by the
/// caller once the borrow ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// The game continues; `stack_held` means auto-advance paused before a stack resolution,
    /// so the caller owes a [`schedule_stack_resolution`] (folded in by [`settle_after_apply`]).
    Live { stack_held: bool },
    /// The engine panicked — quarantine (drop) the table (C3).
    Panicked,
    /// The game ended — evict the table (M3).
    GameOver,
}

/// The outcome of submitting (or driving) an intent against a live table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyResult {
    pub accepted: bool,
    /// Why the intent was rejected, if it was.
    pub reason: Option<String>,
    /// Events produced (empty on reject/panic). Fed to the debug action log.
    pub events: Vec<Event>,
}

/// Outcome of a helpless-dwell toggle. Never a [`Disposition`] — dwell only reseeds the hold
/// countdown (same game `seq`); it must not schedule or cancel a hold via [`settle_after_apply`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DwellResult {
    pub accepted: bool,
    pub reason: Option<String>,
}

/// Server policy for one live table: the three chrome verbs (submit / yield / dwell), then
/// auto-advance and broadcast. Unlock-tail ([`settle_after_apply`]) stays outside this type.
pub struct TableSession<'a> {
    table: &'a mut Table,
}

impl<'a> TableSession<'a> {
    pub fn new(table: &'a mut Table) -> Self {
        Self { table }
    }

    /// Submit an engine intent, auto-pass, and broadcast the resulting delta. Returns the apply
    /// result and the table's disposition, including whether auto-advance paused before a stack
    /// resolution (the caller then owes a hold timer via [`settle_after_apply`]). Always
    /// broadcasts on an accepted intent, even with no events: some intents change state without
    /// emitting events (answering a choice — assign-damage, "may: no", "pay: decline" — clears
    /// the pending choice), and the delta carries the full render state, so the client needs it
    /// to see the choice resolved (otherwise it stays stuck).
    pub fn submit(&mut self, intent: Intent) -> (ApplyResult, Disposition) {
        self.drive(Some(intent), true)
    }

    /// System-initiated submit (stack-hold resolve). Does not clear turn yield.
    pub fn submit_system(&mut self, intent: Intent) -> (ApplyResult, Disposition) {
        self.drive(Some(intent), false)
    }

    /// Stamp this seat's "don't care about this stack" flag and drive auto-advance. Enabling may
    /// unstick the game immediately — the yielder might be the player everyone is waiting on —
    /// so this broadcasts like any intent (including when the flag didn't change).
    /// One-shot arm only (ADR 0027): `enabled: false` is rejected — no chrome/API cancel.
    pub fn set_yield(&mut self, seat: PlayerId, enabled: bool) -> (ApplyResult, Disposition) {
        if !enabled {
            return (
                reject("StackYieldOneShot"),
                Disposition::Live { stack_held: false },
            );
        }
        self.table.yields[seat.0 as usize] = true;
        self.drive(None, false)
    }

    /// Stamp this seat's turn yield (ADR 0029) and drive auto-advance.
    pub fn set_turn_yield(&mut self, seat: PlayerId, enabled: bool) -> (ApplyResult, Disposition) {
        self.table.turn_yields[seat.0 as usize] = enabled;
        self.drive(None, false)
    }

    /// Helpless-reader hover on the stack during a hold (ADR 0026). Hold-tick only: never bumps
    /// game `seq`, never returns a [`Disposition`]. Seats that still have a meaningful action
    /// are rejected with `NotHelpless`.
    pub fn set_dwell(&mut self, seat: PlayerId, dwelling: bool) -> DwellResult {
        let idx = seat.0 as usize;
        if self.table.stack_hold.is_none() {
            self.table.stack_dwell[idx] = false;
            return DwellResult {
                accepted: true,
                reason: None,
            };
        }
        if dwelling {
            let helpless = self
                .table
                .game
                .as_ref()
                .is_some_and(|g| !g.has_meaningful_action(seat));
            self.table.stack_dwell[idx] = helpless;
            self.table.publish_hold_tick();
            if !helpless {
                return DwellResult {
                    accepted: false,
                    reason: Some("NotHelpless".into()),
                };
            }
        } else {
            self.table.stack_dwell[idx] = false;
            self.table.publish_hold_tick();
        }
        DwellResult {
            accepted: true,
            reason: None,
        }
    }

    /// Shared submit / yield-drive path: optional intent → auto-advance → broadcast.
    /// `clear_turn_yield_on_intent` is true for player-initiated HTTP intents (ADR 0029).
    fn drive(
        &mut self,
        intent: Option<Intent>,
        clear_turn_yield_on_intent: bool,
    ) -> (ApplyResult, Disposition) {
        let game = self
            .table
            .game
            .as_mut()
            .expect("TableSession::drive on a started game");
        let yields = &mut self.table.yields;
        let turn_yields = &mut self.table.turn_yields;
        // C3: the engine has reachable panics in resolution paths. Catch them here so one bad game
        // state rejects its own intent and quarantines its own table, instead of poisoning the
        // registry lock and bricking every table. `Game` is a plain value; on unwind we drop it.
        let submitted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut events = match intent {
                Some(intent) => {
                    let player = intent.actor();
                    let more = game.submit(intent)?;
                    // Untap may appear on the player intent itself — clear before auto_advance.
                    clear_turn_yields_on_untap(turn_yields, &more);
                    if clear_turn_yield_on_intent {
                        turn_yields[player.0 as usize] = false;
                    }
                    more
                }
                None => Vec::new(),
            };
            let (auto_events, labels, stack_held) = auto_advance(game, yields, turn_yields);
            events.extend(auto_events);
            Ok::<_, Reject>((events, labels, stack_held))
        }));
        let (events, labels, stack_held) = match submitted {
            Err(_panic) => {
                return (reject("EngineError"), Disposition::Panicked);
            }
            Ok(Err(rejected)) => {
                return (
                    reject(&format!("{rejected:?}")),
                    Disposition::Live { stack_held: false },
                );
            }
            Ok(Ok(result)) => result,
        };

        self.table.seq += 1;
        self.table.broadcast_seq += 1;
        let seq = self.table.seq;
        let game = self
            .table
            .game
            .as_ref()
            .expect("game still live after drive");
        // Carry the post-apply game (and yield flags) so each subscriber builds its own redacted
        // frame. A send error means no one is listening yet — harmless.
        let hold_ms = if stack_held {
            STACK_HOLD.as_millis() as u32
        } else {
            self.table.stack_hold_remaining_ms()
        };
        // One clone: the broadcast owns a copy; ApplyResult keeps the original for the action log.
        let _ = self.table.tx.send(Arc::new(PublishedDelta {
            seq,
            broadcast_seq: self.table.broadcast_seq,
            events: events.clone(),
            game: game.clone(),
            auto_actions: labels,
            yields: self.table.yields,
            turn_yields: self.table.turn_yields,
            stack_hold_remaining_ms: hold_ms,
        }));
        let disposition = if game.winner().is_some() {
            Disposition::GameOver
        } else {
            Disposition::Live { stack_held }
        };
        (
            ApplyResult {
                accepted: true,
                reason: None,
                events,
            },
            disposition,
        )
    }
}

/// Opening auto-advance after [`crate::decks::seed_game`] (stack is empty — never pauses for a
/// hold). Kept as a thin public entry so `auto_advance` itself stays private to this module.
pub fn advance_seeded_game(game: &mut Game) {
    let _ = auto_advance(game, &mut [false; 4], &mut [false; 4]);
}

/// Milliseconds until the stack-hold would resolve, or `0` if no hold is active. Shared by
/// [`Table::stack_hold_remaining_ms`] and hold-tick fan-out.
pub(crate) fn stack_hold_remaining_ms(hold: Option<(u64, Instant)>, any_dwell: bool) -> u32 {
    let Some((_, started)) = hold else {
        return 0;
    };
    let now = Instant::now();
    let deadline = hold_deadline(started, any_dwell);
    if now >= deadline {
        return 0;
    }
    deadline.saturating_duration_since(now).as_millis() as u32
}

/// When the active hold should fire: base [`STACK_HOLD`] unless any helpless dwell is set, then
/// the hard cap (`STACK_HOLD` + [`STACK_HOLD_DWELL_EXTRA`]).
fn hold_deadline(started: Instant, any_dwell: bool) -> Instant {
    let base = started + STACK_HOLD;
    let cap = started + STACK_HOLD + STACK_HOLD_DWELL_EXTRA;
    if any_dwell { cap } else { base }
}

/// The shared tail of every intent application: evict a dead table, and keep the stack-hold
/// chain alive when the game paused before a resolution.
pub fn settle_after_apply(
    reg: &mut crate::Registry,
    state: &AppState,
    table_id: &str,
    disposition: Disposition,
    seq: u64,
) {
    match disposition {
        // C3: the engine panicked mid-submit — the game is in an unknown state. Drop the
        // whole table so the poison can't spread and the players can start a clean one.
        Disposition::Panicked => {
            reg.tables.remove(table_id);
            eprintln!("quarantined table {table_id} after an engine panic");
        }
        // M3: the game is over — evict it (and its broadcast buffer of cloned games).
        Disposition::GameOver => {
            reg.tables.remove(table_id);
        }
        Disposition::Live { stack_held: true } => {
            schedule_stack_resolution(state.clone(), table_id.to_string(), seq);
        }
        Disposition::Live { stack_held: false } => {}
    }
}

/// The pass the stack-hold timer should submit when it fires, re-validated against the
/// *current* table state — or `None` when the hold's premise is gone (the holder reclaimed
/// their window, or the stack no longer owes a resolution). Defense in depth beside the `seq`
/// staleness guard: the timer must never force-pass a player the game should wait on, even if
/// some future mutation path forgets to broadcast.
fn stack_hold_pass(game: &Game, yields: &[bool; 4], turn_yields: &[bool; 4]) -> Option<PlayerId> {
    if !game.next_pass_resolves_stack() {
        return None;
    }
    let holder = game.priority_holder();
    if !yields[holder.0 as usize]
        && !turn_yields[holder.0 as usize]
        && game.has_meaningful_action(holder)
    {
        return None;
    }
    Some(holder)
}

/// After [`STACK_HOLD`] (extendable by helpless dwell up to +[`STACK_HOLD_DWELL_EXTRA`]),
/// submit the final stack-resolving pass on the priority holder's behalf. `seq` guards
/// staleness: any broadcast during the hold bumps `table.seq` (and its own `settle_after_apply`
/// schedules a fresh hold if one is still owed), so a stale timer just evaporates instead of
/// double-resolving. [`stack_hold_pass`] re-validates the rest.
fn schedule_stack_resolution(state: AppState, table_id: String, seq: u64) {
    tokio::spawn(async move {
        {
            let mut reg = lock(&state.reg);
            if let Some(table) = reg.tables.get_mut(&table_id) {
                let now = Instant::now();
                table.stack_hold = Some((seq, now));
                table.stack_dwell = [false; 4];
            }
        }
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            let mut reg = lock(&state.reg);
            let Some(table) = reg.tables.get_mut(&table_id) else {
                return;
            };
            if table.seq != seq {
                if table.stack_hold.is_some_and(|(s, _)| s == seq) {
                    table.stack_hold = None;
                    table.stack_dwell = [false; 4];
                }
                return;
            }
            let Some((_, started)) = table.stack_hold else {
                return;
            };
            let now = Instant::now();
            let any_dwell = table.stack_dwell.iter().any(|&d| d);
            if now < hold_deadline(started, any_dwell) {
                continue;
            }
            table.stack_hold = None;
            table.stack_dwell = [false; 4];
            let Some(game) = table.game.as_ref() else {
                return;
            };
            let Some(holder) = stack_hold_pass(game, &table.yields, &table.turn_yields) else {
                return;
            };
            let mut session = TableSession::new(table);
            let wire = schema::WireIntent::PassPriority { player: holder.0 };
            let (result, disposition) =
                session.submit_system(Intent::PassPriority { player: holder });
            let log_row = crate::action_log::format_row(
                table.seq,
                holder.0,
                &wire,
                &result,
                &result.events,
                table.game.as_ref(),
            );
            let seq = table.seq;
            settle_after_apply(&mut reg, &state, &table_id, disposition, seq);
            drop(reg);
            crate::action_log::append(&table_id, &log_row);
            return;
        }
    });
}

/// Auto-advance the game past every window the client shouldn't have to act on: a *forced*
/// choice (its pending choice has exactly one legal answer, [`Game::forced_action`]) is
/// submitted on the player's behalf, and priority is auto-passed while its holder has no
/// meaningful action — or has yielded ("don't care about this stack"). Returns the accumulated
/// events, a human-readable label for each forced choice submitted (in order) so the caller can
/// fold both into the same broadcast frame as the triggering action, and whether it *paused*:
/// stopped just before the auto-pass that would resolve the top of the stack, so an uncontested
/// spell visibly sits there for [`STACK_HOLD`] (the caller schedules the resolving pass) instead
/// of resolving in the same frame it was cast. Keeps the engine pure — it's still only mutated
/// by `Intent`s; the server just decides which ones to submit on nobody's behalf.
/// ponytail: 256-iteration cap as a runaway guard — a real game never approaches it (each pass
/// advances a step, and each forced submit clears a choice).
fn auto_advance(
    game: &mut Game,
    yields: &mut [bool; 4],
    turn_yields: &mut [bool; 4],
) -> (Vec<Event>, Vec<String>, bool) {
    let mut events = Vec::new();
    let mut labels = Vec::new();
    for _ in 0..256 {
        // A yield lasts exactly as long as the stack it was declared against: the moment the
        // stack is empty (including mid-loop, right after a resolution), every seat is paying
        // attention again — otherwise "don't care" would skip the rest of a player's turn.
        if game.stack_is_empty() {
            yields.fill(false);
        }
        if let Some(choice) = game.pending_choice() {
            // No forced answer: a genuine human decision is owed, so stop here and wait for it.
            let Some(intent) = game.forced_action() else {
                break;
            };
            let label = forced_action_label(game, &choice);
            match game.submit(intent) {
                Ok(more) => {
                    // Clear turn yield as soon as Untap begins — before the next skip check
                    // (ADR 0029). Must not wait until after the whole auto_advance loop.
                    clear_turn_yields_on_untap(turn_yields, &more);
                    events.extend(more);
                }
                Err(_) => break,
            }
            labels.push(label);
            continue;
        }
        let holder = game.priority_holder();
        let skip = yields[holder.0 as usize]
            || turn_yields[holder.0 as usize]
            || !game.has_meaningful_action(holder);
        if !skip {
            break;
        }
        // This auto-pass would complete the round and resolve the top of the stack: pause
        // instead, so the table gets its beat to read the spell before it resolves.
        if game.next_pass_resolves_stack() {
            return (events, labels, true);
        }
        match game.submit(Intent::PassPriority { player: holder }) {
            Ok(more) => {
                clear_turn_yields_on_untap(turn_yields, &more);
                events.extend(more);
            }
            Err(_) => break,
        }
    }
    (events, labels, false)
}

fn clear_turn_yields_on_untap(turn_yields: &mut [bool; 4], events: &[Event]) {
    use engine::Step;
    for e in events {
        if let Event::StepBegan {
            step: Step::Untap,
            active_player,
        } = e
        {
            turn_yields[active_player.0 as usize] = false;
        }
    }
}

fn reject(reason: &str) -> ApplyResult {
    ApplyResult {
        events: Vec::new(),
        accepted: false,
        reason: Some(reason.to_string()),
    }
}

/// A short human sentence for a forced choice `auto_advance` is about to submit, read from the
/// pending choice it answers (not the resolved `Intent`, which has already lost the choice
/// variant that motivated it). One label per forced submit — no attempt to describe *why* the
/// choice was forced beyond what a player glancing at the log needs.
fn forced_action_label(game: &Game, choice: &PendingChoice) -> String {
    use PendingChoice::*;
    match choice {
        DiscardToHandSize { .. } => "Discarded to hand size (forced)".to_string(),
        DiscardCards { .. } => "Discarded (forced)".to_string(),
        ChooseTarget { .. } => "Only one legal target — chosen automatically".to_string(),
        OrderTriggers { .. } => "Trigger order was forced".to_string(),
        // The only sacrifice a `forced_action` ever picks alone is a single-option edict.
        SacrificeEdict { options, .. } => {
            let name = options
                .first()
                .map(|&id| game.def_of(id).name)
                .unwrap_or("a permanent");
            format!("Sacrificed {name} (forced)")
        }
        // `forced_action` never returns `Some` for any other pending-choice kind.
        _ => "Automatic".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::decks::seed_game;
    use crate::test_support::{as_user, seat_deck, user_with_deck};
    use axum::Json;
    use axum::extract::State;
    use engine::PlayerId;
    use schema::{IntentEnvelope, WireIntent, to_intent};

    use crate::game_loop::{YieldRequest, set_yield, submit_intent};

    #[test]
    fn auto_advance_skips_players_with_no_meaningful_action_and_terminates() {
        // A bare game: both players have empty hands and boards, so nobody has a meaningful
        // action. Auto-advance must advance the game (not hang) and stay bounded.
        let mut game = engine::Game::new();
        let step_before = game.current_step();
        let (events, labels, _held) = auto_advance(&mut game, &mut [false; 4], &mut [false; 4]);
        assert!(
            !events.is_empty(),
            "auto-advance emits priority passes / step advances when nobody can act",
        );
        assert!(
            labels.is_empty(),
            "no forced choices came up, so no auto-action labels",
        );
        assert_ne!(
            game.current_step(),
            step_before,
            "the game advanced instead of stalling on a no-op priority window",
        );
    }

    #[test]
    fn answering_a_choice_that_emits_no_events_still_broadcasts() {
        use engine::{Game, Intent, Step};

        let bear = || cards::get("Grizzly Bear").expect("Grizzly Bear in pool");

        let mut game = Game::new();
        let attacker = game.spawn_on_battlefield(PlayerId(0), bear());
        let b1 = game.spawn_on_battlefield(PlayerId(1), bear());
        let b2 = game.spawn_on_battlefield(PlayerId(1), bear());
        game.spawn_on_battlefield(PlayerId(0), cards::get("Mountain").unwrap());
        game.spawn_in_hand(PlayerId(0), cards::get("Shock").unwrap());

        let advance_to = |g: &mut Game, step: Step| {
            while g.current_step() != step {
                let p = g.priority_holder();
                g.submit(Intent::PassPriority { player: p }).unwrap();
            }
        };
        advance_to(&mut game, Step::DeclareAttackers);
        game.submit(Intent::DeclareAttackers {
            player: PlayerId(0),
            attackers: vec![(attacker, PlayerId(1))],
        })
        .unwrap();
        advance_to(&mut game, Step::DeclareBlockers);
        game.submit(Intent::DeclareBlockers {
            player: PlayerId(1),
            blocks: vec![(b1, attacker), (b2, attacker)],
        })
        .unwrap();
        assert!(
            game.pending_choice().is_some(),
            "a multi-block owes the attacker a damage-division choice",
        );

        let mut table = Table::new_lobby();
        let mut rx = table.tx.subscribe();
        table.game = Some(game);
        let before = table.seq;
        let (result, _disposition) = TableSession::new(&mut table).submit(Intent::AssignDamage {
            player: PlayerId(0),
            assignment: vec![(b1, 2), (b2, 0)],
        });

        assert!(result.accepted);
        assert!(
            table.game.as_ref().unwrap().pending_choice().is_none(),
            "the choice resolved",
        );
        assert_eq!(table.seq, before + 1, "the sequence advanced");
        assert!(
            rx.try_recv().is_ok(),
            "resolving the choice broadcast a delta"
        );
    }

    #[test]
    fn a_rejected_intent_does_not_advance_the_sequence() {
        let mut table = Table::new_lobby();
        table.game = Some(seed_game(
            &[(PlayerId(0), seat_deck()), (PlayerId(1), seat_deck())],
            0,
        ));
        let before = table.seq;
        let (result, _) = TableSession::new(&mut table).submit(to_intent(WireIntent::PlayLand {
            player: 0,
            object: 99999,
        }));
        assert!(!result.accepted, "playing a bogus land is rejected");
        assert_eq!(table.seq, before, "a rejected intent broadcasts no delta");
    }

    #[test]
    fn an_action_and_its_auto_passes_fold_into_one_broadcast_frame() {
        let mut table = Table::new_lobby();
        table.game = Some(seed_game(
            &[(PlayerId(0), seat_deck()), (PlayerId(1), seat_deck())],
            0,
        ));
        let mut rx = table.tx.subscribe();

        let mut actions = 0;
        for _ in 0..5 {
            let game = table.game.as_ref().unwrap();
            if game.pending_choice().is_some() {
                break;
            }
            let holder = game.priority_holder();
            let (result, _disp) = TableSession::new(&mut table)
                .submit(engine::Intent::PassPriority { player: holder });
            if !result.accepted {
                break;
            }
            actions += 1;
        }
        assert!(actions > 0, "at least one action applied");

        let mut frames = 0;
        while rx.try_recv().is_ok() {
            frames += 1;
        }
        assert_eq!(
            frames, actions,
            "exactly one broadcast frame per accepted action"
        );
    }

    const FORCED_PINGER: engine::CardDef = engine::CardDef {
        name: "Test Forced Pinger",
        cost: engine::Cost::FREE,
        kind: engine::CardKind::Creature {
            power: 3,
            toughness: 3,
            also: engine::TypeSet::NONE,
        },
        legendary: false,
        uncounterable: false,
        modal: false,
        modal_choose: 1,
        modal_choose_max: None,
        modal_choose_max_if_commander: false,
        identity_pips: &[],
        colors: &[],
        enters_tapped: false,
        enters_tapped_unless: None,
        approximates: None,
        oracle: None,
        set: "",
        subtypes: &[],
        otags: &[],
        keywords: &[],
        conditional_keywords: &[],
        abilities: &[engine::Ability {
            timing: engine::Timing::Triggered(engine::Trigger::Etb),
            effect: engine::Effect::DealDamage {
                amount: engine::Amount::Fixed(1),
                target: engine::TargetSpec::Creature,
                count: engine::TargetCount {
                    min: 1,
                    max: 1,
                    x_scaled: false,
                    sacrifice_scaled: false,
                    strive_scaled: false,
                },
                divided: false,
            },
            optional: false,
            min_level: 0,
            condition: None,
            cost: engine::Cost::FREE,
            once_each_turn: false,
        }],
        cycling: None,
        flashback: None,
        echo: None,
        bestow: None,
        morph: None,
        evoke: None,
        delve: false,
        escape: None,
        retrace: false,
        graveyard_cast_cost: None,
        cascade: false,
        functions_in_graveyard: false,
        enchant: None,
        enchant_graveyard: false,
        back: None,
        adventure: None,
        devour: None,
        demonstrate: false,
        enter_as_copy: None,
        encore: None,
        hand_ability: None,
        may_choose_not_to_untap: false,
        suspend: None,
    };

    fn held(disposition: Disposition) -> bool {
        disposition == Disposition::Live { stack_held: true }
    }

    fn cast(
        table: &mut Table,
        player: PlayerId,
        object: engine::ObjectId,
    ) -> (ApplyResult, Disposition) {
        TableSession::new(table).submit(engine::Intent::Cast {
            player,
            object,
            target: None,
            x: 0,
            modes: vec![],
            discard_cost: vec![],
            graveyard_exile: vec![],
            sacrifice_cost: vec![],
            kicked: false,
            bought_back: false,
            evoked: false,
            strive_count: 0,
            replicate_count: 0,
        })
    }

    fn bear_table() -> (Table, engine::ObjectId) {
        let mut table = Table::new_lobby();
        let mut game = seed_game(&[(PlayerId(0), seat_deck()), (PlayerId(1), seat_deck())], 0);
        game.fund_mana(PlayerId(0));
        let bear = game.spawn_in_hand(PlayerId(0), cards::get("Grizzly Bear").unwrap());
        table.game = Some(game);
        (table, bear)
    }

    fn fire_stack_hold(table: &mut Table) -> (ApplyResult, Disposition) {
        let holder = table.game.as_ref().unwrap().priority_holder();
        TableSession::new(table).submit_system(engine::Intent::PassPriority { player: holder })
    }

    #[test]
    fn a_forced_single_legal_target_choice_auto_resolves_without_a_client_intent() {
        let mut table = Table::new_lobby();
        let mut game = engine::Game::new();
        let pinger = game.spawn_in_hand(PlayerId(0), FORCED_PINGER);
        table.game = Some(game);
        let mut rx = table.tx.subscribe();

        let (result, disp) = cast(&mut table, PlayerId(0), pinger);
        assert!(result.accepted);
        assert!(held(disp));
        let broadcast = rx.try_recv().expect("the cast frame broadcasts");
        assert!(broadcast.auto_actions.is_empty());

        let (_result, _disp) = fire_stack_hold(&mut table);
        let game = table.game.as_ref().unwrap();
        assert!(game.pending_choice().is_none());
        let broadcast = rx.try_recv().expect("the resolution frame broadcasts");
        assert!(!broadcast.auto_actions.is_empty());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn a_genuine_two_target_choice_does_not_auto_resolve() {
        let mut table = Table::new_lobby();
        let mut game = engine::Game::new();
        game.spawn_on_battlefield(PlayerId(1), cards::get("Grizzly Bear").expect("pool card"));
        let pinger = game.spawn_in_hand(PlayerId(0), FORCED_PINGER);
        table.game = Some(game);
        let mut rx = table.tx.subscribe();

        let (result, disp) = cast(&mut table, PlayerId(0), pinger);
        assert!(result.accepted);
        assert!(held(disp));
        let _ = rx.try_recv().expect("the cast frame broadcasts");

        let (_result, _disp) = fire_stack_hold(&mut table);
        let game = table.game.as_ref().unwrap();
        assert!(matches!(
            game.pending_choice(),
            Some(engine::PendingChoice::ChooseTarget { .. })
        ));

        let broadcast = rx.try_recv().expect("the resolution frame broadcasts");
        assert!(broadcast.auto_actions.is_empty());
    }

    #[test]
    fn an_uncontested_cast_pauses_on_the_stack_then_resolves_on_the_held_pass() {
        let (mut table, bear) = bear_table();

        let (result, disp) = cast(&mut table, PlayerId(0), bear);
        assert!(result.accepted);
        assert!(held(disp));
        assert_eq!(
            table.game.as_ref().unwrap().zone_of(bear),
            engine::Zone::Stack,
        );

        let (_result, disp) = fire_stack_hold(&mut table);
        assert!(!held(disp));
        assert_eq!(
            table.game.as_ref().unwrap().zone_of(bear),
            engine::Zone::Battlefield,
        );
    }

    #[test]
    fn a_yielded_seat_is_auto_passed_through_its_reaction_window() {
        let (mut table, bear) = bear_table();
        {
            let game = table.game.as_mut().unwrap();
            game.spawn_on_battlefield(PlayerId(1), cards::get("Mountain").unwrap());
            game.spawn_in_hand(PlayerId(1), cards::get("Shock").unwrap());
        }

        let (result, disp) = cast(&mut table, PlayerId(0), bear);
        assert!(result.accepted);
        assert!(!held(disp));
        assert_eq!(table.game.as_ref().unwrap().priority_holder(), PlayerId(1),);

        let (result, disp) = TableSession::new(&mut table).set_yield(PlayerId(1), true);
        assert!(result.accepted);
        assert!(held(disp));

        let (_result, _disp) = fire_stack_hold(&mut table);
        assert_eq!(
            table.game.as_ref().unwrap().zone_of(bear),
            engine::Zone::Battlefield,
        );
        assert_eq!(table.yields, [false; 4]);
    }

    #[test]
    fn turn_yield_survives_empty_stack() {
        let (mut table, bear) = bear_table();
        {
            let game = table.game.as_mut().unwrap();
            game.spawn_on_battlefield(PlayerId(1), cards::get("Mountain").unwrap());
            game.spawn_in_hand(PlayerId(1), cards::get("Shock").unwrap());
        }
        let (_result, _disp) = cast(&mut table, PlayerId(0), bear);
        let (result, disp) = TableSession::new(&mut table).set_turn_yield(PlayerId(1), true);
        assert!(result.accepted);
        assert!(held(disp));
        assert!(table.turn_yields[1]);

        let (_result, _disp) = fire_stack_hold(&mut table);
        assert!(
            table.turn_yields[1],
            "turn yield must not clear when the stack empties"
        );
    }

    #[test]
    fn turn_yield_clears_on_player_intent() {
        let (mut table, bear) = bear_table();
        {
            let game = table.game.as_mut().unwrap();
            game.spawn_on_battlefield(PlayerId(1), cards::get("Mountain").unwrap());
            game.spawn_in_hand(PlayerId(1), cards::get("Shock").unwrap());
        }
        let (_result, _disp) = cast(&mut table, PlayerId(0), bear);
        assert_eq!(table.game.as_ref().unwrap().priority_holder(), PlayerId(1));
        let (result, _) = TableSession::new(&mut table).set_turn_yield(PlayerId(1), true);
        assert!(result.accepted);
        assert!(table.turn_yields[1]);

        let (result, _) = TableSession::new(&mut table).submit(Intent::PassPriority {
            player: PlayerId(1),
        });
        assert!(result.accepted);
        assert!(!table.turn_yields[1]);
    }

    #[test]
    fn turn_yield_clears_at_untap_so_that_seat_keeps_their_turn() {
        let (mut table, bear) = bear_table();
        {
            let game = table.game.as_mut().unwrap();
            game.spawn_on_battlefield(PlayerId(1), cards::get("Mountain").unwrap());
            game.spawn_in_hand(PlayerId(1), cards::get("Shock").unwrap());
        }
        let (_result, _disp) = cast(&mut table, PlayerId(0), bear);
        let (_result, _) = TableSession::new(&mut table).set_turn_yield(PlayerId(1), true);
        let (_result, _) = fire_stack_hold(&mut table);
        assert!(table.turn_yields[1]);

        // Pass through P0's remaining turn until P1 becomes active.
        for _ in 0..64 {
            let active = table.game.as_ref().unwrap().active_player();
            if active == PlayerId(1) {
                break;
            }
            let holder = table.game.as_ref().unwrap().priority_holder();
            let (result, _) = TableSession::new(&mut table)
                .submit_system(Intent::PassPriority { player: holder });
            assert!(result.accepted, "pass by {holder:?} should advance");
        }

        let game = table.game.as_ref().unwrap();
        assert_eq!(game.active_player(), PlayerId(1), "reached P1's turn");
        assert!(
            !table.turn_yields[1],
            "turn yield must clear at Untap before auto-pass skips their turn"
        );
        assert_eq!(
            game.priority_holder(),
            PlayerId(1),
            "P1 must still hold priority on their turn"
        );
        assert!(
            game.has_meaningful_action(PlayerId(1)),
            "P1 must still be able to act (not auto-passed through main)"
        );
    }

    #[test]
    fn stack_yield_rejects_disable_once_armed() {
        let (mut table, bear) = bear_table();
        {
            let game = table.game.as_mut().unwrap();
            game.spawn_on_battlefield(PlayerId(1), cards::get("Mountain").unwrap());
            game.spawn_in_hand(PlayerId(1), cards::get("Shock").unwrap());
        }
        let (_result, _disp) = cast(&mut table, PlayerId(0), bear);
        let (result, _) = TableSession::new(&mut table).set_yield(PlayerId(1), true);
        assert!(result.accepted);
        assert!(table.yields[1]);

        let before = table.seq;
        let (result, _) = TableSession::new(&mut table).set_yield(PlayerId(1), false);
        assert!(!result.accepted);
        assert_eq!(result.reason.as_deref(), Some("StackYieldOneShot"));
        assert!(table.yields[1], "still armed");
        assert_eq!(table.seq, before, "reject must not advance seq");
    }

    #[test]
    fn an_event_less_yield_arm_still_broadcasts_the_flag() {
        let (mut table, bear) = bear_table();
        {
            let game = table.game.as_mut().unwrap();
            game.spawn_on_battlefield(PlayerId(1), cards::get("Mountain").unwrap());
            game.spawn_in_hand(PlayerId(1), cards::get("Shock").unwrap());
        }
        let (_result, _disp) = cast(&mut table, PlayerId(0), bear);
        // Clear any auto-arm from hold path, then arm via the session verb.
        table.yields = [false; 4];
        let mut rx = table.tx.subscribe();

        let before = table.seq;
        let (result, _disp) = TableSession::new(&mut table).set_yield(PlayerId(1), true);
        assert!(result.accepted);
        assert_eq!(table.seq, before + 1);
        let broadcast = rx.try_recv().expect("the flag change reached the stream");
        assert!(broadcast.yields[1]);
    }

    #[test]
    fn a_yield_is_inert_once_the_stack_is_empty() {
        let mut game = seed_game(&[(PlayerId(0), seat_deck()), (PlayerId(1), seat_deck())], 0);
        assert_eq!(game.priority_holder(), PlayerId(0));

        let mut yields = [true, false, false, false];
        let (events, _labels, held) = auto_advance(&mut game, &mut yields, &mut [false; 4]);
        assert!(events.is_empty());
        assert!(!held);
        assert_eq!(game.priority_holder(), PlayerId(0));
        assert_eq!(yields, [false; 4]);
    }

    #[test]
    fn un_yielding_mid_hold_cancels_the_pending_resolution() {
        let (mut table, bear) = bear_table();
        {
            let game = table.game.as_mut().unwrap();
            game.spawn_on_battlefield(PlayerId(1), cards::get("Mountain").unwrap());
            game.spawn_in_hand(PlayerId(1), cards::get("Shock").unwrap());
        }
        let (_result, _disp) = cast(&mut table, PlayerId(0), bear);

        let (_result, disp) = TableSession::new(&mut table).set_yield(PlayerId(1), true);
        assert!(held(disp));
        table.yields[1] = false;

        let game = table.game.as_ref().unwrap();
        assert_eq!(
            stack_hold_pass(game, &table.yields, &table.turn_yields),
            None
        );

        table.yields[1] = true;
        assert_eq!(
            stack_hold_pass(
                table.game.as_ref().unwrap(),
                &table.yields,
                &table.turn_yields
            ),
            Some(PlayerId(1)),
        );
    }

    #[tokio::test(start_paused = true)]
    async fn the_stack_hold_timer_fires_and_resolves_the_stack() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let (mut table, bear) = bear_table();
        let (_result, disp) = cast(&mut table, PlayerId(0), bear);
        assert!(held(disp));
        let seq = table.seq;
        lock(&state.reg).tables.insert("hold".to_string(), table);

        schedule_stack_resolution(state.clone(), "hold".to_string(), seq);
        tokio::time::sleep(STACK_HOLD * 2).await;

        let reg = lock(&state.reg);
        let game = reg.tables.get("hold").unwrap().game.as_ref().unwrap();
        assert_eq!(game.zone_of(bear), engine::Zone::Battlefield);
    }

    #[tokio::test(start_paused = true)]
    async fn a_stale_stack_hold_timer_does_nothing() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let (mut table, bear) = bear_table();
        let (_result, disp) = cast(&mut table, PlayerId(0), bear);
        assert!(held(disp));
        let stale_seq = table.seq;
        table.seq += 1;
        lock(&state.reg).tables.insert("stale".to_string(), table);

        schedule_stack_resolution(state.clone(), "stale".to_string(), stale_seq);
        tokio::time::sleep(STACK_HOLD * 2).await;

        let reg = lock(&state.reg);
        let game = reg.tables.get("stale").unwrap().game.as_ref().unwrap();
        assert_eq!(game.zone_of(bear), engine::Zone::Stack);
    }

    #[tokio::test(start_paused = true)]
    async fn helpless_dwell_postpones_stack_hold_until_the_hard_cap() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let (mut table, bear) = bear_table();
        let (_result, disp) = cast(&mut table, PlayerId(0), bear);
        assert!(held(disp));
        let seq = table.seq;
        lock(&state.reg).tables.insert("dwell".to_string(), table);

        schedule_stack_resolution(state.clone(), "dwell".to_string(), seq);
        // Let the hold start stamp land.
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        {
            let mut reg = lock(&state.reg);
            let table = reg.tables.get_mut("dwell").unwrap();
            let dwell = TableSession::new(table).set_dwell(PlayerId(1), true);
            assert!(dwell.accepted);
            assert!(table.stack_hold_remaining_ms() > STACK_HOLD.as_millis() as u32);
        }

        tokio::time::sleep(STACK_HOLD).await;
        {
            let reg = lock(&state.reg);
            let game = reg.tables.get("dwell").unwrap().game.as_ref().unwrap();
            assert_eq!(
                game.zone_of(bear),
                engine::Zone::Stack,
                "dwell must keep the stack held past the base 2s"
            );
        }

        tokio::time::sleep(STACK_HOLD_DWELL_EXTRA).await;
        let reg = lock(&state.reg);
        let game = reg.tables.get("dwell").unwrap().game.as_ref().unwrap();
        assert_eq!(
            game.zone_of(bear),
            engine::Zone::Battlefield,
            "hard cap forces resolve"
        );
    }

    #[test]
    fn a_hold_tick_publishes_remaining_without_bumping_game_seq() {
        let (mut table, bear) = bear_table();
        let (_result, disp) = cast(&mut table, PlayerId(0), bear);
        assert!(held(disp));
        // Simulate the scheduled hold stamp (schedule_stack_resolution sets this under the lock).
        table.stack_hold = Some((table.seq, Instant::now()));
        let mut rx = table.tx.subscribe();
        let seq_before = table.seq;
        let bcast_before = table.broadcast_seq;

        let dwell = TableSession::new(&mut table).set_dwell(PlayerId(0), true);
        assert!(dwell.accepted);

        assert_eq!(table.seq, seq_before);
        assert_eq!(table.broadcast_seq, bcast_before + 1);
        let tick = rx.try_recv().expect("hold tick fans out");
        assert_eq!(tick.seq, seq_before);
        assert!(tick.events.is_empty());
        assert!(tick.stack_hold_remaining_ms > STACK_HOLD.as_millis() as u32);
        assert_eq!(
            table.game.as_ref().unwrap().zone_of(bear),
            engine::Zone::Stack
        );
    }

    #[tokio::test]
    async fn take_action_end_to_end_applies_a_valid_stored_action_id() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let _ = user_with_deck(&state, "p0@x.c").await;
        let uid = as_user(&state, "p0@x.c").await.0.id;

        let mut table = Table::new_lobby();
        table.seats[0].user_id = Some(uid);
        table.game = Some(seed_game(
            &[(PlayerId(0), seat_deck()), (PlayerId(1), seat_deck())],
            0,
        ));
        lock(&state.reg).tables.insert("take".to_string(), table);

        let action = {
            let reg = lock(&state.reg);
            let game = reg.tables.get("take").unwrap().game.as_ref().unwrap();
            game.legal_actions()
                .iter()
                .find(|a| a.player == PlayerId(0))
                .copied()
                .expect("the starting player has at least one legal action (a land to play)")
        };

        let ack = submit_intent(
            State(state.clone()),
            as_user(&state, "p0@x.c").await,
            Json(IntentEnvelope {
                table_id: "take".to_string(),
                client_seq: 0,
                intent: WireIntent::TakeAction {
                    player: 0,
                    id: action.id,
                    target: None,
                    x: 0,
                    modes: vec![],
                    sacrifice: vec![],
                    discard_cost: vec![],
                    graveyard_exile: vec![],
                    attackers: vec![],
                    blocks: vec![],
                },
            }),
        )
        .await
        .0;
        assert!(ack.accepted);
    }

    #[tokio::test]
    async fn the_yield_route_records_the_flag_for_the_callers_own_seat_only() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let _ = user_with_deck(&state, "p0@x.c").await;
        let _ = user_with_deck(&state, "p1@x.c").await;
        let uid0 = as_user(&state, "p0@x.c").await.0.id;
        let uid1 = as_user(&state, "p1@x.c").await.0.id;

        let mut table = Table::new_lobby();
        table.seats[0].user_id = Some(uid0);
        table.seats[1].user_id = Some(uid1);
        table.game = Some(seed_game(
            &[(PlayerId(0), seat_deck()), (PlayerId(1), seat_deck())],
            0,
        ));
        lock(&state.reg).tables.insert("y".to_string(), table);

        let ack = set_yield(
            State(state.clone()),
            as_user(&state, "p1@x.c").await,
            Json(YieldRequest {
                table_id: "y".to_string(),
                enabled: true,
            }),
        )
        .await
        .0;
        assert!(ack.accepted);

        let ack = set_yield(
            State(state.clone()),
            as_user(&state, "p0@x.c").await,
            Json(YieldRequest {
                table_id: "y".to_string(),
                enabled: true,
            }),
        )
        .await
        .0;
        assert!(ack.accepted);
        assert!(lock(&state.reg).tables.contains_key("y"));
    }

    #[tokio::test]
    async fn a_finished_game_is_evicted() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let _ = user_with_deck(&state, "p0@x.c").await;
        let uid = as_user(&state, "p0@x.c").await.0.id;

        let mut table = Table::new_lobby();
        table.seats[0].user_id = Some(uid);
        let mut game = seed_game(&[(PlayerId(0), seat_deck()), (PlayerId(1), seat_deck())], 0);
        game.set_life(PlayerId(1), 0);
        table.game = Some(game);
        lock(&state.reg).tables.insert("over".to_string(), table);

        let ack = submit_intent(
            State(state.clone()),
            as_user(&state, "p0@x.c").await,
            Json(IntentEnvelope {
                table_id: "over".to_string(),
                client_seq: 0,
                intent: WireIntent::PassPriority { player: 0 },
            }),
        )
        .await
        .0;
        assert!(ack.accepted);
        assert!(!lock(&state.reg).tables.contains_key("over"));
    }

    #[test]
    fn published_delta_carries_self_sufficient_game_snapshot() {
        let mut table = Table::new_lobby();
        table.game = Some(seed_game(
            &[(PlayerId(0), seat_deck()), (PlayerId(1), seat_deck())],
            0,
        ));
        let mut rx = table.tx.subscribe();
        let (result, _) = TableSession::new(&mut table).set_yield(PlayerId(0), true);
        assert!(result.accepted);
        let delta = rx.try_recv().expect("drive-only apply publishes a delta");
        assert_eq!(delta.seq, table.seq);
        assert_eq!(delta.yields, table.yields);
        assert!(delta.game.player_count() > 0);
    }

    /// Mirrors the client Escape → Exile → target path for Sentinel's Eyes: TakeAction carries
    /// `graveyard_exile` picks plus an Aura host. Without those picks the server rejects; with
    /// them the spell hits the stack with `escape: true`.
    #[tokio::test]
    async fn take_action_escapes_sentinels_eyes_with_exile_picks_and_target() {
        use engine::{Game, Intent, MeaningfulAction, Zone};
        use schema::WireTarget;

        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let _ = user_with_deck(&state, "eyes0@x.c").await;
        let uid = as_user(&state, "eyes0@x.c").await.0.id;

        let mut game = Game::new();
        game.fund_mana(PlayerId(0));
        let bear = game.spawn_on_battlefield(PlayerId(0), cards::get("Grizzly Bear").unwrap());
        let eyes = game.spawn_in_graveyard(PlayerId(0), cards::get("Sentinel's Eyes").unwrap());
        let fodder: Vec<_> = (0..2)
            .map(|_| game.spawn_in_graveyard(PlayerId(0), cards::get("Plains").unwrap()))
            .collect();
        let plains = game.spawn_on_battlefield(PlayerId(0), cards::get("Plains").unwrap());
        game.submit(Intent::TapForMana {
            player: PlayerId(0),
            object: plains,
        })
        .unwrap();

        let action_id = game
            .legal_actions()
            .iter()
            .find(|a| {
                matches!(
                    a.kind,
                    MeaningfulAction::Cast {
                        card,
                        zone: Zone::Graveyard
                    } if card == eyes
                )
            })
            .expect("escape Sentinel's Eyes is listed")
            .id;

        let mut table = Table::new_lobby();
        table.seats[0].user_id = Some(uid);
        table.game = Some(game);
        lock(&state.reg)
            .tables
            .insert("eyes-escape".to_string(), table);

        let missing = submit_intent(
            State(state.clone()),
            as_user(&state, "eyes0@x.c").await,
            Json(IntentEnvelope {
                table_id: "eyes-escape".to_string(),
                client_seq: 0,
                intent: WireIntent::TakeAction {
                    player: 0,
                    id: action_id,
                    target: Some(WireTarget::Object { id: bear }),
                    x: 0,
                    modes: vec![],
                    sacrifice: vec![],
                    discard_cost: vec![],
                    graveyard_exile: vec![],
                    attackers: vec![],
                    blocks: vec![],
                },
            }),
        )
        .await
        .0;
        assert!(!missing.accepted, "escape without exile picks must reject");

        let ack = submit_intent(
            State(state.clone()),
            as_user(&state, "eyes0@x.c").await,
            Json(IntentEnvelope {
                table_id: "eyes-escape".to_string(),
                client_seq: 1,
                intent: WireIntent::TakeAction {
                    player: 0,
                    id: action_id,
                    target: Some(WireTarget::Object { id: bear }),
                    x: 0,
                    modes: vec![],
                    sacrifice: vec![],
                    discard_cost: vec![],
                    graveyard_exile: fodder.clone(),
                    attackers: vec![],
                    blocks: vec![],
                },
            }),
        )
        .await
        .0;
        assert!(ack.accepted, "escape with exile + target: {ack:?}");

        let reg = lock(&state.reg);
        let game = reg
            .tables
            .get("eyes-escape")
            .unwrap()
            .game
            .as_ref()
            .unwrap();
        assert_eq!(game.zone_of(eyes), Zone::Stack);
        for &fid in &fodder {
            assert_eq!(game.zone_of(fid), Zone::Exile);
        }
    }
}
