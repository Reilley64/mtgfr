import * as Effect from "effect/Effect";

interface ImageLike {
  src: string;
  onload: ((this: GlobalEventHandlers, ev: Event) => unknown) | null;
  onerror: ((this: GlobalEventHandlers, ev: Event) => unknown) | null;
}

export class ImageCache {
  private images = new Map<string, ImageLike>();
  private ready = new Set<string>();
  private failed = new Set<string>();
  private listeners = new Set<() => void>();

  // ponytail: no eviction; the table's active card-art set is bounded.
  constructor(
    private onLoad: () => void = () => {},
    private makeImage: () => ImageLike = () => new Image(),
  ) {}

  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  preload(urls: Iterable<string>): void {
    for (const url of urls) {
      if (!url) continue;
      void this.get(url);
    }
  }

  isReady(url: string): boolean {
    return this.ready.has(url);
  }

  isFailed(url: string): boolean {
    return this.failed.has(url);
  }

  get(url: string): HTMLImageElement | undefined {
    const existing = this.images.get(url);
    if (existing) return this.ready.has(url) ? (existing as HTMLImageElement) : undefined;

    const img = this.makeImage();
    this.images.set(url, img);
    this.failed.delete(url);

    const settled = new Promise<boolean>((resolve) => {
      img.onload = () => resolve(true);
      img.onerror = () => resolve(false);
    });
    img.src = url;

    const load = Effect.promise(() => settled).pipe(
      Effect.tap((success) =>
        Effect.sync(() => {
          if (success) {
            this.failed.delete(url);
            this.ready.add(url);
          } else {
            this.ready.delete(url);
            this.failed.add(url);
          }
          this.notifyLoaded();
        }),
      ),
    );
    Effect.runFork(load);

    return undefined;
  }

  private notifyLoaded(): void {
    this.onLoad();
    for (const listener of this.listeners) listener();
  }
}

export const sharedImageCache = new ImageCache();
