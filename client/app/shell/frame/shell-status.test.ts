import { html } from "foldkit/html";
import { describe, expect, it } from "vitest";
import { shellStatusChrome, shellStatusCopy } from "./shell-status";

const h = html<never>();

describe("shellStatusCopy", () => {
  it("names the surface in idle and loading copy", () => {
    expect(shellStatusCopy("Coverage", "idle")).toBe("Coverage has not loaded yet.");
    expect(shellStatusCopy("Coverage", "loading")).toBe("Loading coverage...");
    expect(shellStatusCopy("Leaderboard", "idle")).toBe("Leaderboard has not loaded yet.");
    expect(shellStatusCopy("Leaderboard", "loading")).toBe("Loading leaderboard...");
  });

  it("is silent when ready or in error", () => {
    expect(shellStatusCopy("Coverage", "ready")).toBeNull();
    expect(shellStatusCopy("Coverage", "error")).toBeNull();
  });
});

describe("shellStatusChrome", () => {
  const retry = { testId: "surface-try-again", onClick: null as never };

  it("shows the error alert with role=alert and the retry ghost only in error status", () => {
    const [alert, copy, tryAgain] = shellStatusChrome(h, { noun: "Coverage", status: "error", error: "boom", retry });
    expect(alert).not.toBeNull();
    expect(copy).toBeNull();
    expect(tryAgain).not.toBeNull();
  });

  it("shows loading copy without alert or retry while loading", () => {
    const [alert, copy, tryAgain] = shellStatusChrome(h, {
      noun: "Leaderboard",
      status: "loading",
      error: null,
      retry,
    });
    expect(alert).toBeNull();
    expect(copy).not.toBeNull();
    expect(tryAgain).toBeNull();
  });

  it("keeps a visible error alert even when status moved on", () => {
    const [alert] = shellStatusChrome(h, { noun: "Coverage", status: "ready", error: "stale", retry });
    expect(alert).not.toBeNull();
  });
});
