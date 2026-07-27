import { expect, test } from "vitest";
import { FORMULATOR_FOR_KIND } from "../choice";
import { PendingChoiceViewSchema, VisibleEventSchema } from "./generated/mtgfr/v1/stream_pb";
import { VISIBLE_EVENT_KIND_PRESENCE } from "./visibleEventKindPresence";

/** Mirrors `camelToSnake` in protoMap — keep in sync when renaming. */
function camelToSnake(key: string): string {
  return key.replace(/[A-Z]/g, (m) => `_${m.toLowerCase()}`);
}

function generatedOneofKinds(schema: { field: Record<string, unknown> }): string[] {
  return Object.keys(schema.field).map(camelToSnake).sort();
}

test("hand PendingChoiceView kinds match generated proto oneof", () => {
  expect(Object.keys(FORMULATOR_FOR_KIND).sort()).toEqual(generatedOneofKinds(PendingChoiceViewSchema));
});

test("hand VisibleEvent kinds match generated proto oneof", () => {
  expect(Object.keys(VISIBLE_EVENT_KIND_PRESENCE).sort()).toEqual(generatedOneofKinds(VisibleEventSchema));
});
