// Which permanents are *committed* right now — attacking, blocking, blocked, or targeted.
//
// Cluster collapse in `geometry/layout.ts` merges indistinguishable permanents, which makes every
// copy but the face unreachable. An engaged permanent is no longer interchangeable with its
// clustermates, so layout gives it its own slot; the cluster's face then becomes the next free
// copy. See the cluster-split-on-engagement design.

import type { PromptDraft } from "~/choice";
import type { VisibleState, WireAttack, WireBlock } from "~/wire/types";
import { stackEntryTargets } from "./geometry/stackTargets";

/** The slice of `BoardModel` engagement reads — `BoardModel` satisfies it structurally. */
export type BoardStaging = {
  combatAttackers: readonly WireAttack[];
  combatBlocks: readonly WireBlock[];
  promptDraft: PromptDraft | null;
};

/** Object ids a prompt draft has picked. `StagedAction` holds only *legal* targets, never a
 * chosen one — a staged cast submits on release — so drafts are the whole local target story. */
function draftedIds(draft: PromptDraft | null): readonly number[] {
  if (draft == null) return [];
  if (draft.kind === "card-pick") return draft.picked;
  if (draft.kind === "target") return [draft.id];
  if (draft.kind === "targets") return draft.ids;
  return [];
}

/**
 * Object ids that must not merge into a cluster this frame. Derived every frame, so a permanent
 * rejoins its pile on its own once combat clears or the spell targeting it resolves.
 *
 * Local staging only gains entries on a completed drop or pick, never on drag start — so nothing
 * reflows under a moving pointer.
 */
export function engagedIds(state: VisibleState, local: BoardStaging): ReadonlySet<number> {
  const engaged = new Set<number>();

  for (const attack of [...state.combat.attackers, ...local.combatAttackers]) engaged.add(attack.attacker);
  for (const block of [...state.combat.blocks, ...local.combatBlocks]) engaged.add(block.blocker);
  for (const id of state.combat.blocked_attackers) engaged.add(id);

  for (const entry of state.stack) {
    for (const target of stackEntryTargets(entry)) {
      if (target.kind === "object") engaged.add(target.id);
    }
  }

  for (const id of draftedIds(local.promptDraft)) engaged.add(id);

  return engaged;
}
