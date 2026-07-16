// The wire client: URL-derived table state, intent envelope assembly, and the game delta
// stream as atoms. Wire calls compose directly off the generated Effect client; this module owns
// the URL-derived concerns and exposes the delta stream as a per-table atom whose fiber lifetime is
// its subscription — mounting it (Board) runs the stream, unmounting interrupts it.

import * as Atom from "effect/unstable/reactivity/Atom";
import type { IntentEnvelope, WireIntent } from "~/api/generated";
import { streamDeltas } from "~/effect/stream";
import { applyDelta, applySnapshot, setReject } from "~/store";

/** The table this browser is at, from `/play/:table` (default "t" when absent). */
export function tableId(): string {
  const m = location.pathname.match(/^\/play\/([^/]+)$/);
  if (m) return decodeURIComponent(m[1]);
  return "t";
}

/** True when the guest pasted a share link rather than typing a bare code. */
function looksLikeShareLink(input: string): boolean {
  return (
    input.includes("://") ||
    input.startsWith("/") ||
    input.startsWith("?") ||
    input.includes("?table=") ||
    input.includes("&table=") ||
    /^\/play\/[^/]+/.test(input)
  );
}

/** Normalize guest input: a bare code or a pasted share link → table id. Codes are uppercase. */
export function parseTableCode(input: string): string | null {
  const trimmed = input.trim();
  if (!trimmed) return null;

  if (looksLikeShareLink(trimmed)) {
    try {
      const url = trimmed.includes("://") ? new URL(trimmed) : new URL(trimmed, location.origin);
      const fromPath = url.pathname.match(/^\/play\/([^/]+)$/);
      if (fromPath) return decodeURIComponent(fromPath[1]).trim().toUpperCase();
      const fromQuery = url.searchParams.get("table");
      if (fromQuery != null && fromQuery.trim() !== "") return fromQuery.trim().toUpperCase();
    } catch {
      return null;
    }
    return null;
  }

  return trimmed.toUpperCase();
}

/** Point this browser at a table (reflected in the URL so `tableId()` sees it). */
export function setTableUrl(table: string): void {
  const u = new URL(location.href);
  u.pathname = `/play/${encodeURIComponent(table)}`;
  u.searchParams.delete("table");
  u.searchParams.delete("player");
  history.replaceState(null, "", u);
}

// Monotonic per-session integer for `client_seq`. Must be an integer: the wire type is a
// `u64`, so a fractional value (e.g. `performance.now()`) fails to deserialize → HTTP 422.
let clientSeq = 0;

/** Build the wire envelope for intent: current table plus a fresh `client_seq`. A plain
 * function (no `Api`/`fetch` involved), so the sequencing logic is unit-testable on its own. */
export function buildIntentEnvelope(intent: WireIntent): IntentEnvelope {
  return { table_id: tableId(), client_seq: ++clientSeq, intent };
}

// ── Game ───────────────────────────────────────────────────────────────────────────────

/** Stream connection health, rendered as the reconnect banner. Flipped by the stream's status
 * transitions (down on a drop, back up on the first frame of a healthy reconnect). */
export const connectedAtom = Atom.make(true);

/**
 * Per-table delta-stream atom. Mounting it (Board's `useAtomMount`) runs `streamDeltas` for the
 * lifetime of the subscription: each frame folds into the game store (`applySnapshot`/`applyDelta`),
 * status transitions flip `connectedAtom`, and a terminal 4xx (bad table / expired session — won't
 * self-resolve) sets the reject line. Unmounting interrupts the fiber. The reconnect/backoff
 * pipeline itself lives in `effect/stream.ts`; this only ties its lifecycle to atom mount and
 * routes its callbacks into the store.
 */
export const gameStreamFamily = Atom.family((table: string) =>
  Atom.make((get) =>
    streamDeltas(table, {
      onFrame: (frame) => (frame.frame === "snapshot" ? applySnapshot(frame.seq, frame.state) : applyDelta(frame)),
      onStatus: (up) => get.set(connectedAtom, up),
      onError: (status) => {
        // A terminal 4xx ends the loop without a status transition, so mark the stream down here or
        // the banner keeps claiming we're connected underneath the reject line.
        get.set(connectedAtom, false);
        setReject(status === 401 ? "Session expired — sign in again." : `Lost connection to the table (${status}).`);
      },
    }),
  ),
);
