import { describe, expect, it } from "vitest";
import { assertNoForbiddenKeys, EXCEPTION_TYPE } from "../app/domain/otel/semconv";
import { httpFailureAttrs, LobbyDbError } from "./lobby-http";

describe("lobby-http semconv", () => {
  it("records exception.type without message bodies", () => {
    const attrs = httpFailureAttrs(new LobbyDbError(new Error("secret db payload")));

    assertNoForbiddenKeys(attrs);
    expect(attrs).toMatchObject({ [EXCEPTION_TYPE]: "LobbyDbError" });
    expect(attrs).not.toHaveProperty("exception.message");
    expect(Object.values(attrs)).not.toContain("secret db payload");
  });
});
