import * as Match from "effect/Match";
import type { ObjectView, VisibleState, WireAttack, WireBand, WireBlock, WireIntent } from "~/wire/types";
import { attackablePlaneswalker, attackDrop, blockDrop, type CombatMode, type PrimaryAction } from "./interaction";
import type { RenderCard } from "./layout";

/** Whether the active seat may arm End Turn. Hidden while local combat staging is pending,
 * or while the engine requires at least one attacker (goad / must-attack). */
export function canArmEndTurn(state: VisibleState, pendingAttackers: boolean): boolean {
  if (state.viewer !== state.active_player) return false;
  if (state.stack.length > 0) return false;
  if (pendingAttackers) return false;
  const required = state.actions?.find((a) => a.kind === "declare_attackers")?.required_attacks?.length ?? 0;
  if (required > 0) return false;
  return true;
}

export type CombatDropResult =
  | { kind: "attackers"; value: WireAttack[] }
  | { kind: "blockers"; value: WireBlock[] }
  | { kind: "none" };

/** Pure combat-drag resolution: stage an attacker onto a defender seat, or a blocker onto an
 * attacker creature. Returns which staging list changed, or none when the drop is illegal. */
export function handleCombatDrop(
  mode: CombatMode,
  currentAttackers: WireAttack[],
  currentBlocks: WireBlock[],
  from: Parameters<typeof attackDrop>[1],
  defender: number | null,
  blockTarget: RenderCard | null,
  declaredAttackers: WireAttack[],
  /** Seats this declaration covers (`declaresFor`) — the viewer's own unless it was moved. */
  seats: readonly number[],
  opponents: number[] = [],
): CombatDropResult {
  if (mode === "attackers") {
    const pw = attackablePlaneswalker(blockTarget, opponents);
    const next = attackDrop(currentAttackers, from, defender, pw?.id);
    return next ? { kind: "attackers", value: next } : { kind: "none" };
  }
  if (mode === "blockers") {
    const next = blockDrop(currentBlocks, from.id, blockTarget, declaredAttackers, seats);
    return next ? { kind: "blockers", value: next } : { kind: "none" };
  }
  return { kind: "none" };
}

/** Union staged attackers with engine-required ones (goad), keeping the player's defender
 * choice when they already staged a required creature. */
export function mergeRequiredAttacks(staged: WireAttack[], required: WireAttack[]): WireAttack[] {
  const have = new Set(staged.map((a) => a.attacker));
  return [...staged, ...required.filter((r) => !have.has(r.attacker))];
}

/** Attackers to draw / confirm. Once declaration is final (local latch or wire), do not re-merge
 * required_attacks — the declare_attackers action can linger until SSE, and merging would redraw
 * staging arrows after confirm. */
export function stagedAttackersForDisplay(
  staged: WireAttack[],
  required: WireAttack[],
  declarationDone: boolean,
): WireAttack[] {
  if (declarationDone) return staged;
  return mergeRequiredAttacks(staged, required);
}

/** Whether this permanent prints or has been granted a banding keyword — `banding` (CR 702.22a) or
 * any `bands_with:<quality>` (CR 702.22b). `ObjectView.keywords` is effective, so a quality granted
 * by one of the Legends banding lands counts. */
function bandingCapable(object: ObjectView): boolean {
  return (object.keywords ?? []).some((k) => k === "banding" || k.startsWith("bands_with:"));
}

/** The staged attackers offered as band members, or `[]` when banding is not in play at all.
 *
 * This is a *discoverability* gate, not a legality check: the panel opens once some staged attacker
 * can band, and then every staged attacker is offered — a "bands with other legendary" band may
 * include a legendary creature that has no banding keyword of its own (CR 702.22c). The engine owns
 * legality (`Game::band_is_legal`) and rejects an illegal grouping as `Reject::IllegalDeclaration`.
 * An ordinary attack with no banding creature in it never renders the panel. */
export function bandCandidates(objects: readonly ObjectView[], attackers: readonly WireAttack[]): number[] {
  if (attackers.length < 2) return [];
  const staged = attackers
    .map((a) => objects.find((o) => o.id === a.attacker))
    .filter((o): o is ObjectView => o !== undefined);
  if (!staged.some(bandingCapable)) return [];
  return staged.map((o) => o.id);
}

/** The `bands` field of a declare-attackers intent: the toggled members that are still staged, as
 * one band. Fewer than two members is no band at all (the engine rejects a one-creature band).
 * ponytail: one band per declaration. Two simultaneous bands needs four banding creatures attacking
 * at once; add a second toggle group if a real game ever wants it. */
export function stagedBands(band: readonly number[], attackers: readonly WireAttack[]): WireBand[] {
  const staged = new Set(attackers.map((a) => a.attacker));
  const members = band.filter((id) => staged.has(id));
  return members.length >= 2 ? [{ members }] : [];
}

/** The wire intent the primary board button would submit right now. */
export function primaryActionIntent(
  action: PrimaryAction,
  me: number,
  attackers: WireAttack[],
  blocks: WireBlock[],
): WireIntent {
  return Match.value(action).pipe(
    Match.withReturnType<WireIntent>(),
    Match.discriminatorsExhaustive("kind")({
      "confirm-attackers": () => ({ kind: "declare_attackers", player: me, attackers }),
      "confirm-blockers": () => ({ kind: "declare_blockers", player: me, blocks }),
      pass: () => ({ kind: "pass_priority", player: me }),
    }),
  );
}

/** Staging lists + confirm latches clear only on a real step transition, not on same-step SSE churn. */
export function combatStagingClearsOnStepChange(prevStep: number | undefined, step: number): boolean {
  return prevStep !== undefined && prevStep !== step;
}
