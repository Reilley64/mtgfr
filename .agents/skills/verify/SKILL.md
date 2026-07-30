---
name: verify
description: Drive a live two-player mtgfr game end-to-end to verify engine/server/client changes at the real surface (browser + HTTP API).
---

# Verifying mtgfr changes live

Before claiming the change is verified, follow **`verification-before-completion`**: run the
commands below (or the project `verify` path), read the output, and only then claim green.
When a live drive fails mysteriously, use **`systematic-debugging`** before patching.

## Handles

- **Dev loop is usually already running**: `just dev` = `bacon server` (auto-rebuilds+restarts `target/debug/server serve` — health on :8080, gRPC on :50051 — on source change) + vite on :5173. Check `lsof -nP -i :8080` — if `server`'s parent is `bacon server`, the running binary already has your changes (bacon restarted it after your last build). Don't start a second server; listen addrs come from `Settings` (`config/mtgfr.toml` / env).
- Cold start: `DATABASE_URL="sqlite::memory:" cargo run -p server` + `cd client && bun run dev`. If
  the build has no sqlite feature (`unsupported feature: sqlite feature not enabled`), give the
  smoke run its own two Postgres DBs instead of touching the dev ones: `createdb mtgfr_smoke_api` +
  `server migration apply`, and `createdb mtgfr_smoke` + `WEB_DATABASE_URL=… just client-migrate`
  (the recipe does not default that variable). Postgres itself comes from `docker compose up -d`,
  which also creates `mtgfr_web`. Vite prints the port it actually took — in a worktree alongside a
  running dev loop that is `:3001`, not `:5173`, and on a bare `bun run dev` it is `:3000`.
- **"migrations in the database that do not match any local migration"** means the Drizzle ledger
  outlived the migration files — `client/db/migrations/` was squashed to a single init and the dev
  `mtgfr_web` still carries the pre-squash rows. A fresh DB (`createdb mtgfr_web_smoke` +
  `WEB_DATABASE_URL=… just client-migrate`) is the cheapest way past it for a smoke run. To repair
  the dev DB in place, confirm its schema matches a freshly-migrated one (diff
  `information_schema.columns`), then point the ledger at the squashed migration:
  `update drizzle.__drizzle_migrations set hash = '<hash from the fresh DB>' where id = 1;` and
  delete the leftover rows. Lobby data survives; dropping the DB also works and loses only
  pre-game lobbies.
- Confirm the API is up: `curl -s localhost:8080/health/live`. Every game/auth/decks/cards route is gRPC now (wire-protocol-and-visibility spec) — there's no `/openapi.json` or REST path to curl directly; drive it through the BFF's `/api/rpc` (below) or a gRPC client against `:50051`.
- **An isolated stack may pick its own HTTP, Vite and Postgres ports — never its own gRPC port.**
  Routed table calls ignore `GRPC_UPSTREAM`: the BFF maps a table's `pod_dns` through
  `grpcUpstreamFromPodDns` (`client/app/domain/api-upstream.ts`), which pins every pod to `:50051`.
  `GRPC_UPSTREAM` only covers the *unrouted* default path (auth/decks/cards), so a second server
  on another gRPC port signs you in fine and then fails every game stream with
  `503 connect ECONNREFUSED 127.0.0.1:50051`.

## Seating a 4-player game via the BFF (no UI needed)

The client talks to the BFF at `client/server/routes/api/rpc/[...path].ts` (lobby/table routes in
`client/server/routes/api/[...path].ts`), which dials tonic. Drive the same calls with `curl`
against the BFF (`localhost:5173` in dev) rather than the API directly — cookies still carry the
session (`-c jar.txt` on signup, `-b jar.txt` after). See `client/app/domain/wire/rpcs.ts` for the RPC
names/shapes, or use a gRPC client (e.g. `grpcurl`) straight against `:50051` with
`x-session-token` metadata (see `crates/server/src/grpc/auth_ctx.rs`).

1. Sign up per player, `POST /api/rpc/auth/signup` (fresh throwaway emails — the dev DB persists).
1a. Building a deck instead of using a precon (`POST /api/rpc/decks`): submit the whole candidate
   pool once and read the 422 `problems` — `validate` names *every* off-identity card in one
   response, so the color-identity filter costs one round trip instead of reimplementing CR 903.4.
   Then keep ~64 spells and pad to 99 with basics: **a landless 99 drives nothing.** With an empty
   board `has_meaningful_action` is false for every seat, so `auto_advance` passes whole turns and
   the drive logs a keep_hand and nothing else — that is correct engine behavior, not a wedge.
2. Precons have negative ids (`crates/server/src/precons.rs`: -1 Silverquill … -5 Quandrix,
   -6 Enchantress Rubinia, -7 Deathdancer Xira, -8 Political Puppets, -9 Mirror Mastery,
   -10 Heavenly Inferno); usable by anyone, no deck building.
3. Lobby, one cookie jar per seat: `POST /api/tables/v1` (host — returns `{table_id}`) →
   `POST /api/tables/<table>/join/v1 {deck_id}` per seat → `POST /api/tables/<table>/ready/v1
   {ready:true}` → `POST /api/tables/<table>/start/v1 {}`. **The table id is a path param, not a
   body field** (`client/server/routes/api/tables/[table]/`, per the AGENTS.md routing rule) — a
   body-only `{table_id}` hits no route and Nitro answers 200 with the SPA's index HTML, so a driver
   that only checks the status code sees four silent no-ops and then a lobby that never starts.
   `GET /api/tables/<table>/lobby/v1` reads back `{seats, started, start_error}`.

## Reading state / driving intents

- State: the first frame of `Game.Stream` (BFF: `GET /api/rpc/game/<table>/stream`, SSE) is a full snapshot for that caller's seat — take the first frame where `frame == "snapshot"`, then hang up.
- Intents: `Game.SubmitIntent` (BFF: `POST /api/rpc/game/<table>/intent`) with `{"table_id","client_seq":<int, monotonic>,"intent":{...}}`. Useful kinds: `take_action {player,id}` (ids from `state.actions`), `pass_priority {player}`, `discard {player,cards}`, `arrange_top {player,top,bottom}` (answers scry).
- **A rejected intent still acks HTTP 200.** The ack is `{accepted, reject_reason}` — a driver that
  trusts the status code re-sends the same illegal intent forever and looks like an engine wedge.
  Branch on `accepted`, and remember the rejected `(action id, target)` pair so the search advances.
- **Never invent targets.** `ActionView.targets` already carries `Game::legal_targets` for that
  action; guessing from the battlefield turns one legal cast into dozens of
  `reject.illegal_target` acks (6083 of them in one heavenly-inferno drive, 330 accepted).
  Guess only when `targets` is empty and `needs_target` is true (modal casts carry targets per mode).
- **Answer a modal cast with `modes`.** An `ActionView` carrying `modal {choose, modes}` is
  `reject.illegal_mode` without a `modes: [{index, target?}]` list on the intent — one pick per
  `choose`, skipping any mode whose own `targets` is empty. Blue Elemental Blast ate 40 identical
  rejects in the 2ed drive before the driver read the block the snapshot had already handed it.
- **Park a rejected action id instead of re-picking it.** Keep a `blocked` set keyed on action id
  and report it at the end; otherwise one wedged card starves the rest of the drive of ticks.
- **Submit only when the snapshot still says it's your priority** (`state.priority == viewer`) —
  each seat reads its own SSE snapshot, so the loser of that race collects
  `reject.not_your_priority`. Exempt `keep_hand` / `mulligan`: all four seats decide those before
  anyone holds priority, and guarding them stalls the game before turn one.
- **Pay the cost the action names.** An `ActionView` carries its own cost picks —
  `sacrifice_choices` (Fallen Angel's "Sacrifice a creature"), `discard_choices`/`discard_count`,
  `graveyard_exile_choices`, `has_x`/`min_x`. Omitting one is `reject.cannot_activate` /
  `reject.cannot_pay_cost`, not an engine bug; `Some([])` means the cost exists and nothing can
  pay it, so skip the action entirely.
- **A listed action must be submittable exactly as listed.** When a drive's rejects cluster on one
  card, suspect the *advertisement* before the driver: a `needs_target` the cast gate then rejects
  is a real client-facing bug (post-cast target clauses, CR 601.2c, were exactly this). The same
  holds for a `PendingChoice`: equip listing opponents' creatures and a `ChooseSpellTargets` raised
  at `min: 0, max: 0` with a full `legal` list were the next two clusters, each hidden behind the
  last — so re-run the drive after every fix.
- **`logs/actions.<TABLE>.toon` is the drive-debugging oracle.** Every intent the server saw, one
  row: seq, player, intent, accepted, reject reason, step/active/priority/pending after it, and the
  full event list. Diagnose a wedge from that trace before touching engine code.
- `scratchpad/drive.py` pattern from past runs: loop { answer pending_choice (discard/scry), play a land if offered, else pass } until the state you want. Precon games hit real choices (cleanup discards, scry lands) — handle or the loop wedges. Mirror `client/app/domain/choice.ts` for the answer shapes, and keep a fallback chain per choice (decline the cost, answer "no", try each single target, then empty) — the first answer the UI would send is not always payable.
- Tally every action kind the snapshots *offered*, not just the ones submitted. The gap is the
  honest coverage report: the 2ed drive was offered `declare_blockers` zero times across 70 turns
  because a seat that attacks with everything has nothing untapped left to block with.
- Per-stack yield: `Game.SetYield` `{table_id, enabled}`.

## Watching in the browser

- agent-browser: log in at `localhost:5173/login`, then open `/play/ID`. The `player` URL param is display-only; the server resolves the seat from the session cookie.
- agent-browser saves screenshots relative to its own cwd (often the repo root) — move them out of the repo afterward.
- Timing-sensitive UI (the ~2s stack hold): `agent-browser eval "<js click>"` beats snapshot→click round-trips.

## Gotchas

- Auto-advance means turns fly: a player with no meaningful action is passed instantly, so "wait a step" states are hard to park on. Anything on the stack holds ~2s (`STACK_HOLD`) before auto-resolving — that's the window to screenshot.
- Bare `engine::Game::new()` tables die fast under auto-advance (empty libraries → draw-out deaths). Seed real decks.
- Test tables/users linger in the dev DB and in-memory registry until game over; use recognizable throwaway names.

## Interaction checklist

Required before claiming done when the PR is flagged **Interaction / UI**
(PR template checkbox / AGENTS.md). Always available otherwise.

Drive via browser (`agent-browser`) and/or BFF curls against the running
`just dev` stack. Note which items you exercised in the PR or agent summary.

1. **Host a table** with local defaults after `just migrate` / `just client-migrate` — create succeeds; not a generic “Couldn't reach the table.”
2. **Alt-hold** over a face-up board or hand card — inspect opens; release Alt — inspect closes.
3. **Drag a playable hand card** above the bar — after commit the hand no longer shows a duplicate tile while the flight plays.
4. **Deck builder hover** — move across two pool cards; preview art changes; no native title tooltip.
5. **Lobby with a pre-picked deck** (`/play?deck=…`) — shown deck matches the pick (select value today; Bring text/card once that UX lands).
