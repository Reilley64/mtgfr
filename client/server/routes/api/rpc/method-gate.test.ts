import { createServer } from "node:http";
import { createRouter, toNodeListener } from "h3";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import rpcHandler from "./[...path]";

describe("/api/rpc method gate", () => {
  let port = 0;
  let server: ReturnType<typeof createServer>;

  beforeAll(async () => {
    const router = createRouter().all("/api/rpc/**", rpcHandler);
    server = createServer(toNodeListener(router));
    await new Promise<void>((resolve) => server.listen(0, resolve));
    port = (server.address() as { port: number }).port;
  });

  afterAll(async () => {
    await new Promise<void>((resolve, reject) => server.close((err) => (err ? reject(err) : resolve())));
  });

  it("returns 405 for methods outside GET/POST/PUT/DELETE", async () => {
    const res = await fetch(`http://127.0.0.1:${port}/api/rpc/auth/me`, { method: "PATCH" });
    expect(res.status).toBe(405);
    expect(await res.text()).toBe("Method Not Allowed");
  });

  it("allows exported GET through the gate", async () => {
    const res = await fetch(`http://127.0.0.1:${port}/api/rpc/auth/me`, { method: "GET" });
    expect(res.status).not.toBe(405);
  });
});
