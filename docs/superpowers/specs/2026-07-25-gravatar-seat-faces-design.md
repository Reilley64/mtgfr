# Gravatar Seat Faces Design

**Status:** Accepted
**Date:** 2026-07-25
**Surface:** Board life orbs (avatars), lobby seats, account chrome

## Goal

Make seats easier to tell apart at a glance and make usernames feel like people — without in-app image uploads in v1.

## Design

### Board layout

The life orb circle becomes a **circular face**. Life moves **below** the circle. Layout (top → bottom):

1. `Hand N` above the circle (unchanged)
2. Circle: Gravatar image, clipped; rim keeps priority gold / seat stroke and target rings
3. **Life** total directly below the circle (primary number)
4. Username under life
5. `Cmd N` under username when max commander damage > 0

Lost seats keep muted treatment (desaturated/muted face + existing muted fill behavior on fallback).

HTML hit targets remain on the circle (`life-orb-{seat}`); combat / player-aim drops are unchanged.

This layout is **always** used — there is no dual “old solid orb vs photo orb” mode.

### Identity resolution (v1)

- No uploadable avatars in v1.
- Face = Gravatar for the seat’s account email.
- Hash: trim + lowercase email → SHA-256 hex (Gravatar current algorithm).
- Wire exposes only a public avatar reference on seat/player views — **`avatar_url` or `gravatar_hash`**. Never other players’ emails.
- Recommended URL: `https://www.gravatar.com/avatar/{hash}?s=128&d=404` so a miss can fail closed to monogram.
- Fallback (no email, hash absent, or image load/`d=404` failure): seat-color fill + monogram (first letter of username, or `P{n}` when username empty).
- Local/dev/bot seats without email use the monogram path.

### Profile / account chrome

- `Me` / auth and account-facing chrome show the same face.
- Short copy + outbound link to change the image at gravatar.com. No in-app crop/upload UI.

### Client paint

- Load faces through the existing `sharedImageCache` pattern used for card art.
- Mount `paintAvatars` remains the authoritative visible avatar chrome; keep `canvas/avatars.ts` `avatarShapes` in sync for the vector helper.
- Resting paint keys include avatar identity so a late image load triggers Mount repaint.
- Lobby seat labels and board life orbs share one face helper (URL/hash → image or monogram).

### Server / projection

- When projecting `PlayerView` and lobby seat payloads, resolve the bound user email → gravatar hash/URL at the edge.
- Email remains auth-private (`Me` only); visibility filtering must not add email to game/lobby player DTOs.

## Non-goals

- Uploadable / custom CDN avatars
- Moderation tooling
- Changing Gravatar from inside the app
- Replacing username labels
- Moving life into an HTML overlay (stays bitmap/canvas with the table)

## Tests

- Unit: email → SHA-256 gravatar hash; URL builder (`s`, `d=404`).
- Unit/paint: life below circle; hand above; username / `Cmd N` stacking; monogram fallback on missing/failed image.
- Projection/wire: `PlayerView` carries avatar ref and never email.
- Scene: life-orb hit targets and player-aim / combat drops still work with the new chrome.
- Shell: account/auth chrome shows face + gravatar.com affordance.

## Follow-ups (explicitly deferred)

- Optional upload override (upload if set, else Gravatar, else monogram) if friends still want custom photos after Gravatar ships.
- Commander-art accent ring as a secondary seat cue.
