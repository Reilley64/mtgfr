import type { Command as FoldkitCommand } from "foldkit";
import type { RpcClient } from "../../../resources";
import { ChangedDeckListRoute, type Message } from "./messages";
import type { DeckListSubmodel } from "./submodel";
import { update } from "./update";

export function informRouteChanged(
  model: DeckListSubmodel,
): readonly [DeckListSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  return update(model, ChangedDeckListRoute());
}
