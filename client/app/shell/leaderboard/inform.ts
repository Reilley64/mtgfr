import type { Command as FoldkitCommand } from "foldkit";
import type { RpcClient } from "../../resources";
import { ChangedLeaderboardRoute, type Message } from "./messages";
import type { LeaderboardSubmodel } from "./submodel";
import { update } from "./update";

export function informRouteChanged(
  model: LeaderboardSubmodel,
): readonly [LeaderboardSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  return update(model, ChangedLeaderboardRoute());
}
