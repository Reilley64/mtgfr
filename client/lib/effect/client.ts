// Re-export the Effect RPC client under its original `~/effect/client` path so the
// Foldkit tree (moved from `client/src`) keeps the same import surface after the cutover.
export { type Client, client, makeClient, orNull, statusOf, succeeded } from "../rpc-client";
