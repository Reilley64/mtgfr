import { Effect } from "effect";
import { Navigation } from "foldkit";

type PushUrl = (url: string) => Effect.Effect<void>;
type Raf = (callback: FrameRequestCallback) => number;

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

function browserRaf(): Raf {
  return (
    globalThis.requestAnimationFrame?.bind(globalThis) ?? ((cb) => setTimeout(() => cb(0), 0) as unknown as number)
  );
}

/** Foldkit patches the DOM on the next animation frame after UrlChanged; wait for that patch. */
function waitForFoldkitPaint(raf: Raf): Promise<void> {
  return new Promise((resolve) => {
    raf(() => resolve());
  });
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
    requestAnimationFrame?: Raf;
  } = {},
): Effect.Effect<void> {
  const pushUrl = opts.pushUrl ?? Navigation.pushUrl;
  const startViewTransition = opts.startViewTransition ?? browserStartViewTransition();
  const raf = opts.requestAnimationFrame ?? browserRaf();

  if (!shouldAnimateDeckCardNav(fromPathname, pathnameOnly(url))) return pushUrl(url);
  if (prefersReducedMotion(opts.prefersReducedMotion)) return pushUrl(url);
  if (startViewTransition == null) return pushUrl(url);

  return Effect.promise(() => {
    return new Promise<void>((resolve, reject) => {
      const transition = startViewTransition(() => {
        // pushUrl only updates history + queues UrlChanged; Foldkit's DOM patch
        // runs on the following rAF. The View Transition API snapshots "new"
        // state when this callback settles — so we must wait for that paint.
        const done = (async () => {
          await Effect.runPromise(pushUrl(url));
          await waitForFoldkitPaint(raf);
        })();
        done.then(resolve, reject);
        return done;
      });
      void transition;
    });
  });
}
