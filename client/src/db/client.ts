// Drizzle query builder over `@effect/sql-pg` via pg-proxy (`@effect/sql-drizzle` is still Effect 3).

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
