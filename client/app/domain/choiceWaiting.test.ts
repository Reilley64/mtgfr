import { describe, expect, it } from "vitest";
import { pendingChoiceWaitingText } from "./choiceWaiting";

describe("pendingChoiceWaitingText", () => {
  const players = [
    { player: 0, username: "Alice" },
    { player: 1, username: "Bob" },
    { player: 2, username: "" },
  ];

  it("returns null when there is no pending choice", () => {
    expect(
      pendingChoiceWaitingText({
        pendingPlayer: null,
        viewer: 0,
        players,
      }),
    ).toBeNull();
  });

  it("returns null for the awaited seat", () => {
    expect(
      pendingChoiceWaitingText({
        pendingPlayer: 0,
        viewer: 0,
        players,
      }),
    ).toBeNull();
  });

  it("returns null during mulligan", () => {
    expect(
      pendingChoiceWaitingText({
        pendingPlayer: 1,
        viewer: 0,
        mulliganing: true,
        players,
      }),
    ).toBeNull();
  });

  it("names the awaited player for non-deciders", () => {
    expect(
      pendingChoiceWaitingText({
        pendingPlayer: 1,
        viewer: 0,
        players,
      }),
    ).toBe("Waiting for Bob…");
  });

  it("falls back to P{seat} when username is empty", () => {
    expect(
      pendingChoiceWaitingText({
        pendingPlayer: 2,
        viewer: 0,
        players,
      }),
    ).toBe("Waiting for P2…");
  });

  it("shows for spectators waiting on a seated decider", () => {
    expect(
      pendingChoiceWaitingText({
        pendingPlayer: 1,
        viewer: 255,
        players,
      }),
    ).toBe("Waiting for Bob…");
  });
});
