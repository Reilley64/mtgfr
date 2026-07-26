import { artCropFallbackUrl, cardBackUrl, type ImageFace, type ImageSize, imageUrlByPrint } from "../deck-builder/scryfall";

export function proxiedCardArtUrl(remoteUrl: string): string {
  return `/api/card-art/proxy?url=${encodeURIComponent(remoteUrl)}`;
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
