import { Effect } from "effect";
import { Navigation } from "foldkit";

type PushUrl = (url: string) => Effect.Effect<void>;

function pathnameOnly(path: string): string {
  try {
    return new URL(path, "http://localhost").pathname;
  } catch {
    return path.split(/[?#]/, 1)[0] ?? "";
  }
}

function isHome(path: string): boolean {
  return pathnameOnly(path) === "/";
}

function isPlayDeckEntry(path: string): boolean {
  return /^\/play\/[^/]+$/.test(pathnameOnly(path));
}

function prefersReducedMotion(optsValue: boolean | undefined): boolean {
  if (optsValue != null) return optsValue;
  return globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
}

function browserStartViewTransition(): typeof document.startViewTransition | undefined {
  return globalThis.document?.startViewTransition?.bind(globalThis.document);
}

export function shouldAnimateDeckCardNav(fromPathname: string, toPathname: string): boolean {
  if (isHome(fromPathname) && isPlayDeckEntry(toPathname)) return true;
  return isPlayDeckEntry(fromPathname) && isHome(toPathname);
}

export function pushUrlMaybeViewTransition(
  url: string,
  fromPathname: string,
  opts: {
    startViewTransition?: typeof document.startViewTransition;
    prefersReducedMotion?: boolean;
    pushUrl?: PushUrl;
  } = {},
): Effect.Effect<void> {
  const pushUrl = opts.pushUrl ?? Navigation.pushUrl;
  const startViewTransition = opts.startViewTransition ?? browserStartViewTransition();

  if (!shouldAnimateDeckCardNav(fromPathname, pathnameOnly(url))) return pushUrl(url);
  if (prefersReducedMotion(opts.prefersReducedMotion)) return pushUrl(url);
  if (startViewTransition == null) return pushUrl(url);

  return Effect.promise(async () => {
    let pushed: Promise<void> | undefined;
    startViewTransition(() => {
      pushed = Effect.runPromise(pushUrl(url));
      void pushed;
    });
    await pushed;
  });
}
