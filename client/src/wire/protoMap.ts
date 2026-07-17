// Proto ↔ browser wire: camelCase/`bigint`/`{case,value}` oneofs ↔ snake_case tagged unions.
// Structural walk with heuristics for oneofs, `ObjectIdList`, and amount tuples.

import type {
  SaveDeckRequest as ProtoSaveDeckRequest,
  SeedRequest as ProtoSeedRequest,
} from "~/wire/generated/mtgfr/v1/catalog_effect_grpc";
import type { IntentEnvelope as ProtoIntentEnvelope } from "~/wire/generated/mtgfr/v1/intent_effect_grpc";
import type {
  CatalogCard,
  DeckDetail,
  DeckSummary,
  IntentEnvelope,
  SaveDeckRequest,
  SeedRequest,
  SeedResponse,
  StreamFrame,
} from "~/wire/types";

const ONEOF_WRAPPER_KEYS = new Set(["kind", "intent", "event", "choice", "frame"]);

/** Inbound event fields whose elements are `{id,amount}` / `{player,amount}` → `[id, amount]` tuples. */
const AMOUNT_TUPLE_KEYS = new Set(["assignment", "players"]);

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function camelToSnake(key: string): string {
  return key.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`);
}

function snakeToCamel(key: string): string {
  return key.replace(/_([a-z0-9])/g, (_match, letter: string) => letter.toUpperCase());
}

function isObjectAmountShape(value: unknown): value is { id: number; amount: number } {
  return isPlainObject(value) && Object.keys(value).length === 2 && "id" in value && "amount" in value;
}

function isPlayerAmountShape(value: unknown): value is { player: number; amount: number } {
  return isPlainObject(value) && Object.keys(value).length === 2 && "player" in value && "amount" in value;
}

function toAmountTuple(item: unknown): unknown {
  if (isObjectAmountShape(item)) return [item.id, item.amount];
  if (isPlayerAmountShape(item)) return [item.player, item.amount];
  return convertFromProto(item);
}

/** Flatten a oneof wrapper (`kind`/`intent`/…) to a tagged union, or `undefined` if unset. */
function flattenOneofWrapper(value: Record<string, unknown>): [unknown] | null {
  const keys = Object.keys(value);
  if (keys.length !== 1) return null;
  const [wrapperKey] = keys;
  if (!ONEOF_WRAPPER_KEYS.has(wrapperKey)) return null;
  const oneof = value[wrapperKey];
  if (!isPlainObject(oneof) || !("case" in oneof)) return null;
  const caseValue = oneof.case;
  if (caseValue == null) return [undefined];
  if (typeof caseValue !== "string") return null;

  const tag = wrapperKey === "frame" ? "frame" : "kind";
  const inner = convertFromProto(oneof.value ?? {});
  return [{ [tag]: camelToSnake(caseValue), ...(isPlainObject(inner) ? inner : {}) }];
}

function convertFromProto(value: unknown): unknown {
  if (typeof value === "bigint") return Number(value);
  if (Array.isArray(value)) return value.map((item) => convertFromProto(item));
  if (!isPlainObject(value)) return value;

  const keys = Object.keys(value);
  if (keys.length === 1 && keys[0] === "ids" && Array.isArray(value.ids)) {
    return value.ids.map((id) => convertFromProto(id));
  }

  const oneof = flattenOneofWrapper(value);
  if (oneof !== null) return oneof[0];

  const result: Record<string, unknown> = {};
  for (const key of keys) {
    const raw = value[key];
    if (raw === undefined) continue;
    const snakeKey = camelToSnake(key);
    const converted =
      Array.isArray(raw) && AMOUNT_TUPLE_KEYS.has(snakeKey)
        ? raw.map((item) => toAmountTuple(item))
        : convertFromProto(raw);
    if (converted === undefined) continue;
    result[snakeKey] = converted;
  }
  return result;
}

export function fromProtoWire<T = unknown>(value: unknown): T {
  return convertFromProto(value) as T;
}

function looksLikeFlattenedUnion(value: Record<string, unknown>): value is Record<string, unknown> & { kind: string } {
  return typeof value.kind === "string";
}

function convertToProto(value: unknown): unknown {
  if (Array.isArray(value)) return value.map((item) => convertToProto(item));
  if (!isPlainObject(value)) return value;

  if (looksLikeFlattenedUnion(value)) {
    const { kind, ...rest } = value;
    return { kind: { case: snakeToCamel(kind), value: convertToProto(rest) } };
  }

  const result: Record<string, unknown> = {};
  for (const key of Object.keys(value)) {
    const raw = value[key];
    if (raw === undefined) continue;
    result[snakeToCamel(key)] = convertToProto(raw);
  }
  return result;
}

export function toProtoWire(value: unknown): unknown {
  return convertToProto(value);
}

const BIGINT_FIELDS = new Set(["clientSeq", "hostUserId", "userId", "deckId"]);

function coerceBigints(value: unknown): unknown {
  if (Array.isArray(value)) return value.map((item) => coerceBigints(item));
  if (!isPlainObject(value)) return value;
  const result: Record<string, unknown> = {};
  for (const [key, raw] of Object.entries(value)) {
    result[key] = BIGINT_FIELDS.has(key) && typeof raw === "number" ? BigInt(raw) : coerceBigints(raw);
  }
  return result;
}

export function deckDetailFromProto(proto: unknown): DeckDetail {
  return fromProtoWire<DeckDetail>(proto);
}

export function deckSummaryListFromProto(proto: readonly unknown[]): DeckSummary[] {
  return proto.map((item) => fromProtoWire<DeckSummary>(item));
}

export function saveDeckToProto(deck: SaveDeckRequest): ProtoSaveDeckRequest {
  return toProtoWire(deck) as ProtoSaveDeckRequest;
}

export function catalogCardsFromProto(cards: readonly unknown[]): CatalogCard[] {
  return cards.map((item) => fromProtoWire<CatalogCard>(item));
}

export function seedRequestToProto(request: SeedRequest): ProtoSeedRequest {
  return coerceBigints(toProtoWire(request)) as ProtoSeedRequest;
}

export function seedResponseFromProto(proto: unknown): SeedResponse {
  return fromProtoWire<SeedResponse>(proto);
}

/** `WireIntent` wraps under proto field `intent`, not `kind` — hand-rewrapped here. */
export function intentEnvelopeToProto(envelope: IntentEnvelope): ProtoIntentEnvelope {
  const { kind, ...rest } = envelope.intent;
  return coerceBigints({
    tableId: envelope.table_id,
    clientSeq: envelope.client_seq,
    intent: {
      intent: {
        case: snakeToCamel(kind),
        value: toProtoWire(rest),
      },
    },
  }) as ProtoIntentEnvelope;
}

export function streamFrameFromProto(proto: unknown): StreamFrame {
  return fromProtoWire<StreamFrame>(proto);
}
