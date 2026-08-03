import { describe, expect, it } from "vitest";
import { testHtml } from "~/test-html";
import { promptCardFace } from "./prompt-card-face";

const h = testHtml<never>();

type VNode = {
  data?: { class?: Record<string, boolean>; attrs?: Record<string, string> };
};

function className(node: unknown): string {
  const n = node as VNode;
  return Object.entries(n.data?.class ?? {})
    .filter(([, active]) => active)
    .map(([name]) => name)
    .join(" ");
}

describe("promptCardFace", () => {
  it("renders art at the card aspect with the size's radius", () => {
    const tree = promptCardFace(h, { print: "bolt-print", label: "Lightning Bolt", size: "sm" }) as VNode;
    expect(className(tree)).toContain("aspect-[150/209]");
    expect(className(tree)).toContain("w-[120px]");
    expect(className(tree)).toContain("rounded-[6px]");
    expect((tree as VNode).data?.attrs?.["data-art-url"]).toBeTruthy();
  });

  it("falls back to a name plate when no print resolves", () => {
    const tree = promptCardFace(h, { print: "", label: "Lightning Bolt", size: "md" }) as VNode;
    expect(className(tree)).toContain("aspect-[150/209]");
    expect(className(tree)).toContain("w-[150px]");
    expect(className(tree)).toContain("rounded-[9px]");
    expect(className(tree)).toContain("bg-morph-slate");
  });

  it("sizes the fluid face for the mulligan hand", () => {
    const tree = promptCardFace(h, { print: "", label: "Forest", size: "fluid" }) as VNode;
    expect(className(tree)).toContain("w-[min(22vw,160px)]");

    const art = promptCardFace(h, { print: "forest-print", label: "Forest", size: "fluid" }) as VNode;
    expect(className(art)).toContain("shadow-hand");
  });
});
