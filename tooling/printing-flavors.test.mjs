import assert from "node:assert/strict";
import test from "node:test";

import { printingFlavors } from "./printing-flavors.mjs";

test("preserves every named face and distinguishes absent flavor", () => {
  assert.deepEqual(
    printingFlavors({
      card_faces: [
        { name: "Front Face", flavor_text: "front words" },
        { name: "Back Face" },
      ],
    }),
    {
      flavor: "front words",
      faces: [
        { name: "Front Face", flavor: "front words" },
        { name: "Back Face", flavor: null },
      ],
    },
  );
});

test("keeps legacy single-face printing flavor", () => {
  assert.deepEqual(printingFlavors({ flavor_text: "single words" }), {
    flavor: "single words",
    faces: [],
  });
});
