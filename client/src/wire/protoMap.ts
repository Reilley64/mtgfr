// The BFF-side adapter between the generated Effect-gRPC proto shapes (`~/wire/generated`) and
// the hand-maintained browser wire shapes (`~/wire/types`, ADR 0032). Effect-gRPC decodes proto
// messages into camelCase JS objects with `bigint` for int64 and `{ case, value }` oneofs nested
// under a field (`kind` | `intent` | `event` | `choice` | `frame`); the browser wire shapes are
// snake_case tagged unions (`{ kind: "play_land", ... }`) to stay serde-compatible on the wire to
// the browser. Rather than hand-write a converter per message (the game-state graph is large and
// recursive — see `VisibleState`), `fromProtoWire`/`toProtoWire` walk the two shapes structurally:
// bigint → number, camelCase ↔ snake_case, and a handful of shape-recognizable patterns
// (`ObjectIdList`, `ObjectAmount`/`PlayerAmount`, the oneof wrapper). This buys one conversion that
// covers every message instead of ~40 hand-maintained ones, at the cost of a few structural
// heuristics documented inline where they could otherwise be ambiguous.

import type {
  CatalogCard,
  DeckDetail,
  DeckSummary,
  IntentEnvelope,
  SaveDeckRequest,
  SeedRequest,
  SeedResponse,
  StreamFrame,
  WireIntent,
} from "~/wire/types";

/** Oneof wrapper field names the wire schemas use. All but `frame` flatten to a `kind` tag on the
 * browser side (`StreamFrame` keeps its own field name since it's already called `frame` there). */
const ONEOF_WRAPPER_KEYS = new Set(["kind", "intent", "event", "choice", "frame"]);

/** `assignment`/`players` arrays whose *elements* need the `ObjectAmount`/`PlayerAmount` struct →
 * tuple collapse (proto has no tuple type, so divided-damage events carry `{id,amount}` structs
 * where the browser wire format uses `[id, amount]`). Only inbound (`VisibleEvent`) fields use this
 * shape — outbound `WireIntent` damage-assignment fields carry `WireDamage`/`WireSpellDamage`
 * structs already, which don't match the `{id,amount}`/`{player,amount}` shape below and pass
 * through unchanged. */
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

/** Detects a oneof wrapper: a message whose *entire* content is one field named `kind`/`intent`/
 * `event`/`choice`/`frame` holding an effect-grpc `{ case, value }` union (e.g. `WireTarget`,
 * `WireKind`, `WireIntent`, `VisibleEvent`, `PendingChoiceView`, `StreamFrame`). Detecting it
 * structurally — rather than by message name — is what lets one function flatten all of them.
 * Returns `null` when `value` isn't shaped like a oneof wrapper at all (caller falls through to
 * normal field-by-field conversion), or a single-element tuple `[result]` when it is — `result`
 * is `undefined` for an unset oneof (CR-less "this field is simply absent" case), distinguishing
 * "no oneof here" from "a oneof, but it's empty" without a second sentinel value. */
function flattenOneofWrapper(value: Record<string, unknown>): [unknown] | null {
  const keys = Object.keys(value);
  if (keys.length !== 1) return null;
  const [wrapperKey] = keys;
  if (!ONEOF_WRAPPER_KEYS.has(wrapperKey)) return null;
  const oneof = value[wrapperKey];
  if (!isPlainObject(oneof) || !("case" in oneof)) return null;
  const caseValue = oneof.case;
  if (caseValue == null) return [undefined]; // unset oneof: treat the whole field as absent
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

/** Proto/Effect-gRPC → browser wire shape (`~/wire/types`): bigint → number, camelCase →
 * snake_case, oneofs flattened to a `kind`/`frame` tag, `ObjectIdList` unwrapped to a plain array,
 * and `ObjectAmount`/`PlayerAmount` arrays collapsed to `[id, amount]` tuples. */
export function fromProtoWire<T = unknown>(value: unknown): T {
  return convertFromProto(value) as T;
}

/** A flattened tagged union looks like `{ kind: "creature", power: 2, toughness: 2 }` — the shape
 * every `WireKind`/`WireTarget`-style union takes on the browser side. Detecting it by the
 * presence of a string `kind` field is safe for `toProtoWire`'s actual scope (request payloads:
 * `WireIntent`'s nested `target`s and no card/deck/seed shape uses a `kind` field), but is *not*
 * safe in general — `WireIntent` itself uses `kind` as its tag while the proto wrapper field is
 * named `intent`, not `kind`, which is why `intentEnvelopeToProto` handles that one case by hand
 * instead of relying on this heuristic. */
function looksLikeFlattenedUnion(value: Record<string, unknown>): boolean {
  return typeof value.kind === "string";
}

function convertToProto(value: unknown): unknown {
  if (Array.isArray(value)) return value.map((item) => convertToProto(item));
  if (!isPlainObject(value)) return value;

  if (looksLikeFlattenedUnion(value)) {
    const { kind, ...rest } = value;
    return { kind: { case: snakeToCamel(kind as string), value: convertToProto(rest) } };
  }

  const result: Record<string, unknown> = {};
  for (const key of Object.keys(value)) {
    const raw = value[key];
    if (raw === undefined) continue;
    result[snakeToCamel(key)] = convertToProto(raw);
  }
  return result;
}

/** Browser wire shape → proto/Effect-gRPC: snake_case → camelCase, and (via `looksLikeFlattenedUnion`)
 * `{kind:"object",id}`-style tagged unions rewrapped under `{ kind: { case, value } }`. Does not
 * handle `WireIntent`'s `intent`-named wrapper (see `intentEnvelopeToProto`) or bigint fields (see
 * `coerceBigints`) — both need message-specific knowledge this generic pass doesn't have. */
export function toProtoWire(value: unknown): unknown {
  return convertToProto(value);
}

/** int64 fields across the request messages this module converts. `toProtoWire` can't tell a
 * bigint-typed field from a number-typed one structurally, so the specialized `*ToProto` wrappers
 * below run this afterward wherever the target schema declares `Schema.BigInt`. */
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

export function deckSummaryListFromProto(proto: unknown): DeckSummary[] {
  return (Array.isArray(proto) ? proto : []).map((item) => fromProtoWire<DeckSummary>(item));
}

export function saveDeckToProto(deck: SaveDeckRequest): unknown {
  return toProtoWire(deck);
}

export function catalogCardsFromProto(cards: unknown): CatalogCard[] {
  return (Array.isArray(cards) ? cards : []).map((item) => fromProtoWire<CatalogCard>(item));
}

export function seedRequestToProto(request: SeedRequest): unknown {
  return coerceBigints(toProtoWire(request));
}

export function seedResponseFromProto(proto: unknown): SeedResponse {
  return fromProtoWire<SeedResponse>(proto);
}

/** `IntentEnvelope.intent` is a `WireIntent` message whose *own* sole field (also named `intent`)
 * carries the oneof — the one wrapper in this schema set where the proto field name (`intent`)
 * differs from the browser tag key (`kind`), so `toProtoWire`'s generic `kind`-shaped detection
 * would rewrap it under the wrong field name. Handled by hand; the variant's own fields (including
 * any nested `WireTarget`) still go through `toProtoWire` since those *do* use matching names. */
export function intentEnvelopeToProto(envelope: IntentEnvelope): unknown {
  const { kind, ...rest } = envelope.intent as WireIntent & { kind: string };
  return coerceBigints({
    tableId: envelope.table_id,
    clientSeq: envelope.client_seq,
    intent: {
      intent: {
        case: snakeToCamel(kind),
        value: toProtoWire(rest),
      },
    },
  });
}

export function streamFrameFromProto(proto: unknown): StreamFrame {
  return fromProtoWire<StreamFrame>(proto);
}
