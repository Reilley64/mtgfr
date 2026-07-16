#!/usr/bin/env sh
# Apply Drizzle migrations to mtgfr_web (WEB_DATABASE_URL).
set -e
cd "$(dirname "$0")/.."
if [ -z "${WEB_DATABASE_URL:-}" ]; then
  echo "WEB_DATABASE_URL is required" >&2
  exit 1
fi
exec bunx drizzle-kit migrate
