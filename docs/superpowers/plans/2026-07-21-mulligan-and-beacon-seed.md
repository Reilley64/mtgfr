# Mulligans and Cloudflare Beacon Seeding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a simultaneous pre-game mulligan phase (one free redraw at 7, then draw-to-size down to 1) and replace shared splitmix entropy with Cloudflare drand–seeded, per-player derive-per-op randomness.

**Architecture:** Engine gains `master_seed: [u8; 32]`, per-seat `op_iteration`, and an `OpRng` scoped to each logical random operation (`BLAKE3(master ‖ player ‖ iteration)` → unbiased Fisher–Yates / picks). `seed_game` deals opening hands then enters `mulliganing` without `begin_first_turn`; `KeepHand` / `Mulligan` intents finish the phase. Server fetches `https://drand.cloudflare.com/public/latest` at `Tables.Seed` (env override for tests), persists seed + round on `Table`. Wire/schema/client project mulligan status and controls.

**Tech Stack:** Rust engine/server (`blake3`, existing `reqwest`/`tokio`), prost/tonic proto, schema DTOs, SolidStart client, `cargo nextest` / `just client-test`.

**Spec:** [docs/superpowers/specs/2026-07-21-mulligan-and-beacon-seed-design.md](../specs/2026-07-21-mulligan-and-beacon-seed-design.md)

## Global Constraints

- Engine stays pure — no HTTP, wall-clock, or OS RNG inside `crates/engine`.
- TDD: failing test → implement → pass → commit per task.
- Angular commits (`feat:`, `fix:`, `test:`, `docs:`); PRs squash-merge.
- Branch: continue `cursor/mulligan-and-beacon-seed-design-1e1a` or sibling `cursor/mulligan-and-beacon-seed-1e1a`.
- `Game::with_players(n, seed: u64)` remains the test convenience API: expand `seed` into a 32-byte master (LE bytes in the first 8, rest zero) so existing unit tests keep compiling.
- Production seed never silently falls back to `OsRng` when the beacon fails.
- Library order stays off the event log / wire.
- During `mulliganing`, **disable server auto-pass**; any non-kept seat may `KeepHand`/`Mulligan` without holding priority.

---

## File map

| File | Responsibility |
|------|----------------|
| `crates/engine/Cargo.toml` | Add `blake3` dependency |
| `crates/engine/src/rng.rs` (new) | `OpRng`, `derive_op_key`, unbiased `gen_index`, splitmix64 |
| `crates/engine/src/lib.rs` | `Game` fields: `master_seed`, drop/`replace` `rng_state`; `mulliganing`; wire `rng` module |
| `crates/engine/src/types/card.rs` | `Player`: `op_iteration`, `mulligans_taken`, `hand_kept` |
| `crates/engine/src/core.rs` | `with_master_seed`, `with_op_rng`, remove bare `next_u64` after migration |
| `crates/engine/src/zones.rs` | Shuffle via `with_op_rng` + unbiased indices |
| `crates/engine/src/pending/handlers/dig.rs` | Dig shuffles via `with_op_rng` |
| `crates/engine/src/resolution/resolve_misc.rs` | Random graveyard/opponent picks via `with_op_rng` |
| `crates/engine/src/types/stack.rs` | `Intent::{KeepHand,Mulligan}`, `Event::{MulliganTaken,HandKept,MulligansFinished}`, `MeaningfulAction::{KeepHand,Mulligan}`, rejects |
| `crates/engine/src/mulligan.rs` (new) | `begin_mulligans`, hand-size formula, keep/mulligan handlers, finish → `begin_first_turn` |
| `crates/engine/src/query.rs` / `priority.rs` / `lib.rs` submit | Gate actions + dispatch mulligan intents |
| `crates/engine/tests/game.rs` | RNG + mulligan regression tests; update hard-coded RNG goldens if any break |
| `crates/server/src/decks.rs` | `seed_game` stops in mulligan phase; test helper `keep_all_hands` |
| `crates/server/src/beacon.rs` (new) | Fetch/parse drand latest; env `MTGFR_MASTER_SEED` |
| `crates/server/src/lobby.rs` | Use beacon; set `table.seed` / `beacon_round` |
| `crates/server/src/table.rs` | Persist `seed: [u8; 32]` (or hex/`u128` pair — prefer `[u8; 32]` + `beacon_round: u64`) |
| `crates/server/src/session.rs` | Skip `auto_advance` while `game.mulliganing()` |
| `proto/mtgfr/v1/intent.proto` + snapshot/common as needed | Wire intents + view fields |
| `crates/schema/src/intent.rs`, `dto.rs`, `snapshot.rs`, `event.rs` | Map intents/events/projection |
| `client/src/wire/types.ts` (+ codegen) | Types for new intents/fields |
| `client/src/components/molecules/mulligan-bar.tsx` (new) | Keep / Mulligan UI |
| `client/src/components/organisms/board.tsx` (or play chrome) | Mount mulligan bar when `mulliganing` |
| Specs under `docs/superpowers/specs/` | Short behavior notes once implemented |

---

### Task 1: Derive-per-op RNG + unbiased shuffle

**Files:**
- Create: `crates/engine/src/rng.rs`
- Modify: `crates/engine/Cargo.toml`
- Modify: `crates/engine/src/lib.rs`
- Modify: `crates/engine/src/types/card.rs` (`Player`)
- Modify: `crates/engine/src/core.rs`
- Modify: `crates/engine/src/zones.rs`
- Test: `crates/engine/tests/game.rs` (extend existing seed shuffle test + new cases)

**Interfaces:**
- Consumes: existing `Game::with_players(players: u8, seed: u64)`, `Game::shuffle(player)`
- Produces:
  - `pub struct OpRng { /* splitmix state */ }` with `fn next_u64(&mut self) -> u64` and `fn gen_index(&mut self, upper_exclusive: usize) -> usize`
  - `pub fn derive_op_key(master_seed: &[u8; 32], player: u8, iteration: u64) -> [u8; 32]` — `BLAKE3(master_seed ‖ player ‖ iteration.to_le_bytes())`
  - `Game::with_master_seed(players: u8, master_seed: [u8; 32]) -> Game`
  - `Game::with_op_rng<R>(&mut self, player: PlayerId, f: impl FnOnce(&mut OpRng) -> R) -> R` — bumps that seat’s `op_iteration` once, builds `OpRng` from first 8 bytes of key as splitmix state (or hash-to-u64)
  - `Game::shuffle` uses `with_op_rng(player, |rng| { … gen_index … })`

- [ ] **Step 1: Write the failing tests**

Append to `crates/engine/tests/game.rs`:

```rust
#[test]
fn the_same_master_seed_and_iteration_shuffles_identically() {
    let deck = [
        card("Grizzly Bear"),
        card("Shock"),
        card("Grizzly Bear"),
        card("Shock"),
        card("Grizzly Bear"),
    ];
    let order = |seed: u64| {
        let mut game = Game::with_players(2, seed);
        game.stack_library(PlayerId(0), &deck);
        game.shuffle(PlayerId(0));
        (0..deck.len())
            .flat_map(|_| game.draw_card(PlayerId(0)))
            .filter_map(|e| match e {
                Event::CardDrawn { card, .. } => Some(card.name),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(order(42), order(42));
}

#[test]
fn different_players_get_independent_shuffle_streams() {
    let deck = [
        card("Grizzly Bear"),
        card("Shock"),
        card("Island"),
        card("Mountain"),
        card("Forest"),
    ];
    let mut game = Game::with_players(2, 99);
    game.stack_library(PlayerId(0), &deck);
    game.stack_library(PlayerId(1), &deck);
    game.shuffle(PlayerId(0));
    game.shuffle(PlayerId(1));
    let top0 = match game.draw_card(PlayerId(0)).into_iter().find_map(|e| match e {
        Event::CardDrawn { card, .. } => Some(card.name),
        _ => None,
    }) {
        Some(n) => n,
        None => panic!("expected draw"),
    };
    // Re-deal identical libraries and only shuffle P1 first — P0's first shuffle must match
    // the previous P0 order, proving P1's op did not consume P0's stream.
    let mut game2 = Game::with_players(2, 99);
    game2.stack_library(PlayerId(0), &deck);
    game2.stack_library(PlayerId(1), &deck);
    game2.shuffle(PlayerId(1));
    game2.shuffle(PlayerId(0));
    let top0_b = match game2.draw_card(PlayerId(0)).into_iter().find_map(|e| match e {
        Event::CardDrawn { card, .. } => Some(card.name),
        _ => None,
    }) {
        Some(n) => n,
        None => panic!("expected draw"),
    };
    assert_eq!(top0, top0_b);
}

#[test]
fn gen_index_stays_in_range() {
    use engine::rng::OpRng;
    let mut rng = OpRng::from_seed(0xDEAD_BEEF);
    for upper in [1usize, 2, 7, 99] {
        for _ in 0..200 {
            let i = rng.gen_index(upper);
            assert!(i < upper);
        }
    }
}
```

Export `rng` from `lib.rs` so the test can `use engine::rng::OpRng` (or keep `OpRng` crate-public via `pub use`).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run --profile ci -E 'test(the_same_master_seed_and_iteration_shuffles_identically) + test(different_players_get_independent_shuffle_streams) + test(gen_index_stays_in_range)'`

Expected: FAIL (missing `rng` module / old shared stream couples players — `different_players…` fails on today’s shared `rng_state`).

- [ ] **Step 3: Implement minimal RNG + shuffle**

`crates/engine/Cargo.toml`:

```toml
[dependencies]
blake3 = "1"
# existing deps…
```

`crates/engine/src/rng.rs` (sketch — match project style):

```rust
pub struct OpRng {
    state: u64,
}

impl OpRng {
    pub fn from_seed(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Unbiased index in `0..upper_exclusive`. Panics if `upper_exclusive == 0`.
    pub fn gen_index(&mut self, upper_exclusive: usize) -> usize {
        assert!(upper_exclusive > 0);
        let upper = upper_exclusive as u64;
        let thresh = upper.wrapping_neg() % upper;
        loop {
            let r = self.next_u64();
            if r >= thresh {
                return (r % upper) as usize;
            }
        }
    }
}

pub fn derive_op_key(master_seed: &[u8; 32], player: u8, iteration: u64) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(master_seed);
    h.update(&[player]);
    h.update(&iteration.to_le_bytes());
    *h.finalize().as_bytes()
}

pub fn op_rng_from_key(key: &[u8; 32]) -> OpRng {
    let mut seed_bytes = [0u8; 8];
    seed_bytes.copy_from_slice(&key[..8]);
    OpRng::from_seed(u64::from_le_bytes(seed_bytes))
}
```

On `Game`: replace `rng_state: u64` with `master_seed: [u8; 32]`. On `Player`: add `op_iteration: u64` (default 0).

```rust
pub fn with_master_seed(players: u8, master_seed: [u8; 32]) -> Self { /* … master_seed, … */ }

pub fn with_players(players: u8, seed: u64) -> Self {
    let mut master = [0u8; 32];
    master[..8].copy_from_slice(&seed.to_le_bytes());
    Self::with_master_seed(players, master)
}

pub fn with_op_rng<R>(&mut self, player: PlayerId, f: impl FnOnce(&mut crate::rng::OpRng) -> R) -> R {
    let p = &mut self.players[player.0 as usize];
    let iteration = p.op_iteration;
    p.op_iteration = iteration + 1;
    let key = crate::rng::derive_op_key(&self.master_seed, player.0, iteration);
    let mut rng = crate::rng::op_rng_from_key(&key);
    f(&mut rng)
}
```

`zones.rs` shuffle:

```rust
pub fn shuffle(&mut self, player: PlayerId) {
    let len = self.players[player.0 as usize].library.len();
    if len < 2 {
        return;
    }
    self.with_op_rng(player, |rng| {
        // Need library access: either shuffle indices then apply, or pass library mutably.
    });
}
```

Implementation detail: `with_op_rng` borrows `self` fully — shuffle cannot call it while also mutably borrowing `library`. Prefer:

```rust
pub fn shuffle(&mut self, player: PlayerId) {
    let len = self.players[player.0 as usize].library.len();
    if len < 2 {
        return;
    }
    let mut order: Vec<usize> = (0..len).collect();
    self.with_op_rng(player, |rng| {
        for i in (1..len).rev() {
            let j = rng.gen_index(i + 1);
            order.swap(i, j);
        }
    });
    let lib = &mut self.players[player.0 as usize].library;
    let mut next = Vec::with_capacity(len);
    for i in order {
        next.push(lib[i]);
    }
    *lib = next;
}
```

Remove `Game::next_u64` once call sites are gone (Task 2). For Task 1 only, keep a temporary `next_u64` that uses `active_player` + `with_op_rng` **only if** needed — better: leave `next_u64` as splitmix on a deprecated field until Task 2, but then player-independence test for shuffle already uses the new path. **Do not** dual-write: migrate `shuffle` fully here; leave other `next_u64` callers on a private `legacy_rng_state` only if tests force it — prefer migrating all callers in Task 2 immediately after.

**Practical sequencing:** In Task 1, change `next_u64` to:

```rust
pub(crate) fn next_u64(&mut self) -> u64 {
    let player = self.active_player;
    self.with_op_rng(player, |rng| rng.next_u64())
}
```

That wrongly bumps iteration per sample for dig/misc until Task 2 wraps whole ops — **avoid**. Instead keep `legacy_rng_state: u64` for non-shuffle callers through end of Task 2, and only shuffle uses `with_op_rng`. Document in a one-line comment.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run --profile ci -E 'test(the_same_master_seed) + test(different_players_get_independent) + test(gen_index_stays_in_range) + test(the_same_seed_shuffles_a_library_identically)'`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/engine/Cargo.toml crates/engine/src/rng.rs crates/engine/src/lib.rs \
  crates/engine/src/types/card.rs crates/engine/src/core.rs crates/engine/src/zones.rs \
  crates/engine/tests/game.rs Cargo.lock
git commit -m "feat(engine): derive-per-op PRNG for library shuffles"
```

---

### Task 2: Migrate all engine random ops onto `with_op_rng`

**Files:**
- Modify: `crates/engine/src/pending/handlers/dig.rs`
- Modify: `crates/engine/src/resolution/resolve_misc.rs`
- Modify: `crates/engine/src/core.rs` (delete `next_u64` + `legacy_rng_state`)
- Modify: `crates/engine/tests/game.rs` (fix any goldens that assumed shared-stream `%` picks — search `next_u64` / `seed 7`)

**Interfaces:**
- Consumes: `Game::with_op_rng`, `OpRng::gen_index`
- Produces: every logical random op is one `with_op_rng` call attributed to controller or `active_player`

- [ ] **Step 1: Write / adjust failing tests for attribution**

```rust
#[test]
fn random_opponent_pick_is_stable_for_same_iteration() {
    // Build a 3p board where MustAttackRandomOpponent / ExileRandomFromGraveyard path runs
    // twice from the same master seed and seat sequence — assert identical picks.
    // Mirror an existing resolve_misc test if one exists; otherwise add a minimal unit that
    // calls the same helper the effect uses.
}
```

If adding a full card test is heavy, unit-test a new `Game::pick_index(player, len) -> usize` wrapper used by resolve_misc.

- [ ] **Step 2: Run to see red / broken goldens**

Run: `cargo nextest run --profile ci -E 'test(random_opponent) + test(seed 7)'` and/or full `cargo nextest run --profile ci -p engine --lib` plus the known golden at `game.rs` ~52616.

- [ ] **Step 3: Implement**

In `resolve_misc.rs`:

```rust
let idx = self.with_op_rng(self.active_player, |rng| rng.gen_index(graveyard.len()));
```

(Use the effect’s controller when available in that match arm.)

In `dig.rs`, wrap each shuffle loop in one `with_op_rng(owner, |rng| { for i in … { let j = rng.gen_index(i+1); … } })`.

Delete `next_u64` and any `legacy_rng_state`.

Update broken goldens to the new deterministic outcomes (same seed must still be stable — only the expected card names change).

- [ ] **Step 4: Run engine tests**

Run: `cargo nextest run --profile ci -p engine`

Expected: PASS (fix any remaining goldens in this commit if needed)

- [ ] **Step 5: Commit**

```bash
git add crates/engine/src/pending/handlers/dig.rs crates/engine/src/resolution/resolve_misc.rs \
  crates/engine/src/core.rs crates/engine/tests/game.rs
git commit -m "feat(engine): attribute dig and random picks to derive-per-op RNG"
```

---

### Task 3: Engine mulligan phase

**Files:**
- Create: `crates/engine/src/mulligan.rs`
- Modify: `crates/engine/src/lib.rs` (`mulliganing: bool`, mod)
- Modify: `crates/engine/src/types/card.rs` (`mulligans_taken`, `hand_kept` on `Player`)
- Modify: `crates/engine/src/types/stack.rs` (intents, events, meaningful actions, rejects)
- Modify: `crates/engine/src/lib.rs` `submit_inner` match arms
- Modify: `crates/engine/src/query.rs` (`meaningful_actions` early return when mulliganing)
- Test: `crates/engine/tests/game.rs`

**Interfaces:**
- Consumes: `Game::shuffle`, `draw_card` / draw helpers, `begin_first_turn`
- Produces:
  - `fn hand_size_after_mulligans(mulligans_taken: u8) -> u8` — `7 - max(0, mulligans_taken.saturating_sub(1))` as u8
  - `Game::begin_mulligans(&mut self)` — sets `mulliganing = true`, all `hand_kept = false`, `mulligans_taken = 0`, `refresh_actions`
  - `Game::mulliganing(&self) -> bool`
  - `Intent::KeepHand { player }`, `Intent::Mulligan { player }`
  - `Event::MulliganTaken { player, mulligans_taken, hand_size }`, `Event::HandKept { player }`, `Event::MulligansFinished`
  - On last keep: `mulliganing = false`, append `begin_first_turn()` events

**Rules:**
- `hand_size_after_mulligans(n)` after `n` mulligans completed
- Mulligan illegal if `hand_size_after_mulligans(mulligans_taken + 1) < 1` (floor at 1 → auto-keep when draw size is 1)
- Mulligan: move hand → library (extend library with hand order irrelevant because shuffle follows), `shuffle(player)`, draw N times, bump `mulligans_taken`, if N==1 set `hand_kept`
- Keep/Mulligan **do not** require priority; reject if `!mulliganing` or already kept
- While `mulliganing`, reject other non-Concede intents with a clear `Reject` (add `Reject::Mulliganing` or reuse `IllegalAction`)

- [ ] **Step 1: Write the failing tests**

```rust
fn deal_opening(game: &mut Game, n: usize) {
    let plains = card("Plains");
    for p in 0..game.player_count() {
        let pid = PlayerId(p);
        game.stack_library(pid, &vec![plains; 40]);
        game.shuffle(pid);
    }
    for _ in 0..7 {
        for p in 0..game.player_count() {
            game.draw_card(PlayerId(p));
        }
    }
    game.begin_mulligans();
}

#[test]
fn friendly_mulligan_redraws_seven() {
    let mut game = Game::with_players(2, 1);
    deal_opening(&mut game, 40);
    game.submit(Intent::Mulligan { player: PlayerId(0) }).unwrap();
    assert_eq!(game.hand(PlayerId(0)).len(), 7);
    assert!(game.mulliganing());
}

#[test]
fn second_mulligan_draws_six() {
    let mut game = Game::with_players(2, 1);
    deal_opening(&mut game, 40);
    game.submit(Intent::Mulligan { player: PlayerId(0) }).unwrap();
    game.submit(Intent::Mulligan { player: PlayerId(0) }).unwrap();
    assert_eq!(game.hand(PlayerId(0)).len(), 6);
}

#[test]
fn mulligan_to_one_auto_keeps() {
    let mut game = Game::with_players(2, 1);
    deal_opening(&mut game, 40);
    for _ in 0..7 {
        // 1st keeps 7, then 6,5,4,3,2,1 — adjust loop to reach size 1
        let _ = game.submit(Intent::Mulligan { player: PlayerId(0) });
    }
    assert!(!game.players[0]./* expose via accessor */ /* hand_kept */);
    // Prefer public: assert player 0 cannot mulligan and is kept:
    assert!(game.submit(Intent::Mulligan { player: PlayerId(0) }).is_err());
}

#[test]
fn all_keeps_begin_first_turn() {
    let mut game = Game::with_players(2, 1);
    deal_opening(&mut game, 40);
    game.submit(Intent::KeepHand { player: PlayerId(0) }).unwrap();
    assert!(game.mulliganing());
    let events = game.submit(Intent::KeepHand { player: PlayerId(1) }).unwrap();
    assert!(!game.mulliganing());
    assert!(events.iter().any(|e| matches!(e, Event::MulligansFinished)));
    assert!(events.iter().any(|e| matches!(e, Event::StepBegan { step: Step::Upkeep, .. })));
}

#[test]
fn simultaneous_keeps_ignore_priority() {
    let mut game = Game::with_players(3, 1);
    deal_opening(&mut game, 40);
    // P2 keeps even though priority is P0
    game.submit(Intent::KeepHand { player: PlayerId(2) }).unwrap();
    game.submit(Intent::KeepHand { player: PlayerId(0) }).unwrap();
    game.submit(Intent::KeepHand { player: PlayerId(1) }).unwrap();
    assert!(!game.mulliganing());
}
```

Use existing `Game::hand` / zone query helpers if present; otherwise assert via `schema::complete_visible` hand_count or add `pub fn hand_kept(&self, p) -> bool`.

- [ ] **Step 2: Run tests — expect FAIL**

Run: `cargo nextest run --profile ci -E 'test(friendly_mulligan) + test(second_mulligan) + test(mulligan_to_one) + test(all_keeps_begin) + test(simultaneous_keeps)'`

- [ ] **Step 3: Implement mulligan module + wiring**

`query.rs` at top of `meaningful_actions`:

```rust
if self.mulliganing {
    let mut actions = Vec::new();
    if !self.players[player.0 as usize].hand_kept {
        actions.push(MeaningfulAction::KeepHand);
        let next = hand_size_after_mulligans(self.players[player.0 as usize].mulligans_taken + 1);
        if next >= 1 {
            actions.push(MeaningfulAction::Mulligan);
        }
    }
    return actions;
}
```

`submit_inner`: before other play intents, if `mulliganing` and intent is not Keep/Mulligan/Concede → `Err(Reject::…)`.

Finish path when all `hand_kept`:

```rust
self.mulliganing = false;
events.push(Event::MulligansFinished);
events.extend(self.begin_first_turn());
```

- [ ] **Step 4: Run tests — expect PASS**

Run: same filter as Step 2.

- [ ] **Step 5: Commit**

```bash
git add crates/engine/src/mulligan.rs crates/engine/src/lib.rs crates/engine/src/types/card.rs \
  crates/engine/src/types/stack.rs crates/engine/src/query.rs crates/engine/tests/game.rs
git commit -m "feat(engine): simultaneous friendly-then-draw-to-size mulligans"
```

---

### Task 4: `seed_game` enters mulligan phase + test helper

**Files:**
- Modify: `crates/server/src/decks.rs`
- Modify: `crates/server/src/session.rs` (skip auto-advance while mulliganing)
- Modify: server/schema tests that assume post-seed Main1 playability

**Interfaces:**
- Consumes: `Game::begin_mulligans`, `Intent::KeepHand`
- Produces:
  - `seed_game(...)` deals 7, calls `begin_mulligans()`, does **not** call `begin_first_turn` / `advance_seeded_game`
  - `pub fn keep_all_hands(game: &mut Game)` for tests — submit KeepHand for every non-kept seat until `!mulliganing()`, then `advance_seeded_game(game)`
  - `TableSession` / `auto_advance`: if `game.mulliganing() { return; }`

- [ ] **Step 1: Failing test**

Update `a_seeded_game_deals_seven_to_each_player_and_leaves_the_rest` expectations:

```rust
let mut game = seed_game(&seats, 0);
assert!(game.mulliganing());
assert_eq!(/* hand_count still 7 */);
keep_all_hands(&mut game);
assert!(!game.mulliganing());
```

Add:

```rust
#[test]
fn seed_game_does_not_start_turns_until_keeps() {
    let game = seed_game(&seats, 0);
    assert!(game.mulliganing());
    // step may still be Main1 parked — assert no StepBegan Upkeep yet via a flag or mulliganing
}
```

- [ ] **Step 2: Run — FAIL on old `begin_first_turn` behavior**

- [ ] **Step 3: Implement `seed_game` + `keep_all_hands` + auto_advance guard**

```rust
pub fn seed_game(seats: &[(PlayerId, SeatDeck)], seed: u64) -> Game {
    let mut game = Game::with_players(seats.len() as u8, seed);
    // designate / stack / shuffle / draw 7 as today
    game.begin_mulligans();
    game
}

pub fn keep_all_hands(game: &mut Game) {
    while game.mulliganing() {
        for p in 0..game.player_count() {
            let player = PlayerId(p);
            if game.hand_kept(player) {
                continue;
            }
            let _ = game.submit(Intent::KeepHand { player });
        }
    }
    crate::session::advance_seeded_game(game);
}
```

Update every server test that needs a playable board to call `keep_all_hands` after `seed_game`.

- [ ] **Step 4: Run**

Run: `cargo nextest run --profile ci -p server`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/server/src/decks.rs crates/server/src/session.rs
git commit -m "feat(server): seed into mulligan phase before first turn"
```

---

### Task 5: Wire + schema projection

**Files:**
- Modify: `proto/mtgfr/v1/intent.proto` — add `WireIntentKeepHand`, `WireIntentMulligan` to the `oneof` (next free field numbers)
- Modify: snapshot/common proto if `VisibleState` / `PlayerView` live there — else schema-only JSON fields already used by gRPC mapping
- Modify: `crates/schema/src/intent.rs`, `event.rs`, `dto.rs`, `snapshot.rs`
- Run: `just server-codegen` and client `bun run gen` per repo practice
- Modify: `client/src/wire/types.ts` if hand-maintained alongside codegen

**Interfaces:**
- Consumes: engine intents/events/mulligan accessors
- Produces:
  - `WireIntent::KeepHand { player }`, `WireIntent::Mulligan { player }`
  - `to_intent` / `to_intent_for_seat` mappings
  - `VisibleState.mulliganing: bool`
  - `PlayerView`: `mulligans_taken: u8`, `hand_kept: bool`, `can_mulligan: bool`
  - Event redaction for the three new events (public)

- [ ] **Step 1: Schema unit test (failing)**

In `crates/schema/src/snapshot.rs` tests:

```rust
#[test]
fn mulliganing_snapshot_exposes_seat_status() {
    // build engine game in mulliganing, complete_visible for P0
    // assert snap.mulliganing && !snap.players[0].hand_kept && snap.players[0].can_mulligan
}
```

- [ ] **Step 2: Run — FAIL**

- [ ] **Step 3: Proto + mappings + projection fields** (`#[serde(default)]` on new fields for compat)

Follow existing `PassPriority` patterns exactly for intent mapping. Stamp seat via `to_intent_for_seat`.

- [ ] **Step 4: Codegen + tests**

Run: `just server-codegen` (or project equivalent), then `cargo nextest run --profile ci -p schema`

- [ ] **Step 5: Commit**

```bash
git add proto crates/schema client/src/wire
git commit -m "feat(wire): keep/mulligan intents and mulligan snapshot fields"
```

---

### Task 6: Cloudflare drand beacon at seed

**Files:**
- Create: `crates/server/src/beacon.rs`
- Modify: `crates/server/Cargo.toml` (direct `reqwest` with `json`/`rustls-tls` if not already direct)
- Modify: `crates/server/src/lobby.rs`
- Modify: `crates/server/src/table.rs` (`seed: [u8; 32]`, `beacon_round: u64`)
- Modify: `crates/server/src/lib.rs` / `decks.rs` to pass `[u8; 32]` into `Game::with_master_seed`
- Test: `crates/server/src/beacon.rs` (`#[cfg(test)]` with `mockito` or a `BeaconSource` trait)

**Interfaces:**
- Consumes: HTTPS GET
- Produces:
  - `pub struct MasterEntropy { pub master_seed: [u8; 32], pub beacon_round: u64 }`
  - `pub async fn fetch_master_entropy() -> Result<MasterEntropy, BeaconError>`
  - Env `MTGFR_MASTER_SEED` = 64 hex chars → skip network, `beacon_round = 0`
  - Primary URL: `https://drand.cloudflare.com/public/latest`; fallback `https://api.drand.sh/public/latest`
  - Parse JSON `round` + hex `randomness` (32 bytes); `master_seed = blake3(randomness_bytes)` or use randomness bytes directly if len==32
  - `ponytail:` skip BLS signature verify in v1 (HTTPS + Cloudflare relay); store `round` for audit
  - On failure after 2–3 tries: `StatusCode::SERVICE_UNAVAILABLE` from `seed_table_core`
  - `table.seed = master_seed`, `table.beacon_round = round`

- [ ] **Step 1: Failing unit tests with injected source**

```rust
#[async_trait]
pub trait EntropySource {
    async fn latest(&self) -> Result<MasterEntropy, BeaconError>;
}

#[tokio::test]
async fn env_master_seed_skips_network() {
    // set MTGFR_MASTER_SEED in test, call resolve_entropy(), assert round 0 and bytes
}

#[tokio::test]
async fn beacon_http_failure_surfaces() {
    // mock source returns err → seed_table_core maps to 503
}
```

Prefer a small trait so lobby tests do not hit the network.

- [ ] **Step 2: Run — FAIL**

- [ ] **Step 3: Implement fetch + lobby wiring**

Change `seed_game` signature to take `[u8; 32]` **or** keep `u64` for old tests and add `seed_game_with_master(seats, master: [u8; 32])`. Prefer:

```rust
pub fn seed_game(seats: &[(PlayerId, SeatDeck)], master_seed: [u8; 32]) -> Game {
    let mut game = Game::with_master_seed(seats.len() as u8, master_seed);
    // …
}
```

Update test call sites: `seed_game(&seats, u64_to_master(0))` helper.

- [ ] **Step 4: Run server tests**

Run: `cargo nextest run --profile ci -p server`

- [ ] **Step 5: Commit**

```bash
git add crates/server/src/beacon.rs crates/server/src/lobby.rs crates/server/src/table.rs \
  crates/server/src/decks.rs crates/server/Cargo.toml Cargo.lock
git commit -m "feat(server): seed games from Cloudflare drand beacon"
```

---

### Task 7: Client mulligan UI

**Files:**
- Create: `client/src/components/molecules/mulligan-bar.tsx`
- Create: `client/src/lib/mulligan.ts` (pure helpers: label copy, whether to show)
- Create: `client/src/lib/mulligan.test.ts`
- Modify: `client/src/components/organisms/board.tsx` (or `play.tsx` chrome) to render bar
- Modify: intent submit path already used for `pass_priority`

**Interfaces:**
- Consumes: `VisibleState.mulliganing`, per-seat fields, `session.submit` / `buildIntentEnvelope`
- Produces: Keep / Mulligan buttons for local seat when `mulliganing && !hand_kept`; waiting copy when kept but others pending

- [ ] **Step 1: Failing unit tests**

```ts
import { describe, expect, it } from "vitest";
import { mulliganChrome } from "~/lib/mulligan";

describe("mulliganChrome", () => {
  it("offers keep and mulligan for undecided local seat", () => {
    const c = mulliganChrome({
      mulliganing: true,
      localSeat: 0,
      players: [
        { player: 0, hand_kept: false, can_mulligan: true, mulligans_taken: 0 },
        { player: 1, hand_kept: false, can_mulligan: true, mulligans_taken: 0 },
      ],
    });
    expect(c.show).toBe(true);
    expect(c.canMulligan).toBe(true);
    expect(c.waitingCount).toBe(2);
  });

  it("hides controls when local seat kept but still shows waiting", () => {
    const c = mulliganChrome({
      mulliganing: true,
      localSeat: 0,
      players: [
        { player: 0, hand_kept: true, can_mulligan: false, mulligans_taken: 0 },
        { player: 1, hand_kept: false, can_mulligan: true, mulligans_taken: 0 },
      ],
    });
    expect(c.showControls).toBe(false);
    expect(c.waitingCount).toBe(1);
  });

  it("hidden when not mulliganing", () => {
    expect(mulliganChrome({ mulliganing: false, localSeat: 0, players: [] }).show).toBe(false);
  });
});
```

- [ ] **Step 2: Run**

Run: `cd client && bun run test src/lib/mulligan.test.ts`

Expected: FAIL

- [ ] **Step 3: Implement helpers + bar + board wiring**

Buttons submit `{ kind: "keep_hand", player: seat }` / `{ kind: "mulligan", player: seat }` via existing intent envelope helper. Disable Mulligan when `!can_mulligan`. Use existing `Button` + DESIGN tokens; no new card UI — hand already visible.

Hide `PriorityContextBar` primary pass cluster while `mulliganing` (guard in board).

- [ ] **Step 4: Run client tests**

Run: `just client-test` (or focused vitest)

- [ ] **Step 5: Commit**

```bash
git add client/src/lib/mulligan.ts client/src/lib/mulligan.test.ts \
  client/src/components/molecules/mulligan-bar.tsx client/src/components/organisms/board.tsx
git commit -m "feat(client): keep/mulligan controls during opening mulligans"
```

---

### Task 8: Docs + verification

**Files:**
- Modify: `docs/superpowers/specs/2026-07-20-engine-core-and-event-model.md` (RNG + mulligan phase note)
- Modify: `docs/superpowers/specs/2026-07-20-lobby-table-routing-and-live-game.md` (beacon seed)
- Modify: `docs/superpowers/specs/2026-07-20-wire-protocol-and-visibility.md` (intents/fields)
- Modify: design status line if needed (`Implemented` note)

- [ ] **Step 1: Update the three specs with short behavior paragraphs matching the design**

- [ ] **Step 2: Run verification**

Run: `just server-check` and `just client-check` (or `just check` if feasible)

Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs
git commit -m "docs: record mulligan phase and drand seeding behavior"
```

---

## Self-review (plan vs spec)

| Spec requirement | Task |
|------------------|------|
| Simultaneous mulligans | 3, 7 |
| Friendly then draw-to-size; floor 1 auto-keep | 3 |
| No London bottoming | 3 |
| drand / Cloudflare beacon master seed | 6 |
| Persist seed + round | 6 |
| `hash(master ‖ player ‖ iteration)` per logical op | 1–2 |
| Active player / controller attribution | 2 |
| Unbiased FY | 1 |
| Engine purity | 1–3 vs 6 split |
| Wire intents + private hands + public kept status | 5, 7 |
| No silent OsRng fallback | 6 |
| Tests for mulligan + RNG + beacon mock | 1–6 |
| Disconnect force-keep out of scope | (explicit non-goal; no task) |

No TBD placeholders remain. `seed_game` / `with_players` signatures are consistent across tasks (`[u8; 32]` master with u64 expand helper for tests).
