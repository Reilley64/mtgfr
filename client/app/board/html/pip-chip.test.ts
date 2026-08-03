import { describe, expect, it } from "vitest";
import { testHtml } from "~/test-html";
import { pipChip } from "./pip-chip";

const h = testHtml<never>();

type VNode = {
  data?: { class?: Record<string, boolean>; style?: Record<string, string>; attrs?: Record<string, string> };
  children?: unknown[];
};

function className(node: unknown): string {
  const n = node as VNode;
  return Object.entries(n.data?.class ?? {})
    .filter(([, active]) => active)
    .map(([name]) => name)
    .join(" ");
}

function styleValue(node: unknown, name: string): string | undefined {
  return (node as VNode).data?.style?.[name];
}

describe("pipChip", () => {
  it("sizes via CSS variables and keeps classes for styling", () => {
    const tree = pipChip(h, { ms: "2", code: "2", sizePx: 14 }) as VNode;

    expect(styleValue(tree, "--sz")).toBe("14px");
    expect(styleValue(tree, "--fsz")).toBe("11px");
    expect(styleValue(tree, "--plate")).toBe("#beb9b2");
    expect(styleValue(tree, "width")).toBeUndefined();
    expect(styleValue(tree, "height")).toBeUndefined();
    expect(styleValue(tree, "background-color")).toBeUndefined();
    expect(styleValue(tree, "color")).toBeUndefined();
    expect(styleValue(tree, "font-size")).toBeUndefined();

    expect(className(tree)).toContain("size-(--sz)");
    expect(className(tree)).toContain("bg-(--plate)");
    expect(className(tree)).toContain("text-[#111]");
    expect(className(tree)).toContain("text-[length:var(--fsz)]");
  });

  it("paints the WUBRG plate for a colored pip", () => {
    const tree = pipChip(h, { ms: "g", code: "G", sizePx: 14 }) as VNode;
    expect(styleValue(tree, "--plate")).toBe("#93b483");
  });

  it("carries the call site's extra class and test id", () => {
    const tree = pipChip(h, {
      ms: "w",
      code: "W",
      sizePx: 28,
      extraClass: "group-hover:-translate-y-1",
      testId: "pip-w",
    }) as VNode;
    expect(className(tree)).toContain("group-hover:-translate-y-1");
    expect((tree as VNode).data?.attrs?.["data-testid"]).toBe("pip-w");
  });
});
