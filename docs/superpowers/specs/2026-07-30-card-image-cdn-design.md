# Card image CDN (design)

**Status:** Approved design input (2026-07-30).
**Surfaces:** New target — Cloudflare Worker + R2 bucket in `iac/`; consumes the URL layout owned by `client/app/domain/deck-builder/scryfall.ts` (`buildImageUrl`). Updates `production-topology-and-operations` §Card art CDN at implement time.

---

## Problem Statement

`VITE_CARD_CDN` points at a host serving an 86 GB bulk mirror of Scryfall card images, built and populated by a separate repository. The topology spec records that host as "managed separately," so nothing in this repo describes it, tests it, or keeps its path layout aligned with the layout `buildImageUrl` constructs. New sets need a manual bulk re-run to appear, and a layout mismatch failed quietly: `artCropFallbackUrl` degraded `art_crop` to a direct Scryfall hotlink, while `large` had no fallback at all and showed a broken tile.

## Goal

Serve card art from a CDN this repo owns and can test, populated on demand from Scryfall so new prints appear without running anything, at no recurring cost.

## Locked decisions

| Decision | Choice |
|---|---|
| Hostname | `edh-images.reilley.dev` — first-level, so free Universal SSL covers it. A deeper name like `images.edh.reilley.dev` needs paid Advanced Certificate Manager, which would cost more than everything else here combined |
| Origin | R2 bucket behind a Worker; **not** the BFF (keeps image traffic off the tunnel and out of the Nitro event loop) |
| Population | Fill-on-miss only. Bucket starts empty; no bulk upload |
| Stored format | Scryfall's bytes unchanged — the WebP Scryfall already serves. **No** Images transformations |
| Sizes | Scryfall's own WebP size names (`thumb`, `grid`, `display`, `art`, `crop`), used directly as the client's `ImageSize` rather than mapped from JPEG names |
| Fill failure | `302` to the Scryfall image URL, store nothing, retry on next request |
| Scryfall 404 | `404` — a print that does not exist is not a transient failure |
| Cutover | Flip `VITE_CARD_CDN`; accept the cold-bucket warmup |
| Worker deploy | `cloudflare_workers_script` with `content_file` + `content_sha256` pointed at `iac/workers/card-cdn.js`. No wrangler, no bundler |

## Approaches considered

1. **Worker + R2, fill-on-miss (chosen)** — Edge-local, no home bandwidth, no coupling to the game's request path, and R2 egress is free.
2. Fill-on-miss in the Nitro BFF — Puts 86 GB-scale image traffic through the Cloudflare Tunnel from the homelab and shares the Nitro event loop with gRPC streams; a Scryfall rate-limit stall would occupy slots the game needs. Also inverts the reason `VITE_CARD_CDN` is a separate hostname, and image availability would follow Argo rolls.
3. R2 Sippy — Cloudflare's built-in on-demand fill, but it only pulls from S3 and GCS, not an arbitrary HTTP origin, so Scryfall is out of reach.
4. Cloudflare CDN caching-proxy in front of Scryfall, no storage — Free/Pro/Business CDN terms let Cloudflare limit access for serving "a disproportionate percentage of pictures," which is precisely what a card-art-only hostname is. Also fails Scryfall's ask to store copies rather than lean on their servers.
5. Cloudflare Images storage + delivery — Costs the most at warm scale (storage $5/100k stored/month, delivery $1/100k delivered/month, against free R2 egress) and replaces the path layout with `imagedelivery.net`'s fixed `<hash>/<id>/<variant>` shape.

### WebP without any conversion

Scryfall serves WebP itself, under a second family of size names sitting beside the JPEG ones at identical dimensions:

| JPEG | WebP | Dimensions | JPEG bytes | WebP bytes |
|---|---|---|---|---|
| `small` | `thumb` | 146×204 | 13509 | 8858 |
| `normal` | `grid` | 488×680 | 84925 | 48184 |
| `large` | `display` | 672×936 | 127704 | 65418 |
| `art_crop` | `art` | 626×457 | 59042 | 33136 |
| `border_crop` | `crop` | 480×680 | 84816 | 46698 |

So WebP costs nothing to adopt: no Cloudflare Images transformation (whose 5,000 unique/month free allowance would have capped **how fast the bucket fills**, not merely conversion quality), no conversion step anywhere, roughly half the stored bytes and half the transfer.

The client uses these names directly rather than translating from the JPEG ones. A mapping layer would be a second vocabulary to keep aligned across the client, the Worker's layout matcher, and Scryfall — and `png` has no WebP counterpart, so the mapping would have had to silently answer one size with another's bytes. `png` therefore does not exist as an `ImageSize`; add a `.png` layout branch if a surface ever needs the transparent rounded-corner asset.

One trap this closes: `api.scryfall.com/cards/{id}?format=image&version=…` does **not** accept the WebP names. `version=display`, `version=grid`, and `version=thumb` all return the byte-identical `large` JPEG, and `version=art` returns `503`. Every URL here — CDN branch, Scryfall fallback branch, and the Worker's upstream fetch — uses the `cards.scryfall.io` path layout instead.

## Design

### Components

- **R2 bucket** — keyed exactly as `buildImageUrl` builds paths: `{thumb|grid|display|art|crop}/{front|back}/{a}/{b}/{id}.webp`, where `a`/`b` are the first two characters of the print id. The layout matcher admits all five sizes, not just the two the client asks for today, so a surface can switch to a smaller image without redeploying the Worker.
- **Worker** (`iac/workers/card-cdn.js`) — the only reader/writer of the bucket. Self-contained plain JS with no imports, because Terraform embeds the file directly.
- **DNS + route** — proxied record for `edh-images.reilley.dev` in the existing `reilley.dev` zone, routed to the Worker.

### Request flow

1. Reject non-`GET`/`HEAD` with `405`.
2. Match the path against the layout. Verify `a`/`b` equal the first two characters of `id`. Any mismatch → `404` **before** any outbound request.
3. `env.CARDS.get(key)`. Hit → serve the bytes with `Cache-Control: public, max-age=31536000, immutable`. A read failure is treated as a miss, so step 4 runs and its own failure handling applies.
4. Miss → fetch `cards.scryfall.io/{size}/{face}/{a}/{b}/{id}.webp` (Scryfall's image CDN, which serves
   bytes directly at a path layout identical to ours) with our User-Agent. `api.scryfall.com/cards/{id}?format=image`
   is not usable here: it `302`s to this same CDN URL rather than returning bytes, which would make
   every fill look like a failure — and its `version=` param ignores the WebP size names.
5. `2xx` with a non-empty body → `put()` the bytes into R2 and serve them with the same headers. `2xx` with an empty body is not a successful fill.
6. Upstream `404` → `404`. Any other failure, including an empty `2xx` body → `302` to the same Scryfall image URL.

### Path validation as a trust boundary

The endpoint writes to the bucket on unauthenticated public request, so step 2 is a security control rather than tidiness. Constraining the path to the layout also constrains the outbound URL to `cards.scryfall.io/{size}/{face}/{a}/{b}/{uuid}.webp`, so the Worker cannot be used as a general-purpose proxy. The upstream URL is rebuilt from the validated capture groups rather than passed through from `pathname`, so the two can never diverge.

### Caching

A Worker route runs *before* the CDN cache, so Cloudflare's extension-based edge caching does not apply to this path — the Worker is consulted on every request that reaches the edge, each one counting against the free 100k/day. What suppresses repeat traffic is the year-long `immutable` browser cache, which is accurate because print ids never change.

Consequently the `.webp` extension is not load-bearing for caching here; it is chosen so the path does not misdescribe its bytes. If the Worker is ever removed in favour of serving the bucket directly on a custom domain, the extension starts mattering, because Cloudflare then caches on **file extension, not MIME type**.

### Client change

`ImageSize` (`client/app/domain/deck-builder/scryfall.ts`) becomes Scryfall's WebP size names, and
`buildImageUrl` emits `{base}/{size}/{face}/{a}/{b}/{id}.webp` — one shape for both branches, since
our CDN mirrors Scryfall's layout and only the origin differs. With `VITE_CARD_CDN` unset the base
falls back to `cards.scryfall.io`, so local development bypasses the CDN without a second URL shape.
`scryfallImageUrl` is deleted: it existed only to force the Scryfall branch, which is now what an
empty base already does.

Each surface then asks for the size it renders rather than the largest one — the board and hover
preview keep the full card (`display`), deck-list tiles take `art`, the builder's tile grids take
`grid`, and its 28-40px list rows take `thumb`. `builderCardArt` requires the size as an argument
so no builder surface can silently inherit the `display` default. The per-surface table lives in
[deck-list-and-builder](2026-07-20-deck-list-and-builder.md) §Card art CDN.

The client-side `art_crop` Scryfall fallback (`artCropFallbackUrl`, the `data-art-fallback`
attribute, and the swap in `syncCardArtHost`) is deleted: the Worker's `302` covers Scryfall failure
for every size server-side, and the CDN-side failure the client fallback covered cannot be seen in
isolation, since the site is itself served through Cloudflare. `cardBackUrl()` keeps returning the
local `/card-back.webp` asset — that is a bundled static file, not a CDN path.

### Cost ceiling

No metered Cloudflare product is in the path, so overage is structurally impossible rather than merely unlikely: Workers' 100k/day rejects rather than bills, R2 egress is free, and no Images subscription exists to bill against. Only R2 storage can accrue, and it is bounded by the catalog rather than by traffic — a fully warmed English bucket (`display` plus `art`, ~98 KB per print across ~110k prints) is roughly 11 GB, or a few cents a month past the free 10 GB. The JPEG equivalent would have been ~20 GB. Fills are one Class A operation each, inside the free 1M/month; reads are Class B, inside the free 10M/month.

This ceiling assumes the Workers **Free** plan. On Workers Paid the 100k/day hard stop does not exist, and the enabled Workers Logs observability puts a second metered product in the request path.

Cloudflare offers no hard budget cap, and usage-based billing notifications require Professional plans or higher, so the ceiling has to come from the architecture. It does.

### Error / degradation

| Condition | Behavior |
|---|---|
| Off-layout path, or `a`/`b` mismatch | `404`, no outbound request |
| Non-`GET`/`HEAD` | `405` with `Allow` |
| R2 hit | Stored bytes, `immutable` |
| R2 read failure | Treated as a miss; the fill's own handling applies |
| Scryfall `2xx` with a non-empty body | Stored, then served |
| Scryfall `2xx` with an empty body | `302` to the Scryfall image URL; nothing stored |
| Scryfall `404` | `404` |
| Scryfall `429`/`5xx`/network failure | `302` to the Scryfall image URL; nothing stored |

The redirect is the only art fallback that remains, and it applies to every size. It replaces the
narrower client-side one it made redundant: `cardArt` used to attach `artCropFallbackUrl` only for
the art-crop size, so a throttled fill on the full-card size was a visibly broken card on the board
and in the builder.

## Testing

The handler uses only `fetch`, `Response`, and `URL`, all present in Node, so a plain vitest test with a stubbed `env.CARDS` and a mocked global `fetch` covers it — no `@cloudflare/vitest-pool-workers` and no new dependency. The test lives beside the Worker at `iac/workers/card-cdn.test.ts`, reached by adding one include glob to `client/vitest.config.ts` so `just test` runs it.

Cases:

- Valid path with a bucket hit serves stored bytes and the `immutable` header, without calling Scryfall.
- Bucket miss stores the fetched bytes under the expected key and serves them.
- A back-face request asks Scryfall for the back-face path and stores under the back-face key.
- `a`/`b` not matching the print id prefix returns `404` and makes no outbound request, including a case that isolates each half of the fan-out check.
- Off-layout path (unknown size name, a retired JPEG size name, bad face, non-UUID id shape, wrong extension, extra path segment, bare root) returns `404`, including an id whose fan-out chars agree with a malformed id body so the id group's own shape is what's under test.
- An uppercase-hex UUID returns `404` rather than aliasing the lowercase key.
- Non-`GET` method returns `405` with the `Allow` header.
- Scryfall `429`, and a thrown network failure, both return `302` to the Scryfall URL and store nothing.
- Scryfall `404` returns `404` and stores nothing.
- A zero-length `2xx` fill, and a fill whose body fails to read mid-stream, both return `302` and store nothing.
- An R2 read failure falls through to the fill; an R2 write failure still serves the fetched bytes.
- **Layout round-trip:** every `ImageSize` × `ImageFace` combination `buildImageUrl` can emit parses under the Worker's layout matcher. The Worker cannot import shared code, so this is the test that catches drift between the two copies of the layout.

## Out of scope

- The separate bulk-mirror repository and its 86 GB of images. It serves TTS importers, a different consumer with different needs, and is not a production dependency of this design. Retiring `mtg.reilley.dev` is a follow-up.
- Pre-warming the bucket. Cutover accepts the cold start.
- Format conversion of any kind, and any Cloudflare Images subscription. WebP is what Scryfall already serves.
- Authenticating the CDN. Card art is public data.

## Success criteria

- `edh-images.reilley.dev` serves card art over TLS on a free Universal SSL certificate.
- A print never requested before is stored and served on first request; the second request is served from R2 without touching Scryfall.
- A Scryfall outage or rate-limit window shows real card art via redirect rather than broken tiles, on every size.
- Recurring Cloudflare cost is $0 until the bucket exceeds 10 GB, and cannot exceed single-digit dollars per month by construction.
- `just test` covers the Worker, including the layout round-trip against `buildImageUrl`.
