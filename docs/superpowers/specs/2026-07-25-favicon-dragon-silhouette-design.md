# Favicon Dragon Silhouette Design

**Status:** Design note (as of 2026-07-25)
**Module:** `client/public/favicon.svg`, `client/public/favicon.ico`, `client/index.html`
**Surface spec to update on ship:** [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md) (brand chrome)

---

## Problem Statement

The current favicon is a blocky commander-gold **E** on an opaque `forest-floor` rounded plate. At tab size it reads as a weak generic lettermark, and the plate / ICO matte shows white around the dark square instead of a clean transparent edge.

---

## Goals

- Stronger brand signal at 16×16: an elder-dragon head+neck bust (EDH), not a letter.
- GitHub-logo craft: filled silhouette, bold masses, few nodes — not a stroked outline and not fine scales/teeth.
- Fully transparent outside the mark (no plate, no white halo).
- Explicit HTML icon links so browsers prefer the SVG.

---

## Design

### Visual mark

- Side-profile **head + neck** bust, facing right.
- Solid fill only: `forest-floor` `#0B1310` (same token value as `design.tokens.json`; hardcoded in the SVG, no favicon codegen).
- No stroke, no `commander-gold`, no background `rect` / squircle / disc.
- `viewBox="0 0 32 32"` with ~10–15% padding from the edges so the mark does not clip in browser tabs.
- SVG includes `role="img"` and `aria-label="edh.reilley.dev"`.

### Assets & wiring

- **Source of truth:** hand-authored `client/public/favicon.svg`.
- **Raster:** regenerate `client/public/favicon.ico` from that SVG (at least 16×16 and 32×32) with alpha preserved — no opaque white matte.
- **HTML** (`client/index.html` `<head>`):

  ```html
  <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
  <link rel="icon" href="/favicon.ico" sizes="any" />
  ```

- Prefer SVG; ICO remains the fallback for older clients.

### Docs on ship

- Update [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md) brand section to describe the dragon silhouette favicon and the explicit `<link rel="icon">` tags (not only the wordmark / title).
- This design note stays as design input; it does not replace the surface spec.

---

## Testing

- Cold-load the app and confirm the tab icon is the dark dragon bust with no white box, still legible at tab size.
- Confirm `favicon.svg` has no background plate and `favicon.ico` preserves alpha (no white matte around the mark).
- Lock the two `<link rel="icon">` entries in `index.html` with a small static assertion or grep-style fixture test (`<head>` is outside Foldkit Scene coverage).
- No pixel-diff suite and no multi-browser matrix beyond verifying that the same public files are served in dev and production builds.

---

## Out of Scope

- Apple-touch / PWA / manifest icon sets.
- Dual light/dark marks (`prefers-color-scheme`).
- Replacing the in-app shell wordmark (`edh.reilley.dev` text) with the dragon mark.
- Gold accents, plates, or lettermarks.
