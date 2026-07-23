import type { ZonePileEntrance } from "../../lib/event-fold";
import { describe, extractProvenance } from "../../lib/event-fold";
import type { StreamFrame, VisibleState } from "../../lib/wire/types";

export type DeltaEnvelope = Omit<Extract<StreamFrame, { frame: "delta" }>, "frame">;

export interface LogLine {
  seq: number;
  text: string;
  /** Server auto-submit or the viewer's own draw — shown with an AUTO chip in the log. */
  auto?: boolean;
}

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

export type GameFoldState = {
  seq: number;
  state: VisibleState | null;
  log: ReadonlyArray<LogLine>;
  reject: string | null;
  provenance: FoldProvenance;
  tableFeel: TableFeelBatch;
};

function emptyProvenance(): FoldProvenance {
  return {
    zoneMoves: new Map(),
    resolvedFromStack: new Set(),
    leftStackToPile: new Set(),
    tokenCreators: new Map(),
    landPlayFrom: new Map(),
    zonePileEntrances: new Map(),
    stackEntrances: new Map(),
    priorStackObjectIds: new Set(),
  };
}

function emptyTableFeel(): TableFeelBatch {
  return { land: false, stack: false, resolve: false, damage: false };
}

export function emptyGameFold(): GameFoldState {
  return {
    seq: 0,
    state: null,
    log: [],
    reject: null,
    provenance: emptyProvenance(),
    tableFeel: emptyTableFeel(),
  };
}

export function applySnapshotPure(prev: GameFoldState, seq: number, state: VisibleState): GameFoldState {
  if (seq < prev.seq) return prev;

  return {
    ...prev,
    seq,
    state,
    provenance: emptyProvenance(),
    tableFeel: emptyTableFeel(),
  };
}

export function applyDeltaPure(prev: GameFoldState, delta: DeltaEnvelope): GameFoldState {
  if (delta.seq < prev.seq) return prev;

  if (delta.seq === prev.seq) {
    if (delta.events.length === 0 && prev.state != null) {
      return {
        ...prev,
        state: {
          ...prev.state,
          stack_hold_remaining_ms: delta.state.stack_hold_remaining_ms ?? 0,
        },
      };
    }

    return prev;
  }

  const eventLines: LogLine[] = [];
  for (const event of delta.events) {
    if (event.kind === "card_drawn" && event.player === delta.state.viewer) {
      eventLines.push({
        seq: delta.seq,
        text: event.card ? `Drew ${event.card}` : "Drew a card",
        auto: true,
      });
      continue;
    }

    const text = describe(event, delta.state);
    if (text != null) eventLines.push({ seq: delta.seq, text });
  }

  const autoLines: LogLine[] = (delta.auto_actions ?? []).map((text) => ({
    seq: delta.seq,
    text,
    auto: true,
  }));
  const lines = [...eventLines, ...autoLines];
  const priorStackObjectIds = new Set(prev.state?.stack.map((stackObject) => stackObject.source) ?? []);
  const provenance = extractProvenance(delta.events, priorStackObjectIds, prev.state?.viewer ?? 0);

  return {
    ...prev,
    seq: delta.seq,
    state: delta.state,
    log: lines.length > 0 ? [...prev.log, ...lines].slice(-200) : prev.log,
    provenance: {
      zoneMoves: provenance.moves,
      resolvedFromStack: provenance.fromStack,
      leftStackToPile: provenance.fromStackExit,
      tokenCreators: provenance.tokenCreators,
      landPlayFrom: provenance.landPlays,
      zonePileEntrances: provenance.zonePileEntrances,
      stackEntrances: provenance.stackEntrances,
      priorStackObjectIds,
    },
    tableFeel: {
      land: provenance.landPlays.size > 0,
      stack:
        provenance.stackEntrances.size > 0 ||
        delta.events.some((event) => event.kind === "triggered_ability_on_stack" || event.kind === "spell_copied"),
      resolve:
        provenance.fromStack.size > 0 ||
        provenance.fromStackExit.size > 0 ||
        delta.events.some((event) => event.kind === "ability_resolved"),
      damage: delta.events.some(
        (event) => event.kind === "combat_damage_dealt_to_creature" || event.kind === "combat_damage_dealt_to_player",
      ),
    },
  };
}

export function setRejectPure(prev: GameFoldState, reason: string | null): GameFoldState {
  return { ...prev, reject: reason };
}
