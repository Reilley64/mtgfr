// Faro flushes *events* when the page hides (faro-core `BatchExecutor` listens for
// `visibilitychange`), but its tracing side does not: `TracingInstrumentation` wraps a plain OTel
// `BatchSpanProcessor` (1s timer, 30-span batch) and nobody ever calls `forceFlush`. A span that
// ends within that window dies with the page, while the BFF and API children of the same trace are
// already durable in a long-lived process — Tempo then renders the trace as
// `<root span not yet received>`.
//
// Register this BEFORE `initializeFaro` so it runs ahead of faro-core's own hide listener:
// `FaroTraceExporter.export` pushes synchronously, so the flushed spans land in the transport
// buffer in time for Faro's flush (which sends with `keepalive`).

import { trace } from "@opentelemetry/api";

type Flushable = { forceFlush: () => Promise<unknown> };

type HideTarget = Pick<Document, "addEventListener" | "visibilityState">;

/** The `WebTracerProvider` Faro registered globally, if it exposes `forceFlush`. */
function registeredProvider(): Flushable | null {
  const proxy = trace.getTracerProvider() as { getDelegate?: () => unknown };
  const delegate = typeof proxy.getDelegate === "function" ? proxy.getDelegate() : proxy;
  const flushable = delegate as Partial<Flushable> | null;
  return typeof flushable?.forceFlush === "function" ? (flushable as Flushable) : null;
}

/** Flush buffered browser spans before the page goes away, so their trace keeps its root. */
export function registerSpanFlushOnHide(
  target: HideTarget | null | undefined = globalThis.document,
  getProvider: () => Flushable | null = registeredProvider,
): void {
  if (!target) return;

  const flush = () => {
    // Export failures are already Faro's problem — nothing useful to do on a dying page.
    void getProvider()?.forceFlush();
  };

  target.addEventListener("visibilitychange", () => {
    if (target.visibilityState === "hidden") flush();
  });
  // Safari/iOS can tear a tab down without a `visibilitychange`.
  target.addEventListener("pagehide", flush);
}
