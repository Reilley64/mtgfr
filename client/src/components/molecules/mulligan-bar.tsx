import { Show } from "solid-js";
import { Button } from "~/components/atoms";
import { HAND_BAR_H } from "~/components/molecules/hand";
import type { MulliganChrome } from "~/lib/mulligan";

export function MulliganBar(props: { chrome: MulliganChrome; onKeep: () => void; onMulligan: () => void }) {
  return (
    <div
      style={{ "--b": `${HAND_BAR_H + 10}px` }}
      class="fixed bottom-(--b) left-1/2 z-25 flex w-[min(520px,calc(100vw-32px))] -translate-x-1/2 flex-col items-center gap-xs rounded-game bg-forest-floor/95 px-lg py-md text-center shadow-press"
      data-testid="mulligan-bar"
    >
      <div class="text-label text-mist uppercase tracking-[0.08em]">{props.chrome.title}</div>
      <div class="text-caption text-snow-mint">{props.chrome.status}</div>
      <Show when={props.chrome.showControls}>
        <div class="mt-xs flex flex-wrap justify-center gap-sm">
          <Button type="button" data-testid="mulligan-keep" onClick={props.onKeep} variant="game">
            {props.chrome.keepLabel}
          </Button>
          <Button
            type="button"
            data-testid="mulligan-take"
            disabled={!props.chrome.canMulligan}
            onClick={props.onMulligan}
            variant="game-quiet"
          >
            {props.chrome.mulliganLabel}
          </Button>
        </div>
      </Show>
    </div>
  );
}
