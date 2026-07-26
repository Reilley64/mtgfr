# Commander damage lobby toggle — design

**Status:** Approved (2026-07-26)
**Living surface specs:**
[`2026-07-20-lobby-entry-ui.md`](2026-07-20-lobby-entry-ui.md),
[`2026-07-20-lobby-table-routing-and-live-game.md`](2026-07-20-lobby-table-routing-and-live-game.md),
[`2026-07-20-combat-and-commander-rules.md`](2026-07-20-combat-and-commander-rules.md),
[`2026-07-20-battlefield.md`](2026-07-20-battlefield.md),
[`2026-07-20-card-inspect.md`](2026-07-20-card-inspect.md),
[`2026-07-20-wire-protocol-and-visibility.md`](2026-07-20-wire-protocol-and-visibility.md)

## Problem

Commander damage (CR 903.10a: lose when dealt 21 or more combat damage from a
single commander) is always on. Some tables want a house rule that ignores that
clock. There is no host-facing way to disable it before Start on the seat claim
screen, and no game-options path through lobby → seed → engine.

## Goal

Let the **host** turn **commander damage** on or off on the seated lobby (seat
claim) screen before the game starts. When off: the engine does not track
commander damage, does not lose players at 21, and the board hides all commander
damage chrome. Starting life stays **40**. Default remains **on** (standard EDH).

## Non-goals

- Changing starting life, commander tax, command zone, or Partner.
- Mid-game toggle after Seed.
- A general `GameOptions` bag for multiple house rules (add when a second option
  lands).
- Clearing Ready when the host flips the setting.
- Non-host control of the setting.

## Approach

**Single boolean end-to-end** (rejected: premature `GameOptions` bag; rejected:
client-local-until-Start — refresh loses the value and other players cannot see
it before Start).

### Product rules

| Rule | Behavior |
|------|----------|
| Default | `commander_damage_enabled = true` |
| Off semantics | No tallies, no lose-at-21, hide CMDMG UI; life stays 40 |
| Who toggles | Host only |
| Who sees | Everyone on the table link (seated + watchers) |
| When | Anytime before Start; locked after Seed |
| Ready | Unchanged when the host flips the setting |

### Data flow

```
Host switch (seated lobby)
  → POST /api/tables/options/v1 { table_id, commander_damage_enabled }
  → lobbies.commander_damage_enabled (bool NOT NULL DEFAULT true)
  → GET …/lobby/v1 includes the flag (all clients via poll)
  → Host Start → optional SeedRequest.commander_damage_enabled (absence = true)
  → Engine Game stores the flag at seed (immutable for the match)
  → Snapshot exposes commander_damage_enabled
  → Board gates Cmd N + inspect panel on that flag
```

### Lobby UI (seat claim / seated lobby)

- **Placement:** Options card **above the seat list**, below table-code chrome.
- **Copy:** Title `Commander damage`; helper `Lose at 21 from one commander`.
- **Control:** Switch. Host can flip it. Guests and watchers see the **same
  switch, disabled**, reflecting the polled value.
- **Test ids:** `lobby-commander-damage`, `lobby-commander-damage-switch`
  (disabled / aria when not host).
- Failed write: keep prior value; show short error in existing lobby error chrome.

### BFF / Postgres

- Add `commander_damage_enabled` boolean NOT NULL DEFAULT true on `lobbies`.
- New host-only options route: `POST /api/tables/options/v1`. Errors:
  `NotHost`, `AlreadyStarted`, `UnknownTable`.
- Start reads the column into `SeedRequest`. After Seed succeeds, the start
  commit writes `startedAt` and overwrites `lobbies.commander_damage_enabled`
  with the exact seeded value; after `startedAt` is set, options writes fail
  with `AlreadyStarted`.

### Wire / seed / snapshot

- Proto: optional `SeedRequest.commander_damage_enabled` (bool); absence means
  enabled.
- Schema / tonic mapping: carry the field through Seed.
- Snapshot: add `commander_damage_enabled` on `VisibleState` (proto + schema
  projection) so the client does not infer from empty tallies.

### Engine

- `Game` stores `commander_damage_enabled: bool`, default `true` for
  `Game::new` / `with_players` / existing tests.
- When `false`: combat never emits `CommanderDamageDealt`; player tallies stay
  empty; SBA never loses on the 21 check. Commander designation still sets life
  to 40; tax and command zone unchanged.
- When `true`: today’s behavior.

### In-game client

- When the flag is false: omit `Cmd N` on avatars; omit the commander-damage
  block in Alt life-orb inspect. Gate on the snapshot flag, not on empty rows
  alone.

## Testing

- **Engine:** flag off + ≥21 combat damage from a commander → life drops, no
  `CommanderDamageDealt`, player does not lose to 21. Flag on → existing
  twenty-one-lose test still passes. Default construction remains enabled.
- **BFF:** create defaults to enabled; host can flip; non-host → `NotHost`;
  after start → `AlreadyStarted`; lobby GET and Seed carry the value.
- **Client Scene / unit:** options card + switch on seated lobby; host toggles;
  non-host switch disabled; poll reflects value. Board: flag false → no `Cmd N`,
  inspect omits commander-damage panel.

## Living specs

Behavior, implementation, and testing details live in the surface specs listed
in the header. This design records the accepted shape of the feature; the
surface specs remain the shipped-behavior source of truth.

## Out of scope

Other house rules, life-total picker, Partner multi-commander tracking, mid-game
options changes, and client-only cosmetic toggles that leave engine lose-at-21
active.
