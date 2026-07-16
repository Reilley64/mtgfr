import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { cardBackUrl } from "~/lib/scryfall";

describe("cardBackUrl", () => {
  it("points at a Magic card back that ships in client/public/", () => {
    const url = cardBackUrl();
    expect(url).toMatch(/^\//);
    // Vite serves `public/` at the site root — the file must exist or ImageCache 404s forever.
    const publicFile = join(dirname(fileURLToPath(import.meta.url)), "../../public", url.slice(1));
    expect(existsSync(publicFile), `missing asset at ${publicFile}`).toBe(true);
  });
});
