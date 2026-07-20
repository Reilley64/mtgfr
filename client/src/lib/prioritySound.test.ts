import { afterEach, describe, expect, it, vi } from "vitest";
import { playPrioritySound, resetPrioritySoundForTests, unlockPriorityAudio } from "~/lib/prioritySound";

describe("prioritySound", () => {
  afterEach(() => {
    resetPrioritySoundForTests();
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

  it("no-ops play when AudioContext is unavailable", () => {
    vi.stubGlobal("AudioContext", undefined);
    expect(() => playPrioritySound()).not.toThrow();
  });

  it("unlocks a suspended context via resume", () => {
    const { resume } = stubAudio("suspended");
    unlockPriorityAudio();
    expect(resume).toHaveBeenCalledTimes(1);
  });

  it("skips resume when already running", () => {
    const { resume } = stubAudio("running");
    unlockPriorityAudio();
    expect(resume).not.toHaveBeenCalled();
  });

  it("plays only when the context is already running", () => {
    const { createOscillator, createGain, start, stop, resume } = stubAudio("running");
    playPrioritySound();
    expect(resume).not.toHaveBeenCalled();
    expect(createOscillator).toHaveBeenCalledTimes(2);
    expect(createGain).toHaveBeenCalledTimes(2);
    expect(start).toHaveBeenCalledTimes(2);
    expect(stop).toHaveBeenCalledTimes(2);
  });

  it("stays silent when still suspended (no gesture unlock yet)", () => {
    const { createOscillator, resume } = stubAudio("suspended");
    playPrioritySound();
    expect(resume).not.toHaveBeenCalled();
    expect(createOscillator).not.toHaveBeenCalled();
  });
});
