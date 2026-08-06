// Priority context bar: Next / Resolve card / Resolve stack / End Turn / turn yield.
//
// Ported silhouette from Solid `priority-context-bar.tsx`: game button variants, primary
// emphasis while this seat must act, Arena turn-yield rocker (amber earth, never priority gold, h).

import type { Html, HtmlBuilder } from "foldkit/html";
import { priorityPrimaryClass } from "~/priorityContextChrome";
import {
  turnYieldLabelClass,
  turnYieldRockerClass,
  turnYieldThumbClass,
  turnYieldTrackClass,
  type YieldTone,
} from "~/turnYieldChrome";
import { button } from "~/ui/button";
import type { VisibleState, WireAttack } from "~/wire/types";
import { formatMessage } from "../../domain/i18n/message";
import { bandCandidates, canArmEndTurn, stagedAttackersForDisplay } from "../geometry/combat-staging";
import { type PrimaryAction, primaryActionFor } from "../geometry/interaction";
import {
  CancelActionClicked,
  CombatBandToggled,
  type Message,
  PassClicked,
  PrimaryClicked,
  StackYieldArmed,
  TurnYieldToggled,
} from "../messages";
import { promptPresentation } from "../promptPresentation";
import type { BoardModel } from "../submodel";
import { simplePromptBarActions } from "./prompt-bar-actions";

/** The attacker list the button label, the band panel and the submit path all read (staged ∪ goad). */
function mergedAttackers(board: BoardModel, state: VisibleState): WireAttack[] {
  return stagedAttackersForDisplay(
    board.combatAttackers,
    state.actions?.find((a) => a.kind === "declare_attackers")?.required_attacks ?? [],
    board.attackersConfirmed || state.combat.attackers_declared,
  );
}

/** The same decision the click path makes (`primaryActionFor`) — the button's label and what it
 * submits must never disagree. */
function primaryFor(board: BoardModel, state: VisibleState, attackers: WireAttack[]): PrimaryAction {
  return primaryActionFor({
    step: state.step,
    activePlayer: state.active_player,
    me: state.viewer,
    actions: state.actions,
    attackers,
    blocks: board.combatBlocks,
    attackersConfirmed: board.attackersConfirmed,
    blockersConfirmed: board.blockersConfirmed,
    attackersDeclared: state.combat.attackers_declared,
    blockersDeclared: state.combat.blockers_declared.includes(state.viewer),
  });
}

/** Band grouping affordance (CR 702.22c): one toggle per staged attacker, shown only once some
 * staged attacker can band. Whether the grouping is *legal* is the engine's call — an illegal band
 * comes back as `Reject::IllegalDeclaration` in `board-reject` like any other bad declaration. */
function bandPanelView(
  board: BoardModel,
  state: VisibleState,
  attackers: WireAttack[],
  h: HtmlBuilder<Message>,
): Html | null {
  if (board.attackersConfirmed || state.combat.attackers_declared) return null;
  const candidates = bandCandidates(state.objects, attackers);
  if (candidates.length === 0) return null;

  const chips = candidates.map((id) => {
    const banded = board.combatBand.includes(id);
    return button(
      h,
      {
        testId: `board-band-chip-${id}`,
        onClick: CombatBandToggled({ attackerId: id }),
        variant: "game-quiet",
        // Attribute-driven chrome: JS sets `data-banded`, Tailwind variants own the look.
        class:
          "data-[banded=true]:bg-llanowar data-[banded=true]:text-snow-mint data-[banded=true]:ring-1 data-[banded=true]:ring-llanowar",
        attrs: [
          h.DataAttribute("banded", banded ? "true" : "false"),
          h.Attribute("aria-pressed", banded ? "true" : "false"),
        ],
      },
      [state.objects.find((o) => o.id === id)?.name ?? `#${id}`],
    );
  });

  return h.div(
    [h.DataAttribute("testid", "board-band-panel"), h.Class("flex flex-col items-end gap-xs")],
    [
      h.div([h.Class("text-caption text-mist")], ["Band (attack as one)"]),
      h.div([h.Class("flex flex-row-reverse flex-wrap items-center justify-end gap-xs")], chips),
    ],
  );
}

function canResolveCard(state: VisibleState): boolean {
  return state.stack.length > 0 && state.can_act && state.priority === state.viewer;
}

function canArmStackYield(state: VisibleState, alreadyYielded: boolean): boolean {
  if (alreadyYielded) return false;
  return canResolveCard(state);
}

/** Show End Turn when it can be armed, or while already armed so the seat can cancel. */
function showEndTurn(state: VisibleState, pendingAttackers: boolean): boolean {
  if (state.viewer !== state.active_player) return false;
  if (state.turn_yielded) return true;
  return canArmEndTurn(state, pendingAttackers);
}

function showTurnYield(state: VisibleState): boolean {
  return state.viewer !== state.active_player;
}

/** Arena rocker: a `role="switch"` toggle whose label sits collapsed to its left until hover or
 * keyboard focus opens it. Both turn-yield toggles share this shape — only the armed hue differs. */
function rocker(
  opts: { testId: string; tone: YieldTone; checked: boolean; label: string },
  h: HtmlBuilder<Message>,
): Html {
  return h.button(
    [
      h.Type("button"),
      h.Role("switch"),
      h.DataAttribute("testid", opts.testId),
      h.Attribute("aria-checked", opts.checked ? "true" : "false"),
      h.Attribute("aria-label", opts.label),
      h.Attribute("title", opts.label),
      h.OnClick(TurnYieldToggled({ enabled: !opts.checked })),
      h.Class(turnYieldRockerClass(opts.tone)),
    ],
    [
      h.span([h.Class(turnYieldLabelClass()), h.Attribute("aria-hidden", "true")], [opts.label]),
      h.span(
        [h.Class(turnYieldTrackClass(opts.tone))],
        [h.span([h.Class(turnYieldThumbClass(opts.tone)), h.Attribute("aria-hidden", "true")], ["≫"])],
      ),
    ],
  );
}

export function priorityBarView(
  board: BoardModel,
  state: VisibleState,
  tableId: string | null,
  h: HtmlBuilder<Message>,
): Html | null {
  const presentation = promptPresentation(board, state);
  if (presentation.mode === "modal") {
    // Rich prompts keep answer chrome inside the centered modal; an empty bar above the
    // backdrop would steal pointer events from the modal panel.
    return null;
  }
  if (presentation.mode === "simple") {
    const simpleActions = simplePromptBarActions(board, state, tableId, h);

    return h.div(
      [
        h.DataAttribute("testid", "priority-context-bar"),
        // Above pile (z-29) and prompt-modal (z-40) backdrops so Choose / Confirm stay clickable.
        h.Class("pointer-events-auto fixed bottom-(--b) right-md z-45 flex flex-col items-end gap-sm"),
        h.Style({ "--b": `calc(var(--hand-bar-h) + 10px)` }),
      ],
      [simpleActions, board.reject != null ? rejectView(board.reject, h) : null].filter((v): v is Html => v !== null),
    );
  }

  const attackers = mergedAttackers(board, state);
  const primary = primaryFor(board, state, attackers);
  const yours = state.can_act && state.priority === state.viewer;
  const stackLen = state.stack.length;
  const yielded = state.yielded ?? false;
  const turnYielded = state.turn_yielded ?? false;
  const pendingAttackers = board.combatAttackers.length > 0 && !board.attackersConfirmed;

  const showPrimary = !(stackLen > 0 && primary.kind === "pass");
  const primaryBtn: Html | null = showPrimary
    ? button(
        h,
        {
          testId: "board-primary",
          disabled: !yours,
          onClick: PrimaryClicked(),
          variant: "game",
          class: priorityPrimaryClass(yours),
        },
        [primary.label],
      )
    : null;

  const passBtn: Html | null = canResolveCard(state)
    ? button(h, { testId: "board-pass", onClick: PassClicked(), variant: "game", class: "shadow-glow" }, [
        "Resolve card",
      ])
    : null;

  const stackYieldBtn: Html | null = canArmStackYield(state, yielded)
    ? button(h, { testId: "board-stack-yield", onClick: StackYieldArmed(), variant: "game-quiet" }, ["Resolve stack"])
    : yielded && stackLen > 0
      ? button(h, { testId: "board-stack-yield-armed", disabled: true, variant: "game-yielded" }, ["Resolve stack"])
      : null;

  const endTurnBtn: Html | null = showEndTurn(state, pendingAttackers)
    ? rocker(
        {
          testId: "board-end-turn",
          tone: "end-turn",
          checked: turnYielded,
          // Constant name, like the until-my-turn rocker — armed state is the rocker's job, not the label's.
          label: "End turn",
        },
        h,
      )
    : null;

  const turnYieldBtn: Html | null = showTurnYield(state)
    ? rocker(
        {
          testId: "board-turn-yield",
          tone: "yield",
          checked: turnYielded,
          label: "Auto-pass until my turn",
        },
        h,
      )
    : null;

  const hasStaged =
    board.staged != null ||
    board.playModePick != null ||
    board.xPrompt != null ||
    board.modalCast != null ||
    board.sacrificePick != null ||
    board.discardPick != null ||
    board.gyExilePick != null;
  const cancelBtn: Html | null = hasStaged
    ? button(h, { testId: "board-cancel-target", onClick: CancelActionClicked(), variant: "game-quiet" }, ["Cancel"])
    : null;

  const companions = [passBtn, stackYieldBtn, cancelBtn].filter((v): v is Html => v !== null);
  // End Turn / until-my-turn are standing toggles, not per-window actions — they get their own
  // row under Next and the companions so they never shuffle the action row's silhouette.
  // Exactly one can show: End Turn is the active seat's, the rocker is everyone else's.
  const rockers = [endTurnBtn, turnYieldBtn].filter((v): v is Html => v !== null);

  return h.div(
    [
      h.DataAttribute("testid", "priority-context-bar"),
      h.Class("pointer-events-auto fixed bottom-(--b) right-md z-25 flex flex-col items-end gap-sm"),
      h.Style({ "--b": `calc(var(--hand-bar-h) + 10px)` }),
    ],
    [
      bandPanelView(board, state, attackers, h),
      h.div(
        [h.Class("flex flex-row-reverse flex-wrap items-center justify-end gap-md")],
        [
          primaryBtn,
          companions.length > 0
            ? h.div([h.Class("flex flex-row-reverse flex-wrap items-center justify-end gap-sm")], companions)
            : null,
        ].filter((v): v is Html => v !== null),
      ),
      rockers.length > 0
        ? h.div(
            [
              h.DataAttribute("testid", "priority-bar-rockers"),
              h.Class("flex flex-row-reverse items-center justify-end gap-sm"),
            ],
            rockers,
          )
        : null,
      board.staged != null
        ? h.div(
            [
              h.DataAttribute("testid", "board-staged-hint"),
              h.Class("max-w-[280px] text-right text-caption text-caution-amber"),
            ],
            [`${formatMessage(board.staged.action.label)}: click a highlighted card`],
          )
        : null,
      board.reject != null ? rejectView(board.reject, h) : null,
    ].filter((v): v is Html => v !== null),
  );
}

/** Illegal-action feedback: role="alert" so a failed action announces, not just paints red. */
function rejectView(reject: string, h: HtmlBuilder<Message>): Html {
  return h.div(
    [h.DataAttribute("testid", "board-reject"), h.Role("alert"), h.Class("text-caption text-burn-red")],
    [reject],
  );
}
