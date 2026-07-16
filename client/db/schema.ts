import { boolean, integer, pgTable, primaryKey, text, timestamp } from "drizzle-orm/pg-core";

/** Pre-game lobby row — SolidStart / mtgfr_web only (not Axum Toasty). */
export const lobbies = pgTable("lobbies", {
  tableId: text("table_id").primaryKey(),
  hostUserId: integer("host_user_id").notNull(),
  createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  lastActivity: timestamp("last_activity", { withTimezone: true }).notNull().defaultNow(),
  startedAt: timestamp("started_at", { withTimezone: true }),
});

export const lobbySeats = pgTable(
  "lobby_seats",
  {
    tableId: text("table_id")
      .notNull()
      .references(() => lobbies.tableId, { onDelete: "cascade" }),
    seat: integer("seat").notNull(),
    userId: integer("user_id").notNull(),
    username: text("username").notNull(),
    deckId: integer("deck_id").notNull(),
    deckName: text("deck_name").notNull(),
    ready: boolean("ready").notNull().default(false),
  },
  (t) => [primaryKey({ columns: [t.tableId, t.seat] })],
);

/** In-game BFF routing: table → API pod DNS (TTL + explicit delete). */
export const tableRoutes = pgTable("table_routes", {
  tableId: text("table_id").primaryKey(),
  podDns: text("pod_dns").notNull(),
  createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  expiresAt: timestamp("expires_at", { withTimezone: true }).notNull(),
});
