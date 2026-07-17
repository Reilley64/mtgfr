// Pre-submit cost / modal / X / staged-pick chrome for ActionSession.
// Engine PendingChoice stays on PromptHost — dual stacks by design (ADR 0006 / 0022).

import { type Accessor, createMemo, type JSX, Show } from "solid-js";
import { CardPickPrompt, ModePickPrompt, TargetPickPrompt } from "~/components/molecules/prompt-forms";
import { type CostPicks, type ModalCast, type StagedAction, settleSacrificePick } from "~/controllers/actionExecution";
import { XPromptModal } from "~/controllers/prompt-host";
import type { TargetMode } from "~/lib/targeting";
import { stagedTargetTitle } from "~/lib/targetPrompt";
import type { ActionView, ModeView, ObjectView, VisibleState, WireTarget } from "~/wire/types";

type Vec = { x: number; y: number };

/** Signals ActionChrome needs — private to the session module, not Board. */
export type ActionChromeModel = {
  staged: Accessor<StagedAction | null>;
  /** Unstage with fly-back — staged-pick cancel must not wipe X / cost / modal state. */
  cancelStaged: () => void;
  setStaged: (v: StagedAction | null) => void;
  stagedMode: Accessor<TargetMode>;
  xPrompt: Accessor<{ name: string; submit: (x: number) => void } | null>;
  setXPrompt: (v: { name: string; submit: (x: number) => void } | null) => void;
  modalCast: Accessor<ModalCast | null>;
  setModalCast: (v: ModalCast | null) => void;
  sacrificePick: Accessor<{
    action: ActionView;
    card: ObjectView | null;
    dropSeed: Vec;
    screenOrigin: Vec;
    picks: CostPicks;
  } | null>;
  setSacrificePick: (
    v: {
      action: ActionView;
      card: ObjectView | null;
      dropSeed: Vec;
      screenOrigin: Vec;
      picks: CostPicks;
    } | null,
  ) => void;
  discardPick: Accessor<{
    action: ActionView;
    card: ObjectView | null;
    dropSeed: Vec;
    screenOrigin: Vec;
    picks: CostPicks;
  } | null>;
  setDiscardPick: (
    v: {
      action: ActionView;
      card: ObjectView | null;
      dropSeed: Vec;
      screenOrigin: Vec;
      picks: CostPicks;
    } | null,
  ) => void;
  gyExilePick: Accessor<{
    action: ActionView;
    card: ObjectView | null;
    dropSeed: Vec;
    screenOrigin: Vec;
    picks: CostPicks;
  } | null>;
  setGyExilePick: (
    v: {
      action: ActionView;
      card: ObjectView | null;
      dropSeed: Vec;
      screenOrigin: Vec;
      picks: CostPicks;
    } | null,
  ) => void;
  pendingMode: () => ModeView | null;
  advanceModal: (mc: ModalCast & { chosen: number[] }) => void;
  answerMode: (target: WireTarget) => void;
  continueAfterCostPick: (
    action: ActionView,
    card: ObjectView | null,
    picks: CostPicks,
    dropSeed: Vec,
    screenOrigin: Vec,
  ) => void;
  objectName: (id: number) => string;
  objectPrint: (id: number) => string;
  /** Live visible state for card-art resolution (ADR 0031). */
  getState: () => VisibleState | null;
  aim: (target: WireTarget) => void;
};

export function ActionChrome(props: { model: ActionChromeModel; playerName: (seat: number) => string }): JSX.Element {
  const stagedPickTargets = createMemo(() => {
    const s = props.model.staged();
    const mode = props.model.stagedMode();
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
  const visible = createMemo(() => props.model.getState());
  const stagedPick = createMemo(() => {
    const targets = stagedPickTargets();
    const state = visible();
    return targets && state ? { targets, state } : null;
  });
  const sacrificeCtx = createMemo(() => {
    const sp = props.model.sacrificePick();
    const state = visible();
    return sp && state ? { sp, state } : null;
  });

  return (
    <>
      <Show when={props.model.xPrompt()}>
        {(p) => <XPromptModal name={p().name} onSubmit={p().submit} onCancel={() => props.model.setXPrompt(null)} />}
      </Show>
      {/* A staged action whose legal targets aren't all on the canvas (a card in a graveyard, a
          spell on the stack) asks with a picker instead of the arrow. Same picker after escape /
          delve / discard — the arrow is easy to miss when a modal just closed. Sacrifice costs do
          not force this: the ability stages onto the stack and you aim a creature like any other
          targeted activate. Cancel un-stages it — nothing has been paid yet. */}
      <Show when={stagedPick()}>
        {(ctx) => (
          <TargetPickPrompt
            title={stagedTargetTitle(props.model.staged())}
            targets={ctx().targets}
            state={ctx().state}
            playerName={props.playerName}
            onPick={props.model.aim}
            onCancel={() => props.model.cancelStaged()}
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
              props.model.setSacrificePick(null);
              props.model.continueAfterCostPick(
                settled.action,
                settled.card,
                settled.picks,
                settled.dropSeed,
                settled.screenOrigin,
              );
            }}
            onCancel={() => props.model.setSacrificePick(null)}
          />
        )}
      </Show>
      {/* Additional discard cost — pick N other cards from hand before the cast proceeds. */}
      <Show when={props.model.discardPick()}>
        {(dp) => (
          <CardPickPrompt
            title={`Discard ${dp().action.discard_count ?? 1}: ${dp().action.label}`}
            submitLabel="Discard"
            state={visible() ?? undefined}
            items={(dp().action.discard_choices ?? []).map((id) => ({
              id,
              label: props.model.objectName(id),
              print: props.model.objectPrint(id),
            }))}
            count={dp().action.discard_count ?? 1}
            declineLabel="Cancel"
            onDecline={() => props.model.setDiscardPick(null)}
            onSubmit={(ids) => {
              const { action, card, picks, dropSeed, screenOrigin } = dp();
              props.model.setDiscardPick(null);
              props.model.continueAfterCostPick(
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
      <Show when={props.model.gyExilePick()}>
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
                label: props.model.objectName(id),
                print: props.model.objectPrint(id),
              }))}
              count={exact() ? min() : null}
              minCount={exact() ? undefined : min()}
              maxCount={exact() ? undefined : max()}
              declineLabel="Cancel"
              onDecline={() => props.model.setGyExilePick(null)}
              onSubmit={(ids) => {
                const { action, card, picks, dropSeed, screenOrigin } = gp();
                props.model.setGyExilePick(null);
                props.model.continueAfterCostPick(
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
      <Show when={props.model.modalCast()}>
        {(mc) => (
          <Show
            when={mc().chosen !== null && props.model.pendingMode()}
            fallback={
              <Show when={mc().action.modal}>
                {(modal) => (
                  <ModePickPrompt
                    name={mc().action.label}
                    choose={modal().choose}
                    chooseMax={modal().choose_max}
                    modes={mc().modes}
                    onSubmit={(chosen) => props.model.advanceModal({ ...mc(), chosen })}
                    onCancel={() => props.model.setModalCast(null)}
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
                    onPick={props.model.answerMode}
                    onCancel={() => props.model.setModalCast(null)}
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
