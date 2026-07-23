# Table Audio
**Status:** Current (as of 2026-07-23)
**Module:** `client/lib/tableAudio.ts`, `client/app/board/html/audio-mount.ts`, `client/app/board/html/sound-chrome.ts`, `client/app/shell/lobby/update.ts`, `client/app/game/fold.ts`

## Problem Statement

The board needs lightweight attention and table-feel cues without shipping audio assets, and browser autoplay rules require unlock to happen during a user gesture.

## Solution

Use synthesized Web Audio cues from a shared `AudioContext`. Unlock the context synchronously on lobby Ready and again when Sound is toggled on as recovery. Persist mute preference in `localStorage` under `mtgfr.sound`.

## User Stories

- As a player, I hear a cue when it becomes my turn or I gain priority.
- As a player, I hear soft table-feel cues for land, stack, resolve, and damage events.
- As a player, Ready-up unlocks audio so I do not need to press Sound after the game starts.
- As a player, turning Sound back on can recover a suspended context and plays a confirmation tick when possible.

## Behavior

- Cue set: `playAttentionPriority`, `playAttentionYourTurn`, `playTableFeelLand`, `playTableFeelStack`, `playTableFeelResolve`, `playTableFeelDamage`, and `playUnmuteTick`.
- All cues are synthesized tones; there are no samples, voiceover, or music files.
- `MountBoardAudio` observes board `data-*` attributes for game sequence, viewer, active player, priority, attention eligibility, and table-feel flags.
- Table-feel cues fire once per kind per delta batch.
- Turn cue wins over priority cue when both arrive in the same update.
- Muted or suspended contexts no-op silently.
- Sound is enabled by default unless `mtgfr.sound` is `"0"`.

## Implementation Decisions

- `unlockTableAudio()` creates/resumes the shared context and intentionally swallows resume failures.
- Lobby Ready calls unlock synchronously in the click/update path before async ready work.
- `SoundToggled` on calls `unlockTableAudio()` and then `playUnmuteTick()`; Sound off only updates preference.
- Board audio is mounted on its own hidden DOM node so it does not collide with keyboard or hint mounts.

## Testing Decisions

- Table audio tests use reset/test helpers and stub `AudioContext` behavior.
- Lobby update tests assert Ready invokes unlock.
- Board sound tests assert Sound-on invokes unlock and confirmation tick, and Sound-off does not.
- Manual checks should verify Ready → start → land/priority produces audible cues without pressing Sound.

## Out of Scope

- Sample files, Howler, music, or per-card unique sounds.
- Unlocking on every board pointerdown.
- Error toasts for autoplay or resume failures.

## Further Notes

- The Sound toggle is a mute/recovery control, not the primary happy-path unlock.
