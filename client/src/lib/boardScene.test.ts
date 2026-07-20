import { describe, expect, it } from "vitest";
import { CARD_H, CARD_W, type RenderCard, ZONE } from "~/layout";
import { ATTACK_STROKE, SELECT_STROKE, TARGET_STROKE } from "~/lib/boardPaintPrims";
import { buildBoardScene, type BuildBoardSceneInput } from "~/lib/boardScene";
import type { CardFlight } from "~/lib/cardFlight";

const cam = { panX: 0, panY: 0, zoom: 1 };

const card = (over: Partial<RenderCard> = {}): RenderCard => ({
  id: 1,
  x: 100,
  y: 100,
  w: CARD_W,
  h: CARD_H,
  name: "Bear",
  cardId: "",
  print: "",
  pt: "",
  tapped: false,
  counters: 0,
  markedDamage: 0,
  faceDown: false,
  zone: ZONE.Battlefield,
  controller: 0,
  owner: 0,
  kind: "creature",
  tapsForMana: false,
  summoningSick: false,
  hasHaste: false,
  keywords: [],
  goaded: false,
  isCommander: false,
  prepared: false,
  pile: 0,
  cluster: 0,
  clusterMembers: [],
  attachedTo: null,
  ...over,
});

function flight(over: Partial<CardFlight> & Pick<CardFlight, "id" | "phase" | "kind">): CardFlight {
  return {
    print: "",
    name: "Fly",
    x: 1,
    y: 2,
    scale: 0.5,
    targetX: 10,
    targetY: 20,
    targetScale: 1,
    ...over,
  };
}

function baseInput(over: Partial<BuildBoardSceneInput> = {}): BuildBoardSceneInput {
  return {
    cam,
    cards: [card()],
    me: 0,
    active: 0,
    priority: 0,
    viewer: 0,
    count: 2,
    players: [
      {
        player: 0,
        life: 40,
        username: "A",
        hand_count: 7,
        library_count: 90,
        lost: false,
        commander_tax: 0,
        mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
      },
      {
        player: 1,
        life: 40,
        username: "B",
        hand_count: 7,
        library_count: 90,
        lost: false,
        commander_tax: 0,
        mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
      },
    ],
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
    attackers: [],
    blocks: [],
    aiming: false,
    targetObjects: new Set(),
    targetPlayers: new Set(),
    canvasDrag: null,
    cursor: { x: 0, y: 0 },
    avatarScreenPositions: { 0: { x: 50, y: 50 }, 1: { x: 200, y: 50 } },
    stepIdx: 3,
    selectedId: null,
    paymentObjects: new Set(),
    stackResponseFocus: false,
    responseObjects: new Set(),
    aimFrom: null,
    viewportW: 800,
    viewportH: 600,
    nowMs: 1000,
    reducedMotion: false,
    ...over,
  };
}

describe("buildBoardScene", () => {
  it("dims non-response permanents during stack response focus", () => {
    const bright = card({ id: 2, x: 220 });
    const scene = buildBoardScene(
      baseInput({
        cards: [card({ id: 1 }), bright],
        stackResponseFocus: true,
        responseObjects: new Set([2]),
      }),
    );
    expect(scene.cards.find((c) => c.card.id === 1)?.dim).toBe(true);
    expect(scene.cards.find((c) => c.card.id === 2)?.dim).toBe(false);
  });

  it("skips cards owned by hideCardIds (flight ownership)", () => {
    const scene = buildBoardScene(
      baseInput({
        cards: [card({ id: 1 }), card({ id: 2, x: 220 })],
        hideCardIds: new Set([1]),
      }),
    );
    expect(scene.cards.map((c) => c.card.id)).toEqual([2]);
  });

  it("builds aim arrow keys and target outlines while aiming", () => {
    const scene = buildBoardScene(
      baseInput({
        aiming: true,
        aimFrom: { x: 10, y: 20 },
        cursor: { x: 100, y: 200 },
        targetObjects: new Set([1]),
      }),
    );
    expect(scene.cards[0]?.outline).toEqual(TARGET_STROKE);
    expect(scene.arrows.some((a) => a.key === "aim")).toBe(true);
  });

  it("outlines selected permanents and staged attackers", () => {
    const selected = buildBoardScene(baseInput({ selectedId: 1 }));
    expect(selected.cards[0]?.outline).toEqual(SELECT_STROKE);

    const staged = buildBoardScene(
      baseInput({
        attackers: [{ attacker: 1, defender: 1 }],
      }),
    );
    expect(staged.cards[0]?.outline).toEqual(ATTACK_STROKE);
    expect(staged.arrows.map((a) => a.key)).toContain("stage-atk-1-1");
  });

  it("omits settled flights and keeps in-flight ones", () => {
    const scene = buildBoardScene(
      baseInput({
        flights: [
          flight({ id: 9, phase: "settled", kind: "stack" }),
          flight({ id: 10, phase: "flying", kind: "battlefield" }),
        ],
      }),
    );
    expect(scene.flights.map((f) => f.id)).toEqual([10]);
  });
});
