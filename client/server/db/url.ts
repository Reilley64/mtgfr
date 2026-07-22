/** Local docker-compose default — same host/creds as `config/mtgfr.toml`, DB `mtgfr_web`. */
export const DEFAULT_WEB_DATABASE_URL = "postgresql://mtgfr:mtgfr@127.0.0.1:5432/mtgfr_web";

export function webDatabaseUrl(): string {
  return process.env.WEB_DATABASE_URL ?? DEFAULT_WEB_DATABASE_URL;
}
