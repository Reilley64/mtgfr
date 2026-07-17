import * as Match from "effect/Match";
import { type Accessor, createEffect, createMemo, createSignal } from "solid-js";
import type { RenderCard } from "~/layout";
import {
  attackDrop,
  blockDrop,
  type CombatDeclaration,
  type CombatMode,
  combatMode,
  type PrimaryAction,
  primaryActionFor,
} from "~/lib/interaction";
import type { WireAttack, WireBlock, WireIntent } from "~/wire/types";

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
): CombatDropResult {
  if (mode === "attackers") {
    const next = attackDrop(currentAttackers, from, defender);
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

export interface CombatStagingDeps {
  me: Accessor<number>;
  step: Accessor<number>;
  activePlayer: Accessor<number>;
  spectating: Accessor<boolean>;
  opponents: Accessor<number[]>;
  declaredAttackers: Accessor<WireAttack[]>;
  declaredBlocks: Accessor<WireBlock[]>;
  /** Server: attack declaration is final (including empty). */
  attackersDeclared: Accessor<boolean>;
  /** Server: this seat's block declaration is final (including empty). */
  blockersDeclared: Accessor<boolean>;
  /** Goad (etc.) must-attack pairs from the declare_attackers action. */
  requiredAttacks: Accessor<WireAttack[]>;
  hitSeat: (x: number, y: number, seats: number[]) => number | null;
  hitCard: (x: number, y: number) => RenderCard | null;
  act: (intent: WireIntent) => Promise<boolean>;
}

/** Client-side combat declaration staging: attackers/blocks lists, drop handler, confirm buttons. */
export function useCombatStaging(deps: CombatStagingDeps) {
  const [attackers, setAttackers] = createSignal<WireAttack[]>([]);
  const [blocks, setBlocks] = createSignal<WireBlock[]>([]);
  // Latches cover the HTTP→SSE gap; wire attackers_declared / blockers_declared are the durable
  // source of truth (especially for empty declarations, which leave combat lists empty).
  const [attackersConfirmed, setAttackersConfirmed] = createSignal(false);
  const [blockersConfirmed, setBlockersConfirmed] = createSignal(false);

  const isActive = () => deps.activePlayer() === deps.me();
  const declarationDone = (): CombatDeclaration => ({
    // Local latch OR wire flag OR non-empty list — any one means staging is closed.
    attackersDeclared: attackersConfirmed() || deps.attackersDeclared() || deps.declaredAttackers().length > 0,
    blockersDeclared: blockersConfirmed() || deps.blockersDeclared(),
  });
  const combatStep = () =>
    combatMode(deps.step(), isActive(), deps.spectating(), deps.declaredAttackers(), deps.me(), declarationDone()) !==
    null;

  // Reset staging only when the step *value* changes. Replacing game.state on every SSE delta
  // re-reads the same step number through the store — resetting on that wiped the latch after
  // an empty declare and stuck the button on "No attackers".
  createEffect((prevStep?: number) => {
    const step = deps.step();
    if (combatStagingClearsOnStepChange(prevStep, step)) {
      setAttackers([]);
      setBlocks([]);
      setAttackersConfirmed(false);
      setBlockersConfirmed(false);
    }
    return step;
  });

  // Seed goaded must-attack creatures so the primary button never reads "No attackers" when
  // empty would be IllegalDeclaration (CR 701.38a).
  createEffect(() => {
    if (declarationDone().attackersDeclared) return;
    const mode = combatMode(
      deps.step(),
      isActive(),
      deps.spectating(),
      deps.declaredAttackers(),
      deps.me(),
      declarationDone(),
    );
    if (mode !== "attackers") return;
    const required = deps.requiredAttacks();
    if (required.length === 0) return;
    setAttackers((prev) => (prev.length > 0 ? prev : required));
  });

  const onCombatDrop = (from: RenderCard, x: number, y: number) => {
    const mode = combatMode(
      deps.step(),
      isActive(),
      deps.spectating(),
      deps.declaredAttackers(),
      deps.me(),
      declarationDone(),
    );
    const result = handleCombatDrop(
      mode,
      attackers(),
      blocks(),
      from,
      mode === "attackers" ? deps.hitSeat(x, y, deps.opponents()) : null,
      mode === "blockers" ? deps.hitCard(x, y) : null,
      deps.declaredAttackers(),
      deps.me(),
    );
    if (result.kind === "attackers") setAttackers(result.value);
    if (result.kind === "blockers") setBlocks(result.value);
  };

  const cancelAttacker = (id: number) => {
    const required = new Set(deps.requiredAttacks().map((a) => a.attacker));
    if (required.has(id)) return;
    setAttackers((a) => a.filter((w) => w.attacker !== id));
  };
  const cancelBlocker = (id: number) => setBlocks((b) => b.filter((x) => x.blocker !== id));
  const clearCombat = () => {
    // Escape cancels optional staging but keeps must-attack creatures seeded.
    setAttackers(deps.requiredAttacks());
    setBlocks([]);
    // Leave attackersConfirmed / blockersConfirmed alone — Escape cancels staging, not a
    // declaration already accepted by the server (especially empty declares with no combat yet).
  };

  const effectiveAttackers = createMemo(() =>
    stagedAttackersForDisplay(attackers(), deps.requiredAttacks(), declarationDone().attackersDeclared ?? false),
  );

  const primaryAction = createMemo<PrimaryAction>(() =>
    primaryActionFor({
      step: deps.step(),
      activePlayer: deps.activePlayer(),
      me: deps.me(),
      attackers: effectiveAttackers(),
      blocks: blocks(),
      declaredAttackers: deps.declaredAttackers(),
      declaredBlocks: deps.declaredBlocks(),
      attackersConfirmed: attackersConfirmed(),
      blockersConfirmed: blockersConfirmed(),
      attackersDeclared: deps.attackersDeclared(),
      blockersDeclared: deps.blockersDeclared(),
    }),
  );

  const runPrimaryAction = () => {
    const action = primaryAction();
    const stagedAttackers = effectiveAttackers();
    const stagedBlocks = blocks();
    void (async () => {
      const ok = await deps.act(primaryActionIntent(action, deps.me(), stagedAttackers, stagedBlocks));
      if (!ok) return;
      if (action.kind === "confirm-attackers") {
        setAttackers([]);
        setAttackersConfirmed(true);
      }
      if (action.kind === "confirm-blockers") {
        setBlocks([]);
        setBlockersConfirmed(true);
      }
    })();
  };

  return {
    attackers: effectiveAttackers,
    blocks,
    combatStep,
    onCombatDrop,
    cancelAttacker,
    cancelBlocker,
    clearCombat,
    primaryAction,
    runPrimaryAction,
  };
}

/** Staging lists + confirm latches clear only on a real step transition, not on same-step SSE churn. */
export function combatStagingClearsOnStepChange(prevStep: number | undefined, step: number): boolean {
  return prevStep !== undefined && prevStep !== step;
}
