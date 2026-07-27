# Session Resilience UX Design

**Status:** Design note (as of 2026-07-25)
**Module:** `client/lib/lobby/client.ts`, `client/app/shell/lobby/**`, `client/app/board/view.ts`, `client/app/game/**`

---

## Problem Statement

Session and table failures need player-facing copy that distinguishes stale table links, transient stream disconnects, expired sessions, and missing tables. This design is client-only and does not add durable server resume.

---

## Design

- Lobby client helpers parse structured JSON bodies from non-2xx responses. A 404 `UnknownTable` lobby body is treated as a lobby view error, not an unreachable network failure.
- Lobby `UnknownTable` renders as stale-link copy: the table link is stale or expired and the player should ask the host for a new code.
- Seated lobby watchers who do not claim a seat see `lobby-watch-note`, telling them to stay on the link and that they will enter spectator view when the host starts.
- Board reconnect chrome keeps one banner (`board-reconnecting`) and chooses copy from stream state: transient disconnects reconnect, 401 asks the player to sign in again, and 404 says the table is no longer available.

---

## Testing

- Lobby client tests cover non-2xx JSON body preservation.
- Lobby Scene tests cover stale-link copy and watcher guidance.
- Board chrome tests cover transient, 401, and 404 reconnect banner text.
- Stream tests cover terminal status propagation into Foldkit messages.

---

## Out of Scope

- Durable server resume or persisted game recovery.
- New routes or query-string table identifiers.
- Spectator route changes.
