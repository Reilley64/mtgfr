import { afterEach, describe, expect, it, vi } from "vitest";
import {
  isSoundEnabled,
  playAttentionPriority,
  playAttentionYourTurn,
  playTableFeelDamage,
  playTableFeelLand,
  playTableFeelResolve,
  playTableFeelStack,
  resetTableAudioForTests,
  setSoundEnabledForTests,
  unlockTableAudio,
} from "~/lib/tableAudio";

describe("tableAudio", () => {
  afterEach(() => {
    resetTableAudioForTests();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  function stubAudio(state: AudioContextState = "running") {
    const stop = vi.fn();
    const start = vi.fn();
    const connect = vi.fn();
    const setValueAtTime = vi.fn();
    const linearRampToValueAtTime = vi.fn();
    const exponentialRampToValueAtTime = vi.fn();
    const createOscillator = vi.fn(() => ({
      type: "sine" as OscillatorType,
      frequency: { value: 0 },
      connect,
      start,
      stop,
    }));
    const createGain = vi.fn(() => ({
      gain: { setValueAtTime, linearRampToValueAtTime, exponentialRampToValueAtTime },
      connect,
    }));
    const resume = vi.fn(async () => undefined);

    class FakeAudioContext {
      state = state;
      currentTime = 1;
      destination = {} as AudioDestinationNode;
      resume = resume;
      createOscillator = createOscillator;
      createGain = createGain;
    }

    vi.stubGlobal("AudioContext", FakeAudioContext);
    return { createOscillator, createGain, start, stop, resume };
  }

  it("defaults sound enabled", () => {
    expect(isSoundEnabled()).toBe(true);
  });

  it("no-ops play when AudioContext is unavailable", () => {
    vi.stubGlobal("AudioContext", undefined);
    expect(() => playAttentionPriority()).not.toThrow();
  });

  it("unlocks a suspended context via resume", () => {
    const { resume } = stubAudio("suspended");
    unlockTableAudio();
    expect(resume).toHaveBeenCalledTimes(1);
  });

  it("skips resume when already running", () => {
    const { resume } = stubAudio("running");
    unlockTableAudio();
    expect(resume).not.toHaveBeenCalled();
  });

  it("plays attention priority when running and unmuted", () => {
    const { createOscillator, resume } = stubAudio("running");
    playAttentionPriority();
    expect(resume).not.toHaveBeenCalled();
    expect(createOscillator).toHaveBeenCalledTimes(2);
  });

  it("plays a longer your-turn attention cue", () => {
    const { createOscillator } = stubAudio("running");
    playAttentionYourTurn();
    expect(createOscillator).toHaveBeenCalledTimes(3);
  });

  it("stays silent when still suspended", () => {
    const { createOscillator, resume } = stubAudio("suspended");
    playAttentionPriority();
    expect(resume).not.toHaveBeenCalled();
    expect(createOscillator).not.toHaveBeenCalled();
  });

  it("stays silent when muted", () => {
    const { createOscillator } = stubAudio("running");
    setSoundEnabledForTests(false);
    playAttentionPriority();
    playTableFeelLand();
    expect(createOscillator).not.toHaveBeenCalled();
  });

  it("plays each table-feel cue when unmuted", () => {
    const { createOscillator } = stubAudio("running");
    playTableFeelLand();
    playTableFeelStack();
    playTableFeelResolve();
    playTableFeelDamage();
    expect(createOscillator.mock.calls.length).toBeGreaterThanOrEqual(5);
  });

  it("swallows resume rejection instead of leaving an unhandled promise", async () => {
    const { resume } = stubAudio("suspended");
    resume.mockReturnValueOnce(Promise.reject(new Error("closed")));
    const unhandled: unknown[] = [];
    const onUnhandled = (reason: unknown) => {
      unhandled.push(reason);
    };
    process.on("unhandledRejection", onUnhandled);
    try {
      unlockTableAudio();
      await vi.waitFor(() => expect(resume).toHaveBeenCalledTimes(1));
      await new Promise((r) => setTimeout(r, 0));
      expect(unhandled).toEqual([]);
    } finally {
      process.off("unhandledRejection", onUnhandled);
    }
  });
});
