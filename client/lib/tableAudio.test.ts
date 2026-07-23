import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  audioContextForTests,
  playUnmuteTick,
  resetTableAudioForTests,
  setSoundEnabledForTests,
  unlockTableAudio,
} from "./tableAudio";

class FakeAudioContext {
  state: AudioContextState = "suspended";
  currentTime = 0;
  resume = vi.fn(async () => {
    this.state = "running";
  });
  createOscillator() {
    return {
      type: "sine",
      frequency: { value: 0 },
      connect: vi.fn(),
      start: vi.fn(),
      stop: vi.fn(),
    };
  }
  createGain() {
    return {
      gain: {
        setValueAtTime: vi.fn(),
        linearRampToValueAtTime: vi.fn(),
        exponentialRampToValueAtTime: vi.fn(),
      },
      connect: vi.fn(),
    };
  }
}

describe("tableAudio unlock", () => {
  beforeEach(() => {
    resetTableAudioForTests();
    setSoundEnabledForTests(true);
    vi.stubGlobal("AudioContext", FakeAudioContext);
  });
  afterEach(() => {
    resetTableAudioForTests();
    setSoundEnabledForTests(null);
    vi.unstubAllGlobals();
  });

  it("resume()s a suspended context on unlock", () => {
    unlockTableAudio();
    const ac = audioContextForTests() as unknown as FakeAudioContext;
    expect(ac).not.toBeNull();
    expect(ac.resume).toHaveBeenCalled();
    expect(ac.state).toBe("running");
  });

  it("playUnmuteTick no-ops when muted", () => {
    setSoundEnabledForTests(false);
    unlockTableAudio();
    const ac = audioContextForTests() as unknown as FakeAudioContext;
    const createOscillator = vi.spyOn(ac, "createOscillator");
    playUnmuteTick();
    expect(createOscillator).not.toHaveBeenCalled();
  });

  it("playUnmuteTick plays when unlocked and enabled", () => {
    unlockTableAudio();
    const ac = audioContextForTests() as unknown as FakeAudioContext;
    // Fake resume flips state synchronously inside the mock; if canPlay still
    // sees "suspended", set `ac.state = "running"` before the tick.
    ac.state = "running";
    const createOscillator = vi.spyOn(ac, "createOscillator");
    playUnmuteTick();
    expect(createOscillator).toHaveBeenCalled();
  });
});
