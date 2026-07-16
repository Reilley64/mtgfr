// A tiny async image cache for canvas rendering. `get` returns the decoded image
// if it's ready, or undefined while it loads (the caller draws a placeholder and
// redraws once `onLoad` fires). One HTMLImageElement per URL, loaded once.
//
// The load is an Effect: the first `get` for a URL forks one fiber (via the shared
// `runtime`) that resolves on `onload` / fails on `onerror`. `get` itself stays plain
// synchronous Map lookups — it's called every animation frame from Board's render loop,
// and forking an effect (or allocating a promise) per call would defeat that loop's whole
// point. So only the load is Effect-ified; the read can't be without breaking the caller.

import * as Effect from "effect/Effect";

/** The subset of `HTMLImageElement` the loader touches, kept structural so tests can fake
 * it without a DOM. Handler types match `GlobalEventHandlers` so `new Image()` assigns. */
interface ImageLike {
  src: string;
  onload: ((this: GlobalEventHandlers, ev: Event) => unknown) | null;
  onerror: ((this: GlobalEventHandlers, ev: Event) => unknown) | null;
}

export class ImageCache {
  private images = new Map<string, ImageLike>();
  private ready = new Set<string>();

  // ponytail: no eviction — the board's card set is small and bounded.
  constructor(
    private onLoad: () => void,
    private makeImage: () => ImageLike = () => new Image(),
  ) {}

  get(url: string): HTMLImageElement | undefined {
    const existing = this.images.get(url);
    if (existing) return this.ready.has(url) ? (existing as HTMLImageElement) : undefined;

    const img = this.makeImage();
    this.images.set(url, img);

    // The DOM handlers must attach synchronously, in this call — `runFork` doesn't start the
    // fiber until the next microtask, so wiring `img.onload`/`onerror` *inside* the effect (e.g.
    // `Effect.async`) would leave them unset for a tick after `get` returns. A plain `Promise`
    // bridges that: its executor runs now, and `Effect.promise` just hands the (already-wired)
    // result to the fiber whenever it gets scheduled.
    const settled = new Promise<boolean>((resolve) => {
      img.onload = () => resolve(true);
      img.onerror = () => resolve(false);
    });
    img.src = url;

    const load = Effect.promise(() => settled).pipe(
      Effect.tap((success) =>
        Effect.sync(() => {
          // A failed load never becomes ready and never retries — silent, same as before.
          // Nothing upstream is watching this fiber, so dropping it here is what stops a retry
          // loop, not "die silently" (there's no supervisor to die loudly to).
          if (!success) return;
          this.ready.add(url);
          this.onLoad();
        }),
      ),
    );
    Effect.runFork(load);

    return undefined;
  }
}
