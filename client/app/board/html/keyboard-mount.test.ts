/**
 * @vitest-environment happy-dom
 */

import { describe, expect, it } from "vitest";
import { isAltKeyEvent, shouldIgnoreBoardShortcut } from "./keyboard-mount"; // export the pure helper

function shortcutIgnoredBy(target: Element, init: KeyboardEventInit): boolean {
  let ignored: boolean | undefined;
  target.addEventListener("keydown", (event) => {
    ignored = shouldIgnoreBoardShortcut(event as KeyboardEvent);
  });
  target.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, ...init }));
  if (ignored == null) throw new Error("expected keydown listener to run");
  return ignored;
}

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

describe("shouldIgnoreBoardShortcut", () => {
  it("allows Alt and Escape from focused buttons", () => {
    const button = document.createElement("button");
    document.body.append(button);
    button.focus();

    expect(shortcutIgnoredBy(button, { key: "Alt", code: "AltLeft" })).toBe(false);
    expect(shortcutIgnoredBy(button, { key: "Escape", code: "Escape" })).toBe(false);
  });

  it("keeps Space and Enter guarded for focused buttons", () => {
    const button = document.createElement("button");
    document.body.append(button);
    button.focus();

    expect(shortcutIgnoredBy(button, { key: " ", code: "Space" })).toBe(true);
    expect(shortcutIgnoredBy(button, { key: "Enter", code: "Enter" })).toBe(true);
  });

  it("ignores board shortcuts from text-entry controls", () => {
    const input = document.createElement("input");
    document.body.append(input);
    input.focus();

    expect(shortcutIgnoredBy(input, { key: "Alt", code: "AltLeft" })).toBe(true);
    expect(shortcutIgnoredBy(input, { key: "Escape", code: "Escape" })).toBe(true);
  });
});
