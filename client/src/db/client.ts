/**
 * Drizzle over `@effect/sql-pg` (Effect v4).
 *
 * Official `@effect/sql-drizzle` is still on Effect 3; this mirrors its pg-proxy
 * remote callback so schema/query builder stay Drizzle while the wire driver is
 * Effect's PgClient (`pg` pool).
 */

import { PgClient } from "@effect/sql-pg";
import { drizzle } from "drizzle-orm/pg-proxy";
import type { PgRemoteDatabase } from "drizzle-orm/pg-proxy";
import * as Effect from "effect/Effect";
import * as ManagedRuntime from "effect/ManagedRuntime";
import * as Redacted from "effect/Redacted";
import * as Result from "effect/Result";
import { SqlClient } from "effect/unstable/sql";
import * as schema from "../../db/schema";

export type WebDb = PgRemoteDatabase<typeof schema>;

type SqlRuntime = ManagedRuntime.ManagedRuntime<SqlClient.SqlClient | PgClient.PgClient, unknown>;

type Cache = {
  url: string;
  runtime: SqlRuntime;
  db: WebDb;
};

let cache: Cache | null = null;

function remoteCallback(runtime: SqlRuntime) {
  return (sql: string, params: unknown[], method: "all" | "execute" | "get" | "values") => {
    const program = Effect.gen(function* () {
      const client = yield* SqlClient.SqlClient;
      const statement = client.unsafe(sql, params as ReadonlyArray<unknown>);

      if (method === "execute") {
        const header = yield* statement.raw;
        return { rows: [header] as unknown[] };
      }

      if (method === "all" || method === "values") {
        const rows = yield* statement.values;
        return { rows: rows as unknown[] };
      }

      const rows = yield* statement.withoutTransform;
      if (method === "get") {
        return { rows: [(rows[0] ?? []) as unknown] };
      }
      return { rows: rows as unknown[] };
    });

    return runtime.runPromise(Effect.result(program)).then((res) => {
      if (Result.isFailure(res)) {
        throw res.failure;
      }
      return res.success;
    });
  };
}

/** Shared Drizzle client for SolidStart BFF (`mtgfr_web` only), backed by `@effect/sql-pg`. */
export function createWebDb(url = process.env.WEB_DATABASE_URL): WebDb {
  if (!url) {
    throw new Error("WEB_DATABASE_URL is required");
  }
  if (cache?.url === url) {
    return cache.db;
  }

  const runtime = ManagedRuntime.make(
    PgClient.layer({
      url: Redacted.make(url),
      maxConnections: 4,
    }),
  );
  const db = drizzle(remoteCallback(runtime), { schema });
  cache = { url, runtime, db };
  return db;
}
