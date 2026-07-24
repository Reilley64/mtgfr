function looksLikeShareLink(input: string): boolean {
  return (
    input.includes("://") ||
    input.startsWith("/") ||
    input.startsWith("?") ||
    input.includes("?table=") ||
    input.includes("&table=") ||
    /^\/play\/[^/]+/.test(input)
  );
}

/** Normalize guest input: a bare code or a pasted share link becomes a table id. */
export function parseTableCode(input: string, base = globalThis.location?.origin ?? "http://localhost"): string | null {
  const trimmed = input.trim();
  if (trimmed === "") return null;

  if (!looksLikeShareLink(trimmed)) return trimmed.toUpperCase();

  try {
    const url = trimmed.includes("://") ? new URL(trimmed) : new URL(trimmed, base);
    const fromPath = url.pathname.match(/^\/play\/[^/]+\/([^/]+)$/);
    if (fromPath)
      return decodeURIComponent(fromPath[1] ?? "")
        .trim()
        .toUpperCase();
    const fromQuery = url.searchParams.get("table");
    if (fromQuery != null && fromQuery.trim() !== "") return fromQuery.trim().toUpperCase();
    return null;
  } catch {
    return null;
  }
}
