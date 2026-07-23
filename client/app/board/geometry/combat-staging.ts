import * as Match from "effect/Match";
import type { WireAttack, WireBlock, WireIntent } from "~/wire/types";
import { attackablePlaneswalker, attackDrop, blockDrop, type CombatMode, type PrimaryAction } from "./interaction";
import type { RenderCard } from "./layout";

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
  me: number,
  opponents: number[] = [],
): CombatDropResult {
  if (mode === "attackers") {
    const pw = attackablePlaneswalker(blockTarget, opponents);
    const next = attackDrop(currentAttackers, from, defender, pw?.id);
    return next ? { kind: "attackers", value: next } : { kind: "none" };
  }
  if (mode === "blockers") {
    const next = blockDrop(currentBlocks, from.id, blockTarget, declaredAttackers, me);
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
