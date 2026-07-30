/**
 * @vitest-environment happy-dom
 */

import { describe, expect, it } from "vitest";
import { boardKeyListeners, isAltKeyEvent, isShiftKeyEvent, shouldIgnoreBoardShortcut } from "./keyboard-mount"; // export the pure helper

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

describe("isShiftKeyEvent", () => {
  it("matches ShiftLeft and ShiftRight codes", () => {
    expect(isShiftKeyEvent({ key: "Shift", code: "ShiftLeft" } as KeyboardEvent)).toBe(true);
    expect(isShiftKeyEvent({ key: "Shift", code: "ShiftRight" } as KeyboardEvent)).toBe(true);
  });

  it("matches key Shift even when code is empty", () => {
    expect(isShiftKeyEvent({ key: "Shift", code: "" } as KeyboardEvent)).toBe(true);
  });

  it("ignores unrelated keys", () => {
    expect(isShiftKeyEvent({ key: "S", code: "KeyS" } as KeyboardEvent)).toBe(false);
  });
});

describe("boardKeyListeners", () => {
  function record(): { tags: string[]; listeners: ReturnType<typeof boardKeyListeners> } {
    const tags: string[] = [];
    return { tags, listeners: boardKeyListeners((message) => tags.push(message._tag)) };
  }

  it("emits ShiftDown while shift is held and ShiftUp on release", () => {
    const { tags, listeners } = record();
    listeners.onKeyDown(new KeyboardEvent("keydown", { key: "Shift", code: "ShiftLeft" }));
    listeners.onKeyUp(new KeyboardEvent("keyup", { key: "Shift", code: "ShiftLeft" }));
    expect(tags).toEqual(["ShiftDown", "ShiftUp"]);
  });

  // Alt-tabbing away with shift held sends the keyup to another window. Without the blur release
  // the modifier latches on and every later combat drop silently commits the whole pile.
  it("releases shift when the window loses focus so a lost keyup can't latch it on", () => {
    const { tags, listeners } = record();
    listeners.onKeyDown(new KeyboardEvent("keydown", { key: "Shift", code: "ShiftLeft" }));
    listeners.onBlur();
    expect(tags).toEqual(["ShiftDown", "ShiftUp"]);
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
