import { describe, expect, it } from "vitest";
import { STEP } from "~/layout";
import {
  activatableBattlefieldIds,
  canActOnStack,
  fullBattlefieldWindow,
  instantPriorityFocus,
  type ManaSourceCard,
  showStackYieldArm,
  showStackYieldArmed,
  stackChrome,
  viewerIsHelpless,
} from "~/lib/stackResponse";
import type { ActionView } from "~/wire/types";

const activate = (object: number, over: Partial<ActionView> = {}): ActionView =>
  ({
    id: object,
    kind: "activate",
    object,
    ability_index: 0,
    section: "battlefield",
    label: "Activate",
    needs_target: false,
    targets: [],
    ...over,
  }) as unknown as ActionView;

const land = (id: number, over: Partial<ManaSourceCard> = {}): ManaSourceCard => ({
  id,
  controller: 0,
  tapsForMana: true,
  tapped: false,
  ...over,
});

describe("fullBattlefieldWindow", () => {
  it("covers empty-stack main for the active viewer", () => {
    expect(fullBattlefieldWindow(0, 0, 0, STEP.Main1)).toBe(true);
    expect(fullBattlefieldWindow(0, 0, 0, STEP.Main2)).toBe(true);
    expect(fullBattlefieldWindow(1, 0, 0, STEP.Main1)).toBe(false);
    expect(fullBattlefieldWindow(0, 1, 0, STEP.Main1)).toBe(false);
    expect(fullBattlefieldWindow(0, 0, 0, STEP.Upkeep)).toBe(false);
  });

  it("covers empty-stack declare attackers and blockers for any seat", () => {
    expect(fullBattlefieldWindow(0, 0, 0, STEP.DeclareAttackers)).toBe(true);
    expect(fullBattlefieldWindow(0, 0, 1, STEP.DeclareBlockers)).toBe(true);
    expect(fullBattlefieldWindow(1, 0, 0, STEP.DeclareAttackers)).toBe(false);
  });
});

describe("instantPriorityFocus", () => {
  const base = {
    canAct: true,
    viewer: 0,
    priority: 0,
    active: 0,
    stackLen: 0,
    step: STEP.Main1,
  };

  it("stays off during my empty-stack main and combat declaration", () => {
    expect(instantPriorityFocus(base)).toBe(false);
    expect(instantPriorityFocus({ ...base, step: STEP.DeclareAttackers })).toBe(false);
  });

  it("turns on for stack / opponent turn / instant-only steps", () => {
    expect(instantPriorityFocus({ ...base, stackLen: 1 })).toBe(true);
    expect(instantPriorityFocus({ ...base, active: 1 })).toBe(true);
    expect(instantPriorityFocus({ ...base, step: STEP.Upkeep })).toBe(true);
  });

  it("never focuses for a spectator (even if me() falls back to seat 0)", () => {
    expect(instantPriorityFocus({ ...base, stackLen: 1, spectating: true })).toBe(false);
  });
});

describe("canActOnStack / stack yield arm", () => {
  const act = {
    spectating: false,
    stackLen: 1,
    canAct: true,
    viewer: 0,
    priority: 0,
    actions: [activate(1)],
  };

  it("is true only while this seat can meaningfully act on a non-empty stack", () => {
    expect(canActOnStack(act)).toBe(true);
    expect(canActOnStack({ ...act, actions: [] })).toBe(false);
    expect(canActOnStack({ ...act, priority: 1 })).toBe(false);
    expect(canActOnStack({ ...act, stackLen: 0 })).toBe(false);
    expect(canActOnStack({ ...act, spectating: true })).toBe(false);
  });

  it("arms once then shows disabled armed state with no cancel", () => {
    expect(showStackYieldArm({ ...act, staged: false, yielded: false })).toBe(true);
    expect(showStackYieldArmed({ ...act, staged: false, yielded: false })).toBe(false);

    expect(showStackYieldArm({ ...act, staged: false, yielded: true, actions: [] })).toBe(false);
    expect(showStackYieldArmed({ ...act, staged: false, yielded: true })).toBe(true);

    expect(showStackYieldArm({ ...act, staged: true, yielded: false })).toBe(false);
    expect(showStackYieldArmed({ ...act, staged: true, yielded: true })).toBe(false);
  });
});

describe("activatableBattlefieldIds", () => {
  const actions = [activate(7), activate(3, { section: "hand", kind: "cast", label: "Shock" })];
  const mana = [land(10), land(11, { tapped: true })];

  it("lists activates and untapped mana while focused", () => {
    expect([...activatableBattlefieldIds(actions, true, mana, 0)].sort((a, b) => a - b)).toEqual([7, 10]);
    expect(activatableBattlefieldIds(actions, false, mana, 0).size).toBe(0);
  });
});

describe("viewerIsHelpless", () => {
  it("matches an empty projected action list", () => {
    expect(viewerIsHelpless(undefined)).toBe(true);
    expect(viewerIsHelpless([])).toBe(true);
    expect(viewerIsHelpless([{ id: 1 }])).toBe(false);
  });
});

describe("stackChrome", () => {
  const base = {
    spectating: false,
    staged: false,
    yielded: false,
    stackLen: 1,
    holdRemainingMs: 0,
    canAct: true,
    viewer: 0,
    priority: 0,
    active: 0,
    step: STEP.Main1,
    actions: [activate(1)],
    manaSources: [] as ManaSourceCard[],
  };

  it("keeps Pass and Space in sync while the seat can act on the stack", () => {
    const chrome = stackChrome(base);
    expect(chrome.pass).toBe(true);
    expect(chrome.spaceOnStack).toBe("pass_priority");
    expect(chrome.hideControlsPass).toBe(true);
    expect(chrome.stackYieldArm).toBe(true);
    expect(chrome.stackYieldArmed).toBe(false);
  });

  it("shows armed stack yield as disabled, not as a cancel toggle", () => {
    const chrome = stackChrome({ ...base, yielded: true, actions: [] });
    expect(chrome.stackYieldArm).toBe(false);
    expect(chrome.stackYieldArmed).toBe(true);
    expect(chrome.pass).toBe(false);
  });

  it("blocks Pass / yield arm while a local target is staged (even if aim draw is suspended)", () => {
    const chrome = stackChrome({ ...base, staged: true });
    expect(chrome.pass).toBe(false);
    expect(chrome.spaceOnStack).toBe("ignore");
    expect(chrome.stackYieldArm).toBe(false);
  });

  it("allows dwell only for a helpless seat during hold", () => {
    expect(stackChrome({ ...base, actions: [], holdRemainingMs: 800 }).allowDwell).toBe(true);
    expect(stackChrome({ ...base, actions: [], holdRemainingMs: 0 }).allowDwell).toBe(false);
    expect(stackChrome({ ...base, actions: [], holdRemainingMs: 800, spectating: true }).allowDwell).toBe(false);
  });
});
