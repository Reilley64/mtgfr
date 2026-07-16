// The lobby poll must fire *repeatedly*, not once. The live-observed stall was `useAtomSubscribe`
// subscribing without `{ immediate: true }`: `registry.subscribe` then only attaches a listener and
// never reads (computes) the atom, so the poll stream's fiber never starts — zero requests. The fix
// lives in `startLobbyPoll` (immediate read). Here we drive it against a request-counting stub
// fetch and assert the poll keeps ticking.

import * as Atom from "effect/unstable/reactivity/Atom";
import * as Registry from "effect/unstable/reactivity/AtomRegistry";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { LobbyView } from "~/lib/lobbyTypes";
import { pollStream, startLobbyPoll } from "~/lobbyPoll";

vi.stubGlobal("location", { origin: "http://localhost", pathname: "/" });

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

const view: LobbyView = { table_id: "t", started: false, seats: [], you: null, start_error: null, error: null };

describe("startLobbyPoll", () => {
  let unsub: (() => void) | undefined;
  afterEach(() => {
    unsub?.();
    vi.unstubAllGlobals();
    vi.stubGlobal("location", { origin: "http://localhost", pathname: "/" });
  });

  it("polls the lobby repeatedly while subscribed (≥3 request cycles)", async () => {
    let count = 0;
    vi.stubGlobal(
      "fetch",
      vi.fn(() => {
        count++;
        return Promise.resolve(
          new Response(JSON.stringify(view), {
            status: 200,
            headers: { "content-type": "application/json" },
          }),
        );
      }),
    );
    const registry = Registry.make();
    const atom = Atom.make(pollStream("t"));
    const seen: LobbyView[] = [];

    unsub = startLobbyPoll(registry, atom, (v) => seen.push(v));

    await wait(3500);
    expect(count).toBeGreaterThanOrEqual(3);
    expect(seen.length).toBeGreaterThanOrEqual(3);
  }, 10_000);

  it("stops polling once unsubscribed", async () => {
    let count = 0;
    vi.stubGlobal(
      "fetch",
      vi.fn(() => {
        count++;
        return Promise.resolve(
          new Response(JSON.stringify(view), {
            status: 200,
            headers: { "content-type": "application/json" },
          }),
        );
      }),
    );
    const registry = Registry.make();
    const atom = Atom.make(pollStream("t"));

    unsub = startLobbyPoll(registry, atom, () => {});
    await wait(1200);
    const afterFirst = count;
    expect(afterFirst).toBeGreaterThanOrEqual(1);

    unsub();
    unsub = undefined;
    await wait(1500);
    expect(count).toBeLessThanOrEqual(afterFirst + 1);
  }, 10_000);
});
