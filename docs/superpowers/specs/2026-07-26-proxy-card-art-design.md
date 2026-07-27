# Proxy Card Art Design

**Status:** Accepted  
**Date:** 2026-07-26  
**Surfaces:** Deck builder (`client/app/shell/decks/builder/**`), card art helpers (`client/app/domain/deck-builder/scryfall.ts`, `client/app/domain/ui/card-art.ts`, `client/app/domain/image-cache.ts`), Nitro BFF proxy route, deck/catalog proto + seed overlay (`proto/mtgfr/v1/catalog.proto`, `crates/schema`, `crates/server` lobby/seed/stream)  
**Living module specs (update in the same implementation change):** [deck-list-and-builder](2026-07-20-deck-list-and-builder.md), [accounts-decks-and-catalog](2026-07-20-accounts-decks-and-catalog.md), [wire-protocol-and-visibility](2026-07-20-wire-protocol-and-visibility.md); touch board art choke-point notes in [battlefield](2026-07-20-battlefield.md) / [hand-and-zone-bar](2026-07-20-hand-and-zone-bar.md) / [stack](2026-07-20-stack.md) / [card-inspect](2026-07-20-card-inspect.md) only as needed for the shared helper contract

## Goal

Let a player paste an `https` image URL on a real catalog card in the deck builder so that image is used as alter art for that card whenever the deck is played. Rules identity stays the catalog card; printing preference remains for legality and as the fallback face. Everyone at the table sees the same alter. The browser never loads arbitrary remote hosts for board art — the Nitro BFF proxies the fetch. If the link fails (or the image cannot be decoded), art falls back to the card’s Scryfall printing.

## Decisions

| Topic | Choice |
|-------|--------|
| Purpose | Alter art on a real catalog card only (not a stand-in for missing pool cards) |
| Where set | Deck builder; persisted on the deck |
| Who sees it | All seats at the table |
| Relation to printing | Override: keep `print`; when `proxy_art_url` is set, display uses the proxy (via BFF) first |
| Image load | Lazy BFF proxy at display time (not eager cache-on-save) |
| Failure | Fall back to `imageUrlByPrint(print)` |
| Builder UX | Separate context-menu item **Set proxy art…** + small dialog (printing picker unchanged) |

## Design

### Data model & persistence

- `DeckCardEntry` gains optional `proxy_art_url` (empty / absent = no alter).
- Commander gains optional `commander_proxy_art_url` beside `commander_print`.
- Postgres continues to store `cards` as a JSON blob of entries; `commander_proxy_art_url` follows the same storage pattern as `commander_print`.
- Commander legality and print UUID validation are unchanged. Proxy URL is display-only and is not part of legality.
- Deck-save checks when a URL is present: scheme must be `https`, max length 2048 characters, no credentials in the URL. The server does **not** fetch the image on deck save.
- Precons ship with no proxy URLs.
- Granularity matches today’s deck line: one URL per oracle id line (same as one `print` per line). Basics with `count > 1` share one proxy URL for that line.

### Wire & in-game visibility

- On table seed/start, build a per-seat `card_id → proxy_art_url` map next to today’s `prints` map (omit empty entries).
- Schema snapshot overlay applies the URL onto visible objects the same way `print` is applied — optional `proxy_art_url` on `ObjectView` and on choice/stack (and similar) surfaces that already carry `print` for art.
- The engine remains print- and URL-agnostic.
- Client art resolution order for face-up cards:
  1. If `proxy_art_url` is present → same-origin BFF proxy URL derived from it
  2. On proxy HTTP failure or image decode failure → `imageUrlByPrint(print)`
  3. Face-down / library back unchanged (`card-back.webp`)
- Double-faced cards (v1): proxy applies to the **front** face only; the back continues to use the printing.

### BFF image proxy & fallback

- Nitro route `GET /api/card-art/proxy?url=…` (URL-encoded target) used whenever a proxy URL is set for display. Same-origin only; require an authenticated session if other authenticated BFF routes already do for comparable abuse posture — do not invent a public unauthenticated open relay.
- Fetch rules: `https` only; reject non-http(s) schemes, userinfo/credentials, and private / link-local / metadata hosts (SSRF); short timeout; max response size; require `image/*` (or sniff an allowlist: jpeg, png, webp, gif).
- Response: pass through with short `Cache-Control` so four clients do not re-hammer the origin every frame; do not forward the player’s auth cookies to the remote host.
- Failure modes (non-2xx, timeout, bad type/size, client decode error) clear the effective proxy for that paint path and use the printing — no sticky broken-image state for alters.
- CSP: keep gameplay `img-src` on same-origin (proxy) plus existing CDN / Scryfall hosts; do **not** open `img-src` to the open internet.
- v1 abuse posture: no persistent disk cache; rely on HTTP cache headers and `sharedImageCache`. Modest rate-limiting is welcome if cheap; otherwise a follow-up.

### Deck builder UX

- Card context menu (right-click / long-press) gains **Set proxy art…** for editable deck context. **Choose printing…** remains for Scryfall printings.
- Dialog: card name, URL field, **Save** / **Clear** / close.
- **Save** / **Clear** update the in-builder deck model (same dirty/persist cadence as changing a printing): the URL is written onto that deck line, or onto `commander_proxy_art_url` when editing the commander, and persists when the player saves the deck. Client-side shape validation mirrors deck-save rules; image fetch still happens only at display via the BFF.
- **Clear** removes the proxy URL from the builder model immediately (builder preview returns to printing art); the cleared state persists when the player saves the deck.
- Quiet indicator when a proxy is set (e.g. small “Proxy” chip on the row / commander chrome). Builder thumbnails use the same proxy → fallback path as the board when practical.
- Print-picker dialog behavior is unchanged aside from coexisting with the override.

## Non-goals (v1)

- Uploading or hosting alter binaries on our CDN
- Eager validate-and-cache on deck save
- Per-physical-copy URLs when `count > 1`
- DFC back-face proxy URLs
- In-game / session-only overrides
- Account-wide alter preferences
- Opening CSP `img-src` to arbitrary remote hosts
- Proxy stand-ins for cards missing from the pool

## Tests

- **Server / schema:** deck save round-trip with and without proxy URL; seed overlays `proxy_art_url` onto visible objects for all seats; legality ignores the URL; invalid URL shape rejected on save.
- **BFF proxy:** allow a good `https` image response; reject `http`, private hosts, oversized bodies, and non-images; errors do not leak internals.
- **Client:** builder Scene — menu item, dialog save/clear, proxy indicator; shared art helper prefers proxied URL then falls back to print on failure; board / HTML faces go through that helper (unit or focused test at the choke point). Follow [client interaction test policy](2026-07-22-client-interaction-test-policy-design.md): assert outcomes (URL applied, cleared, fallback), not only control presence.

## Implementation note

Update the living module specs listed above in the same PR that lands the behavior so Behavior / Implementation / Testing describe what ships. This design doc is input; it does not replace those surface specs.
