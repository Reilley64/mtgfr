# Foldkit hand dim retirement + table audio unlock

**Status:** Done  
**Date:** 2026-07-22  
**Plan:** [`docs/superpowers/plans/2026-07-22-foldkit-hand-dim-and-table-audio.md`](../plans/2026-07-22-foldkit-hand-dim-and-table-audio.md)  
**PR:** [#74](https://github.com/Reilley64/mtgfr/pull/74) (`cursor/foldkit-migration-design-1ef0`)  
**Related:** [`2026-07-22-foldkit-devtools-and-playable-chrome-design.md`](./2026-07-22-foldkit-devtools-and-playable-chrome-design.md) (playable borders; dim veil retired), [`2026-07-20-client-game-board-and-interaction.md`](./2026-07-20-client-game-board-and-interaction.md) (table audio)

## Goal

Close two Foldkit playable-chrome gaps vs the intended Arena language and Solid table audio:

1. **Hand/command:** Unplayable cards must not use a darkening veil; castability is the playable border only.
2. **Table audio:** After a normal lobby Ready → start → play path, synth cues must be audible without touching the Sound toggle. The Sound toggle remains a recovery unlock + mute control.

## Problem

### Hand dimming

`client/app/board/html/hand.ts` still applies `brightness-[0.55]` when `dimmed` is true. For hand/command, `dimmed` is set when there is no legal bar action (and also when the slot is staged/in-flight). Playable-chrome already added Arena borders and retired the unplayable veil in DESIGN.md / the playable-chrome spec, but the class was never removed.

### Table audio silent

Foldkit already ports Solid’s synthesized cues (`client/lib/tableAudio.ts`) and board mounting (`MountBoardAudio` + `fold.tableFeel` data attributes). Playback requires `AudioContext.state === "running"` (`canPlay()`). Unlock is only attempted from lobby Ready (`ReadyLobby` → `unlockTableAudio()`). That unlock currently runs inside a Foldkit `Command` Effect; if resume runs after the click gesture stack ends, browsers keep the context suspended and every cue no-ops. The in-game Sound toggle only flips `mtgfr.sound` / `soundOn` and never calls `unlockTableAudio()`, so it cannot recover a suspended context. Happy path must not depend on the Sound button (players may never press it).

## Approach

**Surgical parity (chosen):** remove the unplayable hand/command veil; keep intentional drag/staging fades; fix AudioContext unlock on Ready (gesture-safe) and on Sound-on as recovery; keep the existing six synth cues and feel/attention wiring. Rejected: board-wide pointerdown unlock spam; sample/Howler assets; leaving the hand veil in place.

## Workstream 1 — Hand / command dimming

### Visual

- Do **not** apply `brightness-[0.55]` (or equivalent) because a hand/command card has no legal bar action.
- Keep Arena playable borders via `barZoneAura(zone, playable)`.
- Keep drag-source `opacity-25` when an in-flight hand drag ghost is active.
- Keep flight/staging removal via `hiddenIds` / `hiddenId` (tile absent or non-interactive) — not a substitute unplayable veil.

### Interaction flag split

Today `dimmed` overloads “unplayable → darken” and “staged/in-flight → not interactive” (`playable = action != null && !dimmed`).

- Treat “staged / in-flight” as the only reason a present tile with an action becomes non-playable.
- Unplayable (no action) tiles: full brightness, no playable ring, no grab/activate — borders absent, not darkened.
- Rename or locally clarify the flag in `hand.ts` if it keeps the implementation honest (e.g. only pass a “slot inert” signal for staging/flight); do not keep a visual dim path for “no legal action.”

Graveyard / exile sections already use `dimmed: false`; leave them as non-dimmed unless a regression appears.

## Workstream 2 — Table audio unlock

### Keep

- Cue set: attention priority, attention your-turn, table-feel land / stack / resolve / damage (`tableAudio.ts`).
- Board observation: `MountBoardAudio` reading `data-game-seq`, feel flags, attention fields from `board/view.ts`.
- Feel batching: `client/app/game/fold.ts` → `tableFeel`.
- Pref: `localStorage` `mtgfr.sound`; mute still gates `canPlay()`.

### Unlock rules

1. **Lobby Ready (happy path):** Call `unlockTableAudio()` **synchronously** on the Ready click / update path *before* any async `readyUp` work, so `AudioContext.resume()` rides the user gesture. Keep the existing ReadyLobby unlock call as defense in depth, but do not rely on Effect scheduling alone.
2. **Sound toggle (recovery):** On `SoundToggled` when turning sound **on**, call `unlockTableAudio()` in that click’s update handler, then play a short confirmation tick (new small helper or reuse a soft tone) so unmute is verifiable. Turning **off** only updates the pref (no cue).
3. **Failures stay silent:** If resume fails or the context is not `running`, cues no-op — no error toast.

### Non-goals (audio)

- No new sample files / Howler.
- No unlock on every board pointerdown (Approach 2 rejected).
- No change to when feel/attention flags fire, beyond unlock making those plays audible.

## Testing

### Automated

- Hand HTML/unit: unplayable hand/command tiles must **not** include `brightness-[0.55]`; playable tiles still get the playable border class; drag-source fade still present when `draggingActionId` matches.
- Sound update: `SoundToggled` on → unlock invoked (and confirmation tick only when `canPlay()` allows).
- Ready path: Ready click/update invokes `unlockTableAudio` synchronously (mock/`resetTableAudioForTests`).
- Prefer a focused `tableAudio` test for unlock/resume behavior with a stub `AudioContext` if practical.

### Manual

- Ready → start → play a land and/or gain priority: hear cues **without** pressing Sound.
- Then mute / unmute: unmute plays the confirmation tick; mute silences further cues.

## Out of scope

- Battlefield `DIM_CARD_VEIL` / `options.dim` dead API cleanup (optional follow-up; no callers pass `dim: true` today).
- Broader audio redesign, music, or voice.
- Reintroducing unplayable hand darkening under any name.

## Success criteria

- Unplayable hand/command cards are full brightness; only playable border + interaction affordances distinguish castability.
- Normal Ready → game path produces audible table audio without using the Sound control.
- Sound-on recovers a suspended context and gives an audible confirmation when unmuted.
