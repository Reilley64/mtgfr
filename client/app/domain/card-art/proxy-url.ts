import { artCropFallbackUrl, cardBackUrl, type ImageFace, type ImageSize, imageUrlByPrint } from "../deck-builder/scryfall";

export const PROXY_ART_URL_ERROR = "Proxy art URL must use https, be 2048 characters or fewer, and omit credentials.";

export function proxiedCardArtUrl(remoteUrl: string): string {
  return `/api/card-art/proxy?url=${encodeURIComponent(remoteUrl)}`;
}

export function proxyArtUrlError(raw: string): string | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  if (trimmed.length > 2048) return PROXY_ART_URL_ERROR;

  try {
    const target = new URL(trimmed);
    if (target.protocol !== "https:") return PROXY_ART_URL_ERROR;
    if (target.username.length > 0 || target.password.length > 0) return PROXY_ART_URL_ERROR;
    return null;
  } catch {
    return PROXY_ART_URL_ERROR;
  }
}

export function resolveCardFaceUrls(args: {
  print: string;
  proxyArtUrl?: string;
  size?: ImageSize;
  face?: ImageFace;
}): { url: string; fallback: string | null } {
  const size = args.size ?? "large";
  const face = args.face ?? "front";
  if (!args.print) {
    return { url: cardBackUrl(), fallback: null };
  }

  const printUrl = imageUrlByPrint(args.print, size, face);
  const proxyArtUrl = args.proxyArtUrl?.trim() ?? "";
  if (proxyArtUrl && face === "front") {
    return {
      url: proxiedCardArtUrl(proxyArtUrl),
      fallback: printUrl || null,
    };
  }

  if (size !== "art_crop") {
    return { url: printUrl, fallback: null };
  }

  return {
    url: printUrl,
    fallback: artCropFallbackUrl(args.print, face),
  };
}
