// Engine `pending_choice` prompts, plus pre-submit cost/modal/X pickers owned by the board.
//
// Pending-choice formulators collect answers and route every submission through `choiceIntent`.

import { Match, Option } from "effect";
import { childAttributes, type Html, type HtmlBuilder } from "foldkit/html";
import {
  cardPickIsSearchable,
  filterChoiceItems,
  PICK_CARD_SCROLL_MIN_CLASS,
  searchableChoiceItems,
} from "~/cardPickSearch";
import {
  type AnswerInput,
  buildAnswerFromDraft,
  cardPickReady,
  cardPickRequiredCount,
  choiceIntent,
  chooseTargetIsCardPick,
  type DistributeBucket,
  damageAssignReady,
  declineAnswer,
  FORMULATOR_FOR_KIND,
  initPromptDraft,
  nextDistributeBucket,
} from "~/choice";
import { filterOptionLabels } from "~/optionFilter";
import { manaFontClass } from "~/oracleText";
import { isActivePlayer } from "~/spectator";
import { input, inputClass } from "~/ui/input";
import { menuItemClass, menuPanelClass } from "~/ui/menu";
import type { ChoiceItem, MessageRef, PendingChoiceView, VisibleState, WireModeChoice, WireTarget } from "~/wire/types";
import { clampX, costText, costWithChosenX } from "~/xCost";
import { formatMessage } from "../../domain/i18n/message";
import {
  gyExileCostObjectIds,
  objectName,
  pendingBoardTargetMode,
  pendingDamageAssignBlockers,
  pendingDigCastHostMode,
  pendingDivideSpellObjectIndexes,
  pendingExilePickIds,
  pendingExilePickOneClick,
  pendingGraveyardPickIds,
  pendingGraveyardPickOneClick,
  pendingHandPickIds,
  pendingHandPickOneClick,
  pendingPlayerAimOneClick,
  pendingPlayerAimSeats,
  pendingTargetOneClick,
  playerSeatLabel,
  sacrificeCostObjectIds,
  stagedPickTargets,
  stagedTargetTitle,
} from "../action/targeting";
import { CARD_NAME_COMBOBOX_ID, CardNameCombobox } from "../card-name-combobox";
import { seatColor, ZONE } from "../geometry/layout";
import {
  CancelActionClicked,
  DiscardChosen,
  GotCardNameComboboxMessage,
  GyExileChosen,
  type Message,
  PendingChoiceAnswered,
  PromptCardFilterSet,
  PromptCardToggled,
  PromptDamageSet,
  PromptDeclined,
  PromptModeChoiceToggled,
  PromptNumberSet,
  PromptOptionFilterSet,
  PromptOrderDragEnded,
  PromptOrderMoved,
  PromptOrderRowClicked,
  PromptPartitionSet,
  PromptSubmitted,
  SacrificeChosen,
  TargetChosen,
  XDraftSet,
  XSubmitted,
} from "../messages";
import type { BoardModel } from "../submodel";
import { pipChip } from "./pip-chip";
import { promptCardFace } from "./prompt-card-face";
import { promptModalFrame } from "./prompt-modal";

function messageText(message: MessageRef | null | undefined): string {
  return formatMessage(message);
}

function itemButton(label: string, testId: string, onClick: Message, h: HtmlBuilder<Message>, disabled = false): Html {
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", testId),
      h.OnClick(onClick),
      h.Disabled(disabled),
      h.Class(
        "group relative cursor-pointer rounded-hud border-0 bg-transparent p-0 disabled:cursor-not-allowed disabled:opacity-40",
      ),
    ],
    [
      h.span(
        [
          h.Class(
            "block rounded-hud bg-glass px-3 py-1 text-body text-snow transition-transform duration-150 ease-out group-hover:not-disabled:-translate-y-1 group-hover:not-disabled:bg-glass-dim group-disabled:text-mist",
          ),
        ],
        [label],
      ),
    ],
  );
}

function submitButton(label: string, disabled: boolean, h: HtmlBuilder<Message>): Html {
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", "prompt-submit"),
      h.OnClick(PromptSubmitted()),
      h.Disabled(disabled),
      h.Class(
        "cursor-pointer rounded-hud bg-llanowar px-3 py-1 text-body text-snow hover:bg-llanowar/90 disabled:cursor-not-allowed disabled:bg-glass disabled:text-mist",
      ),
    ],
    [label],
  );
}

function cancelButton(h: HtmlBuilder<Message>): Html {
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", "prompt-cancel"),
      h.OnClick(CancelActionClicked()),
      h.Class("rounded-hud bg-glass px-3 py-1 text-body text-lichen"),
    ],
    ["Cancel"],
  );
}

function frame(testId: string, title: string, body: ReadonlyArray<Html>, h: HtmlBuilder<Message>): Html {
  return h.div(
    [
      h.DataAttribute("testid", testId),
      h.Class(
        "pointer-events-auto fixed top-1/2 left-1/2 z-40 flex max-h-[min(90vh,720px)] max-w-[min(90vw,640px)] -translate-x-1/2 -translate-y-1/2 flex-col gap-2 overflow-y-auto rounded-panel bg-black/70 p-4 text-snow shadow-hud",
      ),
    ],
    [h.div([h.Class("font-semibold text-body")], [title]), ...body],
  );
}

function choiceItemPrint(item: ChoiceItem, state: VisibleState): string {
  if (item.print) return item.print;
  const obj = state.objects.find((o) => o.id === item.id);
  return obj?.print ?? "";
}

function cardPickButton(
  item: ChoiceItem,
  state: VisibleState,
  picked: ReadonlyArray<number>,
  ordered: boolean,
  h: HtmlBuilder<Message>,
): Html {
  const selected = picked.includes(item.id);
  const pickOrder = picked.indexOf(item.id);
  const print = choiceItemPrint(item, state);
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", `prompt-card-${item.id}`),
      h.DataAttribute("selected", selected ? "true" : "false"),
      h.AriaLabel(item.label),
      h.AriaPressed(selected ? "true" : "false"),
      h.OnClick(PromptCardToggled({ id: item.id })),
      h.Class(
        [
          "group/prompt-card relative cursor-pointer rounded-[9px] border-4 border-transparent p-0 transition-transform duration-150 ease-out hover:-translate-y-1",
          "data-[selected=true]:border-llanowar",
        ].join(" "),
      ),
    ],
    [
      promptCardFace(h, { print, label: item.label, size: "sm" }),
      selected && ordered && pickOrder >= 0
        ? h.span(
            [
              h.Class(
                "absolute -top-2 -right-2 flex h-6 w-6 items-center justify-center rounded-full bg-llanowar text-caption font-bold text-snow",
              ),
            ],
            [String(pickOrder + 1)],
          )
        : h.span([], []),
    ],
  );
}

function arrangeLaneCard(
  item: ChoiceItem,
  state: VisibleState,
  laneIds: ReadonlyArray<number>,
  ordered: boolean,
  h: HtmlBuilder<Message>,
): Html {
  const pickOrder = laneIds.indexOf(item.id);
  const print = choiceItemPrint(item, state);
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", `prompt-card-${item.id}`),
      h.AriaLabel(item.label),
      h.OnClick(PromptCardToggled({ id: item.id })),
      h.Class(
        "relative cursor-pointer rounded-[9px] border-4 border-transparent p-0 transition-transform duration-150 ease-out hover:-translate-y-1",
      ),
    ],
    [
      promptCardFace(h, { print, label: item.label, size: "sm" }),
      ordered && pickOrder >= 0
        ? h.span(
            [
              h.Class(
                "absolute -top-2 -right-2 flex h-6 w-6 items-center justify-center rounded-full bg-llanowar text-caption font-bold text-snow",
              ),
            ],
            [String(pickOrder + 1)],
          )
        : h.span([], []),
    ],
  );
}

function arrangeLanesPrompt(
  pending: Extract<PendingChoiceView, { kind: "scry" | "surveil" | "reorder_top" }>,
  state: VisibleState,
  board: BoardModel,
  h: HtmlBuilder<Message>,
): Html {
  const draft = board.promptDraft ?? initPromptDraft(pending, state);
  const buckets =
    draft.kind === "partition" ? draft.buckets : { top: [] as number[], bottom: pending.items.map((it) => it.id) };
  const topIds = buckets.top ?? [];
  const bottomIds = buckets.bottom ?? [];
  const byId = new Map(pending.items.map((it) => [it.id, it]));
  const topItems = topIds.flatMap((id) => {
    const item = byId.get(id);
    return item != null ? [item] : [];
  });
  const bottomItems = bottomIds.flatMap((id) => {
    const item = byId.get(id);
    return item != null ? [item] : [];
  });
  const title = Match.value(pending.kind).pipe(
    Match.withReturnType<string>(),
    Match.when("scry", () => `Scry ${pending.items.length}`),
    Match.when("surveil", () => `Surveil ${pending.items.length}`),
    Match.orElse(() => `Put back ${pending.items.length}`),
  );
  // Natural Selection's cards all go back on top, so its second lane is not a destination —
  // it holds the ones the player has not placed yet, and they follow the ordered pile up.
  const bottomLabel = Match.value(pending.kind).pipe(
    Match.withReturnType<string>(),
    Match.when("surveil", () => "Graveyard"),
    Match.when("reorder_top", () => "Not yet ordered"),
    Match.orElse(() => "Bottom of library"),
  );
  const hint = Match.value(pending.kind).pipe(
    Match.withReturnType<string>(),
    Match.when("surveil", () => "Click a card to move it between Top and Graveyard. Order on Top is left to right."),
    Match.when(
      "reorder_top",
      () => "Click a card to place it on Top. Order on Top is left to right; anything left follows behind it.",
    ),
    Match.orElse(() => "Click a card to move it between Top and Bottom. Order in each lane is left to right."),
  );

  return promptModalFrame(
    {
      testId: "pending-arrange-modal",
      title,
      body: [
        h.div(
          [
            h.DataAttribute("testid", "prompt-arrange-lanes"),
            h.Class("flex min-h-0 w-[min(92vw,720px)] flex-1 flex-col gap-3 overflow-y-auto overscroll-contain"),
          ],
          [
            h.div([h.Class("shrink-0 text-caption text-mist")], [hint]),
            h.div(
              [h.DataAttribute("testid", "prompt-arrange-top"), h.Class("flex flex-col gap-2")],
              [
                h.div(
                  [
                    h.DataAttribute("testid", "prompt-arrange-top-label"),
                    h.Class("text-caption font-semibold text-seafoam"),
                  ],
                  ["Top of library"],
                ),
                h.div(
                  [h.Class("flex min-h-[100px] flex-wrap justify-center gap-2 rounded-panel bg-glass/40 p-2")],
                  topItems.length > 0
                    ? topItems.map((item) => arrangeLaneCard(item, state, topIds, true, h))
                    : [h.div([h.Class("self-center text-caption text-mist")], ["None"])],
                ),
              ],
            ),
            h.div(
              [h.DataAttribute("testid", "prompt-arrange-bottom"), h.Class("flex flex-col gap-2")],
              [
                h.div(
                  [
                    h.DataAttribute("testid", "prompt-arrange-bottom-label"),
                    h.Class("text-caption font-semibold text-seafoam"),
                  ],
                  [bottomLabel],
                ),
                h.div(
                  [h.Class("flex min-h-[100px] flex-wrap justify-center gap-2 rounded-panel bg-glass/40 p-2")],
                  bottomItems.length > 0
                    ? bottomItems.map((item) => arrangeLaneCard(item, state, bottomIds, pending.kind === "scry", h))
                    : [h.div([h.Class("self-center text-caption text-mist")], ["None"])],
                ),
              ],
            ),
          ],
        ),
      ],
      actions: [submitButton("Done", false, h)],
    },
    h,
  );
}

function cardPickPrompt(
  pending: PendingChoiceView,
  items: ReadonlyArray<ChoiceItem>,
  state: VisibleState,
  board: BoardModel,
  config: {
    title: string;
    hint?: string;
    submitLabel: string;
    declineLabel?: string;
    ordered?: boolean;
  },
  h: HtmlBuilder<Message>,
): Html {
  const draft = board.promptDraft ?? initPromptDraft(pending, state);
  const picked = draft.kind === "card-pick" ? draft.picked : [];
  const filter = draft.kind === "card-pick" ? (draft.filter ?? "") : "";
  const ready = cardPickReady(pending, picked);
  const searchable = cardPickIsSearchable(pending.kind);
  const required = searchable ? 1 : null;
  const shown = searchable
    ? required === 1
      ? searchableChoiceItems(items, filter)
      : filterChoiceItems(items, filter)
    : items;

  const hintEl = config.hint != null ? h.div([h.Class("shrink-0 text-caption text-mist")], [config.hint]) : null;
  const filterEl = searchable
    ? input(h, {
        id: "pick-card-filter",
        variant: "hud",
        testId: "pick-card-filter",
        type: "search",
        placeholder: "Filter by name…",
        autofocus: true,
        ariaLabel: "Filter cards by name",
        value: filter,
        onInput: (v) => PromptCardFilterSet({ query: v }),
        class: "w-[min(90vw,320px)]",
      })
    : null;
  const emptyEl =
    searchable && filter.trim() !== "" && shown.length === 0
      ? h.div([h.Class("text-label text-mist")], ["No cards match."])
      : null;
  const cardsEl = h.div(
    [h.Class("flex flex-wrap justify-center gap-2")],
    [...shown.map((item) => cardPickButton(item, state, picked, config.ordered ?? false, h)), emptyEl].filter(
      (v): v is Html => v !== null,
    ),
  );
  const actionsEl = h.div(
    [h.Class("flex shrink-0 flex-wrap gap-2")],
    [
      submitButton(config.submitLabel, !ready, h),
      config.declineLabel != null
        ? itemButton(config.declineLabel, "prompt-decline", PromptDeclined(), h)
        : h.span([], []),
    ],
  );

  const scrollEl = h.div(
    [
      h.DataAttribute("testid", "pick-card-scroll"),
      h.Class(
        `${PICK_CARD_SCROLL_MIN_CLASS} w-full flex-1 overflow-y-auto overscroll-contain rounded-panel bg-glass/30 p-2`,
      ),
    ],
    [cardsEl],
  );

  if (searchable) {
    return promptModalFrame(
      {
        testId: "pending-library-modal",
        title: config.title,
        body: [
          h.div([h.DataAttribute("testid", "pick-title"), h.Class("sr-only")], [config.title]),
          h.div(
            [h.Class("pointer-events-none shrink-0 text-caption text-mist")],
            ["Filter by name, click a card, then Choose — or Fail to find."],
          ),
          h.div(
            [h.Class("flex min-h-0 w-[min(92vw,720px)] flex-1 flex-col gap-2")],
            [filterEl ?? h.span([], []), scrollEl],
          ),
        ].filter((v): v is Html => v !== null),
        actions: [
          submitButton(config.submitLabel, !ready, h),
          config.declineLabel != null
            ? itemButton(config.declineLabel, "prompt-decline", PromptDeclined(), h)
            : h.span([], []),
        ].filter((v): v is Html => v !== null),
      },
      h,
    );
  }

  return promptModalFrame(
    {
      testId: "pending-card-pick-modal",
      title: config.title,
      body: [
        h.div([h.DataAttribute("testid", "pick-title"), h.Class("sr-only")], [config.title]),
        hintEl,
        h.div([h.Class("flex min-h-0 w-[min(92vw,720px)] flex-1 flex-col gap-2")], [scrollEl]),
      ].filter((v): v is Html => v !== null),
      actions: [actionsEl],
    },
    h,
  );
}

function orderPrompt(
  pending: Extract<PendingChoiceView, { kind: "order_triggers" }>,
  board: BoardModel,
  h: HtmlBuilder<Message>,
): Html {
  const draft = board.promptDraft;
  const order = draft?.kind === "order" ? draft.order : pending.labels.map((_, i) => i);
  const pick = board.orderPickPos;
  const rows = order.map((effectIndex, pos) => {
    const selected = pick === pos;
    return h.div(
      [
        h.DataAttribute("testid", `prompt-order-${pos}`),
        h.DataAttribute("selected", selected ? "true" : "false"),
        h.Draggable(true),
        h.OnDragStart(PromptOrderRowClicked({ pos })),
        h.AllowDrop(),
        h.OnDrop(PromptOrderRowClicked({ pos })),
        h.OnDragEnd(PromptOrderDragEnded()),
        h.Class(
          [
            "group/prompt-order flex cursor-grab items-center gap-2 rounded-hud border border-transparent bg-glass/50 px-2 py-2 transition-colors active:cursor-grabbing",
            "data-[selected=true]:border-llanowar data-[selected=true]:bg-llanowar/20 data-[selected=true]:opacity-80",
          ].join(" "),
        ),
      ],
      [
        h.button(
          [
            h.Type("button"),
            h.DataAttribute("testid", `prompt-order-up-${pos}`),
            h.AriaLabel("Move up"),
            h.Disabled(pos === 0),
            h.OnClick(PromptOrderMoved({ pos, delta: -1 })),
            h.Class("rounded-hud bg-glass px-2 py-1 text-body disabled:opacity-40"),
          ],
          ["↑"],
        ),
        h.button(
          [
            h.Type("button"),
            h.DataAttribute("testid", `prompt-order-down-${pos}`),
            h.AriaLabel("Move down"),
            h.Disabled(pos === order.length - 1),
            h.OnClick(PromptOrderMoved({ pos, delta: 1 })),
            h.Class("rounded-hud bg-glass px-2 py-1 text-body disabled:opacity-40"),
          ],
          ["↓"],
        ),
        h.button(
          [
            h.Type("button"),
            h.DataAttribute("testid", `prompt-order-pick-${pos}`),
            h.AriaLabel(selected ? "Cancel move" : "Pick to reorder"),
            h.AriaPressed(selected ? "true" : "false"),
            h.OnClick(PromptOrderRowClicked({ pos })),
            h.Class(
              "min-w-0 flex-1 cursor-pointer rounded-hud border-0 bg-transparent px-2 py-1 text-left text-body text-snow hover:bg-glass",
            ),
          ],
          [messageText(pending.labels[effectIndex])],
        ),
      ],
    );
  });
  return promptModalFrame(
    {
      testId: "pending-order-modal",
      title: "Order these triggers — the last one resolves first",
      body: [
        h.div(
          [h.Class("flex min-h-0 w-[min(92vw,560px)] flex-1 flex-col gap-2")],
          [
            h.div(
              [h.Class("shrink-0 text-caption text-mist")],
              [
                pick == null
                  ? "Drag a trigger to reorder, or click then click where it should go (↑↓ also work)."
                  : "Drop on another row to place it — or click / release to cancel.",
              ],
            ),
            h.div(
              [
                h.DataAttribute("testid", "prompt-order-list"),
                h.Class("flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto overscroll-contain"),
              ],
              rows,
            ),
          ],
        ),
      ],
      actions: [submitButton("Submit", false, h)],
    },
    h,
  );
}

function amountStepper(id: number, amount: number, max: number, h: HtmlBuilder<Message>): Html {
  const value = clampX(amount, 0, max);
  return h.div(
    [h.Class("flex flex-wrap items-center gap-1")],
    [
      itemButton("Min", `prompt-damage-${id}-min`, PromptDamageSet({ id, amount: 0 }), h),
      itemButton("−", `prompt-damage-${id}-dec`, PromptDamageSet({ id, amount: value - 1 }), h, value <= 0),
      h.span(
        [
          h.DataAttribute("testid", `prompt-damage-${id}-value`),
          h.Class("min-w-[2ch] text-center text-body font-semibold text-snow"),
        ],
        [String(value)],
      ),
      itemButton("+", `prompt-damage-${id}-inc`, PromptDamageSet({ id, amount: value + 1 }), h, value >= max),
      itemButton("Max", `prompt-damage-${id}-max`, PromptDamageSet({ id, amount: max }), h),
    ],
  );
}

function damageAssignPrompt(
  pending: Extract<PendingChoiceView, { kind: "assign_combat_damage" }>,
  state: VisibleState,
  board: BoardModel,
  h: HtmlBuilder<Message>,
): Html {
  const draft = board.promptDraft ?? initPromptDraft(pending, state);
  const amounts = draft.kind === "damage" ? draft.amounts : {};
  const source = state.objects.find((o) => o.id === pending.source);
  const power = source?.power ?? 0;
  const trample = source?.keywords?.includes("trample") ?? false;
  const assigned = Object.values(amounts).reduce((s, n) => s + n, 0);
  const ready = damageAssignReady(pending, draft, state);
  const overflow = trample ? Math.max(0, power - assigned) : 0;
  const onBoard = pendingDamageAssignBlockers(pending, state) != null;
  const rows = onBoard
    ? []
    : pending.items.map((it) =>
        h.div(
          [h.Class("flex items-center gap-2")],
          [
            h.span([h.Class("w-28 truncate text-body")], [it.label]),
            amountStepper(it.id, amounts[it.id] ?? 0, power, h),
          ],
        ),
      );
  return h.div(
    [
      h.DataAttribute("testid", "pending-damage-aim"),
      h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
      h.Class(
        [
          "fixed bottom-(--b) left-1/2 z-30 flex max-w-[min(100%-2rem,28rem)] -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
          onBoard ? "pointer-events-none" : "pointer-events-auto",
        ].join(" "),
      ),
    ],
    [
      h.div(
        [h.Class("pointer-events-none text-center font-semibold text-body text-snow")],
        [`Divide ${power} damage among blockers`],
      ),
      onBoard
        ? h.div(
            [h.Class("pointer-events-none text-center text-body text-mist")],
            ["Click a blocker on the board to move 1 damage onto it"],
          )
        : null,
      ...rows,
      h.div(
        [
          h.DataAttribute("testid", "prompt-damage-assigned"),
          h.Class(ready ? "text-assign-clover" : "text-caution-amber"),
        ],
        [`assigned ${assigned} / ${power}`],
      ),
      trample
        ? h.div(
            [h.DataAttribute("testid", "prompt-damage-overflow"), h.Class("text-body text-mist")],
            [`to defender: ${overflow}`],
          )
        : null,
      onBoard ? null : submitButton("Assign", !ready, h),
    ].filter((v): v is Html => v !== null),
  );
}

function targetPickButton(target: WireTarget, state: VisibleState, testId: string, h: HtmlBuilder<Message>): Html {
  if (target.kind === "player") {
    const label = playerSeatLabel(state, target.player);
    return h.button(
      [
        h.Type("button"),
        h.DataAttribute("testid", testId),
        h.AriaLabel(`Player ${label}`),
        h.OnClick(TargetChosen({ target })),
        h.Class(
          "relative cursor-pointer rounded-[9px] p-0 shadow-hand transition-transform duration-150 ease-out hover:-translate-y-2",
        ),
      ],
      [
        h.div(
          [
            h.Style({ "--seat": seatColor(target.player, 0.9) }),
            h.Class(
              "flex aspect-[150/209] w-[150px] flex-col items-center justify-center rounded-[9px] border-4 border-(--seat) bg-morph-slate font-bold text-title text-snow",
            ),
          ],
          [label],
        ),
      ],
    );
  }
  const name = objectName(state, target.id);
  const obj = state.objects.find((o) => o.id === target.id);
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", testId),
      h.AriaLabel(name),
      h.OnClick(TargetChosen({ target })),
      h.Class(
        "relative cursor-pointer rounded-[9px] p-0 shadow-hand transition-transform duration-150 ease-out hover:-translate-y-2",
      ),
    ],
    [promptCardFace(h, { print: obj?.print ?? "", label: name, size: "md" })],
  );
}

function targetPickPrompt(
  title: string,
  targets: ReadonlyArray<WireTarget>,
  state: VisibleState,
  h: HtmlBuilder<Message>,
): Html {
  return promptModalFrame(
    {
      testId: "target-pick-modal",
      title,
      body: [
        h.div(
          [
            h.Class(
              `${PICK_CARD_SCROLL_MIN_CLASS} w-[min(92vw,720px)] flex-1 overflow-y-auto overscroll-contain rounded-panel bg-glass/30 p-2`,
            ),
          ],
          [
            h.div(
              [h.Class("flex flex-wrap justify-center gap-3")],
              targets.map((t, i) => targetPickButton(t, state, `target-pick-${i}`, h)),
            ),
          ],
        ),
      ],
      actions: [cancelButton(h)],
    },
    h,
  );
}

function boardXPrompt(prompt: NonNullable<BoardModel["xPrompt"]>, h: HtmlBuilder<Message>): Html {
  const { minX, maxX, draftX, xCost, name } = prompt;
  const preview = costText(costWithChosenX(xCost, draftX));
  return promptModalFrame(
    {
      testId: "x-prompt-modal",
      title: `Choose X for ${name}`,
      body: [
        h.div(
          [h.Class("flex min-h-0 w-[min(92vw,28rem)] flex-col items-center gap-sm")],
          [
            h.div(
              [
                h.Class("flex items-center justify-center gap-2 text-body text-mist"),
                h.DataAttribute("testid", "x-prompt-preview"),
              ],
              [`Pay ${preview}`],
            ),
            h.div(
              [h.Class("flex flex-wrap items-center justify-center gap-2")],
              [
                itemButton("Min", "x-prompt-min", XDraftSet({ x: minX }), h),
                itemButton("−", "x-prompt-dec", XDraftSet({ x: draftX - 1 }), h, draftX <= minX),
                h.span(
                  [
                    h.DataAttribute("testid", "x-prompt-value"),
                    h.Class("min-w-[2ch] text-center text-body font-semibold text-snow"),
                  ],
                  [String(draftX)],
                ),
                itemButton("+", "x-prompt-inc", XDraftSet({ x: draftX + 1 }), h, draftX >= maxX),
                itemButton("Max", "x-prompt-max", XDraftSet({ x: maxX }), h),
              ],
            ),
          ],
        ),
      ],
      actions: [itemButton("Confirm", "x-prompt-confirm", XSubmitted({ x: draftX }), h), cancelButton(h)],
    },
    h,
  );
}

function costPickPrompt(
  testId: string,
  title: string,
  choices: ReadonlyArray<number>,
  state: VisibleState,
  message: (id: number) => Message,
  h: HtmlBuilder<Message>,
  presentation: "aim" | "modal" = "aim",
): Html {
  const body = h.div(
    [h.Class("flex min-h-0 w-[min(92vw,28rem)] flex-wrap justify-center gap-2")],
    choices.map((id) => {
      const obj = state.objects.find((o) => o.id === id);
      return itemButton(obj?.name ?? `#${id}`, `${testId}-${id}`, message(id), h);
    }),
  );
  if (presentation === "modal") {
    return promptModalFrame(
      {
        testId: `${testId}-modal`,
        title,
        body: [body],
        actions: [cancelButton(h)],
      },
      h,
    );
  }
  return h.div(
    [
      h.DataAttribute("testid", `${testId}-aim`),
      h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
      h.Class(
        "pointer-events-auto fixed bottom-(--b) left-1/2 z-30 flex max-w-[min(100%-2rem,28rem)] -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [
      h.div([h.Class("pointer-events-none text-center font-semibold text-body text-snow")], [title]),
      body,
      cancelButton(h),
    ],
  );
}

function modalPrompt(mc: NonNullable<BoardModel["modalCast"]>, h: HtmlBuilder<Message>): Html {
  if (mc.chosen == null) {
    const title = messageText(mc.action.label) || "Choose modes";
    return h.div(
      [
        h.DataAttribute("testid", "modal-mode-aim"),
        h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
        h.Class(
          "pointer-events-auto fixed bottom-(--b) left-1/2 z-30 flex max-w-[min(100%-2rem,28rem)] -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
        ),
      ],
      [h.div([h.Class("pointer-events-none text-center font-semibold text-body text-snow")], [title])],
    );
  }
  return h.div(
    [
      h.DataAttribute("testid", "modal-waiting-aim"),
      h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
      h.Class(
        "pointer-events-none fixed bottom-(--b) left-1/2 z-30 flex max-w-[min(100%-2rem,28rem)] -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [
      h.div(
        [h.Class("pointer-events-none text-center font-semibold text-body text-snow")],
        ["Pick a target for the chosen mode."],
      ),
    ],
  );
}

function playModePrompt(_pick: NonNullable<BoardModel["playModePick"]>, h: HtmlBuilder<Message>): Html {
  return h.div(
    [
      h.DataAttribute("testid", "play-mode-aim"),
      h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
      h.Class(
        "pointer-events-auto fixed bottom-(--b) left-1/2 z-30 flex max-w-[min(100%-2rem,28rem)] -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [h.div([h.Class("pointer-events-none text-center font-semibold text-body text-snow")], ["Choose how to play"])],
  );
}

function pendingChoiceTitle(pending: PendingChoiceView): string {
  if ("label" in pending) {
    const label = messageText(pending.label);
    if (label !== "") return label;
  }
  return `Choose (${pending.kind})`;
}

function answerButton(
  pending: PendingChoiceView,
  testId: string,
  label: string,
  answer: AnswerInput,
  primary: boolean,
  disabled = false,
  h: HtmlBuilder<Message>,
): Html {
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", testId),
      h.Disabled(disabled),
      h.OnClick(PendingChoiceAnswered({ intent: choiceIntent(pending, answer) })),
      h.DataAttribute("primary", String(primary)),
      h.Class("group relative rounded-hud border-0 bg-transparent p-0 disabled:cursor-not-allowed disabled:opacity-50"),
    ],
    [
      h.span(
        [
          h.Class(
            "block rounded-hud px-3 py-1 text-body transition-transform duration-150 ease-out group-hover:-translate-y-1 group-data-[primary=true]:bg-llanowar group-data-[primary=true]:text-snow group-data-[primary=false]:bg-glass group-data-[primary=false]:text-lichen",
          ),
        ],
        [label],
      ),
    ],
  );
}

function playerSeatFromItem(item: ChoiceItem, state: VisibleState, fallbackIndex: number): number | null {
  if (item.player != null) return item.player;
  const match = item.label.match(/^Player\s+(\d+)$/i);
  if (match != null) {
    const seat = Number.parseInt(match[1] ?? "", 10) - 1;
    if (!Number.isNaN(seat)) return seat;
  }
  const fallback = state.players[fallbackIndex];
  return fallback?.player ?? null;
}

function targetLabel(target: WireTarget, state: VisibleState): string {
  if (target.kind === "player") return playerSeatLabel(state, target.player);
  return objectName(state, target.id);
}

function sameTarget(a: WireTarget | null | undefined, b: WireTarget | null | undefined): boolean {
  if (a == null || b == null) return a == null && b == null;
  if (a.kind !== b.kind) return false;
  if (a.kind === "player" && b.kind === "player") return a.player === b.player;
  if (a.kind === "object" && b.kind === "object") return a.id === b.id;
  return false;
}

function sameModeChoice(a: WireModeChoice, b: WireModeChoice): boolean {
  return a.index === b.index && sameTarget(a.target, b.target);
}

function cardPickDeclineLabel(pending: PendingChoiceView): string | null {
  switch (pending.kind) {
    case "search_library":
      return "Fail to find";
    case "put_land_from_hand":
      return "Don't put a land";
    case "put_creature_from_hand":
      return "Don't put a creature";
    case "choose_exiled_with_card":
    case "opponent_chooses_exiled_nonland":
    case "opponent_chooses_revealed_to_graveyard":
      return "Choose none";
    case "choose_exiled_with_card_to_cast":
    case "choose_exiled_dig_to_cast_free":
      return "Don't cast";
    case "may_return_from_graveyard":
      return pending.mandatory ? null : "Don't return";
    case "may_exile_discarded_to_play":
      return "Don't exile";
    case "choose_attach_host":
      return pending.optional ? "Don't attach" : null;
    case "choose_target":
      return pending.min === 0 ? "No target" : null;
    case "pay_cumulative_upkeep_or_sacrifice":
      return "Don't pay";
    case "choose_dredge":
      return "Draw normally";
    default:
      return null;
  }
}

function cardPickConfig(pending: PendingChoiceView): {
  title: string;
  hint?: string;
  submitLabel: string;
  declineLabel?: string;
  ordered?: boolean;
} {
  const declineLabel = cardPickDeclineLabel(pending) ?? undefined;
  switch (pending.kind) {
    case "choose_target":
      return { title: messageText(pending.label), submitLabel: "Choose", declineLabel };
    case "choose_activation_cost_targets":
      return { title: "Choose cost targets", submitLabel: "Choose" };
    case "decline_untap":
      return {
        title: "Choose permanents to keep tapped",
        // Smoke / Winter Orb: leaving two of a capped group up is rejected by the server, so say
        // why Keep tapped is greyed out rather than letting the answer bounce.
        hint:
          (pending.at_most_one ?? []).length > 0
            ? "Only one of the capped permanents may untap — keep the rest tapped."
            : undefined,
        submitLabel: "Keep tapped",
      };
    case "sacrifice_unless_return_land":
      return { title: "Return a land or sacrifice", submitLabel: "Return land" };
    case "scry":
      return {
        title: `Scry ${pending.items.length}`,
        hint: "Click a card to move it between Top and Bottom. Order in each lane is left to right.",
        submitLabel: "Done",
        ordered: true,
      };
    case "surveil":
      return {
        title: `Surveil ${pending.items.length}`,
        hint: "Click a card to move it between Top and Graveyard. Order on Top is left to right.",
        submitLabel: "Done",
        ordered: true,
      };
    case "search_library":
      return { title: "Search your library", submitLabel: "Choose", declineLabel };
    case "select_from_top":
      return {
        title: `Select up to ${pending.up_to} from the top`,
        hint: "Click cards to take — the rest go to the bottom.",
        submitLabel: "Done",
      };
    case "shuffle_from_graveyard":
      return {
        title: `Choose up to ${pending.max} card${pending.max === 1 ? "" : "s"} to shuffle in`,
        submitLabel: "Shuffle",
      };
    case "sacrifice_edict":
      return {
        title: pending.keep_one ? "Choose permanents to sacrifice (keep one)" : "Choose a permanent to sacrifice",
        submitLabel: "Sacrifice",
      };
    case "proliferate":
      return { title: "Proliferate — choose any number", submitLabel: "Proliferate" };
    case "phase_out":
      return { title: "Choose permanents to phase out", submitLabel: "Phase out" };
    case "may_sacrifice":
      return { title: "You may sacrifice any number", submitLabel: "Continue" };
    case "choose_own_sacrifices":
      return {
        title: `Choose ${pending.count} permanent${pending.count === 1 ? "" : "s"} to sacrifice`,
        submitLabel: "Sacrifice",
      };
    case "devour":
      return { title: "Choose creatures to devour", submitLabel: "Devour" };
    case "exile_from_graveyard":
      return { title: "Choose cards to exile from a graveyard", submitLabel: "Exile" };
    case "caster_keep_permanents":
      return { title: "Choose permanents to keep", submitLabel: "Keep" };
    case "choose_counter_target_for_player":
      return { title: "Choose permanents that get counters", submitLabel: "Choose" };
    case "may_return_from_graveyard":
      return { title: "Choose cards to return from your graveyard", submitLabel: "Return" };
    case "may_exile_discarded_to_play":
      return {
        title: "Choose a discarded nonland card to exile and play this turn",
        submitLabel: "Exile",
        declineLabel,
      };
    case "may_discard":
      return { title: "Choose cards to discard", submitLabel: "Discard" };
    case "discard":
      return { title: `Discard ${pending.count} card${pending.count === 1 ? "" : "s"}`, submitLabel: "Discard" };
    case "put_land_from_hand":
      return {
        title: "Put a land from your hand onto the battlefield",
        submitLabel: "Put onto battlefield",
        declineLabel,
      };
    case "put_creature_from_hand":
      return {
        title: "Put a creature from your hand onto the battlefield",
        submitLabel: "Put onto battlefield",
        declineLabel,
      };
    case "choose_dredge":
      return { title: "Choose a card to dredge", submitLabel: "Dredge", declineLabel };
    case "cast_creature_face_down":
      return { title: "Choose a creature to cast face down", submitLabel: "Cast face down" };
    case "choose_exiled_with_card":
      return { title: "Choose an exiled card", submitLabel: "Choose", declineLabel };
    case "choose_exiled_with_card_to_cast":
      return { title: "Choose an exiled card to cast", submitLabel: "Cast", declineLabel };
    case "choose_exiled_dig_to_cast_free":
      // Word of Command rides this shape with the candidates coming from an opponent's hand.
      if (pending.from_opponent_hand) {
        return { title: "Choose a card from their hand for them to play", submitLabel: "Play", declineLabel };
      }
      return { title: "Choose a card to cast for free", submitLabel: "Cast", declineLabel };
    case "opponent_chooses_exiled_nonland":
      return { title: "Choose an exiled nonland card", submitLabel: "Choose", declineLabel };
    case "choose_exiled_to_cast_free":
      return {
        title: `Choose up to ${pending.count} card${pending.count === 1 ? "" : "s"} to cast for free`,
        submitLabel: "Choose",
      };
    case "choose_copy_target":
      if (pending.put_counter_on_creature) {
        return { title: "Choose a creature to get a +1/+1 counter", submitLabel: "Put counter" };
      }
      if (pending.choose_block_target) {
        return { title: "Choose an attacking creature to block", submitLabel: "Block", declineLabel };
      }
      return { title: "Choose a copy target", submitLabel: "Copy" };
    case "choose_attach_host":
      return { title: "Choose what to attach to", submitLabel: "Attach", declineLabel };
    case "choose_legendary_keep":
      return {
        title: `Legend rule — choose which ${pending.name} to keep`,
        submitLabel: "Keep",
      };
    case "put_from_hand_on_top":
      return {
        title: `Put ${pending.count} card${pending.count === 1 ? "" : "s"} from your hand on top`,
        submitLabel: "Put on top",
      };
    case "opponent_chooses_revealed_to_graveyard":
      return { title: "Choose a revealed card to put into the graveyard", submitLabel: "Choose", declineLabel };
    case "pay_cumulative_upkeep_or_sacrifice":
      return {
        title: `Pay cumulative upkeep — choose ${pending.count} card${pending.count === 1 ? "" : "s"}`,
        submitLabel: "Pay",
        declineLabel,
      };
    default:
      return { title: pendingChoiceTitle(pending), submitLabel: "Choose" };
  }
}

function pendingGraveyardAimCoach(
  kind:
    | "exile_from_graveyard"
    | "may_return_from_graveyard"
    | "may_exile_discarded_to_play"
    | "shuffle_from_graveyard"
    | "choose_dredge"
    | "pay_cumulative_upkeep_or_sacrifice"
    | "choose_activation_cost_targets"
    | "choose_target",
  oneClick: boolean,
): string {
  switch (kind) {
    case "exile_from_graveyard":
      return oneClick ? "Click a card in the graveyard to exile" : "Click cards in the graveyard to exile";
    case "may_return_from_graveyard":
      return "Click cards in the graveyard to return";
    case "may_exile_discarded_to_play":
      return "Click a discarded nonland card in your graveyard to exile";
    case "shuffle_from_graveyard":
      return oneClick ? "Click a card in the graveyard to shuffle in" : "Click cards in the graveyard to shuffle in";
    case "choose_dredge":
      return "Click a card in the graveyard to dredge";
    case "pay_cumulative_upkeep_or_sacrifice":
      return oneClick
        ? "Click a card in a graveyard to pay cumulative upkeep"
        : "Click cards in a graveyard to pay cumulative upkeep";
    case "choose_activation_cost_targets":
      return oneClick
        ? "Click a card in the graveyard for the activation cost"
        : "Click cards in the graveyard for the activation cost";
    case "choose_target":
      return oneClick ? "Click a card in the graveyard to target" : "Click cards in the graveyard to target";
    default: {
      const _exhaustive: never = kind;
      return _exhaustive;
    }
  }
}

function pendingExileAimCoach(
  kind:
    | "choose_exiled_with_card"
    | "choose_exiled_with_card_to_cast"
    | "choose_exiled_dig_to_cast_free"
    | "opponent_chooses_exiled_nonland"
    | "choose_exiled_to_cast_free",
  oneClick: boolean,
): string {
  switch (kind) {
    case "choose_exiled_with_card":
    case "opponent_chooses_exiled_nonland":
      return "Click a card in exile to choose";
    case "choose_exiled_with_card_to_cast":
    case "choose_exiled_dig_to_cast_free":
      return "Click a card in exile to cast";
    case "choose_exiled_to_cast_free":
      return oneClick ? "Click a card in exile to cast" : "Click cards in exile to cast";
    default: {
      const _exhaustive: never = kind;
      return _exhaustive;
    }
  }
}

function pendingHandAimCoach(
  kind:
    | "discard"
    | "may_discard"
    | "put_land_from_hand"
    | "put_creature_from_hand"
    | "put_from_hand_on_top"
    | "cast_creature_face_down",
  oneClick: boolean,
): string {
  switch (kind) {
    case "discard":
    case "may_discard":
      return oneClick ? "Click a card in your hand to discard" : "Click cards in your hand to discard";
    case "put_land_from_hand":
      return oneClick
        ? "Click a land in your hand to put onto the battlefield"
        : "Click a land in your hand, then Confirm";
    case "put_creature_from_hand":
      return oneClick
        ? "Click a creature in your hand to put onto the battlefield"
        : "Click a creature in your hand, then Confirm";
    case "cast_creature_face_down":
      return oneClick
        ? "Click a creature in your hand to cast face down"
        : "Click a creature in your hand, then Confirm";
    case "put_from_hand_on_top":
      return oneClick
        ? "Click a card in your hand to put on top of your library"
        : "Click cards in your hand, then Confirm";
    default: {
      const _exhaustive: never = kind;
      return _exhaustive;
    }
  }
}

function revealedToGraveyardAim(
  pending: Extract<PendingChoiceView, { kind: "opponent_chooses_revealed_to_graveyard" }>,
  state: VisibleState,
  tableId: string | null,
  h: HtmlBuilder<Message>,
): Html {
  const cards = pending.items.map((item) => {
    const print = choiceItemPrint(item, state);
    return h.button(
      [
        h.Type("button"),
        h.DataAttribute("testid", `prompt-card-${item.id}`),
        h.AriaLabel(item.label),
        h.OnClick(
          PendingChoiceAnswered({
            intent: choiceIntent(pending, { kind: "choose_exiled", choice: item.id }),
          }),
        ),
        h.Disabled(tableId == null),
        h.Class(
          "relative cursor-pointer rounded-[9px] border-4 border-transparent p-0 transition-transform duration-150 ease-out hover:-translate-y-1 disabled:cursor-not-allowed disabled:opacity-50",
        ),
      ],
      [promptCardFace(h, { print, label: item.label, size: "sm" })],
    );
  });
  return h.div(
    [
      h.DataAttribute("testid", "pending-revealed-aim"),
      h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
      h.Class(
        "pointer-events-auto fixed bottom-(--b) left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [
      h.div([h.Class("pointer-events-none text-center")], ["Click a revealed card to put into the graveyard"]),
      h.div([h.Class("flex max-w-[min(90vw,720px)] flex-wrap justify-center gap-2")], cards),
    ].filter((v): v is Html => v !== null),
  );
}

function cardPickForKind(
  pending: PendingChoiceView,
  state: VisibleState,
  board: BoardModel,
  tableId: string | null,
  h: HtmlBuilder<Message>,
): Html | null {
  if (pending.kind === "opponent_chooses_revealed_to_graveyard") {
    return revealedToGraveyardAim(pending, state, tableId, h);
  }
  const gyPick = pendingGraveyardPickIds(pending, state);
  if (gyPick != null) {
    const kind = pending.kind;
    if (
      kind !== "exile_from_graveyard" &&
      kind !== "may_return_from_graveyard" &&
      kind !== "may_exile_discarded_to_play" &&
      kind !== "shuffle_from_graveyard" &&
      kind !== "choose_dredge" &&
      kind !== "pay_cumulative_upkeep_or_sacrifice" &&
      kind !== "choose_activation_cost_targets" &&
      kind !== "choose_target"
    ) {
      return null;
    }
    const oneClick = pendingGraveyardPickOneClick(pending);
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    const picked = draft.kind === "card-pick" ? draft.picked : [];
    const required = cardPickRequiredCount(pending);
    const maxHint = kind === "shuffle_from_graveyard" || kind === "choose_target" ? pending.max : required;
    const countLine =
      !oneClick && maxHint != null
        ? h.div(
            [h.DataAttribute("testid", "pending-gy-count"), h.Class("pointer-events-none text-caption text-mist")],
            [`${picked.length} / ${maxHint} selected`],
          )
        : !oneClick
          ? h.div(
              [h.DataAttribute("testid", "pending-gy-count"), h.Class("pointer-events-none text-caption text-mist")],
              [`${picked.length} selected`],
            )
          : null;
    return h.div(
      [
        h.DataAttribute("testid", "pending-gy-aim"),
        h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
        h.Class(
          "pointer-events-none fixed bottom-(--b) left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
        ),
      ],
      [h.div([h.Class("pointer-events-none")], [pendingGraveyardAimCoach(kind, oneClick)]), countLine].filter(
        (v): v is Html => v !== null,
      ),
    );
  }
  const exilePick = pendingExilePickIds(pending, state);
  if (exilePick != null) {
    const kind = pending.kind;
    if (
      kind !== "choose_exiled_with_card" &&
      kind !== "choose_exiled_with_card_to_cast" &&
      kind !== "choose_exiled_dig_to_cast_free" &&
      kind !== "opponent_chooses_exiled_nonland" &&
      kind !== "choose_exiled_to_cast_free"
    ) {
      return null;
    }
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    // Dig-cast Aura: after the exile pick, aim at cast_targets on the battlefield.
    if (kind === "choose_exiled_dig_to_cast_free" && pendingDigCastHostMode(pending, state, draft) != null) {
      // fall through to dig-host / board-target chrome below
    } else {
      const oneClick = pendingExilePickOneClick(pending);
      const picked = draft.kind === "card-pick" ? draft.picked : [];
      const required = cardPickRequiredCount(pending);
      const upTo = kind === "choose_exiled_to_cast_free";
      const countLine =
        !oneClick && required != null
          ? h.div(
              [h.DataAttribute("testid", "pending-exile-count"), h.Class("pointer-events-none text-caption text-mist")],
              [upTo ? `${picked.length} / up to ${required} selected` : `${picked.length} / ${required} selected`],
            )
          : null;
      return h.div(
        [
          h.DataAttribute("testid", "pending-exile-aim"),
          h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
          h.Class(
            "pointer-events-none fixed bottom-(--b) left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
          ),
        ],
        [h.div([h.Class("pointer-events-none")], [pendingExileAimCoach(kind, oneClick)]), countLine].filter(
          (v): v is Html => v !== null,
        ),
      );
    }
  }
  const handPick = pendingHandPickIds(pending, state);
  if (handPick != null) {
    const kind = pending.kind;
    if (
      kind !== "discard" &&
      kind !== "may_discard" &&
      kind !== "put_land_from_hand" &&
      kind !== "put_creature_from_hand" &&
      kind !== "put_from_hand_on_top" &&
      kind !== "cast_creature_face_down"
    ) {
      return null;
    }
    const discardKind = kind === "discard" || kind === "may_discard";
    const oneClick = pendingHandPickOneClick(pending);
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    const picked = draft.kind === "card-pick" ? draft.picked : [];
    const required = cardPickRequiredCount(pending);
    const countLine =
      !oneClick && required != null
        ? h.div(
            [
              h.DataAttribute("testid", discardKind ? "pending-discard-count" : "pending-hand-count"),
              h.Class("pointer-events-none text-caption text-mist"),
            ],
            [`${picked.length} / ${required} selected`],
          )
        : null;
    return h.div(
      [
        h.DataAttribute("testid", discardKind ? "pending-discard-aim" : "pending-hand-aim"),
        h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
        h.Class(
          "pointer-events-none fixed bottom-(--b) left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
        ),
      ],
      [h.div([h.Class("pointer-events-none")], [pendingHandAimCoach(kind, oneClick)]), countLine].filter(
        (v): v is Html => v !== null,
      ),
    );
  }
  if (
    pendingBoardTargetMode(pending, state) != null ||
    pendingDigCastHostMode(pending, state, board.promptDraft) != null
  ) {
    const digHost = pendingDigCastHostMode(pending, state, board.promptDraft);
    const label =
      digHost != null
        ? "Choose what to enchant"
        : "label" in pending
          ? messageText(pending.label)
          : FORMULATOR_FOR_KIND[pending.kind] === "cardPick"
            ? cardPickConfig(pending).title
            : pendingChoiceTitle(pending);
    const oneClick = digHost != null || pendingTargetOneClick(pending);
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    const picked = draft.kind === "card-pick" ? draft.picked : [];
    const max =
      pending.kind === "choose_target"
        ? pending.max
        : pending.kind === "choose_own_sacrifices" || pending.kind === "choose_activation_cost_targets"
          ? pending.count
          : pending.kind === "sacrifice_edict"
            ? cardPickRequiredCount(pending)
            : null;
    const countLine =
      !oneClick && max != null
        ? h.div(
            [h.DataAttribute("testid", "pending-target-count"), h.Class("pointer-events-none text-caption text-mist")],
            [`${picked.length} / ${max} selected`],
          )
        : null;
    return h.div(
      [
        h.DataAttribute("testid", "pending-target-aim"),
        h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
        h.Class(
          "pointer-events-none fixed bottom-(--b) left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
        ),
      ],
      [h.div([h.Class("pointer-events-none")], [label]), countLine].filter((v): v is Html => v !== null),
    );
  }
  if (pending.kind === "choose_target" && !chooseTargetIsCardPick(pending.items)) {
    const buttons = pending.items.flatMap((item, index) => {
      const seat = playerSeatFromItem(item, state, index);
      if (seat == null) return [];
      return [
        answerButton(
          pending,
          `prompt-player-${seat}`,
          item.label,
          { kind: "target", id: item.id, player: seat },
          false,
          tableId == null,
          h,
        ),
      ];
    });
    const decline = declineAnswer(pending);
    if (decline != null) {
      buttons.push(
        answerButton(
          pending,
          "prompt-decline",
          cardPickDeclineLabel(pending) ?? "Decline",
          decline,
          false,
          tableId == null,
          h,
        ),
      );
    }
    return promptModalFrame(
      {
        testId: "pending-player-pick-modal",
        title: messageText(pending.label),
        body: [h.div([h.Class("flex min-h-0 w-[min(92vw,28rem)] flex-wrap justify-center gap-2")], buttons)],
        actions: [],
      },
      h,
    );
  }

  if (pending.kind === "scry" || pending.kind === "surveil" || pending.kind === "reorder_top") {
    return arrangeLanesPrompt(pending, state, board, h);
  }

  if (pending.kind === "select_from_top") {
    return selectFromTopLanesPrompt(pending, state, board, h);
  }

  const items = "items" in pending ? pending.items : [];
  const config = cardPickConfig(pending);
  return cardPickPrompt(pending, items, state, board, config, h);
}

function selectFromTopLanesPrompt(
  pending: Extract<PendingChoiceView, { kind: "select_from_top" }>,
  state: VisibleState,
  board: BoardModel,
  h: HtmlBuilder<Message>,
): Html {
  const draft = board.promptDraft ?? initPromptDraft(pending, state);
  const picked = draft.kind === "card-pick" ? draft.picked : [];
  const byId = new Map(pending.items.map((it) => [it.id, it]));
  const takeItems = picked.flatMap((id) => {
    const item = byId.get(id);
    return item != null ? [item] : [];
  });
  const restItems = pending.items.filter((it) => !picked.includes(it.id));
  return promptModalFrame(
    {
      testId: "pending-select-top-modal",
      title: `Select up to ${pending.up_to} from the top`,
      body: [
        h.div(
          [
            h.DataAttribute("testid", "prompt-select-top-lanes"),
            h.Class("flex min-h-0 w-[min(92vw,720px)] flex-1 flex-col gap-3 overflow-y-auto overscroll-contain"),
          ],
          [
            h.div(
              [h.Class("shrink-0 text-caption text-mist")],
              ["Click a card to take it or put it back. Untaken cards go to the bottom."],
            ),
            h.div(
              [h.DataAttribute("testid", "prompt-select-top-take"), h.Class("flex flex-col gap-2")],
              [
                h.div(
                  [
                    h.DataAttribute("testid", "prompt-select-top-take-label"),
                    h.Class("text-caption font-semibold text-seafoam"),
                  ],
                  [`Take (${picked.length} / ${pending.up_to})`],
                ),
                h.div(
                  [h.Class("flex min-h-[100px] flex-wrap justify-center gap-2 rounded-panel bg-glass/40 p-2")],
                  takeItems.length > 0
                    ? takeItems.map((item) => arrangeLaneCard(item, state, picked, true, h))
                    : [h.div([h.Class("self-center text-caption text-mist")], ["None"])],
                ),
              ],
            ),
            h.div(
              [h.DataAttribute("testid", "prompt-select-top-rest"), h.Class("flex flex-col gap-2")],
              [
                h.div(
                  [
                    h.DataAttribute("testid", "prompt-select-top-rest-label"),
                    h.Class("text-caption font-semibold text-seafoam"),
                  ],
                  ["Bottom of library"],
                ),
                h.div(
                  [h.Class("flex min-h-[100px] flex-wrap justify-center gap-2 rounded-panel bg-glass/40 p-2")],
                  restItems.length > 0
                    ? restItems.map((item) => arrangeLaneCard(item, state, [], false, h))
                    : [h.div([h.Class("self-center text-caption text-mist")], ["None"])],
                ),
              ],
            ),
          ],
        ),
      ],
      actions: [submitButton("Done", false, h)],
    },
    h,
  );
}

function yesNoPrompt(
  pending: Extract<PendingChoiceView, { kind: "may_yes_no" | "dance_exile_more" }>,
  h: HtmlBuilder<Message>,
): Html {
  return h.div(
    [
      h.DataAttribute("testid", "pending-yes-no-aim"),
      h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
      h.Class(
        "pointer-events-auto fixed bottom-(--b) left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [
      h.div(
        [h.Class("pointer-events-none text-center font-semibold text-body text-snow")],
        [pendingChoiceTitle(pending)],
      ),
    ],
  );
}

function payCostPrompt(
  pending: Extract<
    PendingChoiceView,
    {
      kind:
        | "pay_cost"
        | "pay_or_counter"
        | "pay_or_controller_draws"
        | "pay_echo_or_sacrifice"
        | "pay_recover_or_exile"
        | "sacrifice_unless_pay"
        | "pay_life_or_enters_tapped";
    }
  >,
  board: BoardModel,
  _tableId: string | null,
  h: HtmlBuilder<Message>,
): Html {
  // The shockland choice carries a life amount rather than a cost, and no server label.
  const shockland = pending.kind === "pay_life_or_enters_tapped";
  const title = shockland
    ? "Have it enter untapped?"
    : "label" in pending
      ? messageText(pending.label)
      : pendingChoiceTitle(pending);
  const discardNeed = pending.kind === "pay_cost" ? (pending.discard_count ?? 0) : 0;
  const draft = board.promptDraft;
  const picked = discardNeed > 0 && draft?.kind === "card-pick" ? draft.picked : [];
  const countLine =
    discardNeed > 0
      ? h.div(
          [
            h.DataAttribute("testid", "pending-pay-discard-count"),
            h.Class("pointer-events-none text-caption text-mist"),
          ],
          [`${picked.length} / ${discardNeed} selected`],
        )
      : null;
  return h.div(
    [
      h.DataAttribute("testid", "pending-pay-cost-aim"),
      h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
      h.Class(
        "pointer-events-auto fixed bottom-(--b) left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [h.div([h.Class("pointer-events-none text-center font-semibold text-body text-snow")], [title]), countLine].filter(
      (v): v is Html => v !== null,
    ),
  );
}

function modeListPrompt(
  pending: Extract<PendingChoiceView, { kind: "choose_mode" | "choose_trigger_modes" }>,
  board: BoardModel,
  state: VisibleState,
  tableId: string | null,
  h: HtmlBuilder<Message>,
): Html {
  if (pending.kind === "choose_mode") {
    return h.div(
      [
        h.DataAttribute("testid", "pending-mode-aim"),
        h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
        h.Class(
          "pointer-events-auto fixed bottom-(--b) left-1/2 z-30 flex max-w-[min(100%-2rem,28rem)] -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
        ),
      ],
      [h.div([h.Class("pointer-events-none text-center font-semibold text-body text-snow")], ["Choose a mode"])],
    );
  }

  const draft = board.promptDraft ?? initPromptDraft(pending, state);
  const picked = draft.kind === "modes" ? draft.modes : [];
  const concreteChoices: Array<{ choice: WireModeChoice; label: string }> = pending.modes.flatMap((mode, index) => {
    if (!mode.needs_target) {
      return [{ choice: { index } satisfies WireModeChoice, label: messageText(mode.label) }];
    }
    return mode.targets.map((target) => ({
      choice: { index, target } satisfies WireModeChoice,
      label: `${messageText(mode.label)} — ${targetLabel(target, state)}`,
    }));
  });
  return h.div(
    [
      h.DataAttribute("testid", "pending-trigger-modes-aim"),
      h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
      h.Class(
        "pointer-events-auto fixed bottom-(--b) left-1/2 z-30 flex max-w-[min(100%-2rem,28rem)] -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [
      h.div([h.Class("pointer-events-none text-center font-semibold text-body text-snow")], ["Choose trigger modes"]),
      h.div(
        [h.Class("pointer-events-none text-caption text-mist")],
        [pending.optional ? `Choose ${pending.choose} or none` : `Choose ${pending.choose}`],
      ),
      h.div(
        [h.Class("flex w-full flex-col gap-2")],
        concreteChoices.map(({ choice, label }, choiceIndex) => {
          const selected = picked.some((pickedChoice) => sameModeChoice(pickedChoice, choice));
          return h.button(
            [
              h.Type("button"),
              h.DataAttribute("testid", `prompt-mode-choice-${choiceIndex}`),
              h.DataAttribute("selected", selected ? "true" : "false"),
              h.AriaPressed(selected ? "true" : "false"),
              h.Disabled(tableId == null),
              h.OnClick(PromptModeChoiceToggled({ index: choice.index, target: choice.target ?? null })),
              h.Class(
                [
                  "group/prompt-mode rounded-hud bg-glass px-3 py-2 text-left text-body text-snow",
                  "data-[selected=true]:bg-llanowar/25",
                  tableId == null ? "cursor-not-allowed opacity-50" : "hover:bg-glass-dim",
                ].join(" "),
              ),
            ],
            [label],
          );
        }),
      ),
    ],
  );
}

function playerPickPrompt(
  pending: Extract<PendingChoiceView, { kind: "choose_target_players" | "choose_splitting_opponent" }>,
  state: VisibleState,
  board: BoardModel,
  tableId: string | null,
  h: HtmlBuilder<Message>,
): Html {
  if (pendingPlayerAimSeats(pending, state) != null) {
    const oneClick = pendingPlayerAimOneClick(pending);
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    const picked = draft.kind === "player-pick" ? draft.players : [];
    const max = pending.kind === "choose_target_players" ? pending.max : 1;
    const countLine =
      !oneClick && pending.kind === "choose_target_players"
        ? h.div(
            [h.DataAttribute("testid", "pending-player-count"), h.Class("pointer-events-none text-caption text-mist")],
            [`${picked.length} / ${max} selected`],
          )
        : null;
    return h.div(
      [
        h.DataAttribute("testid", "pending-player-aim"),
        h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
        h.Class(
          "pointer-events-none fixed bottom-(--b) left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
        ),
      ],
      [h.div([h.Class("pointer-events-none")], [messageText(pending.label)]), countLine].filter(
        (v): v is Html => v !== null,
      ),
    );
  }

  if (pending.kind === "choose_splitting_opponent") {
    return promptModalFrame(
      {
        testId: "pending-player-pick-modal",
        title: messageText(pending.label),
        body: [
          h.div(
            [h.Class("flex min-h-0 w-[min(92vw,28rem)] flex-wrap justify-center gap-2")],
            pending.items.flatMap((item, index) => {
              const seat = playerSeatFromItem(item, state, index);
              if (seat == null) return [];
              return [
                answerButton(
                  pending,
                  `prompt-player-${seat}`,
                  item.label,
                  { kind: "target", id: item.id, player: seat },
                  false,
                  tableId == null,
                  h,
                ),
              ];
            }),
          ),
        ],
        actions: [],
      },
      h,
    );
  }

  const draft = board.promptDraft ?? initPromptDraft(pending, state);
  const picked = draft.kind === "player-pick" ? draft.players : [];
  const ready = picked.length >= pending.min && picked.length <= pending.max;
  return promptModalFrame(
    {
      testId: "pending-player-pick-modal",
      title: messageText(pending.label),
      body: [
        h.div([h.Class("pointer-events-none text-caption text-mist")], [`${picked.length} / ${pending.max} selected`]),
        h.div(
          [h.Class("flex min-h-0 w-[min(92vw,28rem)] flex-wrap justify-center gap-2")],
          pending.items.flatMap((item, index) => {
            const seat = playerSeatFromItem(item, state, index);
            if (seat == null) return [];
            const selected = picked.includes(seat);
            return [
              h.button(
                [
                  h.Type("button"),
                  h.DataAttribute("testid", `prompt-player-${seat}`),
                  h.DataAttribute("selected", selected ? "true" : "false"),
                  h.AriaPressed(selected ? "true" : "false"),
                  h.Disabled(tableId == null),
                  h.OnClick(PromptCardToggled({ id: seat })),
                  h.Class(
                    [
                      "group/prompt-player rounded-hud bg-glass px-3 py-2 text-body text-snow",
                      "data-[selected=true]:bg-llanowar/25",
                      tableId == null ? "cursor-not-allowed opacity-50" : "hover:bg-glass-dim",
                    ].join(" "),
                  ),
                ],
                [item.label],
              ),
            ];
          }),
        ),
      ],
      actions: [submitButton("Choose", !ready, h), cancelButton(h)],
    },
    h,
  );
}

function divideTotalPrompt(
  pending: Extract<PendingChoiceView, { kind: "divide_spell_damage" | "divide_counters" }>,
  board: BoardModel,
  state: VisibleState,
  h: HtmlBuilder<Message>,
): Html {
  const draft = board.promptDraft ?? initPromptDraft(pending, state);
  const ready = buildAnswerFromDraft(pending, draft) != null;
  if (pending.kind === "divide_spell_damage") {
    const amounts = draft.kind === "divide" ? draft.amounts : {};
    const assigned = Object.values(amounts).reduce((sum, amount) => sum + amount, 0);
    const onBoard = pendingDivideSpellObjectIndexes(pending, state) != null;
    const rows = onBoard
      ? []
      : pending.items.map((item, index) =>
          h.div(
            [h.Class("flex items-center gap-2")],
            [
              h.span([h.Class("w-44 truncate text-body")], [item.label]),
              amountStepper(index, amounts[index] ?? 0, pending.total, h),
            ],
          ),
        );
    return h.div(
      [
        h.DataAttribute("testid", "pending-divide-aim"),
        h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
        h.Class(
          [
            "fixed bottom-(--b) left-1/2 z-30 flex max-w-[min(100%-2rem,28rem)] -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
            onBoard ? "pointer-events-none" : "pointer-events-auto",
          ].join(" "),
        ),
      ],
      [
        h.div(
          [h.Class("pointer-events-none text-center font-semibold text-body text-snow")],
          [`Divide ${pending.total} damage`],
        ),
        onBoard
          ? h.div(
              [h.Class("pointer-events-none text-center text-body text-mist")],
              ["Click a target on the board to move 1 damage onto it"],
            )
          : null,
        ...rows,
        h.div(
          [
            h.DataAttribute("testid", "prompt-damage-assigned"),
            h.Class(assigned === pending.total ? "text-assign-clover" : "text-caution-amber"),
          ],
          [`assigned ${assigned} / ${pending.total}`],
        ),
        onBoard ? null : submitButton("Assign", !ready, h),
      ].filter((v): v is Html => v !== null),
    );
  }

  const amounts = draft.kind === "damage" ? draft.amounts : {};
  const assigned = Object.values(amounts).reduce((sum, amount) => sum + amount, 0);
  const onBoard = pendingDamageAssignBlockers(pending, state) != null;
  const rows = onBoard
    ? []
    : pending.items.map((item) =>
        h.div(
          [h.Class("flex items-center gap-2")],
          [
            h.span([h.Class("w-44 truncate text-body")], [item.label]),
            amountStepper(item.id, amounts[item.id] ?? 0, pending.total, h),
          ],
        ),
      );
  return h.div(
    [
      h.DataAttribute("testid", "pending-divide-counters-aim"),
      h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
      h.Class(
        [
          "fixed bottom-(--b) left-1/2 z-30 flex max-w-[min(100%-2rem,28rem)] -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
          onBoard ? "pointer-events-none" : "pointer-events-auto",
        ].join(" "),
      ),
    ],
    [
      h.div(
        [h.Class("pointer-events-none text-center font-semibold text-body text-snow")],
        [`Divide ${pending.total} counters`],
      ),
      onBoard
        ? h.div(
            [h.Class("pointer-events-none text-center text-body text-mist")],
            ["Click a permanent on the board to move 1 counter onto it"],
          )
        : null,
      ...rows,
      h.div(
        [
          h.DataAttribute("testid", "prompt-damage-assigned"),
          h.Class(assigned === pending.total ? "text-assign-clover" : "text-caution-amber"),
        ],
        [`assigned ${assigned} / ${pending.total}`],
      ),
      onBoard ? null : submitButton("Assign", !ready, h),
    ].filter((v): v is Html => v !== null),
  );
}

function pilePickPrompt(
  pending: Extract<PendingChoiceView, { kind: "opponent_chooses_pile" | "choose_pile_for_hand" }>,
  _tableId: string | null,
  h: HtmlBuilder<Message>,
): Html {
  // Raging River names the attacker each pick is about; every other pile choice is A/B.
  const attacker = pending.kind === "choose_pile_for_hand" ? pending.attacker : undefined;
  const pileBlock = (title: string, items: ReadonlyArray<ChoiceItem>): Html =>
    h.div(
      [h.Class("min-w-[180px] flex-1 rounded-panel bg-glass p-3")],
      [
        h.div([h.Class("mb-2 font-semibold text-body text-snow")], [title]),
        h.div(
          [h.Class("flex flex-col gap-1 text-caption text-mist")],
          items.map((item) => h.span([], [item.label])),
        ),
      ],
    );
  return h.div(
    [
      h.DataAttribute("testid", "pending-pile-aim"),
      h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
      h.Class(
        "pointer-events-auto fixed bottom-(--b) left-1/2 z-30 flex max-w-[min(100%-2rem,40rem)] -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [
      h.div(
        [
          h.DataAttribute("testid", "prompt-pile-heading"),
          h.Class("pointer-events-none text-center font-semibold text-body text-snow"),
        ],
        // Raging River labels each attacker in turn, so the pick is about that creature.
        [attacker == null ? "Choose a pile" : `Send ${attacker.label} left or right`],
      ),
      h.div(
        [h.Class("flex w-full flex-wrap justify-center gap-3")],
        attacker == null
          ? [pileBlock("Pile A", pending.pile_a), pileBlock("Pile B", pending.pile_b)]
          : [pileBlock("Left", pending.pile_a), pileBlock("Right", pending.pile_b)],
      ),
    ],
  );
}

function partitionPrompt(
  pending: Extract<PendingChoiceView, { kind: "partition_revealed" | "distribute_top" }>,
  board: BoardModel,
  state: VisibleState,
  tableId: string | null,
  h: HtmlBuilder<Message>,
): Html {
  const draft = board.promptDraft ?? initPromptDraft(pending, state);

  if (pending.kind === "partition_revealed") {
    const draftBuckets = draft.kind === "partition" ? draft.buckets : { pile_a: [] as number[] };
    const pileAIds = draftBuckets.pile_a ?? [];
    const byId = new Map(pending.items.map((it) => [it.id, it]));
    const pileAItems = pileAIds.flatMap((id) => {
      const item = byId.get(id);
      return item != null ? [item] : [];
    });
    const pileBItems = pending.items.filter((it) => !pileAIds.includes(it.id));
    return promptModalFrame(
      {
        testId: "pending-partition-modal",
        // Raging River and Camouflage divide creatures on the battlefield, not revealed cards.
        title: pending.into_piles === true ? "Choose creatures for this pile" : "Choose cards for Pile A",
        body: [
          h.div(
            [
              h.DataAttribute("testid", "prompt-partition-lanes"),
              h.Class("flex min-h-0 w-[min(92vw,720px)] flex-1 flex-col gap-3 overflow-y-auto overscroll-contain"),
            ],
            [
              h.div(
                [h.Class("shrink-0 text-caption text-mist")],
                ["Click a card to move it between Pile A and Pile B."],
              ),
              h.div(
                [h.DataAttribute("testid", "prompt-partition-a"), h.Class("flex flex-col gap-2")],
                [
                  h.div(
                    [
                      h.DataAttribute("testid", "prompt-partition-a-label"),
                      h.Class("text-caption font-semibold text-seafoam"),
                    ],
                    [`Pile A (${pileAIds.length})`],
                  ),
                  h.div(
                    [h.Class("flex min-h-[100px] flex-wrap justify-center gap-2 rounded-panel bg-glass/40 p-2")],
                    pileAItems.length > 0
                      ? pileAItems.map((item) => arrangeLaneCard(item, state, pileAIds, false, h))
                      : [h.div([h.Class("self-center text-caption text-mist")], ["None"])],
                  ),
                ],
              ),
              h.div(
                [h.DataAttribute("testid", "prompt-partition-b"), h.Class("flex flex-col gap-2")],
                [
                  h.div(
                    [
                      h.DataAttribute("testid", "prompt-partition-b-label"),
                      h.Class("text-caption font-semibold text-seafoam"),
                    ],
                    [`Pile B (${pileBItems.length})`],
                  ),
                  h.div(
                    [h.Class("flex min-h-[100px] flex-wrap justify-center gap-2 rounded-panel bg-glass/40 p-2")],
                    pileBItems.length > 0
                      ? pileBItems.map((item) => arrangeLaneCard(item, state, [], false, h))
                      : [h.div([h.Class("self-center text-caption text-mist")], ["None"])],
                  ),
                ],
              ),
            ],
          ),
        ],
        actions: [submitButton("Lock piles", false, h), cancelButton(h)],
      },
      h,
    );
  }

  return distributeTopLanesPrompt(pending, board, state, tableId, h);
}

function distributeTopLanesPrompt(
  pending: Extract<PendingChoiceView, { kind: "distribute_top" }>,
  board: BoardModel,
  state: VisibleState,
  tableId: string | null,
  h: HtmlBuilder<Message>,
): Html {
  const draft = board.promptDraft ?? initPromptDraft(pending, state);
  const buckets = draft.kind === "partition" ? draft.buckets : {};
  const toHand = buckets.to_hand ?? [];
  const toBottom = buckets.to_bottom ?? [];
  const toExile = buckets.to_exile_may_play ?? [];
  const assigned = new Set([...toHand, ...toBottom, ...toExile]);
  const pool = pending.items.filter((it) => !assigned.has(it.id));
  const byId = new Map(pending.items.map((it) => [it.id, it]));
  const caps = {
    to_hand: pending.to_hand,
    to_bottom: pending.to_bottom,
    to_exile_may_play: pending.to_exile_may_play,
  };
  const counts = {
    to_hand: toHand.length,
    to_bottom: toBottom.length,
    to_exile_may_play: toExile.length,
  };
  const ready =
    toHand.length === pending.to_hand &&
    toBottom.length === pending.to_bottom &&
    toExile.length === pending.to_exile_may_play &&
    toHand.length + toBottom.length + toExile.length === pending.items.length;

  const currentBucket = (id: number): DistributeBucket | null => {
    if (toHand.includes(id)) return "to_hand";
    if (toBottom.includes(id)) return "to_bottom";
    if (toExile.includes(id)) return "to_exile_may_play";
    return null;
  };

  const laneCard = (item: (typeof pending.items)[number]): Html => {
    const current = currentBucket(item.id);
    const next = nextDistributeBucket(current, counts, caps);
    const clickBucket = next ?? current;
    const print = choiceItemPrint(item, state);
    const face = promptCardFace(h, { print, label: item.label, size: "sm" });
    if (clickBucket == null || tableId == null) {
      return h.div(
        [
          h.DataAttribute("testid", `prompt-card-${item.id}`),
          h.Class("relative rounded-[9px] border-4 border-transparent p-0"),
        ],
        [face],
      );
    }
    return h.button(
      [
        h.Type("button"),
        h.DataAttribute("testid", `prompt-card-${item.id}`),
        h.AriaLabel(item.label),
        h.OnClick(PromptPartitionSet({ id: item.id, bucket: clickBucket })),
        h.Class(
          "relative cursor-pointer rounded-[9px] border-4 border-transparent p-0 transition-transform duration-150 ease-out hover:-translate-y-1",
        ),
      ],
      [face],
    );
  };

  const lane = (testId: string, label: string, ids: readonly number[], cap: number): Html => {
    const items = ids.flatMap((id) => {
      const item = byId.get(id);
      return item != null ? [item] : [];
    });
    return h.div(
      [h.DataAttribute("testid", testId), h.Class("flex flex-col gap-2")],
      [
        h.div([h.Class("text-caption font-semibold text-seafoam")], [`${label} (${ids.length} / ${cap})`]),
        h.div(
          [h.Class("flex min-h-[100px] flex-wrap justify-center gap-2 rounded-panel bg-glass/40 p-2")],
          items.length > 0
            ? items.map((item) => laneCard(item))
            : [h.div([h.Class("self-center text-caption text-mist")], ["None"])],
        ),
      ],
    );
  };

  return promptModalFrame(
    {
      testId: "pending-distribute-modal",
      title: "Distribute the revealed cards",
      body: [
        h.div(
          [
            h.DataAttribute("testid", "prompt-distribute-lanes"),
            h.Class("flex min-h-0 w-[min(92vw,720px)] flex-1 flex-col gap-3 overflow-y-auto overscroll-contain"),
          ],
          [
            h.div(
              [h.Class("shrink-0 text-caption text-mist")],
              ["Click a card to cycle Hand → Bottom → Exile (skips full lanes)."],
            ),
            h.div(
              [h.DataAttribute("testid", "prompt-distribute-pool"), h.Class("flex flex-col gap-2")],
              [
                h.div([h.Class("text-caption font-semibold text-seafoam")], [`Revealed (${pool.length})`]),
                h.div(
                  [h.Class("flex min-h-[100px] flex-wrap justify-center gap-2 rounded-panel bg-glass/40 p-2")],
                  pool.length > 0
                    ? pool.map((item) => laneCard(item))
                    : [h.div([h.Class("self-center text-caption text-mist")], ["None"])],
                ),
              ],
            ),
            lane("prompt-distribute-hand", "Hand", toHand, pending.to_hand),
            lane("prompt-distribute-bottom", "Bottom of library", toBottom, pending.to_bottom),
            lane("prompt-distribute-exile", "Exile (may play)", toExile, pending.to_exile_may_play),
          ],
        ),
      ],
      actions: [submitButton("Distribute", !ready, h), cancelButton(h)],
    },
    h,
  );
}

function colorPickPrompt(
  pending: Extract<PendingChoiceView, { kind: "choose_color" | "choose_mana_color" }>,
  tableId: string | null,
  h: HtmlBuilder<Message>,
): Html {
  const colors = [
    { index: 0, code: "W", name: "White" },
    { index: 1, code: "U", name: "Blue" },
    { index: 2, code: "B", name: "Black" },
    { index: 3, code: "R", name: "Red" },
    { index: 4, code: "G", name: "Green" },
  ] as const;
  const sizePx = 28;
  const title = pending.kind === "choose_mana_color" ? "Choose a mana color" : "Choose a color";
  return promptModalFrame(
    {
      testId: "pending-color-modal",
      title,
      body: [
        h.div(
          [h.Class("flex flex-wrap items-center justify-center gap-2")],
          colors.map((color) => {
            const ms = manaFontClass(color.code) ?? color.code.toLowerCase();
            return h.button(
              [
                h.Type("button"),
                h.DataAttribute("testid", `prompt-color-${color.index}`),
                h.AriaLabel(color.name),
                h.Disabled(tableId == null),
                h.OnClick(
                  PendingChoiceAnswered({
                    intent: choiceIntent(
                      pending,
                      pending.kind === "choose_mana_color"
                        ? { kind: "mana_color", color: color.index }
                        : { kind: "color", color: color.index },
                    ),
                  }),
                ),
                h.Class(
                  "group relative cursor-pointer rounded-hud border-0 bg-transparent p-1 disabled:cursor-not-allowed disabled:opacity-50",
                ),
              ],
              [
                pipChip(h, {
                  ms,
                  code: color.code,
                  sizePx,
                  extraClass: "transition-transform duration-150 ease-out group-hover:-translate-y-1",
                  testId: `prompt-color-pip-${color.index}`,
                }),
              ],
            );
          }),
        ),
      ],
      actions: [],
    },
    h,
  );
}

function stringPickPrompt(
  pending: Extract<PendingChoiceView, { kind: "choose_creature_type" | "choose_card_name" }>,
  board: BoardModel,
  state: VisibleState,
  tableId: string | null,
  h: HtmlBuilder<Message>,
): Html {
  if (pending.kind === "choose_card_name") {
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    const value = draft.kind === "string" ? draft.value : "";
    const canSubmit = value.trim() !== "" && tableId != null;
    const suggestions =
      board.cardNameSuggestions != null &&
      board.cardNameSuggestions.query.trim() === value.trim() &&
      board.cardNameSuggestions.names.length > 0
        ? board.cardNameSuggestions.names
        : [];
    return promptModalFrame(
      {
        testId: "pending-card-name-modal",
        title: "Name a card",
        body: [
          h.div(
            [h.Class("flex min-h-0 w-[min(92vw,360px)] flex-1 flex-col gap-sm")],
            [
              // Combobox owns the input text, opens the list as you type, and moves the active row
              // on arrow keys; Enter commits the highlighted name into the draft. It renders the
              // panel only when it has items, and anchors and portals it, so `z-50` is what keeps
              // it over the `z-40` prompt frame.
              h.submodel({
                slotId: CARD_NAME_COMBOBOX_ID,
                model: board.cardNameCombobox,
                view: CardNameCombobox.view,
                viewInputs: {
                  items: suggestions,
                  maybeSelectedValue: value === "" ? Option.none() : Option.some(value),
                  restingInputValue: value,
                  itemToValue: (name: string) => name,
                  itemToDisplayText: (name: string) => name,
                  itemToConfig: (name: string, context: { isActive: boolean }) => ({
                    className: menuItemClass(context.isActive ? "bg-white/8" : undefined, "hud"),
                    content: h.span(
                      [h.DataAttribute("testid", `prompt-name-suggestion-${suggestions.indexOf(name)}`)],
                      [name],
                    ),
                  }),
                  ariaLabel: "Card name",
                  inputPlaceholder: "Card name",
                  inputClassName: inputClass("w-full", "hud"),
                  inputAttributes: childAttributes([h.DataAttribute("testid", "prompt-name-input"), h.Autofocus(true)]),
                  inputWrapperClassName: "w-full",
                  itemsClassName: menuPanelClass("z-50 max-h-[40vh] w-(--button-width) gap-1", "hud"),
                  itemsAttributes: childAttributes([h.DataAttribute("testid", "prompt-name-suggestions")]),
                  itemsScrollClassName: "min-h-0 overflow-y-auto",
                  // Locks the resolved side so the suggestion list doesn't flip above/below the
                  // input as filtering changes its height on every keystroke (@foldkit/ui 0.137).
                  anchor: { placement: "bottom-start" as const, gap: 4, isPlacementLocked: true },
                },
                toParentMessage: (message) => GotCardNameComboboxMessage({ message }),
              }) as Html,
            ],
          ),
        ],
        actions: [submitButton("Name", !canSubmit, h)],
      },
      h,
    );
  }
  return promptModalFrame(
    {
      testId: "pending-creature-type-modal",
      title: "Choose a creature type",
      body: [
        h.div(
          [h.Class("flex min-h-0 w-[min(92vw,360px)] flex-1 flex-col gap-sm")],
          [
            input(h, {
              id: "prompt-type-filter",
              variant: "hud",
              testId: "prompt-type-filter",
              type: "search",
              placeholder: "Filter types…",
              autofocus: true,
              ariaLabel: "Filter creature types",
              value: board.promptOptionFilter,
              onInput: (v) => PromptOptionFilterSet({ query: v }),
              class: "w-full",
            }),
            h.div(
              [
                h.DataAttribute("testid", "prompt-type-scroll"),
                h.Class("min-h-0 w-full flex-1 overflow-y-auto overscroll-contain"),
              ],
              [
                h.div(
                  [h.Class("flex flex-wrap justify-center gap-2")],
                  (() => {
                    const shown = filterOptionLabels(pending.options, board.promptOptionFilter);
                    if (shown.length === 0 && board.promptOptionFilter.trim() !== "") {
                      return [h.div([h.Class("text-label text-mist")], ["No types match."])];
                    }
                    return shown.map((option) => {
                      const index = pending.options.indexOf(option);
                      return answerButton(
                        pending,
                        `prompt-string-${index}`,
                        option,
                        { kind: "creature_type", subtype: option },
                        false,
                        tableId == null,
                        h,
                      );
                    });
                  })(),
                ),
              ],
            ),
          ],
        ),
      ],
      actions: [],
    },
    h,
  );
}

function numberPickTitle(
  pending: Extract<PendingChoiceView, { kind: "may_draw_up_to" | "pay_any_amount_of_mana" }>,
): string {
  if (pending.kind === "pay_any_amount_of_mana") return `Pay any amount of mana (up to ${pending.max})`;
  return messageText(pending.label);
}

function numberPickPrompt(
  pending: Extract<PendingChoiceView, { kind: "may_draw_up_to" | "pay_any_amount_of_mana" }>,
  board: BoardModel,
  state: VisibleState,
  tableId: string | null,
  h: HtmlBuilder<Message>,
): Html {
  if (pending.kind === "pay_any_amount_of_mana") {
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    const max = pending.max;
    const count = clampX(draft.kind === "number" ? draft.count : 0, 0, max);
    return promptModalFrame(
      {
        testId: "pending-join-forces-modal",
        title: numberPickTitle(pending),
        body: [
          h.div(
            [h.Class("flex min-h-0 w-[min(92vw,28rem)] flex-wrap items-center justify-center gap-2")],
            [
              itemButton("Min", "prompt-number-min", PromptNumberSet({ count: 0 }), h),
              itemButton("−", "prompt-number-dec", PromptNumberSet({ count: count - 1 }), h, count <= 0),
              h.span(
                [
                  h.DataAttribute("testid", "prompt-number-value"),
                  h.Class("min-w-[2ch] text-center text-body font-semibold text-snow"),
                ],
                [String(count)],
              ),
              itemButton("+", "prompt-number-inc", PromptNumberSet({ count: count + 1 }), h, count >= max),
              itemButton("Max", "prompt-number-max", PromptNumberSet({ count: max }), h),
            ],
          ),
        ],
        actions: [submitButton(count === 0 ? "Pay 0 (decline)" : `Pay {${count}}`, tableId == null, h)],
      },
      h,
    );
  }
  const answerFor = (count: number): AnswerInput => ({ kind: "draw_count", count });
  return promptModalFrame(
    {
      testId: "pending-draw-count-modal",
      title: numberPickTitle(pending),
      body: [
        h.div(
          [h.Class("flex min-h-0 w-[min(92vw,28rem)] flex-wrap justify-center gap-2")],
          Array.from({ length: pending.max + 1 }, (_, count) =>
            answerButton(
              pending,
              `prompt-number-${count}`,
              String(count),
              answerFor(count),
              count === pending.max,
              tableId == null,
              h,
            ),
          ),
        ),
      ],
      actions: [],
    },
    h,
  );
}

function destinationPickPrompt(
  pending: Extract<
    PendingChoiceView,
    { kind: "choose_countered_spell_destination" | "revealed_card_to_battlefield_or_hand" }
  >,
  state: VisibleState,
  _tableId: string | null,
  h: HtmlBuilder<Message>,
): Html {
  if (pending.kind === "choose_countered_spell_destination") {
    return h.div(
      [
        h.DataAttribute("testid", "pending-destination-aim"),
        h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
        h.Class(
          "pointer-events-auto fixed bottom-(--b) left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
        ),
      ],
      [
        h.div(
          [h.Class("pointer-events-none text-center font-semibold text-body text-snow")],
          ["Put the countered spell on top or bottom?"],
        ),
      ],
    );
  }
  const print = choiceItemPrint(pending.item, state);
  const faceEl = promptCardFace(h, { print, label: pending.item.label, size: "sm", testId: "prompt-revealed-face" });
  return h.div(
    [
      h.DataAttribute("testid", "pending-revealed-destination-aim"),
      h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
      h.Class(
        "pointer-events-auto fixed bottom-(--b) left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [
      h.div(
        [h.Class("pointer-events-none text-center font-semibold text-body text-snow")],
        ["Put the revealed card onto the battlefield or into your hand?"],
      ),
      faceEl,
    ],
  );
}

function pendingChoicePrompt(
  pending: PendingChoiceView,
  state: VisibleState,
  board: BoardModel,
  tableId: string | null,
  h: HtmlBuilder<Message>,
): Html | null {
  const id = FORMULATOR_FOR_KIND[pending.kind];
  switch (id) {
    case "cardPick":
      return cardPickForKind(pending, state, board, tableId, h);
    case "orderTriggers":
      if (pending.kind !== "order_triggers") return frame("pending-choice", pendingChoiceTitle(pending), [], h);
      return orderPrompt(pending, board, h);
    case "damageAssign":
      if (pending.kind !== "assign_combat_damage") return frame("pending-choice", pendingChoiceTitle(pending), [], h);
      return damageAssignPrompt(pending, state, board, h);
    case "yesNo":
      if (pending.kind !== "may_yes_no" && pending.kind !== "dance_exile_more") {
        return frame("pending-choice", pendingChoiceTitle(pending), [], h);
      }
      return yesNoPrompt(pending, h);
    case "payCost":
      if (
        pending.kind !== "pay_cost" &&
        pending.kind !== "pay_or_counter" &&
        pending.kind !== "pay_or_controller_draws" &&
        pending.kind !== "pay_echo_or_sacrifice" &&
        pending.kind !== "pay_recover_or_exile" &&
        pending.kind !== "sacrifice_unless_pay" &&
        pending.kind !== "pay_life_or_enters_tapped"
      ) {
        return frame("pending-choice", pendingChoiceTitle(pending), [], h);
      }
      return payCostPrompt(pending, board, tableId, h);
    case "modeList":
      if (pending.kind !== "choose_mode" && pending.kind !== "choose_trigger_modes") {
        return frame("pending-choice", pendingChoiceTitle(pending), [], h);
      }
      return modeListPrompt(pending, board, state, tableId, h);
    case "playerPick":
      if (pending.kind !== "choose_target_players" && pending.kind !== "choose_splitting_opponent") {
        return frame("pending-choice", pendingChoiceTitle(pending), [], h);
      }
      return playerPickPrompt(pending, state, board, tableId, h);
    case "divideTotal":
      if (pending.kind !== "divide_spell_damage" && pending.kind !== "divide_counters") {
        return frame("pending-choice", pendingChoiceTitle(pending), [], h);
      }
      return divideTotalPrompt(pending, board, state, h);
    case "pilePick":
      if (pending.kind !== "opponent_chooses_pile" && pending.kind !== "choose_pile_for_hand") {
        return frame("pending-choice", pendingChoiceTitle(pending), [], h);
      }
      return pilePickPrompt(pending, tableId, h);
    case "partition":
      if (pending.kind !== "partition_revealed" && pending.kind !== "distribute_top") {
        return frame("pending-choice", pendingChoiceTitle(pending), [], h);
      }
      return partitionPrompt(pending, board, state, tableId, h);
    case "colorPick":
      if (pending.kind !== "choose_color" && pending.kind !== "choose_mana_color") {
        return frame("pending-choice", pendingChoiceTitle(pending), [], h);
      }
      return colorPickPrompt(pending, tableId, h);
    case "stringPick":
      if (pending.kind !== "choose_creature_type" && pending.kind !== "choose_card_name") {
        return frame("pending-choice", pendingChoiceTitle(pending), [], h);
      }
      return stringPickPrompt(pending, board, state, tableId, h);
    case "numberPick":
      if (pending.kind !== "may_draw_up_to" && pending.kind !== "pay_any_amount_of_mana") {
        return frame("pending-choice", pendingChoiceTitle(pending), [], h);
      }
      return numberPickPrompt(pending, board, state, tableId, h);
    case "destinationPick":
      if (
        pending.kind !== "choose_countered_spell_destination" &&
        pending.kind !== "revealed_card_to_battlefield_or_hand"
      ) {
        return frame("pending-choice", pendingChoiceTitle(pending), [], h);
      }
      return destinationPickPrompt(pending, state, tableId, h);
    default: {
      const _exhaustive: never = id;
      return _exhaustive;
    }
  }
}

function shouldShowPendingChoice(state: VisibleState): boolean {
  const pending = state.pending_choice;
  if (pending == null) return false;
  if (!isActivePlayer(state.players, state.viewer)) return false;
  return pending.player === state.viewer;
}

export function promptsView(
  board: BoardModel,
  state: VisibleState,
  tableId: string | null,
  h: HtmlBuilder<Message>,
): Html | null {
  if (board.playModePick != null) return playModePrompt(board.playModePick, h);
  if (board.xPrompt != null) return boardXPrompt(board.xPrompt, h);
  if (board.modalCast != null) return modalPrompt(board.modalCast, h);
  if (board.sacrificePick != null) {
    const choices = board.sacrificePick.action.sacrifice_choices ?? [];
    if (sacrificeCostObjectIds(choices, state) != null) {
      return h.div(
        [
          h.DataAttribute("testid", "sacrifice-cost-aim"),
          h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
          h.Class(
            "pointer-events-none fixed bottom-(--b) left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
          ),
        ],
        [h.div([h.Class("pointer-events-none")], ["Click a permanent to sacrifice"])],
      );
    }
    return costPickPrompt(
      "sacrifice-pick",
      "Choose a permanent to sacrifice",
      choices,
      state,
      (id) => SacrificeChosen({ objectId: id }),
      h,
      "modal",
    );
  }
  if (board.discardPick != null) {
    const choices = board.discardPick.action.discard_choices ?? [];
    const handIds = new Set(
      state.objects.filter((o) => o.zone === ZONE.Hand && o.owner === state.viewer).map((o) => o.id),
    );
    const onHand = choices.length > 0 && choices.every((id) => handIds.has(id));
    if (onHand) {
      const selected = board.discardPick.picks.discard_cost;
      return h.div(
        [
          h.DataAttribute("testid", "discard-cost-aim"),
          h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
          h.Class(
            "pointer-events-none fixed bottom-(--b) left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
          ),
        ],
        [
          h.div([h.Class("pointer-events-none")], ["Click a card in your hand to discard"]),
          h.div(
            [h.DataAttribute("testid", "discard-cost-count"), h.Class("pointer-events-none text-caption text-mist")],
            [`${selected.length} / 1 selected`],
          ),
        ],
      );
    }
    return costPickPrompt(
      "discard-pick",
      "Choose a card to discard",
      choices,
      state,
      (id) => DiscardChosen({ ids: [id] }),
      h,
      "modal",
    );
  }
  if (board.gyExilePick != null) {
    const choices = board.gyExilePick.action.graveyard_exile_choices ?? [];
    const onPile = gyExileCostObjectIds(choices, state) != null;
    if (onPile) {
      const max = board.gyExilePick.action.graveyard_exile_max ?? 0;
      const selected = board.gyExilePick.picks.graveyard_exile;
      const oneClick = max <= 1;
      const countLine = !oneClick
        ? h.div(
            [h.DataAttribute("testid", "gy-exile-cost-count"), h.Class("pointer-events-none text-caption text-mist")],
            [`${selected.length} / ${max} selected`],
          )
        : null;
      return h.div(
        [
          h.DataAttribute("testid", "gy-exile-cost-aim"),
          h.Style({ "--b": `calc(var(--hand-bar-h) + 12px)` }),
          h.Class(
            "pointer-events-none fixed bottom-(--b) left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
          ),
        ],
        [
          h.div(
            [h.Class("pointer-events-none")],
            [oneClick ? "Click a card in the graveyard to exile" : "Click cards in the graveyard to exile"],
          ),
          countLine,
        ].filter((v): v is Html => v !== null),
      );
    }
    return costPickPrompt(
      "gy-exile-pick",
      "Choose cards to exile from graveyard",
      choices,
      state,
      (id) => GyExileChosen({ ids: [id] }),
      h,
    );
  }
  if (board.staged != null) {
    const targets = stagedPickTargets(board.staged, state);
    if (targets != null) {
      return targetPickPrompt(stagedTargetTitle(board.staged), targets, state, h);
    }
  }
  const pending = state.pending_choice;
  if (pending == null) return null;
  if (!shouldShowPendingChoice(state)) return null;
  return pendingChoicePrompt(pending, state, board, tableId, h);
}
