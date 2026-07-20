import { afterEach, describe, expect, it, vi } from "vitest";
import { playPrioritySound, resetPrioritySoundForTests } from "~/lib/prioritySound";

describe("playPrioritySound", () => {
  afterEach(() => {
    resetPrioritySoundForTests();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("no-ops when AudioContext is unavailable", () => {
    vi.stubGlobal("AudioContext", undefined);
    expect(() => playPrioritySound()).not.toThrow();
  });

  it("schedules a two-note chime when AudioContext is available", () => {
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

    class FakeAudioContext {
      state = "running" as AudioContextState;
      currentTime = 1;
      destination = {} as AudioDestinationNode;
      resume = vi.fn(async () => undefined);
      createOscillator = createOscillator;
      createGain = createGain;
    }

    vi.stubGlobal("AudioContext", FakeAudioContext);

    playPrioritySound();

    expect(createOscillator).toHaveBeenCalledTimes(2);
    expect(createGain).toHaveBeenCalledTimes(2);
    expect(start).toHaveBeenCalledTimes(2);
    expect(stop).toHaveBeenCalledTimes(2);
  });
});
