import { defineConfig } from "drizzle-kit";
import { DEFAULT_WEB_DATABASE_URL } from "./server/db/url";

export default defineConfig({
  schema: "./db/schema.ts",
  out: "./db/migrations",
  dialect: "postgresql",
  dbCredentials: {
    url: process.env.WEB_DATABASE_URL ?? DEFAULT_WEB_DATABASE_URL,
  },
});
