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
- Cold start: `DATABASE_URL="sqlite::memory:" cargo run -p server` + `cd client && bun run dev`.
- Confirm the API is up: `curl -s localhost:8080/health/live`. Every game/auth/decks/cards route is gRPC now (wire-protocol-and-visibility spec) — there's no `/openapi.json` or REST path to curl directly; drive it through the BFF's `/api/rpc` (below) or a gRPC client against `:50051`.

## Seating a 2-player game via the BFF (no UI needed)

The client talks to the BFF at `client/src/routes/api/rpc/[...path].ts`, which dials tonic. Drive the same calls with `curl` against the BFF (`localhost:3000` in dev) rather than the API directly — cookies still carry the session (`-c jar.txt` on signup, `-b jar.txt` after). See `client/src/wire/rpcs.ts` for the RPC names/shapes, or use a gRPC client (e.g. `grpcurl`) straight against `:50051` with `x-session-token` metadata (see `crates/server/src/grpc/auth_ctx.rs`).

1. Sign up per player (fresh throwaway emails — the dev DB persists).
2. List decks — precons have negative ids (-1 Silverquill … -5 Quandrix, -6 Enchantress Rubinia, -7 Deathdancer Xira); usable by anyone, no deck building.
3. Seed a table (`Tables.Seed` / the BFF's seed RPC) with both seats' user id + deck id.

## Reading state / driving intents

- State: the first frame of `Game.Stream` is a full snapshot for that caller's seat — take the first frame where `frame == "snapshot"`.
- Intents: `Game.SubmitIntent` with `{"table_id","client_seq":<int, monotonic>,"intent":{...}}`. Useful kinds: `take_action {player,id}` (ids from `state.actions`), `pass_priority {player}`, `discard {player,cards}`, `arrange_top {player,top,bottom}` (answers scry).
- `scratchpad/drive.py` pattern from past runs: loop { answer pending_choice (discard/scry), play a land if offered, else pass } until the state you want. Precon games hit real choices (cleanup discards, scry lands) — handle or the loop wedges.
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
