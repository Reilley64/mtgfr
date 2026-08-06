# Docs

- **[../openspec/specs/](../openspec/specs/)** — living OpenSpec requirements (source of truth for behavior and architecture)
- **[fidelity/](fidelity/)** — per-deck fidelity reports + increments backlogs (owned by the `fidelity-grind` skill)
- **[WIRE_COMPAT.md](WIRE_COMPAT.md)** — expand-only N↔N+1 wire rules during a rolling deploy's drain window; `/v2` for hard breaks
- **[decklists/](decklists/)** — frozen precon target lists (`*.md`; source for legality fixtures)
- **[precons/](precons/)** — frozen soc decklist text files (build input for `decklists/*.md`)
- **[AGENT_NAVIGATION.md](AGENT_NAVIGATION.md)** / **[CR_INDEX.md](CR_INDEX.md)** — Comprehensive Rules ↔ engine navigation (`just engine-cr-index`)
- **[CLIENT_CANVAS_MAP.md](CLIENT_CANVAS_MAP.md)** — client board paint / hits / flights / overlay module map

Root docs: [`README.md`](../README.md) (project overview), `CONTEXT.md` (glossary), `PRODUCT.md`, `DESIGN.md`, `AGENTS.md` (contributor/agent workflow).

Production topology and operations live in
[`../openspec/specs/production-and-ops/spec.md`](../openspec/specs/production-and-ops/spec.md).
