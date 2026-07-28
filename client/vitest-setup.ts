// Vitest runs with `environment: "node"` — Foldkit's Scene harness is a virtual DOM, so browser
// globals are absent by design. `@foldkit/ui` still needs `CSS.escape` outside the browser: its
// submodel components build selector-bearing Commands during `update` (Menu's `InertOthers` /
// `FocusButton` and friends escape the element id), so a plain `Menu.update` throws without it.
//
// ponytail: escapes every non-`[\w-]` character rather than implementing CSS Syntax § 4.3.8.
// Tests only need a total function; browsers supply the real `CSS.escape`.
const escapeIdentifier = (value: string): string => String(value).replace(/[^\w-]/g, (character) => `\\${character}`);

globalThis.CSS ??= { escape: escapeIdentifier } as typeof globalThis.CSS;
