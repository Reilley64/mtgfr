import type { Command as FoldkitCommand } from "foldkit";
import type { RpcClient } from "../../../resources";
import { ChangedBuilderRoute, type Message } from "./messages";
import type { DeckBuilderSubmodel } from "./submodel";
import { update } from "./update";

export function informRouteChanged(
  model: DeckBuilderSubmodel,
  editingId: string | null,
): readonly [DeckBuilderSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  return update(model, ChangedBuilderRoute({ editingId }));
}
