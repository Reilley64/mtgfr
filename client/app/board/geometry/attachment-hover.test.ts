import { describe, expect, it } from "vitest";
import { BLANK_FACE } from "../../domain/card-render/frame";
import { attachmentHoverCards, attachmentHoverOffset, hitAttachmentHover } from "./attachment-hover";
import type { RenderCard } from "./layout";

const identity = { panX: 0, panY: 0, zoom: 1 };

function card(overrides: Partial<RenderCard> = {}): RenderCard {
  return {
    id: 0,
    x: 0,
    y: 100,
    w: 100,
    h: 100,
    name: "Card",
    cardId: "",
    print: "",
    pt: "",
    tapped: false,
    counters: 0,
    markedDamage: 0,
    faceDown: false,
    zone: 1,
    controller: 0,
    owner: 0,
    kind: "creature",
    tapsForMana: false,
    summoningSick: false,
    hasHaste: false,
    keywords: [],
    goaded: false,
    face: BLANK_FACE,
    isCommander: false,
    prepared: false,
    pile: 0,
    cluster: 0,
    clusterMembers: [],
    ...overrides,
  };
}

const cards = [card({ id: 2, attachedTo: 1 }), card({ id: 1 })];
const mirroredCards = [card({ id: 12, attachedTo: 11 }), card({ id: 11, controller: 1 })];
const crossControllerCards = [card({ id: 22, attachedTo: 21, controller: 3 }), card({ id: 21, controller: 0 })];
const nestedCards = [
  card({ id: 33, y: 40, attachedTo: 32 }),
  card({ id: 32, y: 80, attachedTo: 31 }),
  card({ id: 31, y: 80 }),
];

describe("attachment hover geometry", () => {
  it("raises attachments toward the root host controller's avatar", () => {
    expect(attachmentHoverOffset(cards, 2, 1, 0)).toBe(-20);
    expect(attachmentHoverOffset(mirroredCards, 12, 1, 0)).toBe(20);
    expect(attachmentHoverOffset(crossControllerCards, 22, 1, 0)).toBe(-20);
  });

  it("uses a nested attachment's own rendered height for its rise distance", () => {
    const differentlySizedNestedCards = [
      card({ id: 62, y: 100, h: 40, attachedTo: 61 }),
      card({ id: 61, y: 100, h: 70, attachedTo: 60 }),
      card({ id: 60, y: 100, h: 100 }),
    ];

    expect(attachmentHoverOffset(differentlySizedNestedCards, 62, 1, 0)).toBe(-8);
    expect(attachmentHoverCards(differentlySizedNestedCards, new Map([[62, 1]]), 0)[0]?.y).toBe(92);
  });

  it("moves only the hovered nested attachment without changing layout order or dimensions", () => {
    const presented = attachmentHoverCards(nestedCards, new Map([[32, 1]]), 0);

    expect(presented.map((presentedCard) => presentedCard.id)).toEqual(nestedCards.map((nestedCard) => nestedCard.id));
    expect(presented.find((presentedCard) => presentedCard.id === 32)?.y).toBe(60);
    expect(presented.find((presentedCard) => presentedCard.id === 31)?.y).toBe(80);
    expect(presented.find((presentedCard) => presentedCard.id === 33)?.y).toBe(40);
    expect(
      presented.every(
        (presentedCard, index) =>
          presentedCard.w === nestedCards[index]?.w && presentedCard.h === nestedCards[index]?.h,
      ),
    ).toBe(true);
  });

  it("keeps root-host precedence while allowing the hovered attachment's shifted footprint", () => {
    expect(hitAttachmentHover(identity, 50, 70, nestedCards, 32, 0)).toBe(32);
    expect(hitAttachmentHover(identity, 50, 105, nestedCards, 32, 0)).toBeNull();
    expect(hitAttachmentHover(identity, 50, 10, nestedCards, 32, 0)).toBeNull();
  });

  it("rejects cycles and missing attachment hosts", () => {
    const cyclicCards = [card({ id: 41, attachedTo: 42 }), card({ id: 42, attachedTo: 41 })];
    const missingHostCards = [card({ id: 51, attachedTo: 52 })];

    expect(attachmentHoverOffset(cyclicCards, 41, 1, 0)).toBe(0);
    expect(attachmentHoverOffset(missingHostCards, 51, 1, 0)).toBe(0);
    expect(hitAttachmentHover(identity, 50, 110, cyclicCards, 41, 0)).toBeNull();
  });

  it("clamps hover progress before applying the raised presentation", () => {
    const presented = attachmentHoverCards(
      cards,
      new Map([
        [2, 2],
        [1, -1],
      ]),
      0,
    );

    expect(presented.find((presentedCard) => presentedCard.id === 2)?.y).toBe(80);
    expect(presented.find((presentedCard) => presentedCard.id === 1)).toBe(cards[1]);
  });
});
