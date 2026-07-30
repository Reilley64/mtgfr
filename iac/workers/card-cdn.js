// Card art CDN — R2 bucket filled on miss from Scryfall.
// Spec: docs/superpowers/specs/2026-07-30-card-image-cdn-design.md
// Uploaded verbatim by iac/card-cdn.tf, so: plain ES module, no imports, no bundler.

const LAYOUT =
  /^\/(large|art_crop)\/(front|back)\/([0-9a-f])\/([0-9a-f])\/([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})\.jpg$/;

// Print ids never change, so the bytes at a key never change either.
const IMMUTABLE = { "Cache-Control": "public, max-age=31536000, immutable", "Content-Type": "image/jpeg" };
const USER_AGENT = "edh.reilley.dev/0.1";

function scryfallImageUrl(size, face, id) {
  const back = face === "back" ? "&face=back" : "";
  return `https://api.scryfall.com/cards/${id}?format=image&version=${size}${back}`;
}

export default {
  async fetch(request, env) {
    if (request.method !== "GET" && request.method !== "HEAD") {
      return new Response("Method not allowed", { status: 405, headers: { Allow: "GET, HEAD" } });
    }

    const match = LAYOUT.exec(new URL(request.url).pathname);
    if (!match) return new Response("Not found", { status: 404 });

    const [, size, face, a, b, id] = match;
    // Anyone can make this endpoint write to the bucket and fetch from Scryfall, so the path is a
    // trust boundary: pinning the fan-out chars to the id pins the outbound URL to one print.
    if (a !== id[0] || b !== id[1]) return new Response("Not found", { status: 404 });

    const key = `${size}/${face}/${a}/${b}/${id}.jpg`;
    const stored = await env.CARDS.get(key);
    if (stored) return new Response(stored.body, { headers: IMMUTABLE });

    const upstream = scryfallImageUrl(size, face, id);
    const filled = await fetch(upstream, { headers: { "User-Agent": USER_AGENT } }).catch(() => null);
    // A print that does not exist is not transient; anything else might be, so send the browser
    // to Scryfall directly this once. `large` has no client-side fallback of its own.
    if (filled === null) return Response.redirect(upstream, 302);
    if (filled.status === 404) return new Response("Not found", { status: 404 });
    if (!filled.ok) return Response.redirect(upstream, 302);

    const bytes = await filled.arrayBuffer();
    // A failed write only costs the next request another fill — serve what we already have.
    await env.CARDS.put(key, bytes, { httpMetadata: { contentType: "image/jpeg" } }).catch(() => {});
    return new Response(bytes, { headers: IMMUTABLE });
  },
};
