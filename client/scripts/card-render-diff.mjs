/**
 * Score our rendered card face against the real printed card, region by region.
 *
 * `CANONICAL.full` is 745x1040 — exactly Scryfall's `png` size — so the two images compare pixel
 * for pixel with no scaling. The renderer only runs in a browser (canvas, web fonts), so the drawing
 * and the pixel reads happen in the dev server's page via `agent-browser eval`; this file holds the
 * pure scoring maths, is imported by that probe over `/scripts/card-render-diff.mjs`, and doubles as
 * the CLI driver.
 *
 *   node client/scripts/card-render-diff.mjs <print-id> [--images] [--out <dir>]
 *
 * Needs `just dev` (Vite on :3000) and a live `agent-browser` session.
 *
 * A perfect score is not the goal: the printed card also carries a mana cost, set symbol, holo
 * stamp and artist line that we deliberately never draw, and its art is a different crop of the same
 * painting. Read the per-region numbers, and read them as a before/after when tuning the frame.
 *
 * It cannot tune type, though, and must not be asked to. Two sets of glyphs that do not land on each
 * other cost area in proportion to how much ink they carry, so the score rewards *less* ink: at a
 * rules size of 0.08 — text far smaller than any printed card — every reference card here scores
 * better than at the size print actually sets. Type is set by measuring printed glyphs instead: pick
 * a card whose line reads the same on both sides (Guard Gomazoa's `Defender, flying`), and compare
 * that line's inked width and height. Use this score for what it is good at — where a thing sits and
 * what colour it is.
 */

/** Per-channel distance under which two pixels count as the same colour to the eye. */
const NEAR = 16;

/** A card face built from a Scryfall printing — the same shape `faceDataFrom` reads off the board. */
export function faceFromScryfall(card) {
  const typeLine = card.type_line ?? "";
  const wubrg = ["W", "U", "B", "R", "G"];
  return {
    print: card.id ?? "",
    name: card.name ?? "",
    colors: (card.colors ?? []).map((color) => wubrg.indexOf(color)).filter((index) => index >= 0),
    isLand: /\bLand\b/.test(typeLine),
    isToken: false,
    legendary: /^Legendary\b/.test(typeLine),
    power: card.power ?? "",
    toughness: card.toughness ?? "",
    loyalty: card.loyalty ?? card.defense ?? "",
    typeLine,
    oracle: card.oracle_text ?? "",
    flavor: card.flavor_text ?? "",
  };
}

/**
 * Compare one rectangle of two RGBA buffers of the same size.
 * `match` is mean per-channel closeness; `near` is the share of pixels within `NEAR` on every
 * channel — the one that moves when a glyph lands a few pixels off rather than a shade off.
 */
export function compareRegion(a, b, size, rect) {
  const x0 = Math.max(0, Math.round(rect.x));
  const y0 = Math.max(0, Math.round(rect.y));
  const x1 = Math.min(size.w, Math.round(rect.x + rect.w));
  const y1 = Math.min(size.h, Math.round(rect.y + rect.h));
  if (x1 <= x0 || y1 <= y0) return { match: 0, near: 0, pixels: 0 };

  let sum = 0;
  let near = 0;
  for (let y = y0; y < y1; y += 1) {
    for (let x = x0; x < x1; x += 1) {
      const i = (y * size.w + x) * 4;
      const dr = Math.abs(a[i] - b[i]);
      const dg = Math.abs(a[i + 1] - b[i + 1]);
      const db = Math.abs(a[i + 2] - b[i + 2]);
      sum += dr + dg + db;
      if (Math.max(dr, dg, db) <= NEAR) near += 1;
    }
  }
  const pixels = (x1 - x0) * (y1 - y0);
  return { match: 100 * (1 - sum / (pixels * 3 * 255)), near: (100 * near) / pixels, pixels };
}

/** Score the whole face plus each slot the variant draws. */
export function compareFace(a, b, size, slots) {
  const regions = [
    ["card", { x: 0, y: 0, ...size }],
    ["art", slots.art],
    ["title", slots.title],
    ["type", slots.type],
    ["text", slots.text],
    ["p/t", slots.pt],
  ];
  return regions
    .filter(([, rect]) => rect != null)
    .map(([name, rect]) => ({ region: name, ...compareRegion(a, b, size, rect) }));
}

/** Greyscale difference image — white where the two agree, dark where they do not. */
export function heatmap(a, b, size) {
  const out = new Uint8ClampedArray(size.w * size.h * 4);
  for (let i = 0; i < out.length; i += 4) {
    const worst = Math.max(Math.abs(a[i] - b[i]), Math.abs(a[i + 1] - b[i + 1]), Math.abs(a[i + 2] - b[i + 2]));
    const shade = 255 - Math.min(255, worst * 3);
    out[i] = shade;
    out[i + 1] = shade;
    out[i + 2] = shade;
    out[i + 3] = 255;
  }
  return out;
}

/** The browser half: draw the face, fetch the printed png, and read both back as pixels. */
function probeSource(printId, wantImages) {
  return `(async () => {
  const [render, assets, frame, scryfall, diff] = await Promise.all([
    import("/app/domain/card-render/render.ts"),
    import("/app/domain/card-render/assets.ts"),
    import("/app/domain/card-render/frame.ts"),
    import("/app/domain/deck-builder/scryfall.ts"),
    import("/scripts/card-render-diff.mjs"),
  ]);
  await assets.loadCardFonts();
  await document.fonts.ready;

  const print = ${JSON.stringify(printId)};
  const response = await fetch("https://api.scryfall.com/cards/" + print);
  if (!response.ok) throw new Error("scryfall " + response.status + " for " + print);
  const card = await response.json();
  const face = diff.faceFromScryfall(card);

  const load = (url) => new Promise((resolve) => {
    if (!url) return resolve(null);
    const image = new Image();
    image.crossOrigin = "anonymous";
    image.onload = () => resolve(image);
    image.onerror = () => resolve(null);
    image.src = url;
  });
  const urls = render.faceAssetUrls(face);
  const [frameImage, ptImage, crownImage, art, printed] = await Promise.all([
    load(urls.frame),
    load(urls.pt),
    load(urls.crown),
    load(scryfall.imageUrlByPrint(print, "art")),
    load(card.image_uris?.png ?? ""),
  ]);
  if (printed == null) throw new Error("no printed png for " + print);

  const size = frame.CANONICAL.full;
  // Both sides land on white first: the frame and the printed png both leave the corners clear,
  // and a transparent pixel has no colour to compare.
  const paint = (draw) => {
    const canvas = document.createElement("canvas");
    canvas.width = size.w;
    canvas.height = size.h;
    const ctx = canvas.getContext("2d");
    ctx.fillStyle = "#ffffff";
    ctx.fillRect(0, 0, size.w, size.h);
    draw(ctx);
    return { canvas, pixels: ctx.getImageData(0, 0, size.w, size.h).data };
  };
  const mine = paint((ctx) => render.drawFace(ctx, { face, variant: "full", art, frameImage, ptImage, crownImage }));
  const real = paint((ctx) => ctx.drawImage(printed, 0, 0, size.w, size.h));

  const report = {
    name: face.name,
    print,
    set: card.set,
    regions: diff.compareFace(mine.pixels, real.pixels, size, frame.slotRects("full", face)),
  };
  if (!${wantImages ? "true" : "false"}) return report;

  const heat = document.createElement("canvas");
  heat.width = size.w;
  heat.height = size.h;
  heat.getContext("2d").putImageData(new ImageData(diff.heatmap(mine.pixels, real.pixels, size), size.w, size.h), 0, 0);
  return { ...report, images: { mine: mine.canvas.toDataURL(), real: real.canvas.toDataURL(), diff: heat.toDataURL() } };
})()`;
}

function table(report) {
  const rows = report.regions.map((row) => [
    row.region.padEnd(6),
    `${row.match.toFixed(2).padStart(6)}%`,
    `${row.near.toFixed(2).padStart(6)}%`,
    String(row.pixels).padStart(8),
  ]);
  const head = ["region", " match", "  near", "  pixels"];
  return [head, ...rows].map((row) => row.join("  ")).join("\n");
}

async function main() {
  const { execFile } = await import("node:child_process");
  const { promisify } = await import("node:util");
  const { mkdir, writeFile } = await import("node:fs/promises");
  const { join } = await import("node:path");
  const { tmpdir } = await import("node:os");

  const args = process.argv.slice(2);
  const printId = args.find((arg) => !arg.startsWith("--"));
  if (printId == null) {
    console.error("usage: node client/scripts/card-render-diff.mjs <print-id> [--images] [--out <dir>]");
    process.exit(2);
  }
  const wantImages = args.includes("--images");
  const outIndex = args.indexOf("--out");
  const out = outIndex >= 0 ? args[outIndex + 1] : join(tmpdir(), "card-render-diff");

  const run = promisify(execFile);
  // A browser caches an ES module by url for the life of the page, and the probe imports the whole
  // card-render graph — so a second run in the same tab would score the code as it stood when that
  // tab loaded, silently. Reload first, and every run measures the source on disk.
  const { stdout: href } = await run("agent-browser", ["eval", "location.href"]);
  await run("agent-browser", ["open", JSON.parse(href)]);
  const { stdout } = await run("agent-browser", ["eval", probeSource(printId, wantImages)], {
    maxBuffer: 64 * 1024 * 1024,
  });
  const report = JSON.parse(stdout);

  console.log(`${report.name} — ${report.set} ${report.print}`);
  console.log(table(report));
  if (report.images == null) return;

  await mkdir(out, { recursive: true });
  for (const [name, dataUrl] of Object.entries(report.images)) {
    const file = join(out, `${name}.png`);
    await writeFile(file, Buffer.from(dataUrl.split(",")[1], "base64"));
    console.log(`${name}: ${file}`);
  }
}

if (typeof process !== "undefined" && process.argv?.[1]?.endsWith("card-render-diff.mjs")) {
  await main();
}
