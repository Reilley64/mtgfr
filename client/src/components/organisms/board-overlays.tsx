// Connecting banner, result / pile modals, and game log.

import * as Match from "effect/Match";
import { createEffect, For, Show } from "solid-js";
import { Button, CardArt, Hud, Modal } from "~/components/atoms";
import { HAND_BAR_H } from "~/components/molecules/hand";
import { PROMPT_ROW, PROMPT_TITLE } from "~/components/molecules/prompt-forms";
import { cn } from "~/lib/cn";
import type { Outcome } from "~/lib/outcome";
import { playerLabel } from "~/lib/players";
import { game } from "~/store";
import type { ObjectView } from "~/wire/types";

export function Connecting() {
  return (
    <div class="fixed inset-0 flex items-center justify-center">
      <Hud class="animate-breathe text-center">Connecting to the table…</Hud>
    </div>
  );
}

// The end of the game, said out loud. Before this, a winner simply kept priority over a board of
// faded avatars and nobody was told anything. "Keep watching" dismisses it — an eliminated player
// stays at the table to see how it finishes — and "Back to your decks" is the board's only exit.
export function ResultOverlay(props: { outcome: Outcome; onWatch: () => void; onLeave: () => void }) {
  const headline = () =>
    Match.value(props.outcome).pipe(
      Match.withReturnType<string>(),
      Match.discriminatorsExhaustive("kind")({
        won: () => "You win",
        lost: (o) => (o.winner === null ? "You're eliminated" : `${seatName(o.winner)} wins`),
        over: (o) => (o.winner === null ? "Nobody wins" : `${seatName(o.winner)} wins`),
        playing: () => "", // never rendered — the overlay is gated on kind !== "playing"
      }),
    );
  // A player who lost while the game continues is out, but the game isn't over; say which it is.
  const detail = () =>
    Match.value(props.outcome).pipe(
      Match.withReturnType<string>(),
      Match.discriminatorsExhaustive("kind")({
        won: () => "Last player standing.",
        lost: (o) => (o.winner === null ? "The game continues without you." : "You were eliminated."),
        over: () => "The game is over.",
        playing: () => "",
      }),
    );
  const stillRunning = () => props.outcome.kind === "lost" && props.outcome.winner === null;
  return (
    // z 55: above a pending decision's prompt (50) — once the game is over, nothing outranks it.
    <div class="fixed inset-0 z-55 flex items-center justify-center bg-black/70">
      <Modal class="flex max-w-[420px] flex-col items-center gap-lg text-center">
        <div class="font-bold text-title">{headline()}</div>
        <div class="text-label text-lichen">{detail()}</div>
        <div class="flex gap-md">
          <Button type="button" onClick={props.onWatch} variant="ghost">
            {stillRunning() ? "Keep watching" : "Stay on the board"}
          </Button>
          <Button type="button" onClick={props.onLeave}>
            Back to your decks
          </Button>
        </div>
      </Modal>
    </div>
  );
}

/** How a seat is named everywhere else on this board (the turn banner, the priority watch). */
const seatName = (seat: number) => playerLabel(game.state?.players ?? [], seat);

export function LogPanel() {
  let panel: HTMLDivElement | undefined;
  const lines = () => game.log.slice(-30);
  // New lines land at the bottom, but a scroll container stays pinned at the top as it grows — so
  // once the log outgrows its 150px, the line you actually want to read is the one below the fold.
  // Follow it. (Tracking `lines()`, not `game.log.length`, which stops changing once the log caps.)
  createEffect(() => {
    lines();
    if (panel) panel.scrollTop = panel.scrollHeight;
  });
  return (
    <Show when={game.log.length > 0}>
      <Hud
        ref={panel}
        role="log"
        aria-live="polite"
        style={{ "--b": `${HAND_BAR_H + 10}px` }}
        class="fixed bottom-(--b) left-[72px] z-10 max-h-[150px] w-[300px] overflow-y-auto"
      >
        <For each={lines()}>
          {(l) => (
            <div class={cn("text-caption", l.auto ? "flex items-start gap-xs text-snow-mint" : "text-mist")}>
              <Show when={l.auto}>
                <span class="mt-px shrink-0 rounded-full bg-auto-moss px-[5px] py-px font-bold text-micro text-snow-mint tracking-[0.06em]">
                  AUTO
                </span>
              </Show>
              <span>{l.text}</span>
            </div>
          )}
        </For>
      </Hud>
    </Show>
  );
}

export function PileOverlay(props: { cards: ObjectView[]; onClose: () => void }) {
  return (
    // Click-outside-to-dismiss is a redundant shortcut; the Close button below is the keyboard path.
    // biome-ignore lint/a11y/noStaticElementInteractions: backdrop, not a control
    // biome-ignore lint/a11y/useKeyWithClickEvents: same
    <div
      onClick={(e) => {
        if (e.target === e.currentTarget) props.onClose(); // a click on the modal itself isn't "outside"
      }}
      class="fixed inset-0 z-29 bg-black/50"
    >
      <Modal class="fixed top-[45%] left-1/2 z-30 max-w-[520px] -translate-x-1/2 -translate-y-1/2">
        <div class={PROMPT_TITLE}>Pile ({props.cards.length})</div>
        <div class="flex flex-wrap gap-xs">
          <For each={props.cards}>
            {(c) => <CardArt print={c.print ?? ""} alt={c.name} width={90} class="rounded-md" />}
          </For>
        </div>
        <div class={cn(PROMPT_ROW, "mt-sm")}>
          <Button type="button" onClick={props.onClose} variant="ghost">
            Close
          </Button>
        </div>
      </Modal>
    </div>
  );
}
