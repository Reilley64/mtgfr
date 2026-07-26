import { html } from "foldkit/html";
import { describe, expect, it } from "vitest";
import { shellFrame } from "./shell-frame";

type Msg = { _tag: "noop" };

const h = html<Msg>();

function collectTestIds(node: unknown, out: string[] = []): string[] {
  if (node == null || typeof node !== "object") return out;
  const n = node as { data?: { attrs?: Record<string, string> }; children?: unknown[] };
  const id = n.data?.attrs?.["data-testid"];
  if (typeof id === "string") out.push(id);
  for (const child of n.children ?? []) collectTestIds(child, out);
  return out;
}

function findByTestId(node: unknown, testId: string): unknown {
  if (node == null || typeof node !== "object") return null;
  const n = node as { data?: { attrs?: Record<string, string> }; children?: unknown[] };
  if (n.data?.attrs?.["data-testid"] === testId) return node;
  for (const child of n.children ?? []) {
    const found = findByTestId(child, testId);
    if (found != null) return found;
  }
  return null;
}

function classNameOf(node: unknown): string {
  const n = node as { data?: { class?: Record<string, boolean> } };
  return Object.entries(n.data?.class ?? {})
    .filter(([, active]) => active)
    .map(([name]) => name)
    .join(" ");
}

describe("shellFrame", () => {
  it("renders header slots, stage, auth atmosphere, and version badge", () => {
    const tree = shellFrame(h, {
      atmosphere: "auth",
      title: "Sign in",
      subtitle: "Welcome",
      leading: h.div([h.DataAttribute("testid", "lead")], ["Back"]),
      trailing: h.div([h.DataAttribute("testid", "trail")], ["Go"]),
      stage: h.div([h.DataAttribute("testid", "stage-child")], ["Body"]),
      chrome: { version: "1.2.3", faithfulCount: null, oracleTotal: null, coverageHref: null },
    });

    const ids = collectTestIds(tree);
    expect(ids).toContain("shell-frame");
    expect(ids).toContain("shell-header");
    expect(ids).toContain("shell-header-leading");
    expect(ids).toContain("shell-header-title");
    expect(ids).toContain("shell-header-trailing");
    expect(ids).toContain("shell-stage");
    expect(ids).toContain("stage-child");
    expect(ids).toContain("app-version");
    expect(classNameOf(findByTestId(tree, "shell-frame"))).toContain("shell-atmosphere-auth");
    expect(classNameOf(findByTestId(tree, "shell-stage"))).toContain("shell-stage-enter");
  });

  it("uses shell atmosphere variant", () => {
    const tree = shellFrame(h, {
      atmosphere: "shell",
      title: "Decks",
      stage: [],
      chrome: { version: null, faithfulCount: null, oracleTotal: null, coverageHref: null },
    });

    expect(classNameOf(findByTestId(tree, "shell-frame"))).toContain("shell-atmosphere-shell");
  });
});
