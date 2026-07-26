// Shared scaffolding for wire-client tests: a same-origin `location` stub (vitest's node
// environment has none) and helpers to run a generated-client method against a stubbed `fetch`.

import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import type * as Result from "effect/Result";
import { vi } from "vitest";
import { LobbyClient, RpcClient } from "../../resources";
import { type Client as LobbyHttpClient, makeClient as makeLobbyClient } from "../lobby/client";
import { type Client, makeClient } from "../rpc-client";

/** Give the HTTP layer a same-origin base to resolve relative URLs against. Call once per file,
 * inside `beforeAll`. */
export function stubLocation(): void {
  vi.stubGlobal("location", { origin: "http://localhost", pathname: "/" });
}

/** Run a client method against a stubbed `fetch`, returning the raw success value. */
export function run<A, E>(fetchImpl: typeof fetch, use: (client: Client) => Effect.Effect<A, E>): Promise<A> {
  return Effect.runPromise(use(makeClient(fetchImpl)));
}

/** Run a client method that may fail, returning a `Result` so we can assert on the failure. */
export function runEither<A, E>(
  fetchImpl: typeof fetch,
  use: (client: Client) => Effect.Effect<A, E>,
): Promise<Result.Result<A, E>> {
  return Effect.runPromise(Effect.result(use(makeClient(fetchImpl))));
}

/** Run a lobby client method against a stubbed `fetch`, returning the raw success value. */
export function runLobby<A, E>(
  fetchImpl: typeof fetch,
  use: (client: LobbyHttpClient) => Effect.Effect<A, E>,
): Promise<A> {
  return Effect.runPromise(use(makeLobbyClient(fetchImpl)));
}

/** Run a lobby client method that may fail, returning a `Result` for failure assertions. */
export function runLobbyEither<A, E>(
  fetchImpl: typeof fetch,
  use: (client: LobbyHttpClient) => Effect.Effect<A, E>,
): Promise<Result.Result<A, E>> {
  return Effect.runPromise(Effect.result(use(makeLobbyClient(fetchImpl))));
}

/** Merged RpcClient + LobbyClient layers for Command tests that need both wire surfaces. */
export function wireTestResources(fetchImpl: typeof fetch): Layer.Layer<RpcClient | LobbyClient> {
  return Layer.merge(
    Layer.succeed(RpcClient, makeClient(fetchImpl)),
    Layer.succeed(LobbyClient, makeLobbyClient(fetchImpl)),
  );
}

/** A stub `fetch` that always returns the given web `Response`, ignoring the request. */
export function respondWith(response: Response): typeof fetch {
  // Bun's `typeof fetch` includes `preconnect`; stubs only implement the call signature.
  return (() => Promise.resolve(response)) as unknown as typeof fetch;
}

export const ok = () => new Response(null, { status: 200 });
export const status = (code: number) => new Response(null, { status: code });
export const json = (body: unknown, code = 200) =>
  new Response(JSON.stringify(body), { status: code, headers: { "content-type": "application/json" } });
export const networkError: typeof fetch = (() =>
  Promise.reject(new TypeError("Failed to fetch"))) as unknown as typeof fetch;

/** Decode a captured request's JSON body — `bodyUnsafeJson` sends it as a `Uint8Array`, not a
 * string, so a test can't `JSON.parse(init.body)` directly. */
export function bodyOf(init: RequestInit | undefined): unknown {
  return JSON.parse(new TextDecoder().decode(init?.body as Uint8Array));
}

/** A recording `fetch` stub: always answers with `response`, and captures each call's URL/init
 * so a test can assert on the request the Api service actually made (path, method, body). */
export function recordingFetch(response: Response): { fetch: typeof fetch; calls: [URL, RequestInit | undefined][] } {
  const calls: [URL, RequestInit | undefined][] = [];
  const fetchImpl = ((url: URL, init?: RequestInit) => {
    calls.push([url, init]);
    return Promise.resolve(response);
  }) as unknown as typeof fetch;
  return { fetch: fetchImpl, calls };
}
