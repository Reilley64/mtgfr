// Pre-submit cost / modal / X / staged-pick chrome for ActionSession.
// Binds directly to ActionExecution — no wide ActionChromeModel bag at the seam.
// Engine PendingChoice stays on PromptHost — dual stacks by design (wire-protocol-and-visibility spec / choices-actions-and-resolution spec).

import { createMemo, type JSX, Show } from "solid-js";
import { CardPickPrompt, ModePickPrompt, TargetPickPrompt } from "~/components/molecules/prompt-forms";
import { type ActionExecution, settleSacrificePick } from "~/controllers/actionExecution";
import { XPromptModal } from "~/controllers/prompt-host";
import { stagedTargetTitle } from "~/lib/targetPrompt";

/** Pre-submit chrome bound to the session's ActionExecution (opaque to Board). */
export function ActionChrome(props: { execution: ActionExecution; playerName: (seat: number) => string }): JSX.Element {
  const ex = () => props.execution;
  const stagedPickTargets = createMemo(() => {
    const s = ex().staged();
    const mode = ex().stagedMode();
    if (!s || mode.kind === "none" || mode.kind === "impossible") return null;
    if (mode.kind === "pick") return mode.targets;
    if (s.preferPick && mode.kind === "arrow") {
      return [
        ...[...mode.objects].map((id) => ({ kind: "object" as const, id })),
        ...[...mode.players].map((player) => ({ kind: "player" as const, player })),
      ];
    }
    return null;
  });
  const visible = createMemo(() => ex().getState());
  const stagedPick = createMemo(() => {
    const targets = stagedPickTargets();
    const state = visible();
    return targets && state ? { targets, state } : null;
  });
  const sacrificeCtx = createMemo(() => {
    const sp = ex().sacrificePick();
    const state = visible();
    return sp && state ? { sp, state } : null;
  });

  return (
    <>
      <Show when={ex().xPrompt()}>
        {(p) => <XPromptModal name={p().name} onSubmit={p().submit} onCancel={() => ex().setXPrompt(null)} />}
      </Show>
      {/* A staged action whose legal targets aren't all on the canvas (a card in a graveyard, a
          spell on the stack) asks with a picker instead of the arrow. Same picker after escape /
          delve / discard — the arrow is easy to miss when a modal just closed. Sacrifice costs do
          not force this: the ability stages onto the stack and you aim a creature like any other
          targeted activate. Cancel un-stages it — nothing has been paid yet. */}
      <Show when={stagedPick()}>
        {(ctx) => (
          <TargetPickPrompt
            title={stagedTargetTitle(ex().staged())}
            targets={ctx().targets}
            state={ctx().state}
            playerName={props.playerName}
            onPick={(t) => ex().completeTarget(t)}
            onCancel={() => ex().cancelStagedOnly()}
          />
        )}
      </Show>
      {/* "Sacrifice a creature: …" — name the creature that pays before the ability is taken. */}
      <Show when={sacrificeCtx()}>
        {(ctx) => (
          <TargetPickPrompt
            title={`Sacrifice a creature: ${ctx().sp.action.label}`}
            targets={(ctx().sp.action.sacrifice_choices ?? []).map((id) => ({ kind: "object" as const, id }))}
            state={ctx().state}
            playerName={props.playerName}
            onPick={(t) => {
              if (t.kind !== "object") return;
              // Capture before clear — Solid's ctx() tracks sacrificePick() and goes falsy after null.
              const settled = settleSacrificePick(ctx().sp, t.id);
              ex().setSacrificePick(null);
              ex().continueAfterCostPick(
                settled.action,
                settled.card,
                settled.picks,
                settled.dropSeed,
                settled.screenOrigin,
              );
            }}
            onCancel={() => ex().setSacrificePick(null)}
          />
        )}
      </Show>
      {/* Additional discard cost — pick N other cards from hand before the cast proceeds. */}
      <Show when={ex().discardPick()}>
        {(dp) => (
          <CardPickPrompt
            title={`Discard ${dp().action.discard_count ?? 1}: ${dp().action.label}`}
            submitLabel="Discard"
            state={visible() ?? undefined}
            items={(dp().action.discard_choices ?? []).map((id) => ({
              id,
              label: ex().objectName(id),
              print: ex().objectPrint(id),
            }))}
            count={dp().action.discard_count ?? 1}
            declineLabel="Cancel"
            onDecline={() => ex().setDiscardPick(null)}
            onSubmit={(ids) => {
              const { action, card, picks, dropSeed, screenOrigin } = dp();
              ex().setDiscardPick(null);
              ex().continueAfterCostPick(
                action,
                card,
                { ...picks, discard_cost: ids, discard_settled: true },
                dropSeed,
                screenOrigin,
              );
            }}
          />
        )}
      </Show>
      {/* Delve / escape — exile cards from the graveyard as part of casting. */}
      <Show when={ex().gyExilePick()}>
        {(gp) => {
          const min = () => gp().action.graveyard_exile_min ?? 0;
          const max = () => gp().action.graveyard_exile_max ?? 0;
          const exact = () => min() === max() && min() > 0;
          return (
            <CardPickPrompt
              title={
                exact()
                  ? `Exile ${min()} from your graveyard: ${gp().action.label}`
                  : `Exile any number for delve: ${gp().action.label}`
              }
              submitLabel="Exile"
              state={visible() ?? undefined}
              items={(gp().action.graveyard_exile_choices ?? []).map((id) => ({
                id,
                label: ex().objectName(id),
                print: ex().objectPrint(id),
              }))}
              count={exact() ? min() : null}
              minCount={exact() ? undefined : min()}
              maxCount={exact() ? undefined : max()}
              declineLabel="Cancel"
              onDecline={() => ex().setGyExilePick(null)}
              onSubmit={(ids) => {
                const { action, card, picks, dropSeed, screenOrigin } = gp();
                ex().setGyExilePick(null);
                ex().continueAfterCostPick(
                  action,
                  card,
                  { ...picks, graveyard_exile: ids, gy_exile_settled: true },
                  dropSeed,
                  screenOrigin,
                );
              }}
            />
          );
        }}
      </Show>
      {/* A modal spell (CR 700.2): choose the modes, then a target for each mode that wants one. */}
      <Show when={ex().modalCast()}>
        {(mc) => (
          <Show
            when={mc().chosen !== null && ex().pendingMode()}
            fallback={
              <Show when={mc().action.modal}>
                {(modal) => (
                  <ModePickPrompt
                    name={mc().action.label}
                    choose={modal().choose}
                    chooseMax={modal().choose_max}
                    modes={mc().modes}
                    onSubmit={(chosen) => ex().advanceModal({ ...mc(), chosen })}
                    onCancel={() => ex().setModalCast(null)}
                  />
                )}
              </Show>
            }
          >
            {(mode) => (
              <Show when={visible()}>
                {(state) => (
                  <TargetPickPrompt
                    title={`${mc().action.label} — ${mode().label}`}
                    targets={mode().targets}
                    state={state()}
                    playerName={props.playerName}
                    onPick={(t) => ex().answerMode(t)}
                    onCancel={() => ex().setModalCast(null)}
                  />
                )}
              </Show>
            )}
          </Show>
        )}
      </Show>
    </>
  );
}
