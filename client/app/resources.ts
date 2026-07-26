import { Context, Layer } from "effect";
import { type Client, client } from "./domain/rpc-client";

export class RpcClient extends Context.Service<RpcClient, Client>()("RpcClient") {}

export const resources = Layer.succeed(RpcClient, client);
