import { describe, expect, it } from "vitest";
import { json, readJsonObject, tableParam, unknownLobby } from "./lobby-http";
import type { H3Event } from "nitro/h3";

describe("lobby-http", () => {
  it("json sets content-type and status", async () => {
    const res = json({ ok: true }, 201);
    expect(res.status).toBe(201);
    expect(res.headers.get("content-type")).toBe("application/json");
    await expect(res.json()).resolves.toEqual({ ok: true });
  });

  it("tableParam reads event.context.params.table", () => {
    const event = {
      context: { params: { table: "ABC123" } },
    } as unknown as H3Event;
    expect(tableParam(event)).toBe("ABC123");
  });

  it("tableParam returns null when missing or empty", () => {
    expect(tableParam({ context: { params: {} } } as unknown as H3Event)).toBeNull();
    expect(tableParam({ context: {} } as unknown as H3Event)).toBeNull();
  });

  it("readJsonObject parses valid JSON body", async () => {
    const event = {
      req: { text: async () => '{"table_id":"T1","ready":true}' },
    } as unknown as H3Event;
    await expect(readJsonObject(event)).resolves.toEqual({ table_id: "T1", ready: true });
  });

  it("readJsonObject returns null on invalid JSON", async () => {
    const event = {
      req: { text: async () => "not-json" },
    } as unknown as H3Event;
    await expect(readJsonObject(event)).resolves.toBeNull();
  });

  it("unknownLobby returns empty snapshot with hostUserId 0", () => {
    expect(unknownLobby("T99")).toEqual({
      tableId: "T99",
      hostUserId: 0,
      startedAt: null,
      seats: [],
    });
  });
});
