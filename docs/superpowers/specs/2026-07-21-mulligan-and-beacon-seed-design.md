# Mulligans and Cloudflare beacon seeding

**Date:** 2026-07-21  
**Status:** Approved for planning  
**Context:** Opening hands are dealt at `Tables.Seed` with no mulligan path (`OPENING_HAND = 7` — “no mulligan — Phase 3”). Library shuffle uses Fisher–Yates over a single shared splitmix64 PRNG seeded from `OsRng`. Cloudflare appears only as the public tunnel edge, not as entropy. Friends Commander expects one free (friendly) mulligan, then penalty redraws, with publicly auditable seed entropy and per-player derived randomness so seat order does not couple libraries.

## Goals

1. **Simultaneous mulligan phase** after deal, before `begin_first_turn()`.
2. **Friendly then unfriendly redraws:** first mulligan redraws 7; later mulligans draw to size (`6…1`); auto-keep at hand size 1 (no London bottoming).
3. **Master seed from Cloudflare’s drand randomness beacon** at seed time; persist seed (+ beacon round) for replay.
4. **Every logical random operation** derives from `hash(master_seed ‖ player ‖ iteration)` with active-player / controller attribution; unbiased Fisher–Yates for shuffles.
5. **Engine stays pure** — beacon HTTP lives at the server edge; tests inject fixed master seeds.

## Non-goals (v1)

- London mulligan (draw 7 then put N on bottom) or Vancouver scry.
- Client-side verifiable recomputation UI / commit–reveal theater beyond storing round id + seed.
- Host force-keep, mulligan timers, or disconnect auto-keep policy.
- Changing CR 103.8 first-draw skip rules once the game starts.
- Silent production fallback to `OsRng` when the beacon is unreachable.

## Approach

**Engine-native mulligan phase + derive-per-op RNG (Approach 1).**

Chosen over a server-only mulligan loop so rules and randomness stay replayable in-engine under TDD. Chosen over full commit–reveal tickets as YAGNI for friends tables while still using a public beacon and recorded seed.

## Mulligan rules

| Situation | Hand size after |
|-----------|-----------------|
| Opening deal | 7 |
| 1st mulligan (friendly) | 7 |
| 2nd mulligan | 6 |
| 3rd | 5 |
| … | `7 - max(0, mulligans_taken - 1)` |
| At 1 | Auto-keep; further mulligans illegal |

**Per seat state:** `mulligans_taken: u8`, `kept: bool`.

**Legal intents while `!kept` and phase is mulliganing:**

- `KeepHand` → set `kept`.
- `Mulligan` → if the draw size for `mulligans_taken + 1` would be < 1, reject; else return hand to library, shuffle (one logical random op attributed to that seat), draw `hand_size(mulligans_taken + 1)`, increment `mulligans_taken`. If resulting hand size is 1, set `kept`.

**Simultaneous:** any non-kept seat may keep or mulligan; no seat ordering. When all seats have `kept`, emit completion and call existing `begin_first_turn()` (2p first-draw skip unchanged).

**No bottom selection** — draw-to-size only.

## Seeding and derived randomness

### Master seed (server)

At `Tables.Seed`:

1. HTTP GET latest randomness from Cloudflare’s drand relay (`https://drand.cloudflare.com`, with optional League of Entropy fallbacks such as `api.drand.sh`).
2. Take beacon `randomness` bytes (and record `round`).
3. Reduce to engine `master_seed: [u8; 32]` (hash if needed).
4. Persist `master_seed` and `beacon_round` on the table (fixes today’s unused `table.seed`).
5. Construct the engine with that master seed; deal opening hands; enter mulligan phase — do **not** `begin_first_turn` yet.

**Failure:** brief retries across relays; if still failing, fail the seed RPC. Lobby remains ready; host can retry. No silent `OsRng` in production.

**Dev/test:** inject fixed master seed via test API / env (e.g. `MTGFR_MASTER_SEED`) and skip network.

### Derive-per-op (engine)

Replace the single shared `rng_state` consumption model for random ops with:

```text
key = BLAKE3(master_seed ‖ player_index:u8 ‖ op_iteration:u64)
```

Seed a short-lived PRNG from `key` for **that logical operation only**. Advance that player’s `op_iteration` **once** per logical op:

- one library shuffle → one bump (Fisher–Yates runs inside that PRNG);
- one “pick random target” / dig random / similar → one bump.

**Index selection:** unbiased (rejection sampling or Lemire) — not naive `next_u64() % n`.

**Attribution:** `player_index` = controller of the effect when applicable, else **active player**. Setup and mulligan shuffles attribute to the seat whose library is shuffled. Opening `seed_game` shuffles (one per seat before the opening deal) use the same derive API and consume that seat’s first iteration(s).

**Replay:** same `master_seed` + same deck stacking + same intent sequence ⇒ identical iteration counters and outcomes.

## Wire, projection, client

**Proto intents:** `KeepHand`, `Mulligan` (auth/seat only).

**Events:** `MulliganTaken { player, mulligans_taken, hand_size }`, `HandKept { player }`, `MulligansFinished`. Never library order.

**Snapshot / projection:** phase discriminant `Mulliganing`; per seat `{ mulligans_taken, kept, can_mulligan }`. Hands remain private; other players see decision status only.

**Client:** for the local seat while mulliganing and not kept — hand + Keep / Mulligan controls (`Mulligan` disabled when `!can_mulligan`). Chrome shows how many seats are still deciding. After `MulligansFinished`, existing turn/priority UI.

**Seed path:** BFF start → `Tables.Seed` → beacon fetch → inject master seed → deal → mulligan phase → (clients keep/mulligan) → `begin_first_turn`.

## Error handling and edge cases

| Case | Behavior |
|------|----------|
| Beacon down | Seed RPC fails after retries; no partial live game |
| Illegal mulligan/keep | Normal intent rejection |
| Hand size 1 after mulligan | Auto-keep |
| Disconnect while undecided | Seat stays undecided (table waits); force-keep out of scope |
| Empty / illegal deck at seed | Existing seed validation unchanged |

## Testing

TDD at the lowest layer:

1. **Engine mulligan** — free mulligan stays at 7; second draws 6; floor 1 auto-keeps; multi-seat simultaneous keep; first turn does not begin until all kept.
2. **Derived RNG** — same `(master, player, iteration)` ⇒ same shuffle; different player or iteration ⇒ different; FY unbiased helper smoke; attribution uses controller/active player.
3. **Server seed** — persists beacon round + master seed (mocked HTTP); beacon failure fails seed; env/test inject skips network.
4. **Schema/client** — projection fields and intent mapping smoke.

## Out of scope

- Verifiable client recomputation UI.
- London / Vancouver mulligan variants.
- Mulligan timers, host force-keep, disconnect policies.
- Marketing/SEO or Cloudflare products beyond the randomness beacon.

## Relation to existing specs

- Extends [lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md) seed path (replace `OsRng`-only entropy; delay first turn for mulligans).
- Extends [engine-core-and-event-model](2026-07-20-engine-core-and-event-model.md) RNG and game-start (mulligan phase; derive-per-op).
- Extends [wire-protocol-and-visibility](2026-07-20-wire-protocol-and-visibility.md) with keep/mulligan intents and redacted mulligan status.
- First-draw skip remains as in [turn-priority-and-stack](2026-07-20-turn-priority-and-stack.md) after mulligans complete.
