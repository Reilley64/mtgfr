# 0019 — Effect-first client state via atom-solid

Status: **Accepted**; extends [0018](0018-effect-generated-client-and-sse-stream.md).

## Decision

- Async/wire work = Effects/Streams in atoms (`effect/unstable/reactivity` + `@effect/atom-solid` hooks).
- No `createResource(() => run(…))`, no manual fiber lifecycle in components.
- Solid signals/stores for UI-local state and `store.ts` game fold the canvas reads.
- Pin `effect` and `@effect/atom-solid` to same exact beta.

## Consequences

- Shared atoms in `client/src/atoms.ts`; screen-local atoms in owning module. Tests use `AtomRegistry.make()`.
