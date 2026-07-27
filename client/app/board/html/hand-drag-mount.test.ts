/**
 * @vitest-environment happy-dom
 */

import { afterEach, describe, expect, it } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import type { ActionView } from "~/wire/types";
import { armHandDragGrabbingCursor, clientToHandDragPoint, readHandDragPayload, setHandDragGrabbingCursor } from "./hand-drag-mount";

function action(section: ActionView["section"]): ActionView {
  return {
    id: 7,
    kind: "cast",
    label: testMessageRef("Cast Lightning Bolt"),
    needs_target: false,
    object: 42,
    section,
  };
}

function hit(args: { action: ActionView; barZone?: string }): HTMLElement {
  const element = document.createElement("button");
  element.dataset.actionId = String(args.action.id);
  element.dataset.actionPayload = JSON.stringify(args.action);
  element.dataset.cardName = "Lightning Bolt";
  element.dataset.cardPrint = "bolt-print";
  element.dataset.manaCost = JSON.stringify({ generic: 1, colored: [0, 0, 0, 0, 0] });
  if (args.barZone != null) {
    element.dataset.barZone = args.barZone;
  }
  return element;
}

describe("clientToHandDragPoint", () => {
  afterEach(() => {
    document.body.replaceChildren();
  });

  it("maps client coordinates into board logical space via the gesture host", () => {
    const host = document.createElement("div");
    host.dataset.testid = "board-camera-gesture-mount";
    host.dataset.boardWidth = "1440";
    host.dataset.boardHeight = "900";
    host.getBoundingClientRect = () =>
      ({
        left: 0,
        top: 0,
        width: 720,
        height: 450,
        right: 720,
        bottom: 450,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      }) as DOMRect;
    document.body.append(host);

    // Mid-window client point must land at mid-board when the CSS box is half the logical size.
    expect(clientToHandDragPoint(360, 225)).toEqual({ x: 720, y: 450 });
  });

  it("still projects when the pointer is above the hand bar (outside a strict host hit test)", () => {
    const host = document.createElement("div");
    host.dataset.testid = "board-camera-gesture-mount";
    host.dataset.boardWidth = "1440";
    host.dataset.boardHeight = "900";
    host.getBoundingClientRect = () =>
      ({
        left: 0,
        top: 0,
        width: 720,
        height: 450,
        right: 720,
        bottom: 450,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      }) as DOMRect;
    document.body.append(host);

    // 10px above the host — drag must keep tracking in board space, not fall back to raw clientY.
    expect(clientToHandDragPoint(360, -10)).toEqual({ x: 720, y: -20 });
  });
});

describe("setHandDragGrabbingCursor", () => {
  afterEach(() => {
    document.documentElement.style.cursor = "";
  });

  it("sets grabbing and clears", () => {
    setHandDragGrabbingCursor(true);
    expect(document.documentElement.style.cursor).toBe("grabbing");
    setHandDragGrabbingCursor(false);
    expect(document.documentElement.style.cursor).toBe("");
  });
});

describe("armHandDragGrabbingCursor", () => {
  afterEach(() => {
    document.documentElement.style.cursor = "";
  });

  it("sets grabbing when armed", () => {
    armHandDragGrabbingCursor();

    expect(document.documentElement.style.cursor).toBe("grabbing");
  });

  it("clears grabbing through the disposer", () => {
    const clearGrab = armHandDragGrabbingCursor();

    clearGrab();

    expect(document.documentElement.style.cursor).toBe("");
  });

  it("allows the disposer to run twice", () => {
    const clearGrab = armHandDragGrabbingCursor();

    clearGrab();
    clearGrab();

    expect(document.documentElement.style.cursor).toBe("");
  });
});

describe("readHandDragPayload", () => {
  it("prefers the hit dataset bar zone over the action section", () => {
    const payload = readHandDragPayload(hit({ action: action("hand"), barZone: "command" }), 10, 20);
    expect(payload?.zone).toBe("command");
  });

  it("falls back to the action section when the dataset zone is missing", () => {
    const payload = readHandDragPayload(hit({ action: action("graveyard") }), 10, 20);
    expect(payload?.zone).toBe("graveyard");
  });

  it("falls back to hand when neither source resolves to a bar zone", () => {
    const payload = readHandDragPayload(hit({ action: action("battlefield"), barZone: "battlefield" }), 10, 20);
    expect(payload?.zone).toBe("hand");
  });
});
