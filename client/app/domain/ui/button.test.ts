import { html } from "foldkit/html";
import { describe, expect, it } from "vitest";
import { button } from "./button";

type Msg = { _tag: "clicked" };

const h = html<Msg>();
const clicked: Msg = { _tag: "clicked" };

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

describe("button", () => {
  it("renders llanowar primary chrome by default", () => {
    const node = button(h, {}, ["Play"]);

    expect((node as { sel?: string }).sel).toBe("button");
    expect(classes(node)).toContain("bg-llanowar");
    expect(classes(node)).toContain("text-snow-mint");
  });

  it("renders danger as burn-red ink on control chrome", () => {
    const node = button(h, { variant: "danger" }, ["Concede"]);

    expect(classes(node)).toContain("border-burn-red");
    expect(classes(node)).toContain("text-burn-red");
    expect(classes(node)).toContain("rounded-control");
  });

  it("renders link as a vine underline with no button fill", () => {
    const node = button(h, { variant: "link" }, ["Create one"]);

    expect(classes(node)).toContain("text-vine");
    expect(classes(node)).toContain("underline");
    expect(classes(node)).toContain("bg-transparent");
  });

  it("lets game-quiet drop the game min-width", () => {
    const node = button(h, { variant: "game-quiet" }, ["Yield"]);

    expect(classes(node)).toContain("min-w-0");
    expect(classes(node)).not.toContain("min-w-[132px]");
    expect(classes(node)).toContain("bg-tapped-out");
  });

  it("lets a call-site utility win over the variant", () => {
    const node = button(h, { variant: "ghost", class: "text-burn-red" }, ["Leave"]);

    expect(classes(node)).toContain("text-burn-red");
    expect(classes(node)).not.toContain("text-snow-mint");
  });

  it("exposes the test id and aria label callers depend on", () => {
    const node = button(h, { testId: "result-leave", ariaLabel: "Account" }, ["x"]);

    expect(attrs(node)["data-testid"]).toBe("result-leave");
    expect(attrs(node)["aria-label"]).toBe("Account");
  });

  it("defaults to type=button so it cannot submit a form by accident", () => {
    expect(props(button(h, {}, ["x"])).type).toBe("button");
    expect(props(button(h, { type: "submit" }, ["x"])).type).toBe("submit");
  });

  it("marks a disabled button disabled", () => {
    expect(props(button(h, { disabled: true }, ["x"])).disabled).toBe(true);
  });

  it("renders an anchor with href when asked to look like a button", () => {
    const node = button(h, { as: "a", href: "/decks", variant: "ghost" }, ["Play"]);

    expect((node as { sel?: string }).sel).toBe("a");
    expect(props(node).href).toBe("/decks");
    expect(classes(node)).toContain("border-vine");
    expect(props(node).type).toBeUndefined();
  });

  it("dispatches the given message when a button is clicked", () => {
    const node = button(h, { onClick: clicked }, ["x"]);

    expect(typeof on(node).click).toBe("function");
  });

  it("dispatches the given message when an anchor is clicked", () => {
    const node = button(h, { as: "a", href: "/decks", onClick: clicked }, ["Play"]);

    expect(typeof on(node).click).toBe("function");
  });

  it("passes extra attrs through to a rendered button", () => {
    const node = button(h, { attrs: [h.AriaExpanded(true), h.DataAttribute("ui", "menu-trigger")] }, ["x"]);

    expect(attrs(node)["aria-expanded"]).toBe("true");
    expect(attrs(node)["data-ui"]).toBe("menu-trigger");
  });

  it("passes extra attrs through to a rendered anchor", () => {
    const node = button(
      h,
      { as: "a", href: "/decks", attrs: [h.AriaExpanded(true), h.DataAttribute("ui", "menu-trigger")] },
      ["Play"],
    );

    expect(attrs(node)["aria-expanded"]).toBe("true");
    expect(attrs(node)["data-ui"]).toBe("menu-trigger");
  });

  it("drops a null class instead of rendering it as a literal class name", () => {
    const node = button(h, { variant: "ghost", class: null }, ["Leave"]);

    expect(classes(node)).toContain("border-vine");
  });

  it("merges an array of call-site classes with the variant", () => {
    const node = button(h, { variant: "ghost", class: ["text-burn-red", "uppercase"] }, ["Leave"]);

    expect(classes(node)).toContain("text-burn-red");
    expect(classes(node)).toContain("uppercase");
    expect(classes(node)).not.toContain("text-snow-mint");
  });
});
