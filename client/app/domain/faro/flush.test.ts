import { describe, expect, it } from "vitest";
import { registerSpanFlushOnHide } from "./flush";

type Listener = () => void;

function fakeDocument(visibilityState: DocumentVisibilityState) {
  const listeners = new Map<string, Listener[]>();
  return {
    visibilityState,
    addEventListener(type: string, listener: EventListenerOrEventListenerObject) {
      listeners.set(type, [...(listeners.get(type) ?? []), listener as Listener]);
    },
    fire(type: string) {
      for (const listener of listeners.get(type) ?? []) listener();
    },
    listenerCount(type: string) {
      return (listeners.get(type) ?? []).length;
    },
  };
}

function countingProvider() {
  let flushes = 0;
  return {
    provider: {
      forceFlush: () => {
        flushes += 1;
        return Promise.resolve();
      },
    },
    flushes: () => flushes,
  };
}

describe("registerSpanFlushOnHide", () => {
  it("flushes pending spans when the page becomes hidden", () => {
    const doc = fakeDocument("hidden");
    const { provider, flushes } = countingProvider();

    registerSpanFlushOnHide(doc, () => provider);
    doc.fire("visibilitychange");

    expect(flushes()).toBe(1);
  });

  it("keeps spans buffered while the page is still visible", () => {
    const doc = fakeDocument("visible");
    const { provider, flushes } = countingProvider();

    registerSpanFlushOnHide(doc, () => provider);
    doc.fire("visibilitychange");

    expect(flushes()).toBe(0);
  });

  it("flushes on pagehide, which Safari fires without a visibilitychange", () => {
    const doc = fakeDocument("visible");
    const { provider, flushes } = countingProvider();

    registerSpanFlushOnHide(doc, () => provider);
    doc.fire("pagehide");

    expect(flushes()).toBe(1);
  });

  it("is inert when no tracer provider is registered", () => {
    const doc = fakeDocument("hidden");

    registerSpanFlushOnHide(doc, () => null);

    expect(() => doc.fire("visibilitychange")).not.toThrow();
  });

  it("registers nothing without a document (server render)", () => {
    expect(() => registerSpanFlushOnHide(null, () => null)).not.toThrow();
  });
});
