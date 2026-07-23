// Priority context bar: Next / Resolve card / Resolve stack / End Turn / turn yield.
//
// Ported silhouette from Solid `priority-context-bar.tsx`: game button variants, primary
// emphasis while this seat must act, Arena turn-yield rocker (amber earth, never priority gold).

import { type Html, html } from "foldkit/html";
import { priorityPrimaryClass } from "~/priorityContextChrome";
import { turnYieldRockerClass, turnYieldThumbClass, turnYieldTrackClass } from "~/turnYieldChrome";
import { gameButtonClass } from "~/ui/buttonClass";
import type { VisibleState } from "~/wire/types";
import { STEP } from "../geometry/layout";
import {
  CancelActionClicked,
  type Message,
  PassClicked,
  PrimaryClicked,
  StackYieldArmed,
  TurnYieldToggled,
} from "../messages";
import type { BoardModel } from "../submodel";
import { HAND_BAR_H } from "./hand";

const h = html<Message>();

/** Same shape as primaryFor() in submodel — repeated here for view-only reasoning. */
type Primary = { kind: "pass" | "confirm-attackers" | "confirm-blockers"; label: string };

function primaryFor(board: BoardModel, state: VisibleState): Primary {
  const me = state.viewer;
  const active = state.active_player;
  const step = state.step;
  const attackers = board.combatAttackers;
  const blocks = board.combatBlocks;
  const declaredAttackers = state.combat.attackers;
  const attackDone = board.attackersConfirmed || state.combat.attackers_declared || declaredAttackers.length > 0;
  const blockDone = board.blockersConfirmed || state.combat.blockers_declared.includes(me);
  const attackingMe = declaredAttackers.some((a) => a.defender === me);
  if (step === STEP.DeclareAttackers && active === me && !attackDone) {
    return attackers.length
      ? { kind: "confirm-attackers", label: `Attack (${attackers.length})` }
      : { kind: "confirm-attackers", label: "No attackers" };
  }
  if (step === STEP.DeclareBlockers && attackingMe && !blockDone) {
    return blocks.length
      ? { kind: "confirm-blockers", label: `Block (${blocks.length})` }
      : { kind: "confirm-blockers", label: "No blockers" };
  }
  if (step === STEP.Draw && active === me) return { kind: "pass", label: "Draw" };
  return { kind: "pass", label: "Next" };
}

function canResolveCard(state: VisibleState): boolean {
  return state.stack.length > 0 && state.can_act && state.priority === state.viewer;
}

function canArmStackYield(state: VisibleState, alreadyYielded: boolean): boolean {
  if (alreadyYielded) return false;
  return canResolveCard(state);
}

function showEndTurn(state: VisibleState, pendingAttackers: boolean): boolean {
  if (state.viewer !== state.active_player) return false;
  if (state.stack.length > 0) return false;
  if (pendingAttackers) return false;
  return true;
}

function showTurnYield(state: VisibleState): boolean {
  return state.viewer !== state.active_player;
}

export function priorityBarView(board: BoardModel, state: VisibleState): Html {
  const primary = primaryFor(board, state);
  const yours = state.can_act && state.priority === state.viewer;
  const stackLen = state.stack.length;
  const yielded = state.yielded ?? false;
  const turnYielded = state.turn_yielded ?? false;
  const pendingAttackers = board.combatAttackers.length > 0 && !board.attackersConfirmed;

  const showPrimary = !(stackLen > 0 && primary.kind === "pass");
  const primaryBtn: Html | null = showPrimary
    ? h.button(
        [
          h.Type("button"),
          h.DataAttribute("testid", "board-primary"),
          h.Disabled(!yours),
          h.OnClick(PrimaryClicked()),
          h.Class(gameButtonClass("game", priorityPrimaryClass(yours))),
        ],
        [primary.label],
      )
    : null;

  const passBtn: Html | null = canResolveCard(state)
    ? h.button(
        [
          h.Type("button"),
          h.DataAttribute("testid", "board-pass"),
          h.OnClick(PassClicked()),
          h.Class(gameButtonClass("game", "shadow-glow")),
        ],
        ["Resolve card"],
      )
    : null;

  const stackYieldBtn: Html | null = canArmStackYield(state, yielded)
    ? h.button(
        [
          h.Type("button"),
          h.DataAttribute("testid", "board-stack-yield"),
          h.OnClick(StackYieldArmed()),
          h.Class(gameButtonClass("game-quiet")),
        ],
        ["Resolve stack"],
      )
    : yielded && stackLen > 0
      ? h.button(
          [
            h.Type("button"),
            h.DataAttribute("testid", "board-stack-yield-armed"),
            h.Disabled(true),
            h.Class(gameButtonClass("game-yielded")),
          ],
          ["Resolve stack"],
        )
      : null;

  const endTurnBtn: Html | null = showEndTurn(state, pendingAttackers)
    ? h.button(
        [
          h.Type("button"),
          h.DataAttribute("testid", "board-end-turn"),
          h.Attribute("aria-pressed", turnYielded ? "true" : "false"),
          h.Attribute("title", turnYielded ? "Cancel end turn" : "End turn (Enter)"),
          h.OnClick(TurnYieldToggled({ enabled: !turnYielded })),
          h.Class(gameButtonClass(turnYielded ? "game-yielded" : "game-quiet")),
        ],
        [turnYielded ? "Ending turn…" : "End Turn"],
      )
    : null;

  const turnYieldBtn: Html | null = showTurnYield(state)
    ? h.button(
        [
          h.Type("button"),
          h.Role("switch"),
          h.DataAttribute("testid", "board-turn-yield"),
          h.Attribute("aria-checked", turnYielded ? "true" : "false"),
          h.Attribute("aria-label", "Auto-pass until my turn"),
          h.Attribute("title", "Auto-pass until my turn"),
          h.OnClick(TurnYieldToggled({ enabled: !turnYielded })),
          h.Class(turnYieldRockerClass(turnYielded)),
        ],
        [
          h.span(
            [h.Class(turnYieldTrackClass(turnYielded))],
            [h.span([h.Class(turnYieldThumbClass(turnYielded)), h.Attribute("aria-hidden", "true")], ["≫"])],
          ),
        ],
      )
    : null;

  const hasStaged =
    board.staged != null ||
    board.xPrompt != null ||
    board.modalCast != null ||
    board.sacrificePick != null ||
    board.discardPick != null ||
    board.gyExilePick != null;
  const cancelBtn: Html | null = hasStaged
    ? h.button(
        [
          h.Type("button"),
          h.DataAttribute("testid", "board-cancel-target"),
          h.OnClick(CancelActionClicked()),
          h.Class(gameButtonClass("game-quiet")),
        ],
        ["Cancel"],
      )
    : null;

  const companions = [endTurnBtn, passBtn, stackYieldBtn, turnYieldBtn, cancelBtn].filter((v): v is Html => v !== null);

  return h.div(
    [
      h.DataAttribute("testid", "priority-context-bar"),
      h.Class("pointer-events-auto fixed right-md z-25 flex flex-col items-end gap-sm"),
      h.Style({ bottom: `${HAND_BAR_H + 10}px` }),
    ],
    [
      h.div(
        [h.Class("flex flex-row-reverse flex-wrap items-center justify-end gap-md")],
        [
          primaryBtn,
          companions.length > 0
            ? h.div([h.Class("flex flex-row-reverse flex-wrap items-center justify-end gap-sm")], companions)
            : null,
        ].filter((v): v is Html => v !== null),
      ),
      board.staged != null
        ? h.div(
            [
              h.DataAttribute("testid", "board-staged-hint"),
              h.Class("max-w-[280px] text-right text-caption text-caution-amber"),
            ],
            [`${board.staged.action.label}: click a highlighted card`],
          )
        : null,
      board.reject != null
        ? h.div([h.DataAttribute("testid", "board-reject"), h.Class("text-caption text-burn-red")], [board.reject])
        : null,
    ].filter((v): v is Html => v !== null),
  );
}
