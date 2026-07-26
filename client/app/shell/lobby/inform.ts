import type { Command as FoldkitCommand } from "foldkit";
import { ChangedLobbyRoute, type Message } from "./messages";
import type { LobbySlice } from "./submodel";
import { update } from "./update";

export function informRouteChanged(
  model: LobbySlice,
  route: { tableId: string | null; selectedDeckId: number | null },
): readonly [LobbySlice, ReadonlyArray<FoldkitCommand.Command<Message>>] {
  return update(model, ChangedLobbyRoute(route), []);
}
