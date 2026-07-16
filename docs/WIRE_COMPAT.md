# Wire backwards compatibility

Durable rules for the OpenAPI / `crates/schema` wire contract during a rolling deploy. See
[ADR 0030](adr/0030-table-instance-affinity-for-drain-rolls.md) and
[ADR 0021](adr/0021-live-games-in-memory-only.md).

## Why this exists

Rolling deploy keeps **outgoing** API pods Terminating (SIGTERM drain) while **newest** accepts
new tables via Service `edh-api`. The SolidStart SPA may roll with newest; mid-game clients still
talk to older pods via BFF `table_routes` → pod DNS on the headless Service.

So every concurrent instance version must speak a wire protocol the current SPA can parse —
**expand-only** across the whole set until grace expires / pods exit.

## 1. Compatibility window

All concurrent API binaries until each Terminating pod exits (tables empty or
`terminationGracePeriodSeconds`). No ConfigMap peer registry.

## 2. Expand-only during that window

Within one release's changes to `crates/schema`, `crates/server`, and `openapi.json`:

- **Additive optional fields only.** New fields get `#[serde(default)]` / `Option<T>`.
- **New endpoints / Intent / Event variants** are safe to add — old peers never send them.
- **Do not rename, remove, or repurpose** fields while any older binary may still serve a table
  the current SPA still reaches via `table_routes`.

## 3. Lobby vs game

Lobby HTTP is owned by the BFF (`mtgfr_web`). Game stream/intent paths stay on Axum; table id is
in the **path** (`/tables/{table}/intent/v1`, etc.).
