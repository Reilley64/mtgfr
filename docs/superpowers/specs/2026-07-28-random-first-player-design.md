# Random first player + reveal — design

**Status:** Approved for planning (2026-07-28)  
**Living surface specs to update at implement time:**
[`2026-07-20-engine-core-and-event-model.md`](2026-07-20-engine-core-and-event-model.md),
[`2026-07-20-turn-and-priority-chrome.md`](2026-07-20-turn-and-priority-chrome.md)

## Problem

Nobody rolls for the first turn. `Game` is constructed with
`active_player: PlayerId(0)` (`crates/engine/src/core.rs:42`), and the lobby hands
out seats in join order (`seat = snap.seats.length`,
`client/app/domain/lobby-store.ts:150`), which `start/v1.post.ts` then sorts
ascending before seeding. The host therefore takes the first turn of every game.

CR 103.1 ("the players determine which one of them chooses who takes the first
turn") is unimplemented and carries no `ponytail:` / `approximates` flag. The
engine's own constructor doc already prescribes the missing half: *"Player 0 is
the starting active player and holds priority; a lobby that wants a random first
player randomizes the seat→person assignment instead."* The lobby never does.

## Goal

The starting player is chosen at random from the table's master seed, and the
board shows a short spotlight reveal naming the winner before opening hands are
decided.

## Non-goals

- Player-chosen first player (dice in chat, "winner of last game starts", host
  picks). The roll is unconditional.
- Changing seating order. Turn order stays lobby seating; only the entry point
  moves.
- A synchronized cross-client reveal, a server-held reveal window, or gating
  mulligan intents behind the animation.
- New canvas/rAF machinery, new proto fields, or a new engine event.

## Approach

### The roll — engine, from the master seed

`Game::choose_starting_player()` lands in `crates/engine/src/core.rs` beside
`with_op_rng` and `begin_first_turn_events` (the existing game-start seam):

```rust
/// CR 103.1 — the game's first random op picks who takes the first turn.
pub fn choose_starting_player(&mut self) {
    let count = self.players.len();
    let seat = self.with_op_rng(PlayerId(0), |rng| rng.gen_index(count));
    self.active_player = PlayerId(seat as u8);
    self.priority = PlayerId(seat as u8);
}
```

`with_op_rng(PlayerId(0), …)` keys the derive-per-op stream as
`derive_op_key(master_seed, 0, iteration)` — a game-level op carried on seat 0's
counter, spending that seat's iteration `0`.

`seed_game` (`crates/server/src/decks.rs:33`) calls it immediately after
`Game::with_master_seed`, **before** the per-seat library/hand loop, so it is
always operation zero of the game and reproduces exactly on replay from the
drand seed recorded on the table (`Table::seed`). Rolling before hands are dealt
also matches rules order: CR 103.1 precedes 103.2/103.4.

The raw constructors keep parking at `PlayerId(0)`, so direct-API engine tests
that build boards by hand are unaffected.

**Rejected alternatives.** Rotating the seat list at seed time so the winner
becomes `PlayerId(0)` is a smaller diff, but `PlayerId(0)` then always starts —
the reveal animates toward a foregone slot — and a player's game seat index/color
stops matching the lobby seat they took. Shuffling in the BFF start route was
rejected outright: the BFF has no master seed, so the choice would not reproduce
on replay and randomness would live outside the engine's seeded story.

### Downstream — nothing else moves

- Turn advance is `(active + 1) % n`, so seating stays lobby order and only the
  entry point changes (P2 → P3 → P0 → P1, CR 103.7a).
- `begin_first_turn_events` already reads `self.active_player`, including the
  two-player first-draw skip (CR 103.8a/c).
- `VisibleState.active_player` is already projected and on the wire
  (`client/app/domain/wire/types.ts:700`), so the client learns the winner with
  **no proto or schema change**.
- No `FirstPlayerChosen` event: `active_player` already carries the fact, and an
  event would cost proto + schema + client mapping for nothing new.

### The reveal — DOM overlay, no frame loop

New `client/app/board/html/first-player-reveal.ts`, composed in `boardOverlays`
at `z-50` (above the `z-40` mulligan overlay) and shown to **every** viewer,
spectators included — no `seatedViewer` gate.

- **Layout.** Seat chips sit in the same 2×2 quadrant the board uses —
  `(seat - viewer + count) % count`, the `seatCell` rule in
  `client/app/board/geometry/layout.ts` — so the spotlight lands where that
  player actually sits. Chip tint from `seatColor`.
- **Motion.** A pure `spotlightSteps(winnerSlot, seatCount)` in
  `client/app/board/first-player-reveal.ts` returns the hop schedule (≈3
  decelerating laps, ~1.8s, final step on the winner). The view hands each chip
  its `animation-delay` from that schedule; CSS keyframes do the flashing and the
  winner's chip holds lit. No rAF, so `bitmapFrameNeedsRaf` — which today spins
  the canvas clock only while flights or exit FX exist — is untouched.
- **Hooks.** `data-testid="first-player-reveal"`, per-chip
  `data-testid="reveal-seat-<n>"` with `data-winner="true|false"`, and
  `data-testid="reveal-winner"` on the "<username> goes first" banner.

### Reveal state — a one-shot per table

`BoardModel.firstPlayerReveal: { winner: number } | null`, armed on the first
fold where `VisibleState.mulliganing` is true.

A page reload rebuilds the board model, so "plays once, never replays" has to
persist outside it: `sessionStorage` key
`mtgfr:first-player-reveal:<tableId>`, written at arm time. Reloading mid-reveal
therefore goes straight to the opening hand.

A 2.4s `Command.define` sleep — the ~1.8s hop plus a ~0.6s hold on the winner,
the `client/app/board/log-commands.ts` command pattern — dispatches
`FirstPlayerRevealFinished`, which clears the field. Driving the
dismissal by message rather than by CSS end-state is what makes it testable.

### Edge cases

- **`prefers-reduced-motion`:** no hop — winner chip and banner appear at once
  and hold ~1.2s, same finish path.
- **Blocking:** the overlay is `pointer-events-auto`, so mulligan controls are
  unreachable until it clears. Space and Enter are already inert during
  mulligans (`client/app/board/submodel.ts:3110`).
- **Joining after mulligans** (spectator, late reconnect): `mulliganing` is
  false, so the reveal never arms.
- **`sessionStorage` throws** (privacy modes): caught and treated as "not yet
  seen". Worst case is a replay on reload; it never blocks entry to the game.

## Testing

**Engine.** Same master seed → same starting seat; the roll sets `priority` as
well as `active_player`; across a spread of seeds it lands on every seat rather
than pinning to 0; it consumes exactly one op on seat 0 and leaves the other
seats' shuffle streams untouched. Regression: the two-player first-draw skip
(CR 103.8a) follows a rolled non-zero starter.

**Server.** `seed_game` starts the game on the rolled seat. Existing
`PlayerId(0)` assertions (`crates/server/src/decks.rs:132`,
`crates/server/src/session.rs:1452`, `:1512`) get their seed constant pinned to
one that rolls seat 0, so those tests keep testing what they were written to
test. `seed_game_smoothed_opening_burns_two_ops_per_seat`
(`crates/server/src/decks.rs:136`) expects seat 0's extra op.

**Client unit.** `spotlightSteps` — delays monotonic, lap count, final step is
the winner, reduced-motion schedule has no hop. One-shot arming — arms once,
skips when the sessionStorage key is present, survives a throwing
`sessionStorage`.

**Client Scene** (`client/app/board/html/surfaces.test.ts`, per the every-surface
rule): the reveal renders with `data-winner="true"` on the winner's screen slot
while `mulligan-overlay` sits underneath unreachable; after
`FirstPlayerRevealFinished` the reveal is gone and mulligan controls are live.

## Spec updates at implement time

- **engine-core-and-event-model:** the roll in the real-setup sequence, and the
  derive-per-op accounting section (currently line 141) gaining the game-level
  op carried on seat 0.
- **turn-and-priority-chrome:** owns `mulligan-overlay.ts`; gains
  `first-player-reveal.ts` in its Module list, Behavior, and Testing.
- `just engine-cr-index` regenerates `docs/CR_INDEX.md` for the new CR 103.1
  citation.

No new indexed surface spec: this adds no new code target, only a surface inside
an existing one.

## Out of scope / follow-ups

- Server-timed synchronized reveal if players ask to watch it together.
- Audio cue for the reveal (`table-audio` has the cue plumbing if wanted).
- Surfacing the roll in the board log / action log.
