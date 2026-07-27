import { createServer } from "node:http";
import { createRouter, defineEventHandler, toNodeListener } from "h3";
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import { handleCardArtProxyRequest } from "./proxy.get";

describe("/api/card-art/proxy", () => {
  let port = 0;
  let server: ReturnType<typeof createServer>;

  beforeAll(async () => {
    const handler = defineEventHandler((event) =>
      handleCardArtProxyRequest(event, {
        buildGrpcEnv: async (sessionToken) => ({ sessionToken, traceparent: null }),
        fetchMe: vi.fn(async () => ({ id: 7, email: "player@example.com", username: "player" })),
        fetchProxyArt: vi.fn(async (rawUrl: string) => {
          if (rawUrl.includes("127.0.0.1")) return { ok: false as const, status: 400 as const };
          return {
            ok: true as const,
            body: Uint8Array.from([1, 2, 3]),
            contentType: "image/png",
          };
        }),
      }),
    );
    const router = createRouter().get("/api/card-art/proxy", handler);
    server = createServer(toNodeListener(router));
    await new Promise<void>((resolve) => server.listen(0, resolve));
    port = (server.address() as { port: number }).port;
  });

  afterAll(async () => {
    await new Promise<void>((resolve, reject) => server.close((err) => (err ? reject(err) : resolve())));
  });

  it("returns 401 without a session cookie", async () => {
    const res = await fetch(`http://127.0.0.1:${port}/api/card-art/proxy?url=https%3A%2F%2Fcdn.example.com%2Fa.png`);
    expect(res.status).toBe(401);
    expect(await res.text()).toBe("Unauthorized");
  });

  it("returns 400 for authenticated unsafe targets", async () => {
    const res = await fetch(`http://127.0.0.1:${port}/api/card-art/proxy?url=https%3A%2F%2F127.0.0.1%2Fa.png`, {
      headers: { cookie: "session=tok" },
    });
    expect(res.status).toBe(400);
    expect(await res.text()).toBe("Bad Request");
  });

  it("returns proxied image bytes with cache headers for authenticated players", async () => {
    const res = await fetch(`http://127.0.0.1:${port}/api/card-art/proxy?url=https%3A%2F%2Fcdn.example.com%2Fa.png`, {
      headers: { cookie: "session=tok" },
    });
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toBe("image/png");
    expect(res.headers.get("cache-control")).toBe("private, max-age=300");
    expect(new Uint8Array(await res.arrayBuffer())).toEqual(Uint8Array.from([1, 2, 3]));
  });
});
