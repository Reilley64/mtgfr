# Task 6 report

## Status

Complete.

## Scope shipped

- Added `client/app/domain/card-art/proxy-fetch.ts` with:
  - `PROXY_ART_MAX_BYTES`
  - `assertSafeProxyTarget(raw)`
  - `fetchProxyCardArt(raw, deps)`
- Added `client/server/routes/api/card-art/proxy.get.ts` as an authenticated Nitro
  `GET /api/card-art/proxy` route.
- Updated living specs:
  - `docs/superpowers/specs/2026-07-20-deck-list-and-builder.md`
  - `docs/superpowers/specs/2026-07-20-shell-routes-and-auth.md`
  - `docs/superpowers/specs/2026-07-20-wire-protocol-and-visibility.md`

## Behavior shipped

- Requires the BFF `session` cookie and re-checks auth with `fetchMe`.
- Accepts only `https` URLs with no credentials.
- Rejects private, link-local, localhost, metadata, and internal-style hosts, plus
  public hostnames that resolve to blocked IPs.
- Fetches with `redirect: "manual"` and `Accept: image/*`.
- Never forwards browser cookies to the remote host.
- Allows only `image/jpeg`, `image/png`, `image/webp`, and `image/gif`.
- Caps response size at 5 MiB and returns `Cache-Control: public, max-age=300`
  on success.
- Maps unsafe input to `400`, missing/invalid auth to `401`, and upstream/type/size
  failures to `502` without echoing remote bodies.

## TDD evidence

### Red

`cd client && bun run test -- app/domain/card-art/proxy-fetch.test.ts server/routes/api/card-art/proxy.get.test.ts`

- failed because `./proxy-fetch` did not exist
- failed because `./proxy.get` did not exist

### Green

`cd client && bun run test -- app/domain/card-art/proxy-fetch.test.ts server/routes/api/card-art/proxy.get.test.ts`

- passed: 2 files, 9 tests

## Verification

- `just client-check`
  - format passed
  - lint passed
  - typecheck passed
  - 120 test files passed
  - 1226 tests passed

## Notes

- Left unrelated existing edits untouched:
  - `.superpowers/sdd/task-4-report.md`
  - `client/app/domain/card-art/proxy-url.ts`
  - `client/app/domain/ui/card-art.test.ts`

## Review follow-up — 2026-07-26

- Finding 1 fixed: `GET /api/card-art/proxy` now returns `Cache-Control: private, max-age=300`.
- Finding 2 fixed: the proxy resolves the hostname, rejects blocked results, then pins the HTTPS
  request lookup to the vetted address set instead of re-resolving during connect.
- Regression coverage added:
  - `client/app/domain/card-art/proxy-fetch.test.ts` asserts the request path receives a pinned
    lookup that yields only the vetted DNS result.
  - `client/server/routes/api/card-art/proxy.get.test.ts` asserts the success header is private.
- Fresh verification:
  - `cd client && bun run test -- app/domain/card-art/proxy-fetch.test.ts server/routes/api/card-art/proxy.get.test.ts`
    - passed: 2 files, 10 tests
  - `cd client && bun run typecheck`
    - passed

## Review follow-up — 2026-07-26 (Task 6)

- Fixed the proxy timeout gap so one `AbortSignal.timeout(timeoutMs)` now covers the full
  upstream operation: DNS-vetted fetch, response headers, and body download.
- `fetchPinnedHttps` keeps its abort hook active until the upstream response closes instead of
  detaching it as soon as headers arrive.
- `readResponseBodyCapped` now listens to the same signal, cancels the reader on abort, and fails
  the proxy request instead of hanging forever on a stalled body read.
- Regression coverage added in `client/app/domain/card-art/proxy-fetch.test.ts` with an injected
  fetch response whose reader returns one chunk and then never resolves; the helper now returns
  `{ ok: false, status: 502 }` within the timeout and cancels the reader.
- Fresh verification:
  - `cd client && bun run test app/domain/card-art/proxy-fetch.test.ts server/routes/api/card-art/proxy.get.test.ts`
    - passed: 2 files, 11 tests
  - `cd client && bun run typecheck`
    - passed
