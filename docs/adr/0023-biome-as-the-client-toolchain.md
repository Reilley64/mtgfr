# 0023 — Biome as client toolchain

Status: **Accepted**; required by [0024](0024-tailwind-as-the-design-system-runtime.md).

## Decision

- Biome 2.5.3 only — format, lint, organize imports (`assist.actions.source.organizeImports`, `sortBareImports: true`). Domains: `solid`, `test` (recommended).
- Exclude `src/api/generated.ts`. CSS: `css.parser.tailwindDirectives: true`.
- `nursery/useSortedClasses` at **error** for Tailwind class sorting (and `cn` / `clsx` string args). Shared class constants go through `cn("…")` so the rule sees them. Consistent order makes repeated utility sequences longer LZ77 matches under **gzip** on the shipped JS/HTML (see Deployment PRD `mtgfr static` compression). The rule's fix is **unsafe** — `bun run lint:fix` alone will not sort; use `bunx biome check --write --unsafe --only=lint/nursery/useSortedClasses` (or apply the editor fix).

## Consequences

- `bun run format` / `bun run lint` / `lint:fix`. Unsorted utilities fail lint; remediating them needs the unsafe Biome fix above, not `lint:fix`. JSX prop order convention (`class` then `style` last) is review-only — Biome can't enforce it.
