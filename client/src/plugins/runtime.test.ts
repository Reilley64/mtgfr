import { describe, expect, it, vi } from "vitest";
import { defineClientPlugin } from "./runtime";

describe("defineClientPlugin", () => {
  it("returns a branded client plugin with callable setup", async () => {
    const setup = vi.fn();
    const plugin = defineClientPlugin(setup);

    expect(plugin.__kind).toBe("client");
    await plugin.setup({});
    expect(setup).toHaveBeenCalledOnce();
  });
});
