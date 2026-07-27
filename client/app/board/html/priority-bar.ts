// Priority context bar: Next / Resolve card / Resolve stack / End Turn / turn yield.
//
// Ported silhouette from Solid `priority-context-bar.tsx`: game button variants, primary
// emphasis while this seat must act, Arena turn-yield rocker (amber earth, never priority gold).

import { type Html, html } from "foldkit/html";
import { priorityPrimaryClass } from "~/priorityContextChrome";
import { turnYieldRockerClass, turnYieldThumbClass, turnYieldTrackClass } from "~/turnYieldChrome";
import { gameButtonClass } from "~/ui/buttonClass";
import type { VisibleState } from "~/wire/types";
import { formatMessage } from "../../domain/i18n/message";
import { canArmEndTurn, stagedAttackersForDisplay } from "../geometry/combat-staging";
import { type PrimaryAction, primaryActionFor } from "../geometry/interaction";
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

/** The same decision the click path makes (`primaryActionFor`) — the button's label and what it
 * submits must never disagree. */
function primaryFor(board: BoardModel, state: VisibleState): PrimaryAction {
  const attackers = stagedAttackersForDisplay(
    board.combatAttackers,
    state.actions?.find((a) => a.kind === "declare_attackers")?.required_attacks ?? [],
    board.attackersConfirmed || state.combat.attackers_declared,
  );
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
    board.playModePick != null ||
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
            [`${formatMessage(board.staged.action.label)}: click a highlighted card`],
          )
        : null,
      board.reject != null
        ? h.div([h.DataAttribute("testid", "board-reject"), h.Class("text-caption text-burn-red")], [board.reject])
        : null,
    ].filter((v): v is Html => v !== null),
  );
}
