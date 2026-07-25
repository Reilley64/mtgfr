import type { MessageRef } from "../wire/types";

export function testMessageRef(text: string): MessageRef {
  return { key: "card.name", params: [{ name: "name", string_value: text }], children: [] };
}
