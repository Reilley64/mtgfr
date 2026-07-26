# Favicon Dragon Silhouette Design

**Status:** Implemented (as of 2026-07-25)
**Module:** `client/public/favicon.svg`, `client/public/favicon.ico`, `client/index.html`
**Surface spec to update on ship:** [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md) (brand chrome)

---

## Problem Statement

The current favicon is a blocky commander-gold **E** on an opaque `forest-floor` rounded plate. At tab size it reads as a weak generic lettermark, and the plate / ICO matte shows white around the dark square instead of a clean transparent edge.

---

## Goals

- Stronger brand signal at 16×16: an elder-dragon head+neck bust (EDH), not a letter.
- GitHub Invertocat craft: dark circular disc with the mark as **transparent negative space** inside it.
- Fully transparent outside the circle (no white square halo).
- Explicit HTML icon links so browsers prefer the SVG.

---

## Design

### Visual mark

- **Disc:** filled circle of `forest-floor` `#0B1310`.
- **Cutout:** side-profile **closed-mouth** head + neck bust, facing right, as a hole through the disc (transparent negative space) — same idea as GitHub’s circular Invertocat. Neck base is planted on the bottom rim of the disc (badge carve-out, not a floating bust).
- Technique: one path (or compound path) with `fill-rule="evenodd"` (circle outer + dragon inner); no stroke, no `commander-gold`, no square/`rect` plate.
- Outside the circle: fully transparent.
- `viewBox="0 0 32 32"` with ~8–12% padding from the viewBox edges to the circle so it does not clip in browser tabs.
- SVG includes `role="img"` and `aria-label="edh.reilley.dev"`.
- Craft: bold masses, few path nodes; readable as “dragon head” at 16×16 (light tab color shows through the cutout against the dark disc).

### Assets & wiring

- **Source of truth:** hand-authored `client/public/favicon.svg`.
- **Raster:** regenerate `client/public/favicon.ico` from that SVG (at least 16×16 and 32×32) with alpha preserved — transparent outside the circle and inside the dragon cutout; no opaque white matte.
- **HTML** (`client/index.html` `<head>`):

  ```html
  <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
  <link rel="icon" href="/favicon.ico" sizes="any" />
  ```

- Prefer SVG; ICO remains the fallback for older clients.
- Fill color `#0B1310` is hardcoded in the SVG (same token value as `design.tokens.json`); no favicon codegen.

### Docs on ship

- Update [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md) brand section to describe the circular dragon-cutout favicon and the explicit `<link rel="icon">` tags (not only the wordmark / title).
- This design note stays as design input; it does not replace the surface spec.

---

## Testing

- Cold-load the app and confirm the tab icon is a dark circle with a light (tab-colored) dragon cutout, with **no white box** outside the circle.
- Confirm `favicon.svg` uses an evenodd circle+cutout (no background `rect`) and `favicon.ico` preserves alpha.
- Lock the two `<link rel="icon">` entries in `index.html` with a small static assertion or grep-style fixture test (`<head>` is outside Foldkit Scene coverage).
- No pixel-diff suite and no multi-browser matrix beyond verifying that the same public files are served in dev and production builds.

---

## Further Notes

- **PWA install icons shipped (Wave 3).** `client/public/apple-touch-icon.png`, `pwa-192.png`, and `pwa-512.png` derive from the same dragon-on-disc art family as the favicon and are referenced by `client/index.html` and the generated manifest — see [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md) Brand display / Installable PWA.

---

## Out of Scope

- Dual light/dark marks (`prefers-color-scheme`).
- Replacing the in-app shell wordmark (`edh.reilley.dev` text) with the dragon mark.
- Gold accents, square plates, or lettermarks.
- Solid-filled dragon on transparent (no disc) — superseded by this circular cutout treatment.
