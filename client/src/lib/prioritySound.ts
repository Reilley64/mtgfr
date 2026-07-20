// Soft chime when priority arrives. Synthesized via Web Audio so we don't ship a binary asset.
// Browsers gate AudioContext behind a user gesture — unlock at lobby Ready up, then play later
// from the priority transition effect without needing a fresh activation.

let sharedCtx: AudioContext | null = null;

/** Clears the cached context between tests. */
export function resetPrioritySoundForTests(): void {
  sharedCtx = null;
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

function tone(ac: AudioContext, freq: number, start: number, dur: number, peak: number): void {
  const osc = ac.createOscillator();
  const gain = ac.createGain();
  osc.type = "sine";
  osc.frequency.value = freq;
  gain.gain.setValueAtTime(0, start);
  gain.gain.linearRampToValueAtTime(peak, start + 0.02);
  gain.gain.exponentialRampToValueAtTime(0.001, start + dur);
  osc.connect(gain);
  gain.connect(ac.destination);
  osc.start(start);
  osc.stop(start + dur);
}

function chime(ac: AudioContext): void {
  if (ac.state !== "running") return;
  const t = ac.currentTime;
  tone(ac, 523.25, t, 0.12, 0.08);
  tone(ac, 659.25, t + 0.1, 0.18, 0.07);
}

/** Call from a user gesture (lobby Ready up). Creates + resumes the shared context. */
export function unlockPriorityAudio(): void {
  const ac = audioContext();
  if (!ac) return;
  if (ac.state === "running") return;
  void ac.resume().catch(() => {
    // Closed / autoplay-blocked — play will no-op until another gesture unlocks.
  });
}

/** Two soft notes (C5 → E5). Silent when AudioContext is missing or still locked. */
export function playPrioritySound(): void {
  const ac = audioContext();
  if (!ac || ac.state !== "running") return;
  chime(ac);
}
