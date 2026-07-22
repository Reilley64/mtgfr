import { describe, expect, it } from "vitest";
import { isAltKeyEvent } from "./keyboard-mount"; // export the pure helper

describe("isAltKeyEvent", () => {
  it("matches AltLeft and AltRight codes", () => {
    expect(isAltKeyEvent({ key: "Alt", code: "AltLeft" } as KeyboardEvent)).toBe(true);
    expect(isAltKeyEvent({ key: "Alt", code: "AltRight" } as KeyboardEvent)).toBe(true);
  });

  it("matches key Alt even when code is empty", () => {
    expect(isAltKeyEvent({ key: "Alt", code: "" } as KeyboardEvent)).toBe(true);
  });

  it("ignores unrelated keys", () => {
    expect(isAltKeyEvent({ key: "a", code: "KeyA" } as KeyboardEvent)).toBe(false);
  });
});
