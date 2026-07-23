import { Context, Layer } from "effect";
import { type Client, client } from "../lib/rpc-client";

export class RpcClient extends Context.Service<RpcClient, Client>()("RpcClient") {}

export const resources = Layer.succeed(RpcClient, client);
