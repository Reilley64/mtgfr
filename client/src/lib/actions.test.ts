import { describe, expect, it } from "vitest";
import type { ActionView } from "~/api/generated";
import { autoTapPreviewIds, byObject, bySection, handExtras } from "~/lib/actions";

function mkAction(overrides: Partial<ActionView> = {}): ActionView {
  return { id: 0, kind: "cast", label: "Card", needs_target: false, section: "hand", ...overrides };
}

describe("bySection", () => {
  it("buckets actions by their section and defaults every section to []", () => {
    const g = bySection([
      mkAction({ id: 1, section: "hand" }),
      mkAction({ id: 2, section: "command" }),
      mkAction({ id: 3, section: "hand" }),
      mkAction({ id: 4, section: "battlefield", kind: "activate" }),
    ]);
    expect(g.hand.map((a) => a.id)).toEqual([1, 3]);
    expect(g.command.map((a) => a.id)).toEqual([2]);
    expect(g.battlefield.map((a) => a.id)).toEqual([4]);
    expect(g.graveyard).toEqual([]);
    expect(g.exile).toEqual([]);
    expect(g.combat).toEqual([]);
  });

  it("tolerates undefined and drops unknown sections", () => {
    expect(bySection(undefined).hand).toEqual([]);
    const g = bySection([mkAction({ section: "somethingNew" })]);
    expect(Object.values(g).every((v) => v.length === 0)).toBe(true);
  });
});

describe("byObject", () => {
  it("indexes by object id, prefers cast/play_land over cycle, skips objectless actions", () => {
    const m = byObject([
      mkAction({ id: 1, object: 10, kind: "cycle" }),
      mkAction({ id: 2, object: 10, kind: "play_land" }),
      mkAction({ id: 3, object: 11 }),
      mkAction({ id: 4, object: null, kind: "declare_attackers" }),
    ]);
    expect(m.get(10)?.id).toBe(2);
    expect(m.get(11)?.id).toBe(3);
    expect(m.size).toBe(2);
  });
});

describe("handExtras", () => {
  it("returns alternative actions overshadowed by a play action on the same object", () => {
    const play = mkAction({ id: 1, object: 10, kind: "play_land" });
    const cycle = mkAction({ id: 2, object: 10, kind: "cycle", label: "Cycle: Forest" });
    expect(handExtras([play, cycle])).toEqual([cycle]);
    expect(handExtras([cycle])).toEqual([]);
  });

  it("surfaces suspend and discard-ability siblings beside a cast on the same card", () => {
    const cast = mkAction({ id: 1, object: 20, kind: "cast", label: "Magma Opus" });
    const suspend = mkAction({ id: 2, object: 20, kind: "suspend", label: "Suspend: Magma Opus" });
    const discard = mkAction({ id: 3, object: 20, kind: "activate_hand_ability", label: "Discard: Magma Opus" });
    expect(handExtras([cast, suspend, discard])).toEqual([suspend, discard]);
  });
});

describe("autoTapPreviewIds", () => {
  it("returns the action's auto_tap set, empty when absent", () => {
    expect([...autoTapPreviewIds(null)]).toEqual([]);
    expect([...autoTapPreviewIds(mkAction())]).toEqual([]);
    expect([...autoTapPreviewIds(mkAction({ auto_tap: [3, 7] }))].sort()).toEqual([3, 7]);
  });
});
