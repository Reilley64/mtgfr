import { describe, expect, it } from "vitest";
import { formatMessage } from "./message";

describe("formatMessage", () => {
  it("formats effect.draw_cards", () => {
    expect(
      formatMessage({
        key: "effect.draw_cards",
        params: [{ name: "count", int_value: 2 }],
        children: [],
      }),
    ).toBe("Draw 2");
  });

  it("joins sequence children with then", () => {
    expect(
      formatMessage({
        key: "effect.sequence",
        params: [],
        children: [
          {
            key: "effect.draw_cards",
            params: [{ name: "count", int_value: 2 }],
            children: [],
          },
          {
            key: "effect.discard",
            params: [{ name: "count", int_value: 2 }],
            children: [],
          },
        ],
      }),
    ).toBe("Draw 2, then Discard 2");
  });

  it("returns raw key when missing", () => {
    expect(formatMessage({ key: "effect.unknown_zz", params: [], children: [] })).toBe("effect.unknown_zz");
  });

  it("does not pass bare strings through", () => {
    // @ts-expect-error strings are not MessageRef values.
    expect(() => formatMessage("Draw 2")).toThrow(TypeError);
  });

  it("formats effect.control_tap_target from an explicit historical catalog entry", () => {
    expect(formatMessage({ key: "effect.control_tap_target", params: [], children: [] })).toBe("Tap target");
  });

  it("formats reject.illegal_target", () => {
    expect(formatMessage({ key: "reject.illegal_target", params: [], children: [] })).toBe("Pick a legal target.");
  });
});
