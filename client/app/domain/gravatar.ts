export async function gravatarHash(email: string): Promise<string> {
  const normalized = email.trim().toLowerCase();
  const data = new TextEncoder().encode(normalized);
  const digest = await crypto.subtle.digest("SHA-256", data);
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

export function gravatarUrl(hash: string, size: number = 128): string | null {
  const h = hash.trim();
  if (!h) return null;
  return `https://www.gravatar.com/avatar/${h}?s=${size}&d=404`;
}

export function monogramLetter(username: string | null | undefined, seat: number): string {
  const name = username?.trim() ?? "";
  if (!name) return `${seat}`;
  return name.charAt(0).toUpperCase();
}
