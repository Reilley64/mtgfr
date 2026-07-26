import type { Command as FoldkitCommand } from "foldkit";
import type { RpcClient } from "../../resources";
import { ChangedCoverageRoute, type Message } from "./messages";
import type { CoverageSubmodel } from "./submodel";
import { update } from "./update";

export function informRouteChanged(
  model: CoverageSubmodel,
): readonly [CoverageSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  return update(model, ChangedCoverageRoute());
}
