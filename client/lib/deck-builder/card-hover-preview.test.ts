import { Submodel } from "foldkit";
import { html } from "foldkit/html";
import { Scene } from "foldkit/test";
import { test } from "vitest";
import type { CatalogCard } from "~/wire/types";
import { cardHoverPreviewView } from "./card-hover-preview";

const h = html<never>();

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

const view = Submodel.defineView<
  { hover: { id: string; print: string; x: number; y: number }; card?: CatalogCard },
  never
>((model) =>
  cardHoverPreviewView(h, {
    hover: model.hover,
    card: model.card,
    testId: "deck-list-hover-preview",
  }),
);

test("card hover preview renders art and oracle text", () => {
  Scene.scene(
    { update: (m) => [m, []], view },
    Scene.with({ hover: { id: "sol-ring", print: "sol-ring-print", x: 120, y: 80 }, card: solRing }),
    Scene.expect(Scene.selector('[data-testid="deck-list-hover-preview"]')).toExist(),
  );
});
