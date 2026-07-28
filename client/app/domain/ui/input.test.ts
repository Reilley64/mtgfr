import { html } from "foldkit/html";
import { describe, expect, it } from "vitest";
import { input } from "./input";

type Msg = { _tag: "typed"; value: string };

const h = html<Msg>();
const typed = (value: string): Msg => ({ _tag: "typed", value });

/** snabbdom stores classes as a truth-map under data.class. */
function classes(node: unknown): string[] {
  const n = node as { data?: { class?: Record<string, boolean> } };
  return Object.keys(n.data?.class ?? {});
}

function attrs(node: unknown): Record<string, string> {
  const n = node as { data?: { attrs?: Record<string, string> } };
  return n.data?.attrs ?? {};
}

function props(node: unknown): Record<string, unknown> {
  const n = node as { data?: { props?: Record<string, unknown> } };
  return n.data?.props ?? {};
}

/** snabbdom stores event listeners under data.on, separate from data.props/data.attrs. */
function on(node: unknown): Record<string, unknown> {
  const n = node as { data?: { on?: Record<string, unknown> } };
  return n.data?.on ?? {};
}

describe("input", () => {
  it("renders glass-on-vine field chrome by default", () => {
    const node = input(h, { id: "auth-email" });

    expect((node as { sel?: string }).sel).toBe("input");
    expect(classes(node)).toEqual(
      expect.arrayContaining(["rounded-control", "border", "border-vine", "bg-glass", "text-body", "text-snow"]),
    );
  });

  it("renders hud as borderless glass sized for prompt chrome", () => {
    const node = input(h, { id: "pick-card-filter", variant: "hud" });

    expect(classes(node)).toEqual(expect.arrayContaining(["rounded-hud", "bg-glass", "shrink-0", "text-body"]));
    expect(classes(node)).not.toContain("border-vine");
  });

  it("lets a call-site utility win over the variant", () => {
    const node = input(h, { id: "share-code", class: "text-chip" });

    expect(classes(node)).toContain("text-chip");
    expect(classes(node)).not.toContain("text-body");
  });

  it("merges an array of call-site classes with the variant", () => {
    const node = input(h, { id: "coverage-search", class: ["w-full", "text-chip"] });

    expect(classes(node)).toContain("w-full");
    expect(classes(node)).toContain("text-chip");
    expect(classes(node)).not.toContain("text-body");
    expect(classes(node)).toContain("bg-glass");
  });

  it("sizes a hud field from the call site without losing the prompt chrome", () => {
    const node = input(h, { id: "pick-card-filter", variant: "hud", class: "w-[min(90vw,320px)]" });

    expect(classes(node)).toContain("w-[min(90vw,320px)]");
    expect(classes(node)).toEqual(expect.arrayContaining(["rounded-hud", "bg-glass", "shrink-0"]));
  });

  it("identifies the field so a label can point at it", () => {
    expect(props(input(h, { id: "deck-name" })).id).toBe("deck-name");
  });

  it("carries the caller's value, placeholder and type", () => {
    const node = input(h, { id: "pool-search", type: "search", value: "Sol Ring", placeholder: "Search cards" });

    expect(props(node).value).toBe("Sol Ring");
    expect(props(node).placeholder).toBe("Search cards");
    expect(props(node).type).toBe("search");
  });

  it("types text unless the caller asks for another type", () => {
    expect(props(input(h, { id: "deck-name" })).type).toBe("text");
  });

  it("takes focus on open when asked", () => {
    expect(props(input(h, { id: "prompt-name-input", autofocus: true })).autofocus).toBe(true);
    expect(props(input(h, { id: "prompt-name-input" })).autofocus).toBeUndefined();
  });

  it("dispatches a message on every keystroke", () => {
    const node = input(h, { id: "pool-search", onInput: typed });

    expect(typeof on(node).input).toBe("function");
  });

  it("exposes the test id and aria label callers depend on", () => {
    const node = input(h, { id: "deck-list-search", testId: "deck-list-search", ariaLabel: "Search decks" });

    expect(attrs(node)["data-testid"]).toBe("deck-list-search");
    expect(attrs(node)["aria-label"]).toBe("Search decks");
  });

  it("passes extra attrs through to the field", () => {
    const node = input(h, { id: "table-code", attrs: [h.Autocomplete("off"), h.Spellcheck(false)] });

    expect(props(node).autocomplete).toBe("off");
    expect(attrs(node).spellcheck).toBe("false");
  });

  it("drops a null class instead of rendering it as a literal class name", () => {
    const node = input(h, { id: "auth-email", class: null });

    expect(classes(node)).not.toContain("null");
    expect(classes(node)).toContain("bg-glass");
  });
});
