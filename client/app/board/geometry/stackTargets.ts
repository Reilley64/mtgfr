import type { PlayerView, StackObjectView, VisibleState, WireTarget } from "~/wire/types";

export function stackEntryTargets(entry: StackObjectView): WireTarget[] {
  if (entry.targets != null && entry.targets.length > 0) return [...entry.targets];
  if (entry.target != null) return [entry.target];
  return [];
}

function oneLabel(target: WireTarget, state: VisibleState): string {
  if (target.kind === "player") {
    return state.players.find((p: PlayerView) => p.player === target.player)?.username ?? `Seat ${target.player + 1}`;
  }
  const obj = state.objects.find((o) => o.id === target.id);
  return obj?.name ?? "";
}

/** Caption suffix: ` → A, B` or empty. Skips unresolved object names. */
export function formatStackTargetSuffix(targets: WireTarget[], state: VisibleState): string {
  const labels = targets.map((t) => oneLabel(t, state)).filter((s) => s !== "");
  if (labels.length === 0) return "";
  return ` → ${labels.join(", ")}`;
}
