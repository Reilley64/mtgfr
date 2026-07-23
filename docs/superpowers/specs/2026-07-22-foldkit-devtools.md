# Foldkit DevTools
**Status:** Current (as of 2026-07-23)
**Module:** `client/vite.config.ts`, `client/app/entry.ts`, `.cursor/mcp.json`, `.agents/skills/`

## Problem Statement

Agents and developers need live visibility into the Foldkit runtime while debugging the board and shell. The tooling should attach to the running app without changing product behavior.

## Solution

Enable Foldkit DevTools MCP on the Vite relay port and keep Foldkit runtime devtools metadata available from the app entry point. Vendor Foldkit-focused skills for local agent guidance.

## User Stories

- As an agent, I can list a live Foldkit runtime when the app tab is open.
- As an agent, I can dispatch or inspect Foldkit messages during local debugging.
- As a developer, DevTools setup is documented in the repo instead of rediscovered per task.

## Behavior

- `client/vite.config.ts` passes `devToolsMcpPort: 9988` to `foldkit()`.
- `client/app/entry.ts` keeps `devTools: { Message }` in `Runtime.makeApplication`.
- `.cursor/mcp.json` registers `foldkit-devtools` with `npx @foldkit/devtools-mcp`.
- `foldkit_list_runtimes` sees a runtime only while the client app is open in a browser tab.
- Foldkit agent skills live under `.agents/skills/` and point to this repo’s `client/app/` and `client/node_modules/foldkit`.

## Implementation Decisions

- The MCP server is development tooling only and does not affect production board behavior.
- Port `9988` is the agreed Vite relay port for this repo.
- The app exposes the board message schema to DevTools through `Message`.
- The repo uses installed Foldkit sources and vendored skills; no framework source subtree is added.

## Testing Decisions

- Manual tooling check: run the client, open the app in a browser, and confirm `foldkit_list_runtimes` sees the runtime.
- Config review checks `client/vite.config.ts`, `client/app/entry.ts`, and `.cursor/mcp.json`.

## Out of Scope

- Playable-chrome or board UI behavior.
- Product telemetry or observability wiring.
- Adding a Foldkit source subtree to the repo.

## Further Notes

- Board feature specs document product behavior; this spec documents only DevTools MCP setup and supporting skills.
