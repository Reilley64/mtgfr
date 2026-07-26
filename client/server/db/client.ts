// Drizzle query builder over `@effect/sql-pg` via the native `drizzle-orm/effect-postgres` driver.
// Queries are Effects: `yield* db.select().from(...)` inside an Effect that provides `WebDb`.

import { PgClient } from "@effect/sql-pg";
import * as PgDrizzle from "drizzle-orm/effect-postgres";
import * as Context from "effect/Context";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as ManagedRuntime from "effect/ManagedRuntime";
import * as Redacted from "effect/Redacted";
import { webDatabaseUrl } from "./url";

export { DEFAULT_WEB_DATABASE_URL, webDatabaseUrl } from "./url";

const dbEffect = PgDrizzle.makeWithDefaults();

type WebDbHandle = Effect.Success<typeof dbEffect>;

/** Effect Drizzle handle for `mtgfr_web`. Provide via `WebDbLive` (or a `webDbLayer(url)`). */
export class WebDb extends Context.Service<WebDb, WebDbHandle>()("WebDb") {}

/** Build a `WebDb` layer over a `PgClient` pool for `url`. */
export function webDbLayer(url = webDatabaseUrl()) {
  const client = PgClient.layer({
    url: Redacted.make(url),
    maxConnections: 4,
  });
  return Layer.effect(WebDb, dbEffect).pipe(Layer.provide(client));
}

/** Default `WebDb` layer; reads `WEB_DATABASE_URL` when the layer is built (not at import). */
export const WebDbLive = Layer.unwrap(Effect.sync(() => webDbLayer()));

function makeRuntime(url: string) {
  return ManagedRuntime.make(webDbLayer(url));
}

let cache: { url: string; runtime: ReturnType<typeof makeRuntime> } | null = null;

/**
 * Temporary Promise bridge for callers not yet running Effects at their edge.
 * Prefer `yield* op.pipe(Effect.provide(WebDbLive))` inside an existing Effect;
 * this reuses one pooled runtime per URL so store ops can run from Promise code.
 */
export function runWebDb<A, E>(effect: Effect.Effect<A, E, WebDb>, url = webDatabaseUrl()): Promise<A> {
  if (cache?.url !== url) {
    cache = { url, runtime: makeRuntime(url) };
  }
  return cache.runtime.runPromise(effect);
}
