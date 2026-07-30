import * as Match from "effect/Match";
import type { VisibleState, WireAttack, WireBlock, WireIntent } from "~/wire/types";
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
  /** Extra copies committed alongside `from` — the rest of a shift-dropped cluster. Cluster members
   * are identical by construction, so the legality guards that pass for `from` pass for them all,
   * and an illegal face rejects the whole pile rather than staging part of it. */
  alsoIds: readonly number[] = [],
): CombatDropResult {
  if (mode === "attackers") {
    const pw = attackablePlaneswalker(blockTarget, opponents);
    let next = attackDrop(currentAttackers, from, defender, pw?.id);
    if (!next) return { kind: "none" };
    for (const id of alsoIds) {
      // `?? next` is defensive only: attackDrop rejects only on from.tapped / summoningSick-without-haste
      // / a null defender, all facts clusterKey holds equal across members, so a member can't fail here
      // once the face above has already passed.
      next = attackDrop(next, { ...from, id }, defender, pw?.id) ?? next;
    }
    return { kind: "attackers", value: next };
  }
  if (mode === "blockers") {
    let next = blockDrop(currentBlocks, from.id, blockTarget, declaredAttackers, seats);
    if (!next) return { kind: "none" };
    for (const id of alsoIds) {
      // `?? next` is defensive only: blockDrop rejects only on a missing target or an undeclared
      // attacker, neither of which reads the blocker id, so a member can't fail here once the face
      // above has already passed.
      next = blockDrop(next, id, blockTarget, declaredAttackers, seats) ?? next;
    }
    return { kind: "blockers", value: next };
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
