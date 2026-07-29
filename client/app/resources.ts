import { Context, Layer } from "effect";
import { browserTracerLayer } from "./domain/faro/tracer";
import { type Client as LobbyHttpClient, client as lobbyHttpClient } from "./domain/lobby/client";
import { type Client, client } from "./domain/rpc-client";

export class RpcClient extends Context.Service<RpcClient, Client>()("RpcClient") {}
export class LobbyClient extends Context.Service<LobbyClient, LobbyHttpClient>()("LobbyClient") {}

/** `browserTracerLayer` must be merged in: every request span Effect opens is injected as a
 * `traceparent`, and only Faro's provider ever exports it (see `domain/faro/tracer`). */
export const resources = Layer.mergeAll(
  Layer.succeed(RpcClient, client),
  Layer.succeed(LobbyClient, lobbyHttpClient),
  browserTracerLayer,
);
