// The client's view of the game. Each stream delta now carries the viewer's full render
// state *and* the events that produced it (ADR 0006): we replace the rendered `state` and
// narrate the `events` into a scrollable game log. No snapshot refetch mid-stream.

import { createStore } from "solid-js/store";
import type { ZonePileEntrance } from "~/lib/eventFold";
import { describe, extractProvenance } from "~/lib/eventFold";
import type { StreamFrame, VisibleState } from "~/wire/types";

export type { ZonePileEntrance } from "~/lib/eventFold";

/** A delta's payload (`seq`, `events`, `state`) — the non-snapshot arm of `StreamFrame` minus its
 * `frame` tag. The generator inlines it rather than exporting a named `DeltaEnvelope`. */
type DeltaEnvelope = Omit<Extract<StreamFrame, { frame: "delta" }>, "frame">;

/** `VisibleState.viewer` for a spectator — a watcher with no seat (server: `schema::SPECTATOR_VIEWER`).
 * The board renders read-only: no hand, no action affordances. */
export const SPECTATOR_VIEWER = 255;

export interface LogLine {
  seq: number;
  text: string;
  /** Server auto-submit or the viewer's own draw — shown with an AUTO chip in the log. */
  auto?: boolean;
}

export interface GameStore {
  state: VisibleState | null;
  seq: number;
  reject: string | null;
  log: LogLine[];
}

export const [game, setGame] = createStore<GameStore>({
  state: null,
  seq: 0,
  reject: null,
  log: [],
});

/** Reset to a blank game: called on Board mount so a new table doesn't render the last one's state. */
export function resetGame(): void {
  moveMap = new Map();
  stackResolved = new Set();
  stackExits = new Set();
  tokenCreators = new Map();
  landPlays = new Map();
  zonePileEntrances = new Map();
  stackEntrances = new Map();
  prevStackIds = new Set();
  stackIdsAtDeltaStart = new Set();
  tableFeelBatch = { land: false, stack: false, resolve: false, damage: false };
  setGame({ state: null, seq: 0, reject: null, log: [] });
}

/** Replace the view with a snapshot, ignoring any that's older than what we already show. */
export function applySnapshot(seq: number, state: VisibleState): void {
  if (state && seq >= game.seq) {
    moveMap = new Map(); // a snapshot carries no events → no zone-move glides
    stackResolved = new Set();
    stackExits = new Set();
    tokenCreators = new Map();
    landPlays = new Map();
    zonePileEntrances = new Map();
    stackEntrances = new Map();
    prevStackIds = new Set(state.stack.map((s) => s.source));
    stackIdsAtDeltaStart = new Set();
    tableFeelBatch = { land: false, stack: false, resolve: false, damage: false };
    setGame({ state, seq });
  }
}

/**
 * Fold a delta: the delta is self-sufficient (full render `state` + `events` to narrate).
 * Same-`seq` empty-event frames are hold ticks (dwell) and only refresh the countdown.
 * Server auto-submitted actions (`auto_actions`) and the viewer's own draws append as `auto`
 * log lines (AUTO chip in the panel) — no toast.
 */
export function applyDelta(delta: DeltaEnvelope): void {
  if (delta.seq < game.seq) return;
  if (delta.seq === game.seq) {
    if (delta.events.length === 0 && game.state) {
      setGame("state", "stack_hold_remaining_ms", delta.state.stack_hold_remaining_ms ?? 0);
    }
    return;
  }
  const viewer = delta.state.viewer;
  const eventLines: LogLine[] = [];
  for (const e of delta.events) {
    // Viewer's draws: one AUTO log line (name the card when known). Skip the generic `describe`
    // line so we don't get "P0 draws Shock" and "Drew Shock" back-to-back.
    if (e.kind === "card_drawn" && e.player === viewer) {
      eventLines.push({
        seq: delta.seq,
        text: e.card ? `Drew ${e.card}` : "Drew a card",
        auto: true,
      });
      continue;
    }
    const text = describe(e, delta.state);
    if (text != null) eventLines.push({ seq: delta.seq, text });
  }
  const autoLines: LogLine[] = (delta.auto_actions ?? []).map((text) => ({
    seq: delta.seq,
    text,
    auto: true,
  }));
  const lines = [...eventLines, ...autoLines];
  // Provenance for the canvas glide, rebuilt before the board re-lays out.
  const priorStack = prevStackIds;
  ({
    moves: moveMap,
    fromStack: stackResolved,
    fromStackExit: stackExits,
    tokenCreators,
    landPlays,
    zonePileEntrances,
    stackEntrances,
  } = extractProvenance(delta.events, priorStack, game.state?.viewer ?? 0));
  // Freeze the pre-delta stack for token-creator seeding this frame (before advancing).
  stackIdsAtDeltaStart = priorStack;
  prevStackIds = new Set(delta.state.stack.map((s) => s.source));
  tableFeelBatch = {
    land: landPlays.size > 0,
    stack:
      stackEntrances.size > 0 ||
      delta.events.some((e) => e.kind === "triggered_ability_on_stack" || e.kind === "spell_copied"),
    resolve: stackResolved.size > 0 || stackExits.size > 0 || delta.events.some((e) => e.kind === "ability_resolved"),
    damage: delta.events.some(
      (e) => e.kind === "combat_damage_dealt_to_creature" || e.kind === "combat_damage_dealt_to_player",
    ),
  };
  setGame({ state: delta.state, seq: delta.seq });
  if (lines.length) setGame("log", (log) => [...log, ...lines].slice(-200));
}

export function setReject(reason: string | null): void {
  setGame("reject", reason);
}

// Zone-change provenance from the last delta: new object id → the id it came `from`. A zone change
// mints a fresh object id (a hand card and its battlefield permanent are different ids), so this is
// how the canvas tween knows a card *moved* rather than appeared — it seeds the new card's glide at
// the old one's position (see Board.tsx). Rebuilt per delta; a snapshot carries no events, so empty.
let moveMap = new Map<number, number>();

// Permanents in the most recent delta that entered by a spell resolving off the stack
// (`permanent_entered.from` is the spell's stack object — the engine emits this event only
// from spell resolution; tokens, land drops, and reanimations have their own events). The
// stack renders as a DOM overlay, not canvas cards, so these have no canvas origin to glide
// from — the board seeds their entrance at the overlay's anchor instead.
let stackResolved = new Set<number>();

/** Cards that left the stack to GY/exile in the most recent delta. */
let stackExits = new Set<number>();

/** Token id → creator object id from the most recent delta. */
let tokenCreators = new Map<number, number>();

/** Land permanent id → hand card id (`from`) for play-origin matching. */
let landPlays = new Map<number, number>();

let zonePileEntrances = new Map<number, ZonePileEntrance>();

/** Stack object id → { controller, from hand/command card id } for play-in. */
let stackEntrances = new Map<number, { controller: number; from: number }>();

/** Stack object ids from the last applied state (next delta's prior). */
let prevStackIds = new Set<number>();
/**
 * Stack object ids *before* the most recent delta — used for token creator hybrid
 * (resolving stack object → overlay origin). Distinct from `prevStackIds`, which advances
 * to the post-delta stack as soon as the delta is applied.
 */
let stackIdsAtDeltaStart = new Set<number>();

/** Canvas glide provenance from the most recent fold — one seam for TableSurface / Board. */
export type FoldProvenance = {
  zoneMoves: Map<number, number>;
  resolvedFromStack: Set<number>;
  leftStackToPile: Set<number>;
  tokenCreators: Map<number, number>;
  landPlayFrom: Map<number, number>;
  zonePileEntrances: Map<number, ZonePileEntrance>;
  stackEntrances: Map<number, { controller: number; from: number }>;
  priorStackObjectIds: Set<number>;
};

/** One-shot table-feel flags from the most recent delta (one cue per kind per delta). */
export type TableFeelBatch = {
  land: boolean;
  stack: boolean;
  resolve: boolean;
  damage: boolean;
};

let tableFeelBatch: TableFeelBatch = { land: false, stack: false, resolve: false, damage: false };

export function lastTableFeelBatch(): TableFeelBatch {
  return tableFeelBatch;
}

/** Provenance shape TableSurface needs after the latest delta fold. */
export function foldProvenance(): FoldProvenance {
  return {
    zoneMoves: moveMap,
    resolvedFromStack: stackResolved,
    leftStackToPile: stackExits,
    tokenCreators,
    landPlayFrom: landPlays,
    zonePileEntrances,
    stackEntrances,
    priorStackObjectIds: stackIdsAtDeltaStart,
  };
}
