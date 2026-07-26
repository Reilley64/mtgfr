import type { VisibleState } from "~/wire/types";

/** Step ids for declare attackers / blockers — keep in sync with board geometry `STEP`. */
const DECLARE_ATTACKERS = 5;
const DECLARE_BLOCKERS = 6;

export type CombatCoachInput = {
  step: number;
  activePlayer: number;
  viewer: number;
  attackersDeclared: boolean;
  blockersDeclaredForViewer: boolean;
  /** True when any declared attacker has this viewer as defender. */
  attackingViewer: boolean;
  attackersConfirmedLocally: boolean;
  blockersConfirmedLocally: boolean;
};

export function combatCoachFromState(
  state: VisibleState,
  opts: { attackersConfirmed: boolean; blockersConfirmed: boolean },
): string | null {
  return combatCoachText({
    step: state.step,
    activePlayer: state.active_player,
    viewer: state.viewer,
    attackersDeclared: state.combat.attackers_declared,
    blockersDeclaredForViewer: state.combat.blockers_declared.includes(state.viewer),
    attackingViewer: state.combat.attackers.some((a) => a.defender === state.viewer),
    attackersConfirmedLocally: opts.attackersConfirmed,
    blockersConfirmedLocally: opts.blockersConfirmed,
  });
}

/** Contextual coach copy while the local seat is declaring attackers or blockers. */
export function combatCoachText(input: CombatCoachInput): string | null {
  const attackDone = input.attackersConfirmedLocally || input.attackersDeclared;
  if (input.step === DECLARE_ATTACKERS && input.activePlayer === input.viewer && !attackDone) {
    return "Drag a creature onto an opponent to attack · Confirm with Attack";
  }
  const blockDone = input.blockersConfirmedLocally || input.blockersDeclaredForViewer;
  if (input.step === DECLARE_BLOCKERS && input.attackingViewer && !blockDone) {
    return "Drag a creature onto an attacker to block · Confirm with Block";
  }
  return null;
}
