// Sparse table audio (client-game-board-and-interaction spec): synthesized attention + table-feel cues.
// Unlock at lobby Ready up (user gesture); play later without a fresh activation.

export const SOUND_PREF_KEY = "mtgfr.sound";

let sharedCtx: AudioContext | null = null;
/** Test override; `null` means read localStorage / default on. */
let soundEnabledOverride: boolean | null = null;

/** Clears cached context + mute override between tests. */
export function resetTableAudioForTests(): void {
  sharedCtx = null;
  soundEnabledOverride = null;
}

export function isSoundEnabled(): boolean {
  if (soundEnabledOverride !== null) return soundEnabledOverride;
  if (typeof localStorage === "undefined") return true;
  return localStorage.getItem(SOUND_PREF_KEY) !== "0";
}

export function setSoundEnabled(on: boolean): void {
  soundEnabledOverride = null;
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(SOUND_PREF_KEY, on ? "1" : "0");
  }
}

/** Test-only mute without touching localStorage. */
export function setSoundEnabledForTests(on: boolean | null): void {
  soundEnabledOverride = on;
}

function audioContext(): AudioContext | null {
  if (typeof globalThis === "undefined") return null;
  const AC =
    globalThis.AudioContext ??
    (globalThis as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  if (!AC) return null;
  sharedCtx ??= new AC();
  return sharedCtx;
}

function canPlay(): AudioContext | null {
  if (!isSoundEnabled()) return null;
  const ac = audioContext();
  if (ac?.state !== "running") return null;
  return ac;
}

function tone(
  ac: AudioContext,
  freq: number,
  start: number,
  dur: number,
  peak: number,
  type: OscillatorType = "sine",
): void {
  const osc = ac.createOscillator();
  const gain = ac.createGain();
  osc.type = type;
  osc.frequency.value = freq;
  gain.gain.setValueAtTime(0, start);
  gain.gain.linearRampToValueAtTime(peak, start + 0.015);
  gain.gain.exponentialRampToValueAtTime(0.001, start + dur);
  osc.connect(gain);
  gain.connect(ac.destination);
  osc.start(start);
  osc.stop(start + dur);
}

/** Test-only: current shared context, if any. */
export function audioContextForTests(): AudioContext | null {
  return sharedCtx;
}

/** Call from a user gesture (lobby Ready up). Creates + resumes the shared context. */
export function unlockTableAudio(): void {
  const ac = audioContext();
  if (!ac) return;
  if (ac.state === "running") return;
  void ac.resume().catch(() => {
    // Closed / autoplay-blocked — play will no-op until another gesture unlocks.
  });
}

/** Short confirmation when the player unmutes (Sound toggle on). */
export function playUnmuteTick(): void {
  const ac = canPlay();
  if (!ac) return;
  const t = ac.currentTime;
  tone(ac, 660, t, 0.05, 0.04, "sine");
}

/** Short attention ping when you gain priority. */
export function playAttentionPriority(): void {
  const ac = canPlay();
  if (!ac) return;
  const t = ac.currentTime;
  tone(ac, 523.25, t, 0.1, 0.07);
  tone(ac, 659.25, t + 0.08, 0.14, 0.06);
}

/** Warmer / longer attention cue when you become the active player. */
export function playAttentionYourTurn(): void {
  const ac = canPlay();
  if (!ac) return;
  const t = ac.currentTime;
  tone(ac, 392.0, t, 0.14, 0.08);
  tone(ac, 523.25, t + 0.1, 0.16, 0.07);
  tone(ac, 659.25, t + 0.22, 0.2, 0.06);
}

/** Soft land-on-felt tick. */
export function playTableFeelLand(): void {
  const ac = canPlay();
  if (!ac) return;
  const t = ac.currentTime;
  tone(ac, 180, t, 0.06, 0.035, "triangle");
  tone(ac, 240, t + 0.02, 0.05, 0.025, "triangle");
}

/** Soft whoosh when a spell/ability hits the stack. */
export function playTableFeelStack(): void {
  const ac = canPlay();
  if (!ac) return;
  const t = ac.currentTime;
  tone(ac, 440, t, 0.05, 0.03, "sine");
  tone(ac, 660, t + 0.04, 0.07, 0.025, "sine");
}

/** Soft settle when something leaves the stack. */
export function playTableFeelResolve(): void {
  const ac = canPlay();
  if (!ac) return;
  const t = ac.currentTime;
  tone(ac, 330, t, 0.08, 0.03, "triangle");
}

/** Soft thud for combat damage (one cue per damage batch). */
export function playTableFeelDamage(): void {
  const ac = canPlay();
  if (!ac) return;
  const t = ac.currentTime;
  tone(ac, 110, t, 0.07, 0.04, "triangle");
  tone(ac, 90, t + 0.03, 0.08, 0.03, "sine");
}
