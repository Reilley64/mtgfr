// DOM card art that registers with the shared ImageCache so canvas and overlays share one
// decode pipeline. Slate background shows until the browser paints the image.

import { createEffect, type JSX, splitProps } from "solid-js";
import { cardArtFaceTag } from "~/lib/cardArtFace";
import { cn } from "~/lib/cn";
import { sharedImageCache } from "~/lib/imageCache";
import { type ImageFace, type ImageSize, imageUrlByPrint } from "~/lib/scryfall";

type ImgProps = Omit<JSX.ImgHTMLAttributes<HTMLImageElement>, "src" | "alt">;

export type CardArtProps = ImgProps & {
  print: string;
  alt?: string;
  size?: ImageSize;
  face?: ImageFace;
  /** Extra classes behind the image (slate while loading / empty print). */
  placeholderClass?: string;
};

export function CardArt(props: CardArtProps) {
  const [local, rest] = splitProps(props, ["print", "alt", "size", "face", "placeholderClass", "class"]);
  const url = () => imageUrlByPrint(local.print, local.size ?? "large", local.face ?? "front");

  createEffect(() => {
    const u = url();
    if (u) void sharedImageCache.get(u);
  });

  // Locked via `cardArtFaceTag`: never swap to a <div> for empty print (that dropped `{...rest}`).
  const tag = cardArtFaceTag(url());
  if (tag !== "img") {
    const _exhaustive: never = tag;
    return _exhaustive;
  }

  return (
    <img
      {...rest}
      src={url()}
      alt={local.alt ?? ""}
      class={cn("bg-morph-slate", local.class, local.placeholderClass)}
    />
  );
}

/** Register prints with the shared cache without rendering (e.g. when a prompt opens). */
export function preloadPrints(prints: Iterable<string>, size: ImageSize = "large"): void {
  const urls: string[] = [];
  for (const print of prints) {
    const url = imageUrlByPrint(print, size);
    if (url) urls.push(url);
  }
  sharedImageCache.preload(urls);
}
