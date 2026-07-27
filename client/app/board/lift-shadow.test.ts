import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { DROP_SHADOW_DRAG_CSS, LIFT_SHADOW_BLUR, LIFT_SHADOW_COLOR, LIFT_SHADOW_OFFSET_Y } from "./lift-shadow";

const cssPath = new URL("../../styles/tokens.generated.css", import.meta.url);

describe("lift shadow (drag token)", () => {
  it("matches --drop-shadow-drag in tokens.generated.css", () => {
    const css = readFileSync(cssPath, "utf8");
    expect(css).toContain(`--drop-shadow-drag: ${DROP_SHADOW_DRAG_CSS};`);
  });

  it("maps the drag token to canvas shadow fields", () => {
    expect(DROP_SHADOW_DRAG_CSS).toBe("0 16px 36px rgb(0 0 0 / 0.72)");
    expect(LIFT_SHADOW_OFFSET_Y).toBe(16);
    expect(LIFT_SHADOW_BLUR).toBe(36);
    expect(LIFT_SHADOW_COLOR).toBe("rgba(0,0,0,0.72)");
  });
});
