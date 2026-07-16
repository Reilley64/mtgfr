---
name: verify
description: Drive a live two-player mtgfr game end-to-end to verify engine/server/client changes at the real surface (browser + HTTP API).
---

# Verifying mtgfr changes live

## Handles

- **Dev loop is usually already running**: `just dev` = `bacon server` (auto-rebuilds+restarts `target/debug/server serve` on :8080 on source change) + vite on :5173. Check `lsof -nP -i :8080` — if `server`'s parent is `bacon server`, the running binary already has your changes (bacon restarted it after your last build). Don't start a second server; listen addr comes from `Settings` (`config/mtgfr.toml` / env).
- Cold start: `DATABASE_URL="sqlite::memory:" cargo run -p server` + `cd client && bun run dev`.
- Confirm a new route is live: `curl -s localhost:8080/openapi.json | grep <path>` (401 = live-but-auth, 404 = not registered).

## Seating a 2-player game via curl (no UI needed)

All POST bodies are JSON; cookies carry the session (`-c jar.txt` on signup, `-b jar.txt` after).

1. `POST /auth/signup/v1` `{"email","password"}` per player (fresh throwaway emails — the dev DB persists).
2. `GET /decks/v1` — precons have negative ids (-1 Silverquill … -5 Quandrix); usable by anyone, no deck building.
3. `POST /tables/v1` (host) → `table_id`; `POST /tables/join/v1` `{table_id, deck_id}` per player; `POST /tables/ready/v1` `{table_id, ready:true}` per player; `POST /tables/start/v1` `{table_id}` (host).

## Reading state / driving intents

- State: first SSE frame of `GET /tables/{table}/stream/v1` is a full snapshot for that cookie's seat: `curl -sN --max-time 1 -b jar.txt ...` and take the first `data:` line where `frame == "snapshot"`. Keep `--max-time` at 1s — curl otherwise sits open the full timeout.
- Intents: `POST /intent/v1` with `{"table_id","player_id","client_seq":<int, monotonic>,"intent":{...}}`. Useful kinds: `take_action {player,id}` (ids from `state.actions`), `pass_priority {player}`, `discard {player,cards}`, `arrange_top {player,top,bottom}` (answers scry).
- `scratchpad/drive.py` pattern from past runs: loop { answer pending_choice (discard/scry), play a land if offered, else pass } until the state you want. Precon games hit real choices (cleanup discards, scry lands) — handle or the loop wedges.
- Per-stack yield: `POST /yield/v1` `{table_id, player_id, enabled}`.

## Watching in the browser

- agent-browser: log in at `localhost:5173/login`, then open `/play/ID`. The `player` URL param is display-only; the server resolves the seat from the session cookie.
- agent-browser saves screenshots relative to its own cwd (often the repo root) — move them out of the repo afterward.
- Timing-sensitive UI (the ~2s stack hold): `agent-browser eval "<js click>"` beats snapshot→click round-trips.

## Gotchas

- Auto-advance means turns fly: a player with no meaningful action is passed instantly, so "wait a step" states are hard to park on. Anything on the stack holds ~2s (`STACK_HOLD`) before auto-resolving — that's the window to screenshot.
- Bare `engine::Game::new()` tables die fast under auto-advance (empty libraries → draw-out deaths). Seed real decks.
- Test tables/users linger in the dev DB and in-memory registry until game over; use recognizable throwaway names.
