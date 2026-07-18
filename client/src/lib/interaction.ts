// The board's interaction state machine: the pan/click/attack-drag/block-drag pointer decision,
// combat staging, and camera fitting. Pure — a function of (state, event) like camera.ts — so
// the "was that a pan, a click, an attack, or a block?" question is decidable in a unit test
// with no canvas. Board.tsx owns the DOM/canvas/network; it feeds this module resolved
// hit-tests and executes the intents it returns. Payment (which lands to tap, commander tax,
// pain modes) is the engine's, settled inside the cast — the client plans no taps.

import { boardBounds, type RenderCard, STEP, ZONE } from "~/layout";
import type { Camera } from "~/lib/camera";
import type { ObjectView, VisibleState, WireAttack, WireBlock, WireIntent, WireTarget } from "~/wire/types";

// ── Camera fitting ─────────────────────────────────────────────────────────────────
// Frame the whole table in the space between the turn banner and the hand bar, centered.
// TOP_MARGIN reserves room for the fixed phase-track HUD (Board.tsx's PHASE_TRACK: turn label +
// phase segments + priority watch, ~10px top offset + ~8px padding top/bottom + up to three text
// rows) so the topmost seat's life-orb avatar never renders underneath it. Static estimate, not a
// measured DOM rect (fitCamera is pure/DOM-free) — bump it if the HUD gains rows.
export const TOP_MARGIN = 92;
export function fitCamera(size: { x: number; y: number }, count: number, handBarH: number): Camera {
  const b = boardBounds(count);
  const bw = b.maxX - b.minX;
  const bh = b.maxY - b.minY;
  const availW = Math.max(200, size.x - 32);
  const availH = Math.max(200, size.y - handBarH - TOP_MARGIN - 12);
  // Cap keeps text/orbs readable on ultrawide; below the cap we fill the playable frame.
  const zoom = Math.min(availW / bw, availH / bh, 1.35);
  const panX = (size.x - bw * zoom) / 2 - b.minX * zoom;
  const panY = TOP_MARGIN - b.minY * zoom;
  return { panX, panY, zoom };
}

// ── Pointer state machine: pan vs click vs combat-drag ───────────────────────────────
// A press on your own creature during a combat step starts an attack/block drag; a press on any
// other card is a click candidate; a press on empty space pans. The 3px threshold separates a
// click from a drag. `pan` and `press` both remember where the threshold is measured from, but
// they update it differently: a pan tracks the cursor each move (so releasing after a pan is a
// near-zero delta), a press keeps the original press point (so a real drag exceeds the threshold).

const DRAG_THRESHOLD = 3;

export type PointerPhase =
  | { kind: "idle" }
  | { kind: "pan"; x: number; y: number }
  | { kind: "press"; card: RenderCard; x: number; y: number }
  | { kind: "drag"; card: RenderCard; x: number; y: number; moved: boolean };

export function pointerDown(
  hit: RenderCard | null,
  x: number,
  y: number,
  combatStep: boolean,
  me: number,
): PointerPhase {
  if (hit && hit.kind === "creature" && hit.controller === me && combatStep)
    return { kind: "drag", card: hit, x, y, moved: false };
  if (hit) return { kind: "press", card: hit, x, y };
  return { kind: "pan", x, y };
}

/** Advance on a move. Returns the next phase and, for a pan, the screen delta to apply. */
export function pointerMove(
  phase: PointerPhase,
  x: number,
  y: number,
): { phase: PointerPhase; pan: { dx: number; dy: number } | null } {
  if (phase.kind === "drag") {
    const moved = phase.moved || Math.abs(x - phase.x) + Math.abs(y - phase.y) > DRAG_THRESHOLD;
    return { phase: { ...phase, moved }, pan: null };
  }
  if (phase.kind === "pan") {
    return { phase: { kind: "pan", x, y }, pan: { dx: x - phase.x, dy: y - phase.y } };
  }
  return { phase, pan: null };
}

export type PointerRelease =
  | { kind: "click"; card: RenderCard }
  | { kind: "combat-drop"; card: RenderCard; x: number; y: number }
  | { kind: "none" };

/** Resolve a release. `hitAtUp` is the card under the release point (Board hit-tests it). */
export function pointerUp(phase: PointerPhase, x: number, y: number, hitAtUp: RenderCard | null): PointerRelease {
  if (phase.kind === "drag") {
    if (phase.moved) return { kind: "combat-drop", card: phase.card, x, y };
    return { kind: "click", card: phase.card };
  }
  // press or pan: a click only if released within the threshold of the remembered point, over a
  // card. (A pan remembers the last move point, so releasing over a card after panning still
  // clicks it — a pre-existing quirk, preserved deliberately; see interaction.test.ts.)
  if (phase.kind === "idle") return { kind: "none" };
  if (hitAtUp && Math.abs(x - phase.x) + Math.abs(y - phase.y) <= DRAG_THRESHOLD)
    return { kind: "click", card: hitAtUp };
  return { kind: "none" };
}

// ── Combat staging ───────────────────────────────────────────────────────────────────

/** Stage `from` as an attacker on `defender` (a seat), or null if it can't attack / no avatar was
 * hit. Re-dropping an already-staged attacker retargets it. */
export function attackDrop(
  attackers: WireAttack[],
  from: Pick<RenderCard, "id" | "tapped" | "summoningSick" | "hasHaste">,
  defender: number | null,
): WireAttack[] | null {
  // Can't attack if tapped, or summoning sick without haste (matches the engine).
  if (from.tapped || (from.summoningSick && !from.hasHaste)) return null;
  if (defender == null) return null;
  const rest = attackers.filter((w) => w.attacker !== from.id);
  return [...rest, { attacker: from.id, defender }];
}

/** Stage `blockerId` blocking the creature dropped onto, or null if that card isn't an attacker
 * declared against `me`. Only the attacked player may block an attacker (rule 509.1a) — in a
 * 4-player game most declared attackers are aimed at somebody else's face.
 *
 * Re-dropping an already-staged blocker retargets it, exactly as `attackDrop` retargets an
 * attacker: a creature blocks one attacker unless something says otherwise (CR 509.1a), so
 * appending a second block for the same blocker would stage a declaration the engine rejects
 * wholesale — losing every other block the player had lined up. */
export function blockDrop(
  blocks: WireBlock[],
  blockerId: number,
  target: RenderCard | null,
  declaredAttackers: WireAttack[],
  me: number,
): WireBlock[] | null {
  if (!target) return null;
  if (!declaredAttackers.some((a) => a.attacker === target.id && a.defender === me)) return null;
  const rest = blocks.filter((b) => b.blocker !== blockerId);
  return [...rest, { blocker: blockerId, attacker: target.id }];
}

// ── The single primary board button (Next / confirm attackers / confirm blockers) ──────
// One selector shared by the click path and the Space/Enter keyboard shortcut, so "what does the
// primary button do right now" has exactly one answer.
export type PrimaryAction =
  | { kind: "pass"; label: string }
  | { kind: "confirm-attackers"; label: string }
  | { kind: "confirm-blockers"; label: string };

/** True once this seat's block declaration is on the board. Blocks against attackers aimed at you
 * are yours alone (CR 509.1a), so any such block means you've already declared. */
function blockersDeclaredFor(me: number, declaredAttackers: WireAttack[], declaredBlocks: WireBlock[]): boolean {
  const myAttackers = new Set(declaredAttackers.filter((a) => a.defender === me).map((a) => a.attacker));
  if (myAttackers.size === 0) return false;
  return declaredBlocks.some((b) => myAttackers.has(b.attacker));
}

/**
 * Local latches cover the HTTP→SSE gap; wire `attackers_declared` / `blockers_declared` are the
 * durable source of truth (empty declarations leave combat lists empty).
 */
export type PrimaryActionInput = {
  step: number;
  activePlayer: number;
  me: number;
  attackers?: WireAttack[];
  blocks?: WireBlock[];
  declaredAttackers?: WireAttack[];
  declaredBlocks?: WireBlock[];
  attackersConfirmed?: boolean;
  blockersConfirmed?: boolean;
  attackersDeclared?: boolean;
  blockersDeclared?: boolean;
};

export function primaryActionFor(input: PrimaryActionInput): PrimaryAction {
  const {
    step,
    activePlayer,
    me,
    attackers = [],
    blocks = [],
    declaredAttackers = [],
    declaredBlocks = [],
    attackersConfirmed = false,
    blockersConfirmed = false,
    attackersDeclared = false,
    blockersDeclared = false,
  } = input;
  // Being attacked (not merely "not active") gates blocker confirm — aligned with combatMode /
  // CR 509.1a so an uninvolved seat never gets a phantom Block button.
  const attackingMe = declaredAttackers.some((a) => a.defender === me);
  const attackDone = attackersConfirmed || attackersDeclared || declaredAttackers.length > 0;
  const blockDone = blockersConfirmed || blockersDeclared || blockersDeclaredFor(me, declaredAttackers, declaredBlocks);

  if (step === STEP.DeclareAttackers && activePlayer === me && !attackDone) {
    return attackers.length
      ? { kind: "confirm-attackers", label: `Attack (${attackers.length})` }
      : { kind: "confirm-attackers", label: "No attackers" };
  }
  if (step === STEP.DeclareBlockers && attackingMe && !blockDone) {
    return blocks.length
      ? { kind: "confirm-blockers", label: `Block (${blocks.length})` }
      : { kind: "confirm-blockers", label: "No blockers" };
  }
  // "Draw" is phase chrome: the card is already drawn as a turn-based action; this pass only
  // advances priority (including on a skipped first draw in two-player games).
  if (step === STEP.Draw && activePlayer === me) return { kind: "pass", label: "Draw" };
  return { kind: "pass", label: "Next" };
}

// ── Combat step + click semantics ──────────────────────────────────────────────────────

/** Which combat declaration, if any, this player is in: they declare attackers on their own
 * declare-attackers step, blockers on the attacker's declare-blockers step — but only if somebody
 * is actually attacking *them*. In a 4-player game most attacks are aimed at someone else's face,
 * and only the attacked player may block (CR 509.1a), so an uninvolved seat gets no blocker
 * affordance: dragging their creatures around would look like a legal move that never lands.
 *
 * One source of truth for the "is this a combat step for me?" check that combatStep(), onCombatDrop
 * and resolveClick share. */
export type CombatMode = "attackers" | "blockers" | null;

/** Optional declaration-final flags — without them, empty declares leave lists empty and staging
 * stays live after the engine has already closed the declaration. */
export type CombatDeclaration = {
  attackersDeclared?: boolean;
  blockersDeclared?: boolean;
};

export function combatMode(
  step: number,
  isActive: boolean,
  spectating: boolean,
  declaredAttackers: WireAttack[],
  me: number,
  declaration: CombatDeclaration = {},
): CombatMode {
  if (spectating) return null;
  const attackersDeclared = declaration.attackersDeclared ?? false;
  const blockersDeclared = declaration.blockersDeclared ?? false;
  if (step === STEP.DeclareAttackers && isActive && !attackersDeclared) return "attackers";
  if (
    step === STEP.DeclareBlockers &&
    !isActive &&
    !blockersDeclared &&
    declaredAttackers.some((a) => a.defender === me)
  ) {
    return "blockers";
  }
  return null;
}

/** What a click on a board card means, as data. Board's onClickCard is then a dumb switch that
 * only *fires* these (no rules left in the view). */
export type ClickResult =
  | { kind: "expand"; zone: number; owner: number } // a graveyard/exile pile → open the overlay
  | { kind: "cast"; card: ObjectView; target: WireTarget | null } // commander recast, or a staged spell's target chosen
  | { kind: "cancel-attacker"; id: number } // un-stage a creature you'd declared as an attacker
  | { kind: "cancel-blocker"; id: number } // un-stage a declared blocker
  | { kind: "intent"; intent: WireIntent } // fire this intent as-is
  | { kind: "select"; id: number } // select your permanent for the activation radial
  | { kind: "none" };

/** Context the click decision needs beyond the state: what the player has staged in the UI. */
export interface ClickContext {
  spectating: boolean;
  staged: ObjectView | null; // a targeted spell awaiting its target
  /** Object ids the staged spell may legally target, from the engine's own enumeration
   * (`ActionView.targets`; see lib/targeting.ts). Empty when nothing is staged. */
  stagedTargets: ReadonlySet<number>;
  attackers: WireAttack[]; // creatures staged as attackers this declaration
  blocks: WireBlock[]; // creatures staged as blockers this declaration
}

export function resolveClick(state: VisibleState | null, me: number, card: RenderCard, ctx: ClickContext): ClickResult {
  // A pile (graveyard/exile stand-in) expands for anyone, spectators included.
  if (card.pile > 0) return { kind: "expand", zone: card.zone, owner: card.owner };
  if (ctx.spectating) return { kind: "none" }; // a spectator inspects piles but takes no action

  // Completing a staged targeted spell: click one of the targets the engine says is legal. Which
  // cards those are is the engine's answer (`ActionView.targets`), not a rule re-derived here — so
  // a spell that targets an artifact, a land, or a creature you don't control aims exactly as well
  // as one that targets a creature. (A legal *player* target is clicked on their life orb, which is
  // not a card and so never reaches this function.)
  if (ctx.staged) {
    if (ctx.stagedTargets.has(card.id))
      return { kind: "cast", card: ctx.staged, target: { kind: "object", id: card.id } };
    return { kind: "none" };
  }

  // Recast your commander from the command zone (the engine folds in the commander tax).
  if (card.zone === ZONE.Command && card.isCommander && card.owner === me) {
    const cmdr = state?.objects.find((o) => o.id === card.id);
    if (!cmdr) return { kind: "none" };
    return { kind: "cast", card: cmdr, target: null };
  }

  if (card.zone !== ZONE.Battlefield || card.controller !== me) return { kind: "none" };

  // Clicking a creature you've staged as an attacker/blocker cancels it.
  const mode = combatMode(state?.step ?? -1, state?.active_player === me, false, state?.combat.attackers ?? [], me, {
    attackersDeclared: state?.combat.attackers_declared ?? false,
    blockersDeclared: state?.combat.blockers_declared?.includes(me) ?? false,
  });
  if (mode === "attackers" && ctx.attackers.some((w) => w.attacker === card.id))
    return { kind: "cancel-attacker", id: card.id };
  if (mode === "blockers" && ctx.blocks.some((b) => b.blocker === card.id))
    return { kind: "cancel-blocker", id: card.id };

  // Select your permanent — tap-for-mana and activates live on the activation radial, not as a
  // raw board click (one-click auto-tap fought select + on-permanent chips).
  return { kind: "select", id: card.id };
}
