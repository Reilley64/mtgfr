import { Context, Layer } from "effect";
import { type Client as LobbyHttpClient, client as lobbyHttpClient } from "./domain/lobby/client";
import { type Client, client } from "./domain/rpc-client";

export class RpcClient extends Context.Service<RpcClient, Client>()("RpcClient") {}
export class LobbyClient extends Context.Service<LobbyClient, LobbyHttpClient>()("LobbyClient") {}

export const resources = Layer.merge(
  Layer.succeed(RpcClient, client),
  Layer.succeed(LobbyClient, lobbyHttpClient),
);
