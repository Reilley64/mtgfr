import { Submodel } from "foldkit";
import { html } from "foldkit/html";
import { Scene } from "foldkit/test";
import { expect, test } from "vitest";
import type { CatalogCard } from "~/wire/types";
import { BindCardArt, CardArtTick } from "../ui/card-art";
import { cardHoverPreviewView } from "./card-hover-preview";

type PreviewMessage = "dismiss" | "flip";

const h = html<PreviewMessage>();

const solRing: CatalogCard = {
  color_identity: [],
  cost: { colored: [0, 0, 0, 0, 0], generic: 1 },
  default_print: "sol-ring-print",
  id: "sol-ring",
  keywords: [],
  kind: { kind: "artifact" },
  legendary: false,
  name: "Sol Ring",
  oracle: "{T}: Add {C}{C}.",
  otags: [],
  set: "c14",
  subtypes: [],
  summary: "",
};

const followView = Submodel.defineView<
  { hover: { id: string; print: string; x: number; y: number }; card?: CatalogCard },
  PreviewMessage
>((model) =>
  cardHoverPreviewView(h, {
    hover: model.hover,
    card: model.card,
    testId: "deck-list-hover-preview",
  }),
);

const dockView = Submodel.defineView<
  {
    print: string;
    name: string;
    oracle?: string | null;
    approximates?: string | null;
    withExtras?: boolean;
  },
  PreviewMessage
>((model) =>
  cardHoverPreviewView(h, {
    mode: "dock",
    print: model.print,
    name: model.name,
    oracle: model.oracle,
    approximates: model.approximates,
    extras: model.withExtras
      ? [
          h.button([h.Type("button"), h.OnClick("flip"), h.DataAttribute("testid", "dock-flip")], ["Flip"]),
          h.div([h.DataAttribute("testid", "dock-extra")], ["Extra ledger"]),
        ]
      : undefined,
    onDismiss: "dismiss",
    testId: "inspect-overlay",
  }),
);

test("card hover preview renders art and oracle text", () => {
  Scene.scene(
    { update: (m) => [m, []], view: followView },
    Scene.with({ hover: { id: "sol-ring", print: "sol-ring-print", x: 120, y: 80 }, card: solRing }),
    Scene.expect(Scene.selector('[data-testid="deck-list-hover-preview"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-list-hover-preview"]')).toHaveClass("top-(--y)"),
    Scene.expect(Scene.selector('[data-testid="deck-list-hover-preview"]')).toHaveClass("left-(--x)"),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
  );
});

test("dock mode renders backdrop and left-docked preview", () => {
  Scene.scene(
    { update: (m) => [m, []], view: dockView },
    Scene.with({
      print: "sol-ring-print",
      name: "Sol Ring",
      oracle: "{T}: Add {C}.",
    }),
    Scene.expect(Scene.testId("inspect-overlay")).toExist(),
    Scene.expect(Scene.testId("inspect-overlay")).toHaveClass("bg-black/55"),
    Scene.expect(Scene.testId("inspect-overlay")).toHaveClass("fixed"),
    Scene.expect(Scene.testId("inspect-overlay")).toHaveClass("inset-0"),
    Scene.expect(Scene.testId("inspect-overlay")).not.toHaveClass("top-(--y)"),
    Scene.expect(Scene.testId("inspect-overlay")).not.toHaveClass("left-(--x)"),
    Scene.expect(Scene.testId("inspect-overlay")).toContainText(": Add ."),
    Scene.expect(Scene.selector('[aria-label="{C}"]')).toExist(),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
  );
});

test("dock mode includes extras after the oracle panel", () => {
  Scene.scene(
    { update: (m) => [m, []], view: dockView },
    Scene.with({
      print: "sol-ring-print",
      name: "Sol Ring",
      oracle: "{T}: Add {C}.",
      withExtras: true,
    }),
    Scene.expect(Scene.testId("inspect-overlay")).toExist(),
    Scene.expect(Scene.testId("dock-extra")).toExist(),
    Scene.expect(Scene.testId("dock-extra")).toHaveText("Extra ledger"),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
  );
});

test("dock mode does not dismiss when a content control is clicked", () => {
  const messages: PreviewMessage[] = [];
  Scene.scene(
    {
      update: (model, message) => {
        if (message === "dismiss" || message === "flip") messages.push(message);
        return [model, []];
      },
      view: dockView,
    },
    Scene.with({
      print: "sol-ring-print",
      name: "Sol Ring",
      oracle: "{T}: Add {C}.",
      withExtras: true,
    }),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
    Scene.click(Scene.testId("dock-flip")),
  );

  expect(messages).toEqual(["flip"]);
});

test("dock mode dismisses from the backdrop", () => {
  const messages: PreviewMessage[] = [];
  Scene.scene(
    {
      update: (model, message) => {
        if (message === "dismiss" || message === "flip") messages.push(message);
        return [model, []];
      },
      view: dockView,
    },
    Scene.with({
      print: "sol-ring-print",
      name: "Sol Ring",
      oracle: "{T}: Add {C}.",
    }),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
    Scene.click(Scene.testId("inspect-overlay-backdrop")),
  );

  expect(messages).toEqual(["dismiss"]);
});
