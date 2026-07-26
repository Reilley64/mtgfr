import { describe, expect, it } from "vitest";
import { extractProvenance } from "./event-fold";
import type { VisibleEvent } from "./wire/types";

describe("extractProvenance battlefieldExits", () => {
  it("tags BF→graveyard as destroy path", () => {
    const events: VisibleEvent[] = [{ kind: "moved_to_graveyard", card: 10, from: 10 }];
    const priorBf = new Set([10]);
    const p = extractProvenance(events, new Set(), 0, priorBf);
    expect(p.battlefieldExits.get(10)).toBe("graveyard");
    expect(p.moves.get(10)).toBe(10);
  });

  it("tags BF→exile as exile path", () => {
    const events: VisibleEvent[] = [{ kind: "moved_to_exile", card: 11, from: 11 }];
    const p = extractProvenance(events, new Set(), 0, new Set([11]));
    expect(p.battlefieldExits.get(11)).toBe("exile");
  });

  it("does not tag mill or non-BF graveyard entrance", () => {
    const events: VisibleEvent[] = [
      { kind: "milled", card: 12, from: 12, player: 0 },
      { kind: "moved_to_graveyard", card: 13, from: 13 },
    ];
    const p = extractProvenance(events, new Set(), 0, new Set()); // 13 not on BF
    expect(p.battlefieldExits.has(12)).toBe(false);
    expect(p.battlefieldExits.has(13)).toBe(false);
  });

  it("tags when prior BF id is `from` after rebind-style id change", () => {
    const events: VisibleEvent[] = [{ kind: "moved_to_graveyard", card: 20, from: 19 }];
    const p = extractProvenance(events, new Set(), 0, new Set([19]));
    expect(p.battlefieldExits.get(20)).toBe("graveyard");
  });
});
