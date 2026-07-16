import { describe, expect, it } from "vitest";
import { lobbyIsHost } from "~/lib/lobby";

describe("lobbyIsHost", () => {
  const seats = [{ is_host: true }, { is_host: false }, { is_host: false }];

  it("treats seat 0 as a real host (not a missing seat)", () => {
    expect(lobbyIsHost(0, seats)).toBe(true);
  });

  it("is false for a non-host seat and when you are unset", () => {
    expect(lobbyIsHost(1, seats)).toBe(false);
    expect(lobbyIsHost(null, seats)).toBe(false);
    expect(lobbyIsHost(undefined, seats)).toBe(false);
  });
});
