import { describe, expect, it } from "vitest";
import type { ObjectView, VisibleState } from "~/api/generated";
import { AVATAR_R, avatarPos, boardBounds, type RenderCard, ZONE } from "~/layout";
import { worldToScreen } from "~/lib/camera";
import {
  attackDrop,
  blockDrop,
  combatMode,
  fitCamera,
  type PointerPhase,
  pointerDown,
  pointerMove,
  pointerUp,
  primaryActionFor,
  resolveClick,
  TOP_MARGIN,
} from "~/lib/interaction";

const MAIN_1 = 3;

// A minimal RenderCard; override the fields a test cares about.
function card(over: Partial<RenderCard> = {}): RenderCard {
  return {
    id: 1,
    x: 0,
    y: 0,
    w: 96,
    h: 134,
    name: "c",
    cardId: "",
    print: "",
    pt: "",
    tapped: false,
    counters: 0,
    markedDamage: 0,
    faceDown: false,
    zone: ZONE_BATTLEFIELD,
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
    ...over,
  };
}
const ZONE_BATTLEFIELD = 2;

describe("pointer state machine", () => {
  it("presses on empty space pan", () => {
    const p = pointerDown(null, 10, 10, false, 0);
    expect(p.kind).toBe("pan");
  });

  it("a press on your creature in a combat step starts a drag", () => {
    const p = pointerDown(card({ controller: 0, kind: "creature" }), 10, 10, true, 0);
    expect(p.kind).toBe("drag");
  });

  it("a creature you don't control is a click candidate, not a drag", () => {
    const p = pointerDown(card({ controller: 1, kind: "creature" }), 10, 10, true, 0);
    expect(p.kind).toBe("press");
  });

  it("a creature is a click candidate outside a combat step", () => {
    const p = pointerDown(card({ controller: 0 }), 10, 10, false, 0);
    expect(p.kind).toBe("press");
  });

  it("a press-then-release under the 3px threshold is a click", () => {
    const down = pointerDown(card({ id: 7 }), 100, 100, false, 0);
    const rel = pointerUp(down, 102, 100, card({ id: 7 }));
    expect(rel).toEqual({ kind: "click", card: card({ id: 7 }) });
  });

  it("a press dragged past 3px is not a click", () => {
    const down = pointerDown(card({ id: 7 }), 100, 100, false, 0);
    const rel = pointerUp(down, 110, 100, card({ id: 7 }));
    expect(rel.kind).toBe("none");
  });

  it("panning emits the per-move screen delta", () => {
    const phase: PointerPhase = pointerDown(null, 100, 100, false, 0);
    const a = pointerMove(phase, 130, 120);
    expect(a.pan).toEqual({ dx: 30, dy: 20 });
    // deltas are incremental: the next move measures from the last, not the press.
    const b = pointerMove(a.phase, 140, 120);
    expect(b.pan).toEqual({ dx: 10, dy: 0 });
  });

  it("a drag past the threshold releases as a combat-drop; under it, a click (tap-in-place)", () => {
    const start = pointerDown(card({ id: 3, controller: 0 }), 50, 50, true, 0);
    const dragged = pointerMove(start, 80, 50).phase; // moved 30px
    expect(pointerUp(dragged, 80, 50, null)).toMatchObject({ kind: "combat-drop", card: { id: 3 } });

    const tapped = pointerMove(start, 51, 50).phase; // moved 1px — a click
    expect(pointerUp(tapped, 51, 50, null)).toMatchObject({ kind: "click", card: { id: 3 } });
  });

  it("moved stays sticky once the threshold is crossed", () => {
    const start = pointerDown(card({ controller: 0 }), 50, 50, true, 0);
    const far = pointerMove(start, 90, 50).phase;
    const back = pointerMove(far, 50, 50).phase; // returned to origin
    expect(pointerUp(back, 50, 50, null).kind).toBe("combat-drop");
  });

  // Pinned quirk: a pan tracks the cursor, so releasing over a card right after panning STILL
  // clicks it (the threshold is measured from the last move, ~0 away). Looks accidental — flagged
  // for follow-up — but preserved to keep behavior identical.
  it("releasing over a card immediately after a pan clicks it (pinned quirk)", () => {
    const down = pointerDown(null, 100, 100, false, 0);
    const panned = pointerMove(down, 300, 300).phase;
    const rel = pointerUp(panned, 300, 300, card({ id: 9 }));
    expect(rel).toMatchObject({ kind: "click", card: { id: 9 } });
  });
});

// Cast timing, payment planning, and commander tax are the engine's now (it settles the whole
// payment inside the cast, auto-tapping lands after full validation) — no client plan to test.

describe("combat staging", () => {
  it("stages an attacker on the dropped-over avatar", () => {
    const next = attackDrop([], card({ id: 3 }), 1);
    expect(next).toEqual([{ attacker: 3, defender: 1 }]);
  });

  it("re-dropping an attacker retargets it (no duplicate)", () => {
    const next = attackDrop([{ attacker: 3, defender: 1 }], card({ id: 3 }), 2);
    expect(next).toEqual([{ attacker: 3, defender: 2 }]);
  });

  it("won't attack tapped or summoning-sick (without haste), or with no avatar hit", () => {
    expect(attackDrop([], card({ id: 3, tapped: true }), 1)).toBeNull();
    expect(attackDrop([], card({ id: 3, summoningSick: true }), 1)).toBeNull();
    expect(attackDrop([], card({ id: 3, summoningSick: true, hasHaste: true }), 1)).toEqual([
      { attacker: 3, defender: 1 },
    ]);
    expect(attackDrop([], card({ id: 3 }), null)).toBeNull();
  });

  it("blocks only a declared attacker", () => {
    const attackers = [{ attacker: 7, defender: 0 }];
    expect(blockDrop([], 3, card({ id: 7 }), attackers, 0)).toEqual([{ blocker: 3, attacker: 7 }]);
    expect(blockDrop([], 3, card({ id: 8 }), attackers, 0)).toBeNull();
    expect(blockDrop([], 3, null, attackers, 0)).toBeNull();
  });

  it("blocks only an attacker declared against you (multiplayer)", () => {
    // Seat 2 is attacking seat 1. Seat 0 is a bystander and may not block for them.
    const attackers = [{ attacker: 7, defender: 1 }];
    expect(blockDrop([], 3, card({ id: 7 }), attackers, 1)).toEqual([{ blocker: 3, attacker: 7 }]);
    expect(blockDrop([], 3, card({ id: 7 }), attackers, 0)).toBeNull();
  });

  it("re-dropping a staged blocker retargets it instead of blocking twice", () => {
    // A creature blocks one attacker (CR 509.1a). Appending a second block would stage a
    // declaration the engine rejects wholesale, silently losing the other blocks too.
    const attackers = [
      { attacker: 7, defender: 0 },
      { attacker: 8, defender: 0 },
    ];
    const staged = blockDrop([], 3, card({ id: 7 }), attackers, 0);
    expect(staged).toBeDefined();
    if (!staged) throw new Error("expected staged block");
    expect(blockDrop(staged, 3, card({ id: 8 }), attackers, 0)).toEqual([{ blocker: 3, attacker: 8 }]);
  });

  it("a second blocker joins the first rather than replacing it (double block)", () => {
    const attackers = [{ attacker: 7, defender: 0 }];
    const staged = blockDrop([], 3, card({ id: 7 }), attackers, 0);
    expect(staged).toBeDefined();
    if (!staged) throw new Error("expected staged block");
    expect(blockDrop(staged, 4, card({ id: 7 }), attackers, 0)).toEqual([
      { blocker: 3, attacker: 7 },
      { blocker: 4, attacker: 7 },
    ]);
  });
});

describe("fitCamera", () => {
  it("frames the board and never over-zooms past 1.1", () => {
    const cam = fitCamera({ x: 5000, y: 5000 }, 4, 210);
    expect(cam.zoom).toBeLessThanOrEqual(1.1);
    expect(cam.zoom).toBeGreaterThan(0);
  });

  // A 2-player table is a single column (you + the seat across from you) — much narrower than the
  // 2x2 grid a 3- or 4-player table needs — so fitting it should zoom in further, not leave it
  // shrunk into a quadrant of the viewport (the P1 bug: the camera was fit assuming 4 players
  // regardless of the actual seat count). Three-row seat bands make 2p and 4p the same *height*,
  // so at typical aspect ratios height binds first and zoom equalizes; use a tall viewport so the
  // width difference is what the assertion measures.
  it("fits a 2-player table tighter (higher zoom) than a 4-player table at the same viewport", () => {
    const size = { x: 2000, y: 4000 };
    const zoom2 = fitCamera(size, 2, 210).zoom;
    const zoom4 = fitCamera(size, 4, 210).zoom;
    expect(zoom2).toBeGreaterThan(zoom4);
  });

  // The phase-track HUD is fixed top-center; the top-row seat's avatar must never render under it.
  // fitCamera always places the board's topmost world point (the top-row avatar's top edge) at
  // screen y = TOP_MARGIN, so this holds by construction for any player count — pinned here so a
  // future refactor of boardBounds/fitCamera can't silently reintroduce the overlap.
  it("never overlaps the top-row avatar with the reserved HUD margin, for 2/3/4 players", () => {
    for (const count of [2, 3, 4]) {
      const size = { x: 1600, y: 1000 };
      const cam = fitCamera(size, count, 210);
      const bounds = boardBounds(count);
      // Find whichever seat sits topmost (row 0, if any occupied) and check its avatar's top edge.
      for (let seat = 0; seat < count; seat++) {
        const a = avatarPos(seat, 0, count);
        if (a.y - AVATAR_R !== bounds.minY) continue; // not the top-row seat
        const screenTop = worldToScreen(cam, a.x, a.y - AVATAR_R).y;
        expect(screenTop).toBeGreaterThanOrEqual(TOP_MARGIN - 0.01); // float slack
      }
    }
  });
});

const DECLARE_ATTACKERS = 5;
const DECLARE_BLOCKERS = 6;

describe("combatMode", () => {
  const atMe = [{ attacker: 7, defender: 0 }]; // an attacker declared against seat 0

  it("is 'attackers' on your declare-attackers step, 'blockers' on the opponent's declare-blockers", () => {
    expect(combatMode(DECLARE_ATTACKERS, true, false, [], 0)).toBe("attackers");
    expect(combatMode(DECLARE_BLOCKERS, false, false, atMe, 0)).toBe("blockers");
  });
  it("is null off-combat, on the wrong side, or while spectating", () => {
    expect(combatMode(DECLARE_ATTACKERS, false, false, [], 0)).toBeNull(); // not your attackers step
    expect(combatMode(DECLARE_BLOCKERS, true, false, atMe, 0)).toBeNull(); // your own turn, no blocks
    expect(combatMode(MAIN_1, true, false, atMe, 0)).toBeNull(); // not a combat step
    expect(combatMode(DECLARE_ATTACKERS, true, true, [], 0)).toBeNull(); // spectator
    // A 4-player table: seat 2 is attacking seat 1, so seat 0 has nothing to block (CR 509.1a) and
    // gets no blocker affordance at all — no drag, no arrow, no phantom "Block (0)".
    expect(combatMode(DECLARE_BLOCKERS, false, false, [{ attacker: 7, defender: 1 }], 0)).toBeNull();
    expect(combatMode(DECLARE_BLOCKERS, false, false, [], 0)).toBeNull(); // nobody has attacked yet
  });
  it("is null once the attack or block declaration is final (including empty)", () => {
    expect(combatMode(DECLARE_ATTACKERS, true, false, [], 0, { attackersDeclared: true })).toBeNull();
    expect(combatMode(DECLARE_BLOCKERS, false, false, atMe, 0, { blockersDeclared: true })).toBeNull();
  });
});

describe("resolveClick", () => {
  function state(over: Partial<VisibleState> = {}): VisibleState {
    return {
      active_player: 0,
      can_act: true,
      combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
      objects: [],
      players: [],
      priority: 0,
      stack: [],
      step: MAIN_1,
      viewer: 0,
      ...over,
    };
  }
  const noCtx = { spectating: false, staged: null, stagedTargets: new Set<number>(), attackers: [], blocks: [] };
  const commander: ObjectView = {
    controller: 0,
    has_haste: false,
    id: 50,
    is_commander: true,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { generic: 1, colored: [1, 0, 0, 0, 0] },
    marked_damage: 0,
    name: "Cmdr",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 2,
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: ZONE.Command,
  };

  it("expands a pile — even for a spectator", () => {
    const pile = card({ id: -1, pile: 5, zone: ZONE.Graveyard, owner: 3 });
    expect(resolveClick(state(), 0, pile, { ...noCtx, spectating: true })).toEqual({
      kind: "expand",
      zone: ZONE.Graveyard,
      owner: 3,
    });
  });

  it("a spectator's click on a non-pile card does nothing", () => {
    expect(resolveClick(state(), 0, card(), { ...noCtx, spectating: true })).toEqual({ kind: "none" });
  });

  it("a permanent cluster selects the top member — it is not a pile expand", () => {
    const cluster = card({
      id: 10,
      controller: 0,
      zone: ZONE.Battlefield,
      cluster: 4,
      clusterMembers: [10, 11, 12, 13],
    });
    expect(resolveClick(state(), 0, cluster, noCtx)).toEqual({ kind: "select", id: 10 });
  });

  it("a staged spell casts at a clicked card the engine listed as legal", () => {
    const staged = commander; // any ObjectView; it's the spell being cast
    const target = card({ id: 9, zone: ZONE.Battlefield, kind: "creature" });
    expect(resolveClick(state(), 0, target, { ...noCtx, staged, stagedTargets: new Set([9]) })).toEqual({
      kind: "cast",
      card: staged,
      target: { kind: "object", id: 9 },
    });
  });

  it("a staged spell aims at any legal permanent, not just creatures", () => {
    // Abrade targets an artifact; Beast Within targets any nonland permanent. The card's *kind* is
    // not the question — whether the engine listed it is.
    const staged = commander;
    const artifact = card({ id: 9, zone: ZONE.Battlefield, kind: "artifact" });
    expect(resolveClick(state(), 0, artifact, { ...noCtx, staged, stagedTargets: new Set([9]) })).toEqual({
      kind: "cast",
      card: staged,
      target: { kind: "object", id: 9 },
    });
  });

  it("a staged spell clicked onto a card the engine didn't list does nothing (stays staged)", () => {
    const creature = card({ id: 9, zone: ZONE.Battlefield, kind: "creature" });
    expect(resolveClick(state(), 0, creature, { ...noCtx, staged: commander, stagedTargets: new Set([4]) })).toEqual({
      kind: "none",
    });
  });

  it("recasts your commander from the command zone (the engine settles the tax)", () => {
    const s = state({ objects: [commander] });
    const cmdrCard = card({ id: 50, zone: ZONE.Command, isCommander: true, owner: 0, controller: 0 });
    expect(resolveClick(s, 0, cmdrCard, noCtx)).toEqual({ kind: "cast", card: commander, target: null });
  });

  it("won't recast another player's commander", () => {
    const cmdrCard = card({ id: 50, zone: ZONE.Command, isCommander: true, owner: 1, controller: 1 });
    expect(resolveClick(state({ objects: [commander] }), 0, cmdrCard, noCtx)).toEqual({ kind: "none" });
  });

  it("cancels a staged attacker when you click it on your declare-attackers step", () => {
    const s = state({ step: DECLARE_ATTACKERS, active_player: 0 });
    const creature = card({ id: 3, zone: ZONE.Battlefield, controller: 0, kind: "creature" });
    expect(resolveClick(s, 0, creature, { ...noCtx, attackers: [{ attacker: 3, defender: 1 }] })).toEqual({
      kind: "cancel-attacker",
      id: 3,
    });
  });

  it("cancels a staged blocker when you click it on the opponent's declare-blockers step", () => {
    // Creature 7 is attacking seat 0 — without that there'd be nothing to block, and no blocker
    // could have been staged in the first place.
    const s = state({
      step: DECLARE_BLOCKERS,
      active_player: 1,
      combat: {
        attackers: [{ attacker: 7, defender: 0 }],
        blocks: [],
        attackers_declared: true,
        blockers_declared: [],
      },
    });
    const creature = card({ id: 3, zone: ZONE.Battlefield, controller: 0, kind: "creature" });
    expect(resolveClick(s, 0, creature, { ...noCtx, blocks: [{ blocker: 3, attacker: 7 }] })).toEqual({
      kind: "cancel-blocker",
      id: 3,
    });
  });

  it("selects an untapped mana source you control (tap lives on the radial)", () => {
    const land = card({ id: 4, zone: ZONE.Battlefield, controller: 0, kind: "land", tapsForMana: true });
    expect(resolveClick(state(), 0, land, noCtx)).toEqual({ kind: "select", id: 4 });
  });

  it("selects a mana rock or dork you control", () => {
    const ring = card({ id: 5, zone: ZONE.Battlefield, controller: 0, kind: "artifact", tapsForMana: true });
    expect(resolveClick(state(), 0, ring, noCtx)).toEqual({ kind: "select", id: 5 });
  });

  it("selects a permanent you control that makes no mana", () => {
    const creature = card({ id: 6, zone: ZONE.Battlefield, controller: 0, kind: "creature" });
    expect(resolveClick(state(), 0, creature, noCtx)).toEqual({ kind: "select", id: 6 });
  });

  it("selects an already-tapped mana source (radial omits tap)", () => {
    const land = card({ id: 4, zone: ZONE.Battlefield, controller: 0, kind: "land", tapsForMana: true, tapped: true });
    expect(resolveClick(state(), 0, land, noCtx)).toEqual({ kind: "select", id: 4 });
  });

  it("does nothing for a battlefield card you don't control", () => {
    const enemy = card({ id: 8, zone: ZONE.Battlefield, controller: 1, kind: "creature" });
    expect(resolveClick(state(), 0, enemy, noCtx)).toEqual({ kind: "none" });
  });
});

describe("primaryActionFor", () => {
  const attack = [{ attacker: 1, defender: 1 }];
  const atMe = [{ attacker: 1, defender: 0 }];
  const atOther = [{ attacker: 1, defender: 2 }];
  const block = [{ blocker: 2, attacker: 1 }];
  const DRAW = 2;

  it("defaults to pass-priority ('Next') off-combat", () => {
    expect(primaryActionFor({ step: MAIN_1, activePlayer: 0, me: 0 })).toEqual({
      kind: "pass",
      label: "Next",
    });
  });
  it("labels the draw step 'Draw' on your turn (phase chrome after the TBA draw)", () => {
    expect(primaryActionFor({ step: DRAW, activePlayer: 0, me: 0 })).toEqual({
      kind: "pass",
      label: "Draw",
    });
    expect(primaryActionFor({ step: DRAW, activePlayer: 1, me: 0 })).toEqual({
      kind: "pass",
      label: "Next",
    });
  });
  it("confirms staged attackers on your own declare-attackers step", () => {
    expect(primaryActionFor({ step: DECLARE_ATTACKERS, activePlayer: 0, me: 0, attackers: attack })).toEqual({
      kind: "confirm-attackers",
      label: "Attack (1)",
    });
  });
  it("confirms an empty attack declaration as 'No attackers'", () => {
    expect(primaryActionFor({ step: DECLARE_ATTACKERS, activePlayer: 0, me: 0 })).toEqual({
      kind: "confirm-attackers",
      label: "No attackers",
    });
  });
  it("confirms staged blockers when you're being attacked", () => {
    expect(
      primaryActionFor({
        step: DECLARE_BLOCKERS,
        activePlayer: 1,
        me: 0,
        blocks: block,
        declaredAttackers: atMe,
      }),
    ).toEqual({
      kind: "confirm-blockers",
      label: "Block (1)",
    });
  });
  it("confirms an empty block declaration as 'No blockers' when you're being attacked", () => {
    expect(
      primaryActionFor({
        step: DECLARE_BLOCKERS,
        activePlayer: 1,
        me: 0,
        declaredAttackers: atMe,
      }),
    ).toEqual({
      kind: "confirm-blockers",
      label: "No blockers",
    });
  });
  it("stays 'Next' once attackers are on the board, or after a local confirm (SSE lag)", () => {
    expect(
      primaryActionFor({
        step: DECLARE_ATTACKERS,
        activePlayer: 0,
        me: 0,
        declaredAttackers: attack,
      }),
    ).toEqual({ kind: "pass", label: "Next" });
    expect(
      primaryActionFor({
        step: DECLARE_ATTACKERS,
        activePlayer: 0,
        me: 0,
        attackersConfirmed: true,
      }),
    ).toEqual({
      kind: "pass",
      label: "Next",
    });
  });
  it("stays 'Next' once the server reports attackers_declared (empty declare)", () => {
    // Empty declare leaves combat.attackers empty; without the wire flag the button sticks on
    // "No attackers" and every retry is IllegalDeclaration.
    expect(
      primaryActionFor({
        step: DECLARE_ATTACKERS,
        activePlayer: 0,
        me: 0,
        attackersDeclared: true,
      }),
    ).toEqual({
      kind: "pass",
      label: "Next",
    });
  });
  it("stays 'Next' once your blocks are on the board, or after a local confirm", () => {
    expect(
      primaryActionFor({
        step: DECLARE_BLOCKERS,
        activePlayer: 1,
        me: 0,
        declaredAttackers: atMe,
        declaredBlocks: block,
      }),
    ).toEqual({
      kind: "pass",
      label: "Next",
    });
    expect(
      primaryActionFor({
        step: DECLARE_BLOCKERS,
        activePlayer: 1,
        me: 0,
        declaredAttackers: atMe,
        blockersConfirmed: true,
      }),
    ).toEqual({
      kind: "pass",
      label: "Next",
    });
  });
  it("stays 'Next' once the server reports blockers_declared (empty declare)", () => {
    expect(
      primaryActionFor({
        step: DECLARE_BLOCKERS,
        activePlayer: 1,
        me: 0,
        declaredAttackers: atMe,
        blockersDeclared: true,
      }),
    ).toEqual({ kind: "pass", label: "Next" });
  });
  it("stays 'Next' on the wrong side of combat or when attacks aim at someone else", () => {
    expect(primaryActionFor({ step: DECLARE_ATTACKERS, activePlayer: 1, me: 0, attackers: attack })).toEqual({
      kind: "pass",
      label: "Next",
    });
    expect(primaryActionFor({ step: DECLARE_BLOCKERS, activePlayer: 0, me: 0, blocks: block })).toEqual({
      kind: "pass",
      label: "Next",
    });
    expect(
      primaryActionFor({
        step: DECLARE_BLOCKERS,
        activePlayer: 1,
        me: 0,
        declaredAttackers: atOther,
      }),
    ).toEqual({
      kind: "pass",
      label: "Next",
    });
  });
});
