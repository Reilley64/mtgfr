import type { Html, html } from "foldkit/html";
import { cn } from "../cn";
import { gravatarUrl, monogramLetter } from "../gravatar";

type HtmlFactory<Message> = ReturnType<typeof html<Message>>;

export type SeatFaceOptions = {
  seat: number;
  username: string | null | undefined;
  gravatarHash: string | null | undefined;
  className?: string;
};

export function seatFace<Message>(h: HtmlFactory<Message>, options: SeatFaceOptions): Html {
  const className = cn(
    "inline-flex size-8 shrink-0 items-center justify-center overflow-hidden rounded-full bg-vine-dim font-bold text-caption text-snow",
    options.className,
  );
  const url = gravatarUrl(options.gravatarHash ?? "", 64);
  const testId = `seat-face-${options.seat}`;
  const alt = options.username?.trim() ? `${options.username.trim()} avatar` : `Seat ${options.seat + 1} avatar`;

  if (url == null) {
    return h.span(
      [h.Class(className), h.DataAttribute("testid", testId), h.Attribute("aria-label", alt)],
      [monogramLetter(options.username, options.seat)],
    );
  }

  return h.img([
    h.Class(className),
    h.DataAttribute("testid", testId),
    h.Src(url),
    h.Alt(alt),
    h.Width("32"),
    h.Height("32"),
    h.Loading("lazy"),
    h.Attribute("referrerpolicy", "no-referrer"),
  ]);
}
