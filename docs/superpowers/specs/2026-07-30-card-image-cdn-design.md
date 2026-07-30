# Card image CDN (design)

**Status:** Approved design input (2026-07-30).
**Surfaces:** New target — Cloudflare Worker + R2 bucket in `iac/`; consumes the URL layout owned by `client/app/domain/deck-builder/scryfall.ts` (`buildImageUrl`). Updates `production-topology-and-operations` §Card art CDN at implement time.

---

## Problem Statement

`VITE_CARD_CDN` points at a host serving an 86 GB bulk mirror of Scryfall card images, built and populated by a separate repository. The topology spec records that host as "managed separately," so nothing in this repo describes it, tests it, or keeps its path layout aligned with the layout `buildImageUrl` constructs. New sets need a manual bulk re-run to appear, and a layout mismatch fails quietly: `artCropFallbackUrl` degrades `art_crop` to a direct Scryfall hotlink, while `large` has no fallback at all and shows a broken tile.

## Goal

Serve card art from a CDN this repo owns and can test, populated on demand from Scryfall so new prints appear without running anything, at no recurring cost.

## Locked decisions

| Decision | Choice |
|---|---|
| Hostname | `edh-images.reilley.dev` — first-level, so free Universal SSL covers it. A deeper name like `images.edh.reilley.dev` needs paid Advanced Certificate Manager, which would cost more than everything else here combined |
| Origin | R2 bucket behind a Worker; **not** the BFF (keeps image traffic off the tunnel and out of the Nitro event loop) |
| Population | Fill-on-miss only. Bucket starts empty; no bulk upload |
| Stored format | Scryfall's bytes unchanged (JPEG). **No** Images transformations |
| Path extension | `.jpg` — `buildImageUrl` changes from `.webp`, since stored bytes are JPEG |
| Fill failure | `302` to the Scryfall image URL, store nothing, retry on next request |
| Scryfall 404 | `404` — a print that does not exist is not a transient failure |
| Cutover | Flip `VITE_CARD_CDN`; accept the cold-bucket warmup |
| Worker deploy | `cloudflare_workers_script` reading `iac/workers/card-cdn.js` via `file()`. No wrangler, no bundler |

## Approaches considered

1. **Worker + R2, fill-on-miss (chosen)** — Edge-local, no home bandwidth, no coupling to the game's request path, and R2 egress is free.
2. Fill-on-miss in the Nitro BFF — Puts 86 GB-scale image traffic through the Cloudflare Tunnel from the homelab and shares the Nitro event loop with gRPC streams; a Scryfall rate-limit stall would occupy slots the game needs. Also inverts the reason `VITE_CARD_CDN` is a separate hostname, and image availability would follow Argo rolls.
3. R2 Sippy — Cloudflare's built-in on-demand fill, but it only pulls from S3 and GCS, not an arbitrary HTTP origin, so Scryfall is out of reach.
4. Cloudflare CDN caching-proxy in front of Scryfall, no storage — Free/Pro/Business CDN terms let Cloudflare limit access for serving "a disproportionate percentage of pictures," which is precisely what a card-art-only hostname is. Also fails Scryfall's ask to store copies rather than lean on their servers.
5. Cloudflare Images storage + delivery — Costs the most at warm scale (storage $5/100k stored/month, delivery $1/100k delivered/month, against free R2 egress) and replaces the path layout with `imagedelivery.net`'s fixed `<hash>/<id>/<variant>` shape.

### Why JPEG rather than WebP

WebP needs Images transformations, whose free allowance is 5,000 unique/month. Under fill-on-miss that allowance caps **how fast the bucket fills**, not just conversion quality: once exhausted, each new print fails the transform, redirects to Scryfall, and stores nothing — so it retries and redirects again. At ~110k English prints, full coverage would be a ~2-year horizon, and a deck-builder session (`PAGE = 100`, paged) can spend a month's allowance quickly. Lifting the cap requires an Images subscription, which also removes the hard $0 ceiling. Storing Scryfall's JPEG is uncapped, free, and roughly 2× the bytes of storage that is itself nearly free.

## Design

### Components

- **R2 bucket** — keyed exactly as `buildImageUrl` builds paths: `{large|art_crop}/{front|back}/{a}/{b}/{id}.jpg`, where `a`/`b` are the first two characters of the print id.
- **Worker** (`iac/workers/card-cdn.js`) — the only reader/writer of the bucket. Self-contained plain JS with no imports, because Terraform embeds the file directly.
- **DNS + route** — proxied record for `edh-images.reilley.dev` in the existing `reilley.dev` zone, routed to the Worker.
- **Rate-limiting rule** — one rule scoped to the hostname. The free plan includes exactly one and `iac/` does not currently use it.

### Request flow

1. Reject non-`GET`/`HEAD` with `405`.
2. Match the path against the layout. Verify `a`/`b` equal the first two characters of `id`. Any mismatch → `404` **before** any outbound request.
3. `env.CARDS.get(key)`. Hit → serve the bytes with `Cache-Control: public, max-age=31536000, immutable`.
4. Miss → fetch `api.scryfall.com/cards/{id}?format=image&version={size}` (plus `&face=back` for back faces) with our User-Agent.
5. `2xx` → `put()` the bytes into R2 and serve them with the same headers.
6. Upstream `404` → `404`. Any other failure → `302` to the same Scryfall image URL.

### Path validation as a trust boundary

The endpoint writes to the bucket on unauthenticated public request, so step 2 is a security control rather than tidiness. Constraining the path to the layout also constrains the outbound URL to `api.scryfall.com/cards/{uuid}`, so the Worker cannot be used as a general-purpose proxy.

### Caching

A Worker route runs *before* the CDN cache, so Cloudflare's extension-based edge caching does not apply to this path — the Worker is consulted on every request that reaches the edge, each one counting against the free 100k/day. What suppresses repeat traffic is the year-long `immutable` browser cache, which is accurate because print ids never change.

Consequently the `.jpg` extension is not load-bearing for caching here; it is chosen so the path does not misdescribe its bytes. If the Worker is ever removed in favour of serving the bucket directly on a custom domain, the extension starts mattering, because Cloudflare then caches on **file extension, not MIME type**.

### Client change

`buildImageUrl` (`client/app/domain/deck-builder/scryfall.ts`) emits `.jpg` instead of `.webp` for
the CDN branch, and the client-side `art_crop` Scryfall fallback (`artCropFallbackUrl`, the
`data-art-fallback` attribute, and the swap in `syncCardArtHost`) is deleted: the Worker's `302`
covers Scryfall failure for both sizes server-side, and the CDN-side failure the client fallback
covered cannot be seen in isolation, since the site is itself served through Cloudflare.
`cardBackUrl()` keeps returning the local `/card-back.webp` asset — that is a bundled static file,
not a CDN path. The Scryfall-direct branch is unchanged.

### Cost ceiling

No metered Cloudflare product is in the path, so overage is structurally impossible rather than merely unlikely: Workers' 100k/day rejects rather than bills, R2 egress is free, and no Images subscription exists to bill against. Only R2 storage can accrue, and it is bounded by the catalog rather than by traffic — a fully warmed English bucket (`large` plus `art_crop`) is roughly 20 GB, or about $0.15/month past the free 10 GB. Fills are one Class A operation each, inside the free 1M/month; reads are Class B, inside the free 10M/month.

Cloudflare offers no hard budget cap, and usage-based billing notifications require Professional plans or higher, so the ceiling has to come from the architecture. It does.

### Error / degradation

| Condition | Behavior |
|---|---|
| Off-layout path, or `a`/`b` mismatch | `404`, no outbound request |
| Non-`GET`/`HEAD` | `405` with `Allow` |
| R2 hit | Stored bytes, `immutable` |
| Scryfall `2xx` | Stored, then served |
| Scryfall `404` | `404` |
| Scryfall `429`/`5xx`/network failure | `302` to the Scryfall image URL; nothing stored |

The redirect is the only art fallback that remains, and it applies to both sizes. It replaces the
narrower client-side one it made redundant: `cardArt` used to attach `artCropFallbackUrl` when
`size === "art_crop"`, so a throttled fill on `large` was a visibly broken card on the board and
in the builder.

## Testing

The handler uses only `fetch`, `Response`, and `URL`, all present in Node, so a plain vitest test with a stubbed `env.CARDS` and a mocked global `fetch` covers it — no `@cloudflare/vitest-pool-workers` and no new dependency. The test lives beside the Worker at `iac/workers/card-cdn.test.ts`, reached by adding one include glob to `client/vitest.config.ts` so `just test` runs it.

Cases:

- Valid path with a bucket hit serves stored bytes and the `immutable` header, without calling Scryfall.
- Bucket miss stores the fetched bytes under the expected key and serves them.
- `a`/`b` not matching the print id prefix returns `404` and makes no outbound request.
- Off-layout path (bad size, bad face, non-UUID id, wrong extension) returns `404`.
- Non-`GET` method returns `405`.
- Scryfall `429` returns `302` to the Scryfall URL and stores nothing.
- Scryfall `404` returns `404`.
- **Layout round-trip:** URLs produced by `buildImageUrl` parse under the Worker's layout matcher. The Worker cannot import shared code, so this is the test that catches drift between the two copies of the layout.

## Out of scope

- The separate bulk-mirror repository and its 86 GB of images. It serves TTS importers, a different consumer with different needs, and is not a production dependency of this design. Retiring `mtg.reilley.dev` is a follow-up.
- Pre-warming the bucket. Cutover accepts the cold start.
- WebP or any format conversion, and any Cloudflare Images subscription.
- Authenticating the CDN. Card art is public data.

## Success criteria

- `edh-images.reilley.dev` serves card art over TLS on a free Universal SSL certificate.
- A print never requested before is stored and served on first request; the second request is served from R2 without touching Scryfall.
- A Scryfall outage or rate-limit window shows real card art via redirect rather than broken tiles, on both `large` and `art_crop`.
- Recurring Cloudflare cost is $0 until the bucket exceeds 10 GB, and cannot exceed single-digit dollars per month by construction.
- `just test` covers the Worker, including the layout round-trip against `buildImageUrl`.
