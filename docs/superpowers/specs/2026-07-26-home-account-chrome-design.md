# Home Account Chrome Design

**Status:** Implemented
**Date:** 2026-07-26
**Surface:** Home deck list (`/`), leaderboard (`/leaderboard`) account chrome

## Goal

Simplify the home header: drop the Top players teaser, put leaderboard behind a header control, move New deck into the deck grid as a create tile, and tuck Sign out / Gravatar under an avatar menu with the username as the menu title.

## Decisions

| Topic | Choice |
|---|---|
| New deck affordance | First grid tile, same footprint as deck tiles, dashed/empty create treatment (not a filled primary CTA) |
| Leaderboard entry from home | Ghost/link in the right account cluster (not near the title, not only inside the avatar menu) |
| Leaderboard page chrome | Same account avatar menu as home; keep Play → `/` |
| Username placement | Inside the avatar menu as a non-action title (not beside the avatar in the header) |
| Home teaser | Remove UI **and** the top-5 teaser fetch/state |
| Shared vs global nav | Shared account-chrome helper for home + leaderboard; **no** persistent authenticated app header |

## Design

### Home header (`deck-list-header`)

Left: “Your decks”.

Right account cluster:

1. **Leaderboard** — navigates to `/leaderboard`
2. **Avatar** — `seatFace` / `seat-face-0`; button-like control (accessible name e.g. “Account”); toggles the account menu

Removed from the header: inline username, “Change at Gravatar”, Sign out, New deck.

### Avatar menu

Anchored near the avatar. Contents top → bottom:

1. **Username** — title/label only (not a link)
2. **Change at Gravatar** — outbound `https://gravatar.com` (`account-gravatar-link`, `target=_blank`, `rel=noopener noreferrer`)
3. **Sign out** — existing `RequestedLogout`

Dismiss: click outside / catcher (match deck-list context menu). Opening the account menu closes an open deck context menu; opening a deck context menu closes the account menu.

### New deck create tile

- First cell in `deck-list-grid`, same `minmax(220px, 1fr)` track as deck tiles.
- Dashed/empty create treatment: plus affordance + “New deck” label (no commander art, no Play label, no FLIP morph, no context menu).
- Whole tile links to `/decks/new` (`data-testid="deck-list-new-deck"`).
- Not part of the deck list model; not affected by search.
- After load, shown whenever the grid renders — including **zero decks** (create tile alone; drop the old “No decks yet — build one to get started.” empty copy). While `loading`, keep “Loading decks…” without the create tile.
- Search field still appears only when there is at least one deck (unchanged). Search filters deck tiles only; the create tile stays first. If search matches nothing, keep the create tile first and retain the existing “No decks match.” copy.

### Home Top players teaser — removed

Remove:

- Teaser view (`leaderboard-teaser`, `leaderboard-teaser-link`)
- `leaderboardTeaser` on the deck-list submodel
- `FetchDeckListLeaderboardTeaser` and success/failure messages
- Home entry path that loads top-5 alongside decks
- Related Stories / Scene expectations

Full standings remain on `/leaderboard` only.

### Leaderboard header

- Keep page title and **Play** → `/`.
- Replace standalone Sign out with the shared account cluster: avatar + menu (username title, Gravatar, Sign out).
- Omit the Leaderboard link on this page (already there).

### Shared helper

Extract a small view helper (e.g. `client/app/shell/account-chrome.ts`) used by home and leaderboard:

- Inputs: `username`, `gravatarHash`, `menuOpen`, whether to show the Leaderboard link, message constructors for toggle/dismiss/logout.
- Not a persistent shell nav; lobby/builder/auth stay unchanged.

### State

- `accountMenuOpen: boolean` on the deck-list and leaderboard submodels.
- Session `me` / `meGravatarHash` unchanged (hash still from `HashMeGravatar` after `ReceivedMe`).

## Living specs to update (implementation)

This design is input only. The same implementation change must update:

- [deck-list-and-builder](2026-07-20-deck-list-and-builder.md) — header, create tile, teaser removal, testing notes
- [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md) — home teaser fetch / leaderboard header chrome / Scene notes as needed

## Non-goals

- Persistent global nav or brand header on lobby/builder
- Leaderboard ranking or ratings API changes
- Gravatar hashing / `seatFace` visual redesign beyond trigger + menu
- Changing deck tile Play/FLIP behavior or search matching rules (beyond create-tile exceptions above)
- In-app avatar upload

## Tests

Scene / Stories assert product outcomes:

- Home has no `leaderboard-teaser`; header has Leaderboard link + avatar; no header New deck / Sign out / inline Gravatar
- Avatar open → menu shows username title, `account-gravatar-link`, Sign out; Sign out still logs out
- Grid first child is `deck-list-new-deck` → `/decks/new`; present with zero decks after load
- Leaderboard keeps Play; no duplicate Sign out button; account menu matches home
- Route/entry Stories: home load no longer expects a teaser fetch

## Follow-ups (deferred)

- Promoting account chrome into a true authenticated shell header across more routes
- Optional current-page styling if Leaderboard is ever shown while already on `/leaderboard`
