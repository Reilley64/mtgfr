// The lobby poll must fire *repeatedly*, not once. The live-observed stall was `useAtomSubscribe`
// subscribing without `{ immediate: true }`: `registry.subscribe` then only attaches a listener and
// never reads (computes) the atom, so the poll stream's fiber never starts — zero requests. The fix
// lives in `startLobbyPoll` (immediate read). Here we drive it against a request-counting stub
// client and assert the poll keeps ticking. Real (short) time, not a TestClock: the schedule sleeps
// on the live clock and we only need a few one-second ticks.

import * as Atom from "effect/unstable/reactivity/Atom";
import * as Registry from "effect/unstable/reactivity/AtomRegistry";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { LobbyView } from "~/api/generated";
import { makeClient } from "~/effect/client";
import { pollStream, startLobbyPoll } from "~/lobbyPoll";

// vitest's node env has no `location`; the HTTP layer needs an origin to resolve `/api/...`.
vi.stubGlobal("location", { origin: "http://localhost", pathname: "/" });

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** A `fetch` that answers every `/tables/lobby` GET with the given view and counts the calls. */
function countingClient(view: LobbyView) {
  let count = 0;
  const fetchImpl = (() => {
    count++;
    return Promise.resolve(
      new Response(JSON.stringify(view), { status: 200, headers: { "content-type": "application/json" } }),
    );
  }) as typeof fetch;
  return { client: makeClient(fetchImpl), requests: () => count };
}

const view: LobbyView = { table_id: "t", started: false, seats: [] };

describe("startLobbyPoll", () => {
  let unsub: (() => void) | undefined;
  afterEach(() => unsub?.());

  it("polls the lobby repeatedly while subscribed (≥3 request cycles)", async () => {
    const { client, requests } = countingClient(view);
    const registry = Registry.make();
    const atom = Atom.make(pollStream("t", client));
    const seen: LobbyView[] = [];

    unsub = startLobbyPoll(registry, atom, (v) => seen.push(v));

    // Immediate tick + ~one per second: ~4 requests over 3.5s. Before the fix this stayed at 0.
    await wait(3500);
    expect(requests()).toBeGreaterThanOrEqual(3);
    expect(seen.length).toBeGreaterThanOrEqual(3);
  }, 10_000);

  it("stops polling once unsubscribed", async () => {
    const { client, requests } = countingClient(view);
    const registry = Registry.make();
    const atom = Atom.make(pollStream("t", client));

    unsub = startLobbyPoll(registry, atom, () => {});
    await wait(1200);
    const afterFirst = requests();
    expect(afterFirst).toBeGreaterThanOrEqual(1);

    unsub();
    unsub = undefined;
    await wait(1500);
    // No further requests after teardown (allow one already-in-flight tick to land).
    expect(requests()).toBeLessThanOrEqual(afterFirst + 1);
  }, 10_000);
});
