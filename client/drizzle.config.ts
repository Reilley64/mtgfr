import { defineConfig } from "drizzle-kit";

export default defineConfig({
  schema: "./db/schema.ts",
  out: "./db/migrations",
  dialect: "postgresql",
  dbCredentials: {
    url: process.env.WEB_DATABASE_URL ?? "postgresql://mtgfr:mtgfr@127.0.0.1:5432/mtgfr_web",
  },
});
