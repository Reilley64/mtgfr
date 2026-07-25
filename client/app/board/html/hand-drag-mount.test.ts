/**
 * @vitest-environment happy-dom
 */

import { afterEach, describe, expect, it } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import type { ActionView } from "~/wire/types";
import { readHandDragPayload, setHandDragGrabbingCursor } from "./hand-drag-mount";

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
