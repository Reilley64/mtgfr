import { expect, test } from "vitest";
import { parseTableCode } from "./code";

test("parseTableCode reads table from /play/:deckId/:table", () => {
  expect(parseTableCode("http://localhost/play/7/ABC123")).toBe("ABC123");
});

test("parseTableCode still accepts bare codes", () => {
  expect(parseTableCode("abc123")).toBe("ABC123");
});
