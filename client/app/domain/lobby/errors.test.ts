import { describe, expect, it } from "vitest";
import { LobbyHttpError, LobbyUnauthorized } from "./errors";

describe("lobby errors", () => {
  it("tags Unauthorized and HttpError", () => {
    expect(new LobbyUnauthorized()._tag).toBe("LobbyUnauthorized");
    expect(new LobbyHttpError({ status: 500, description: "boom" }).status).toBe(500);
  });
});
