# Table Audio
**Status:** Current (as of 2026-07-26)
**Module:** `client/app/domain/tableAudio.ts`, `client/app/board/html/audio-mount.ts`, `client/app/board/html/sound-chrome.ts`, `client/app/board/view.ts`, `client/app/shell/lobby/update.ts`, `client/app/game/fold.ts`

## Problem Statement

The board needs lightweight attention and table-feel cues without shipping audio assets, and browser autoplay rules require unlock to happen during a user gesture.

## Solution

Use synthesized Web Audio cues from a shared `AudioContext`. Unlock the context synchronously on lobby Ready and again when Sound is toggled on as recovery. Persist mute preference in `localStorage` under `mtgfr.sound`.

## User Stories

- As a player, I hear a cue when it becomes my turn or I gain priority.
- As a player, I hear soft table-feel cues for land, stack, resolve, and damage events.
- As a player, I hear distinct battlefield destroy and exile cues when those exits happen without reduced motion enabled.
- As a player, Ready-up unlocks audio so I do not need to press Sound after the game starts.
- As a player, turning Sound back on can recover a suspended context and plays a confirmation tick when possible.

## Behavior

- Cue set: `playAttentionPriority`, `playAttentionYourTurn`, `playTableFeelLand`, `playTableFeelStack`, `playTableFeelResolve`, `playTableFeelDamage`, `playTableFeelDestroy`, `playTableFeelExile`, and `playUnmuteTick`.
- All cues are synthesized tones; there are no samples, voiceover, or music files.
- `MountBoardAudio` observes board `data-*` attributes for game sequence, viewer, active player, priority, attention eligibility, and table-feel flags including `data-feel-land`, `data-feel-stack`, `data-feel-resolve`, `data-feel-damage`, `data-feel-destroy`, and `data-feel-exile`.
- Table-feel cues fire once per kind per delta batch.
- `tableFeel.destroy` and `tableFeel.exile` are one-shot flags derived from battlefield exit provenance, so only battlefield -> graveyard and battlefield -> exile deltas trigger those cues.
- `playTableFeelDestroy` and `playTableFeelExile` are skipped when `prefers-reduced-motion: reduce` is active; land, stack, resolve, damage, and attention cues keep their normal audio behavior.
- Turn cue wins over priority cue when both arrive in the same update.
- Muted or suspended contexts no-op silently.
- Sound is enabled by default unless `mtgfr.sound` is `"0"`.

## Implementation Decisions

- `unlockTableAudio()` creates/resumes the shared context and intentionally swallows resume failures.
- Lobby Ready calls unlock synchronously in the click/update path before async ready work.
- `SoundToggled` on calls `unlockTableAudio()` and then `playUnmuteTick()`; Sound off only updates preference.
- `game/fold.ts` derives `tableFeel.destroy` / `tableFeel.exile` from `provenance.battlefieldExits`, and `board/view.ts` exports them as `data-feel-destroy` / `data-feel-exile` on the board audio mount.
- `MountBoardAudio` reads reduced motion only around destroy / exile playback, so those two cues alone are suppressed for reduced-motion users.
- Board audio is mounted on its own hidden DOM node so it does not collide with keyboard or hint mounts.

## Testing Decisions

- `client/app/domain/tableAudio.test.ts` uses reset/test helpers and stub `AudioContext` behavior to cover destroy / exile cue playback and mute/unlock paths.
- `client/app/domain/event-fold.test.ts` and `client/app/game/fold.test.ts` cover battlefield exit provenance and the `tableFeel.destroy` / `tableFeel.exile` flags that drive the board `data-*` attributes.
- Lobby update tests assert Ready invokes unlock.
- Board sound tests assert Sound-on invokes unlock and confirmation tick, and Sound-off does not.
- Manual checks should verify Ready -> start -> land/priority produces audible cues without pressing Sound, and that destroy / exile cues stay silent only under reduced motion.

## Out of Scope

- Sample files, Howler, music, or per-card unique sounds.
- Unlocking on every board pointerdown.
- Error toasts for autoplay or resume failures.

## Further Notes

- The Sound toggle is a mute/recovery control, not the primary happy-path unlock.
