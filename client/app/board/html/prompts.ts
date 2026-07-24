// Engine `pending_choice` prompts, plus pre-submit cost/modal/X pickers owned by the board.
//
// Pending-choice formulators collect answers and route every submission through `choiceIntent`.

import { Option } from "effect";
import { type Html, html } from "foldkit/html";
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
import { costPipPlate } from "~/costPips";
import { filterOptionLabels } from "~/optionFilter";
import { manaFontClass } from "~/oracleText";
import { isActivePlayer } from "~/spectator";
import { cardArt } from "~/ui/card-art";
import type { ChoiceItem, PendingChoiceView, VisibleState, WireModeChoice, WireTarget } from "~/wire/types";
import { clampX, costText, costWithChosenX } from "~/xCost";
import { modeAvailable } from "../action/modal";
import {
  gyExileCostObjectIds,
  objectName,
  pendingBoardTargetMode,
  pendingDamageAssignBlockers,
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
import { seatColor, ZONE } from "../geometry/layout";
import {
  CancelActionClicked,
  DiscardChosen,
  GyExileChosen,
  GyExileConfirmed,
  type Message,
  ModalModesChosen,
  ModalModeToggled,
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
  PromptStringSet,
  PromptSubmitted,
  SacrificeChosen,
  TargetChosen,
  XDraftSet,
  XSubmitted,
} from "../messages";
import type { BoardModel } from "../submodel";
import { HAND_BAR_H } from "./hand";

const h = html<Message>();

function itemButton(label: string, testId: string, onClick: Message, disabled = false): Html {
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", testId),
      h.OnClick(onClick),
      h.Disabled(disabled),
      h.Class(
        disabled
          ? "group relative cursor-not-allowed rounded-hud border-0 bg-transparent p-0 opacity-40"
          : "group relative cursor-pointer rounded-hud border-0 bg-transparent p-0",
      ),
    ],
    [
      h.span(
        [
          h.Class(
            disabled
              ? "block rounded-hud bg-glass px-3 py-1 text-body text-mist"
              : "block rounded-hud bg-glass px-3 py-1 text-body text-snow transition-transform duration-150 ease-out group-hover:-translate-y-1 hover:bg-glass-dim",
          ),
        ],
        [label],
      ),
    ],
  );
}

function submitButton(label: string, disabled: boolean): Html {
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", "prompt-submit"),
      h.OnClick(PromptSubmitted()),
      h.Disabled(disabled),
      h.Class(
        disabled
          ? "cursor-not-allowed rounded-hud bg-glass px-3 py-1 text-body text-mist"
          : "cursor-pointer rounded-hud bg-llanowar px-3 py-1 text-body text-snow hover:bg-llanowar/90",
      ),
    ],
    [label],
  );
}

function cancelButton(): Html {
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

function frame(testId: string, title: string, body: ReadonlyArray<Html>): Html {
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

function cardPickButton(item: ChoiceItem, state: VisibleState, picked: ReadonlyArray<number>, ordered: boolean): Html {
  const selected = picked.includes(item.id);
  const pickOrder = picked.indexOf(item.id);
  const print = choiceItemPrint(item, state);
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", `prompt-card-${item.id}`),
      h.AriaLabel(item.label),
      h.AriaPressed(selected ? "true" : "false"),
      h.OnClick(PromptCardToggled({ id: item.id })),
      h.Class(
        [
          "relative cursor-pointer rounded-[9px] border-4 p-0 transition-transform duration-150 ease-out hover:-translate-y-1",
          selected ? "border-llanowar" : "border-transparent",
        ].join(" "),
      ),
    ],
    [
      print
        ? cardArt(h, {
            print,
            size: "large",
            alt: "",
            className: "block aspect-[150/209] w-[120px] rounded-[6px] bg-morph-slate",
          })
        : h.div(
            [
              h.Class(
                "flex aspect-[150/209] w-[120px] items-center justify-center rounded-[6px] bg-morph-slate px-2 text-caption text-snow",
              ),
            ],
            [item.label],
          ),
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
      print
        ? cardArt(h, {
            print,
            size: "large",
            alt: "",
            className: "block aspect-[150/209] w-[120px] rounded-[6px] bg-morph-slate",
          })
        : h.div(
            [
              h.Class(
                "flex aspect-[150/209] w-[120px] items-center justify-center rounded-[6px] bg-morph-slate px-2 text-caption text-snow",
              ),
            ],
            [item.label],
          ),
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
  pending: Extract<PendingChoiceView, { kind: "scry" | "surveil" }>,
  state: VisibleState,
  board: BoardModel,
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
  const title = pending.kind === "scry" ? `Scry ${pending.items.length}` : `Surveil ${pending.items.length}`;
  const bottomLabel = pending.kind === "surveil" ? "Graveyard" : "Bottom of library";
  const hint =
    pending.kind === "surveil"
      ? "Click a card to move it between Top and Graveyard. Order on Top is left to right."
      : "Click a card to move it between Top and Bottom. Order in each lane is left to right.";

  return h.div(
    [
      h.DataAttribute("testid", "pending-arrange-aim"),
      h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
      h.Class(
        "pointer-events-auto fixed left-1/2 z-30 flex max-h-[min(70vh,560px)] w-[min(92vw,720px)] -translate-x-1/2 flex-col gap-2 overflow-hidden rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-snow shadow-hud",
      ),
    ],
    [
      h.div([h.Class("shrink-0 font-semibold text-body")], [title]),
      h.div(
        [
          h.DataAttribute("testid", "prompt-arrange-lanes"),
          h.Class("flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto overscroll-contain"),
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
                  ? topItems.map((item) => arrangeLaneCard(item, state, topIds, true))
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
                  ? bottomItems.map((item) => arrangeLaneCard(item, state, bottomIds, pending.kind === "scry"))
                  : [h.div([h.Class("self-center text-caption text-mist")], ["None"])],
              ),
            ],
          ),
        ],
      ),
      h.div([h.Class("flex shrink-0 flex-wrap gap-2")], [submitButton("Done", false)]),
    ],
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
    ? h.input([
        h.DataAttribute("testid", "pick-card-filter"),
        h.Type("search"),
        h.Placeholder("Filter by name…"),
        h.Autofocus(true),
        h.AriaLabel("Filter cards by name"),
        h.Value(filter),
        h.OnInput((v) => PromptCardFilterSet({ query: v })),
        h.Class("w-[min(90vw,320px)] shrink-0 rounded-hud bg-glass px-3 py-1 text-body text-snow"),
      ])
    : null;
  const emptyEl =
    searchable && filter.trim() !== "" && shown.length === 0
      ? h.div([h.Class("text-label text-mist")], ["No cards match."])
      : null;
  const cardsEl = h.div(
    [h.Class("flex flex-wrap justify-center gap-2")],
    [...shown.map((item) => cardPickButton(item, state, picked, config.ordered ?? false)), emptyEl].filter(
      (v): v is Html => v !== null,
    ),
  );
  const actionsEl = h.div(
    [h.Class("flex shrink-0 flex-wrap gap-2")],
    [
      submitButton(config.submitLabel, !ready),
      config.declineLabel != null
        ? itemButton(config.declineLabel, "prompt-decline", PromptDeclined())
        : h.span([], []),
    ],
  );

  if (!searchable) {
    const body: Html[] = [];
    if (hintEl != null) body.push(hintEl);
    body.push(cardsEl);
    body.push(actionsEl);
    return frame("pending-choice", config.title, body);
  }

  // Library search: dock near the hand bar so the board stays visible (Arena tutor chrome).
  return h.div(
    [
      h.DataAttribute("testid", "pending-library-aim"),
      h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
      h.Class(
        "pointer-events-auto fixed left-1/2 z-30 flex max-h-[min(70vh,560px)] w-[min(92vw,720px)] -translate-x-1/2 flex-col gap-2 overflow-hidden rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-snow shadow-hud",
      ),
    ],
    [
      h.div([h.DataAttribute("testid", "pick-title"), h.Class("shrink-0 font-semibold text-body")], [config.title]),
      h.div(
        [h.Class("pointer-events-none shrink-0 text-caption text-mist")],
        ["Filter by name, click a card, then Choose — or Fail to find."],
      ),
      hintEl,
      filterEl,
      h.div(
        [
          h.DataAttribute("testid", "pick-card-scroll"),
          h.Class(
            `${PICK_CARD_SCROLL_MIN_CLASS} w-full flex-1 overflow-y-auto overscroll-contain rounded-panel bg-glass/30 p-2`,
          ),
        ],
        [cardsEl],
      ),
      actionsEl,
    ].filter((v): v is Html => v !== null),
  );
}

function orderPrompt(pending: Extract<PendingChoiceView, { kind: "order_triggers" }>, board: BoardModel): Html {
  const draft = board.promptDraft;
  const order = draft?.kind === "order" ? draft.order : pending.labels.map((_, i) => i);
  const pick = board.orderPickPos;
  const rows = order.map((effectIndex, pos) => {
    const selected = pick === pos;
    return h.div(
      [
        h.DataAttribute("testid", `prompt-order-${pos}`),
        h.Draggable(true),
        h.OnDragStart(PromptOrderRowClicked({ pos })),
        h.AllowDrop(),
        h.OnDrop(PromptOrderRowClicked({ pos })),
        h.OnDragEnd(PromptOrderDragEnded()),
        h.Class(
          [
            "flex cursor-grab items-center gap-2 rounded-hud border px-2 py-2 transition-colors active:cursor-grabbing",
            selected ? "border-llanowar bg-llanowar/20 opacity-80" : "border-transparent bg-glass/50",
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
          [pending.labels[effectIndex] ?? ""],
        ),
      ],
    );
  });
  return h.div(
    [
      h.DataAttribute("testid", "pending-order-aim"),
      h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
      h.Class(
        "pointer-events-auto fixed left-1/2 z-30 flex max-h-[min(70vh,560px)] w-[min(92vw,560px)] -translate-x-1/2 flex-col gap-2 overflow-hidden rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-snow shadow-hud",
      ),
    ],
    [
      h.div([h.Class("shrink-0 font-semibold text-body")], ["Order these triggers — the last one resolves first"]),
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
      submitButton("Submit", false),
    ],
  );
}

function amountStepper(id: number, amount: number, max: number): Html {
  const value = clampX(amount, 0, max);
  return h.div(
    [h.Class("flex flex-wrap items-center gap-1")],
    [
      itemButton("Min", `prompt-damage-${id}-min`, PromptDamageSet({ id, amount: 0 })),
      itemButton("−", `prompt-damage-${id}-dec`, PromptDamageSet({ id, amount: value - 1 }), value <= 0),
      h.span(
        [
          h.DataAttribute("testid", `prompt-damage-${id}-value`),
          h.Class("min-w-[2ch] text-center text-body font-semibold text-snow"),
        ],
        [String(value)],
      ),
      itemButton("+", `prompt-damage-${id}-inc`, PromptDamageSet({ id, amount: value + 1 }), value >= max),
      itemButton("Max", `prompt-damage-${id}-max`, PromptDamageSet({ id, amount: max })),
    ],
  );
}

function damageAssignPrompt(
  pending: Extract<PendingChoiceView, { kind: "assign_combat_damage" }>,
  state: VisibleState,
  board: BoardModel,
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
          [h.span([h.Class("w-28 truncate text-body")], [it.label]), amountStepper(it.id, amounts[it.id] ?? 0, power)],
        ),
      );
  return frame("pending-choice", `Divide ${power} damage among blockers`, [
    onBoard
      ? h.div(
          [h.DataAttribute("testid", "pending-damage-aim"), h.Class("text-body text-mist")],
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
    submitButton("Assign", !ready),
  ]);
}

function targetPickButton(target: WireTarget, state: VisibleState, testId: string): Html {
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
    [
      obj?.print
        ? cardArt(h, {
            print: obj.print,
            size: "large",
            alt: "",
            className: "block aspect-[150/209] w-[150px] rounded-[9px] bg-morph-slate",
          })
        : h.div(
            [
              h.Class(
                "flex aspect-[150/209] w-[150px] items-center justify-center rounded-[9px] bg-morph-slate px-2 text-body text-snow",
              ),
            ],
            [name],
          ),
    ],
  );
}

function targetPickPrompt(title: string, targets: ReadonlyArray<WireTarget>, state: VisibleState): Html {
  return frame("target-pick", title, [
    h.div(
      [h.Class("flex max-w-[min(90vw,1040px)] flex-wrap justify-center gap-3")],
      targets.map((t, i) => targetPickButton(t, state, `target-pick-${i}`)),
    ),
    cancelButton(),
  ]);
}

function boardXPrompt(prompt: NonNullable<BoardModel["xPrompt"]>): Html {
  const { minX, maxX, draftX, xCost, name } = prompt;
  const preview = costText(costWithChosenX(xCost, draftX));
  return frame("x-prompt", `Choose X for ${name}`, [
    h.div(
      [
        h.Class("mb-sm flex items-center justify-center gap-2 text-body text-mist"),
        h.DataAttribute("testid", "x-prompt-preview"),
      ],
      [`Pay ${preview}`],
    ),
    h.div(
      [h.Class("flex flex-wrap items-center justify-center gap-2")],
      [
        itemButton("Min", "x-prompt-min", XDraftSet({ x: minX })),
        itemButton("−", "x-prompt-dec", XDraftSet({ x: draftX - 1 }), draftX <= minX),
        h.span(
          [
            h.DataAttribute("testid", "x-prompt-value"),
            h.Class("min-w-[2ch] text-center text-body font-semibold text-snow"),
          ],
          [String(draftX)],
        ),
        itemButton("+", "x-prompt-inc", XDraftSet({ x: draftX + 1 }), draftX >= maxX),
        itemButton("Max", "x-prompt-max", XDraftSet({ x: maxX })),
      ],
    ),
    h.div(
      [h.Class("flex flex-wrap items-center justify-center gap-2")],
      [itemButton("Confirm", "x-prompt-confirm", XSubmitted({ x: draftX })), cancelButton()],
    ),
  ]);
}

function costPickPrompt(
  testId: string,
  title: string,
  choices: ReadonlyArray<number>,
  state: VisibleState,
  message: (id: number) => Message,
): Html {
  return frame(testId, title, [
    h.div(
      [h.Class("flex flex-wrap gap-2")],
      choices.map((id) => {
        const obj = state.objects.find((o) => o.id === id);
        return itemButton(obj?.name ?? `#${id}`, `${testId}-${id}`, message(id));
      }),
    ),
    cancelButton(),
  ]);
}

function modalPrompt(mc: NonNullable<BoardModel["modalCast"]>): Html {
  if (mc.chosen == null) {
    const choose = mc.action.modal?.choose ?? 1;
    const chooseMax = mc.action.modal?.choose_max ?? choose;
    const multi = chooseMax > 1;
    const picked = multi ? mc.modeDraft : [];
    const ready = multi ? picked.length >= choose && picked.length <= chooseMax : true;
    const countHint = choose === chooseMax ? `Choose ${choose}` : `Choose ${choose}–${chooseMax}`;
    return frame("modal-mode-picker", mc.action.label || "Choose modes", [
      h.div([h.Class("text-caption text-mist")], [countHint]),
      h.div(
        [h.Class("flex flex-col gap-1")],
        mc.modes.map((mode, i) => {
          const selected = picked.includes(i);
          const available = modeAvailable(mode);
          if (multi) {
            return h.button(
              [
                h.Type("button"),
                h.DataAttribute("testid", `modal-mode-${i}`),
                h.AriaPressed(selected ? "true" : "false"),
                h.Disabled(!available),
                h.OnClick(ModalModeToggled({ index: i })),
                h.Class(
                  [
                    "rounded-hud px-3 py-2 text-left text-body",
                    selected ? "bg-llanowar/25 text-snow" : "bg-glass text-snow",
                    !available ? "cursor-not-allowed opacity-40" : "hover:bg-glass-dim",
                  ].join(" "),
                ),
              ],
              [mode.label, !available ? " (no legal target)" : ""],
            );
          }
          return itemButton(mode.label, `modal-mode-${i}`, ModalModesChosen({ chosen: [i] }));
        }),
      ),
      multi
        ? h.div(
            [h.Class("flex gap-2")],
            [
              h.button(
                [
                  h.Type("button"),
                  h.DataAttribute("testid", "modal-cast"),
                  h.Disabled(!ready),
                  h.OnClick(ModalModesChosen({ chosen: [...picked] })),
                  h.Class(
                    ready
                      ? "cursor-pointer rounded-hud bg-llanowar px-3 py-1 text-body text-snow"
                      : "cursor-not-allowed rounded-hud bg-glass px-3 py-1 text-body text-mist",
                  ),
                ],
                ["Cast"],
              ),
              cancelButton(),
            ],
          )
        : cancelButton(),
    ]);
  }
  return frame("modal-waiting", "Pick a target for the chosen mode.", [cancelButton()]);
}

function pendingChoiceTitle(pending: PendingChoiceView): string {
  if ("label" in pending && typeof pending.label === "string" && pending.label !== "") return pending.label;
  return `Choose (${pending.kind})`;
}

function answerButton(
  pending: PendingChoiceView,
  testId: string,
  label: string,
  answer: AnswerInput,
  primary: boolean,
  disabled = false,
): Html {
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", testId),
      h.Disabled(disabled),
      h.OnClick(PendingChoiceAnswered({ intent: choiceIntent(pending, answer) })),
      h.Class("group relative rounded-hud border-0 bg-transparent p-0 disabled:cursor-not-allowed disabled:opacity-50"),
    ],
    [
      h.span(
        [
          h.Class(
            [
              "block transition-transform duration-150 ease-out group-hover:-translate-y-1",
              primary
                ? "rounded-hud bg-llanowar px-3 py-1 text-body text-snow"
                : "rounded-hud bg-glass px-3 py-1 text-body text-lichen",
            ].join(" "),
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
    case "choose_attach_host":
      return pending.optional ? "Don't attach" : null;
    case "choose_target":
      return pending.optional ? "No target" : null;
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
      return { title: pending.label, submitLabel: "Choose", declineLabel };
    case "choose_spell_targets":
    case "choose_ability_targets":
      return { title: pending.label, submitLabel: "Choose" };
    case "choose_activation_cost_targets":
      return { title: "Choose cost targets", submitLabel: "Choose" };
    case "decline_untap":
      return { title: "Choose permanents to keep tapped", submitLabel: "Keep tapped" };
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
      return { title: "Choose a card to cast for free", submitLabel: "Cast", declineLabel };
    case "opponent_chooses_exiled_nonland":
      return { title: "Choose an exiled nonland card", submitLabel: "Choose", declineLabel };
    case "choose_exiled_to_cast_free":
      return {
        title: `Choose ${pending.count} card${pending.count === 1 ? "" : "s"} to cast for free`,
        submitLabel: "Choose",
      };
    case "choose_copy_target":
      return { title: "Choose a copy target", submitLabel: "Copy" };
    case "choose_attach_host":
      return { title: "Choose what to attach to", submitLabel: "Attach", declineLabel };
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
    | "shuffle_from_graveyard"
    | "choose_dredge"
    | "pay_cumulative_upkeep_or_sacrifice"
    | "choose_activation_cost_targets"
    | "choose_target"
    | "choose_spell_targets"
    | "choose_ability_targets",
  oneClick: boolean,
): string {
  switch (kind) {
    case "exile_from_graveyard":
      return oneClick ? "Click a card in the graveyard to exile" : "Click cards in the graveyard to exile";
    case "may_return_from_graveyard":
      return "Click cards in the graveyard to return";
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
    case "choose_spell_targets":
    case "choose_ability_targets":
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
      return "Click a land in your hand to put onto the battlefield";
    case "put_creature_from_hand":
      return "Click a creature in your hand to put onto the battlefield";
    case "cast_creature_face_down":
      return "Click a creature in your hand to cast face down";
    case "put_from_hand_on_top":
      return oneClick
        ? "Click a card in your hand to put on top of your library"
        : "Click cards in your hand to put on top of your library";
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
      [
        print
          ? cardArt(h, {
              print,
              size: "large",
              alt: "",
              className: "block aspect-[150/209] w-[120px] rounded-[6px] bg-morph-slate",
            })
          : h.div(
              [
                h.Class(
                  "flex aspect-[150/209] w-[120px] items-center justify-center rounded-[6px] bg-morph-slate px-2 text-caption text-snow",
                ),
              ],
              [item.label],
            ),
      ],
    );
  });
  const decline = declineAnswer(pending);
  return h.div(
    [
      h.DataAttribute("testid", "pending-revealed-aim"),
      h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
      h.Class(
        "pointer-events-auto fixed left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [
      h.div([h.Class("pointer-events-none text-center")], ["Click a revealed card to put into the graveyard"]),
      h.div([h.Class("flex max-w-[min(90vw,720px)] flex-wrap justify-center gap-2")], cards),
      decline != null
        ? answerButton(
            pending,
            "prompt-decline",
            cardPickDeclineLabel(pending) ?? "Choose none",
            decline,
            false,
            tableId == null,
          )
        : null,
    ].filter((v): v is Html => v !== null),
  );
}

function cardPickForKind(
  pending: PendingChoiceView,
  state: VisibleState,
  board: BoardModel,
  tableId: string | null,
): Html | null {
  if (pending.kind === "opponent_chooses_revealed_to_graveyard") {
    return revealedToGraveyardAim(pending, state, tableId);
  }
  const gyPick = pendingGraveyardPickIds(pending, state);
  if (gyPick != null) {
    const kind = pending.kind;
    if (
      kind !== "exile_from_graveyard" &&
      kind !== "may_return_from_graveyard" &&
      kind !== "shuffle_from_graveyard" &&
      kind !== "choose_dredge" &&
      kind !== "pay_cumulative_upkeep_or_sacrifice" &&
      kind !== "choose_activation_cost_targets" &&
      kind !== "choose_target" &&
      kind !== "choose_spell_targets" &&
      kind !== "choose_ability_targets"
    ) {
      return null;
    }
    const oneClick = pendingGraveyardPickOneClick(pending);
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    const picked = draft.kind === "card-pick" ? draft.picked : [];
    const ready = !oneClick && cardPickReady(pending, picked);
    const required = cardPickRequiredCount(pending);
    const maxHint =
      kind === "shuffle_from_graveyard" ||
      kind === "choose_target" ||
      kind === "choose_spell_targets" ||
      kind === "choose_ability_targets"
        ? pending.max
        : required;
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
    const actions: Html[] = [];
    if (!oneClick) {
      const submitLabel =
        kind === "exile_from_graveyard"
          ? "Exile"
          : kind === "may_return_from_graveyard"
            ? "Return"
            : kind === "shuffle_from_graveyard"
              ? "Shuffle"
              : kind === "pay_cumulative_upkeep_or_sacrifice"
                ? "Pay"
                : "Confirm";
      actions.push(submitButton(submitLabel, !ready));
    }
    const decline = declineAnswer(pending);
    if (decline != null) {
      actions.push(
        answerButton(
          pending,
          "prompt-decline",
          cardPickDeclineLabel(pending) ?? "Decline",
          decline,
          false,
          tableId == null,
        ),
      );
    }
    return h.div(
      [
        h.DataAttribute("testid", "pending-gy-aim"),
        h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
        h.Class(
          [
            "fixed left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
            actions.length > 0 ? "pointer-events-auto" : "pointer-events-none",
          ].join(" "),
        ),
      ],
      [
        h.div([h.Class("pointer-events-none")], [pendingGraveyardAimCoach(kind, oneClick)]),
        countLine,
        actions.length > 0 ? h.div([h.Class("flex flex-wrap justify-center gap-2")], actions) : null,
      ].filter((v): v is Html => v !== null),
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
    const oneClick = pendingExilePickOneClick(pending);
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    const picked = draft.kind === "card-pick" ? draft.picked : [];
    const ready = !oneClick && cardPickReady(pending, picked);
    const required = cardPickRequiredCount(pending);
    const countLine =
      !oneClick && required != null
        ? h.div(
            [h.DataAttribute("testid", "pending-exile-count"), h.Class("pointer-events-none text-caption text-mist")],
            [`${picked.length} / ${required} selected`],
          )
        : null;
    const actions: Html[] = [];
    if (!oneClick) {
      actions.push(submitButton("Choose", !ready));
    }
    const decline = declineAnswer(pending);
    if (decline != null) {
      actions.push(
        answerButton(
          pending,
          "prompt-decline",
          cardPickDeclineLabel(pending) ?? "Decline",
          decline,
          false,
          tableId == null,
        ),
      );
    }
    return h.div(
      [
        h.DataAttribute("testid", "pending-exile-aim"),
        h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
        h.Class(
          [
            "fixed left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
            actions.length > 0 ? "pointer-events-auto" : "pointer-events-none",
          ].join(" "),
        ),
      ],
      [
        h.div([h.Class("pointer-events-none")], [pendingExileAimCoach(kind, oneClick)]),
        countLine,
        actions.length > 0 ? h.div([h.Class("flex flex-wrap justify-center gap-2")], actions) : null,
      ].filter((v): v is Html => v !== null),
    );
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
    const ready = !oneClick && cardPickReady(pending, picked);
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
    const actions: Html[] = [];
    if (!oneClick) {
      const submitLabel =
        kind === "may_discard" ? "Continue" : kind === "put_from_hand_on_top" ? "Put on top" : "Discard";
      actions.push(submitButton(submitLabel, !ready));
    }
    const decline = declineAnswer(pending);
    if (decline != null) {
      actions.push(
        answerButton(
          pending,
          "prompt-decline",
          cardPickDeclineLabel(pending) ?? "Decline",
          decline,
          false,
          tableId == null,
        ),
      );
    }
    return h.div(
      [
        h.DataAttribute("testid", discardKind ? "pending-discard-aim" : "pending-hand-aim"),
        h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
        h.Class(
          [
            "fixed left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
            actions.length > 0 ? "pointer-events-auto" : "pointer-events-none",
          ].join(" "),
        ),
      ],
      [
        h.div([h.Class("pointer-events-none")], [pendingHandAimCoach(kind, oneClick)]),
        countLine,
        actions.length > 0 ? h.div([h.Class("flex flex-wrap justify-center gap-2")], actions) : null,
      ].filter((v): v is Html => v !== null),
    );
  }
  if (pendingBoardTargetMode(pending, state) != null) {
    const decline = declineAnswer(pending);
    const label = "label" in pending && typeof pending.label === "string" ? pending.label : pendingChoiceTitle(pending);
    const oneClick = pendingTargetOneClick(pending);
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    const picked = draft.kind === "card-pick" ? draft.picked : [];
    const max =
      pending.kind === "choose_target" ||
      pending.kind === "choose_spell_targets" ||
      pending.kind === "choose_ability_targets"
        ? pending.max
        : pending.kind === "choose_own_sacrifices" || pending.kind === "choose_activation_cost_targets"
          ? pending.count
          : pending.kind === "sacrifice_edict"
            ? cardPickRequiredCount(pending)
            : null;
    const ready = !oneClick && cardPickReady(pending, picked);
    const countLine =
      !oneClick && max != null
        ? h.div(
            [h.DataAttribute("testid", "pending-target-count"), h.Class("pointer-events-none text-caption text-mist")],
            [`${picked.length} / ${max} selected`],
          )
        : null;
    const actions: Html[] = [];
    if (!oneClick) {
      actions.push(submitButton("Confirm", !ready));
    }
    if (decline != null) {
      actions.push(
        answerButton(
          pending,
          "prompt-decline",
          cardPickDeclineLabel(pending) ?? "Decline",
          decline,
          false,
          tableId == null,
        ),
      );
    }
    return h.div(
      [
        h.DataAttribute("testid", "pending-target-aim"),
        h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
        h.Class(
          [
            "fixed left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
            actions.length > 0 ? "pointer-events-auto" : "pointer-events-none",
          ].join(" "),
        ),
      ],
      [
        h.div([h.Class("pointer-events-none")], [label]),
        countLine,
        actions.length > 0 ? h.div([h.Class("flex flex-wrap justify-center gap-2")], actions) : null,
      ].filter((v): v is Html => v !== null),
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
        ),
      );
    }
    return frame("pending-choice", pending.label, [h.div([h.Class("flex flex-wrap gap-2")], buttons)]);
  }

  if (pending.kind === "scry" || pending.kind === "surveil") {
    return arrangeLanesPrompt(pending, state, board);
  }

  if (pending.kind === "select_from_top") {
    return selectFromTopLanesPrompt(pending, state, board);
  }

  const items = "items" in pending ? pending.items : [];
  const config = cardPickConfig(pending);
  return cardPickPrompt(pending, items, state, board, config);
}

function selectFromTopLanesPrompt(
  pending: Extract<PendingChoiceView, { kind: "select_from_top" }>,
  state: VisibleState,
  board: BoardModel,
): Html {
  const draft = board.promptDraft ?? initPromptDraft(pending, state);
  const picked = draft.kind === "card-pick" ? draft.picked : [];
  const byId = new Map(pending.items.map((it) => [it.id, it]));
  const takeItems = picked.flatMap((id) => {
    const item = byId.get(id);
    return item != null ? [item] : [];
  });
  const restItems = pending.items.filter((it) => !picked.includes(it.id));
  return h.div(
    [
      h.DataAttribute("testid", "pending-select-top-aim"),
      h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
      h.Class(
        "pointer-events-auto fixed left-1/2 z-30 flex max-h-[min(70vh,560px)] w-[min(92vw,720px)] -translate-x-1/2 flex-col gap-2 overflow-hidden rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-snow shadow-hud",
      ),
    ],
    [
      h.div([h.Class("shrink-0 font-semibold text-body")], [`Select up to ${pending.up_to} from the top`]),
      h.div(
        [
          h.DataAttribute("testid", "prompt-select-top-lanes"),
          h.Class("flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto overscroll-contain"),
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
                  ? takeItems.map((item) => arrangeLaneCard(item, state, picked, true))
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
                  ? restItems.map((item) => arrangeLaneCard(item, state, [], false))
                  : [h.div([h.Class("self-center text-caption text-mist")], ["None"])],
              ),
            ],
          ),
        ],
      ),
      h.div([h.Class("flex shrink-0 flex-wrap gap-2")], [submitButton("Done", false)]),
    ],
  );
}

function yesNoPrompt(
  pending: Extract<PendingChoiceView, { kind: "may_yes_no" | "dance_exile_more" | "trade_secrets_repeat" }>,
  tableId: string | null,
): Html {
  return h.div(
    [
      h.DataAttribute("testid", "pending-yes-no-aim"),
      h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
      h.Class(
        "pointer-events-auto fixed left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [
      h.div(
        [h.Class("pointer-events-none text-center font-semibold text-body text-snow")],
        [pendingChoiceTitle(pending)],
      ),
      h.div(
        [h.Class("flex flex-wrap justify-center gap-2")],
        [
          answerButton(pending, "prompt-yes", "Yes", { kind: "may", yes: true }, true, tableId == null),
          answerButton(pending, "prompt-no", "No", { kind: "may", yes: false }, false, tableId == null),
        ],
      ),
    ],
  );
}

function payCostDeclineLabel(
  kind:
    | "pay_cost"
    | "pay_or_counter"
    | "pay_or_controller_draws"
    | "pay_echo_or_sacrifice"
    | "pay_recover_or_exile"
    | "sacrifice_unless_pay",
): string {
  switch (kind) {
    case "pay_or_counter":
      return "Let it be countered";
    case "pay_or_controller_draws":
      return "Let them draw";
    case "pay_echo_or_sacrifice":
    case "sacrifice_unless_pay":
      return "Sacrifice";
    case "pay_recover_or_exile":
      return "Exile";
    case "pay_cost":
      return "Don't pay";
    default: {
      const _exhaustive: never = kind;
      return _exhaustive;
    }
  }
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
        | "sacrifice_unless_pay";
    }
  >,
  tableId: string | null,
): Html {
  const title = "label" in pending ? pending.label : pendingChoiceTitle(pending);
  const payLabel = `Pay ${costText(pending.cost)}`;
  const declineLabel = payCostDeclineLabel(pending.kind);
  return h.div(
    [
      h.DataAttribute("testid", "pending-pay-cost-aim"),
      h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
      h.Class(
        "pointer-events-auto fixed left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [
      h.div([h.Class("pointer-events-none text-center font-semibold text-body text-snow")], [title]),
      h.div(
        [h.Class("flex flex-wrap justify-center gap-2")],
        [
          answerButton(pending, "prompt-pay", payLabel, { kind: "pay", pay: true }, true, tableId == null),
          answerButton(pending, "prompt-decline", declineLabel, { kind: "pay", pay: false }, false, tableId == null),
        ],
      ),
    ],
  );
}

function modeListPrompt(
  pending: Extract<PendingChoiceView, { kind: "choose_mode" | "choose_trigger_modes" }>,
  board: BoardModel,
  state: VisibleState,
  tableId: string | null,
): Html {
  if (pending.kind === "choose_mode") {
    return h.div(
      [
        h.DataAttribute("testid", "pending-mode-aim"),
        h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
        h.Class(
          "pointer-events-auto fixed left-1/2 z-30 flex max-w-[min(100%-2rem,28rem)] -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
        ),
      ],
      [
        h.div([h.Class("pointer-events-none text-center font-semibold text-body text-snow")], ["Choose a mode"]),
        h.div(
          [h.Class("flex flex-col gap-2")],
          pending.labels.map((label, index) =>
            answerButton(
              pending,
              `prompt-mode-${index}`,
              label,
              { kind: "mode", mode: index },
              index === 0,
              tableId == null,
            ),
          ),
        ),
      ],
    );
  }

  const draft = board.promptDraft ?? initPromptDraft(pending, state);
  const picked = draft.kind === "modes" ? draft.modes : [];
  const concreteChoices: Array<{ choice: WireModeChoice; label: string }> = pending.modes.flatMap((mode, index) => {
    if (!mode.needs_target) {
      return [{ choice: { index } satisfies WireModeChoice, label: mode.label }];
    }
    return mode.targets.map((target) => ({
      choice: { index, target } satisfies WireModeChoice,
      label: `${mode.label} — ${targetLabel(target, state)}`,
    }));
  });
  const ready = picked.length === pending.choose || (pending.optional && picked.length === 0);
  return h.div(
    [
      h.DataAttribute("testid", "pending-trigger-modes-aim"),
      h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
      h.Class(
        "pointer-events-auto fixed left-1/2 z-30 flex max-w-[min(100%-2rem,28rem)] -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [
      h.div(
        [h.Class("pointer-events-none text-center font-semibold text-body text-snow")],
        ["Choose trigger modes"],
      ),
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
              h.AriaPressed(selected ? "true" : "false"),
              h.Disabled(tableId == null),
              h.OnClick(PromptModeChoiceToggled({ index: choice.index, target: choice.target ?? null })),
              h.Class(
                [
                  "rounded-hud px-3 py-2 text-left text-body",
                  selected ? "bg-llanowar/25 text-snow" : "bg-glass text-snow",
                  tableId == null ? "cursor-not-allowed opacity-50" : "hover:bg-glass-dim",
                ].join(" "),
              ),
            ],
            [label],
          );
        }),
      ),
      h.div([h.Class("flex flex-wrap justify-center gap-2")], [submitButton("Choose", !ready), cancelButton()]),
    ],
  );
}

function playerPickPrompt(
  pending: Extract<PendingChoiceView, { kind: "choose_target_players" | "choose_splitting_opponent" }>,
  state: VisibleState,
  board: BoardModel,
  tableId: string | null,
): Html {
  if (pendingPlayerAimSeats(pending, state) != null) {
    const oneClick = pendingPlayerAimOneClick(pending);
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    const picked = draft.kind === "player-pick" ? draft.players : [];
    const max = pending.kind === "choose_target_players" ? pending.max : 1;
    const ready =
      pending.kind === "choose_target_players" ? picked.length >= pending.min && picked.length <= pending.max : false;
    const countLine =
      !oneClick && pending.kind === "choose_target_players"
        ? h.div(
            [h.DataAttribute("testid", "pending-player-count"), h.Class("pointer-events-none text-caption text-mist")],
            [`${picked.length} / ${max} selected`],
          )
        : null;
    const actions: Html[] = [];
    if (!oneClick) {
      actions.push(submitButton("Confirm", !ready));
    }
    return h.div(
      [
        h.DataAttribute("testid", "pending-player-aim"),
        h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
        h.Class(
          [
            "fixed left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
            actions.length > 0 ? "pointer-events-auto" : "pointer-events-none",
          ].join(" "),
        ),
      ],
      [
        h.div([h.Class("pointer-events-none")], [pending.label]),
        countLine,
        actions.length > 0 ? h.div([h.Class("flex flex-wrap justify-center gap-2")], actions) : null,
      ].filter((v): v is Html => v !== null),
    );
  }

  if (pending.kind === "choose_splitting_opponent") {
    return frame("pending-choice", pending.label, [
      h.div(
        [h.Class("flex flex-wrap gap-2")],
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
            ),
          ];
        }),
      ),
    ]);
  }

  const draft = board.promptDraft ?? initPromptDraft(pending, state);
  const picked = draft.kind === "player-pick" ? draft.players : [];
  const ready = picked.length >= pending.min && picked.length <= pending.max;
  return frame("pending-choice", pending.label, [
    h.div(
      [h.Class("flex flex-wrap gap-2")],
      pending.items.flatMap((item, index) => {
        const seat = playerSeatFromItem(item, state, index);
        if (seat == null) return [];
        const selected = picked.includes(seat);
        return [
          h.button(
            [
              h.Type("button"),
              h.DataAttribute("testid", `prompt-player-${seat}`),
              h.AriaPressed(selected ? "true" : "false"),
              h.Disabled(tableId == null),
              h.OnClick(PromptCardToggled({ id: seat })),
              h.Class(
                [
                  "rounded-hud px-3 py-2 text-body",
                  selected ? "bg-llanowar/25 text-snow" : "bg-glass text-snow",
                  tableId == null ? "cursor-not-allowed opacity-50" : "hover:bg-glass-dim",
                ].join(" "),
              ),
            ],
            [item.label],
          ),
        ];
      }),
    ),
    h.div([h.Class("flex gap-2")], [submitButton("Choose", !ready), cancelButton()]),
  ]);
}

function divideTotalPrompt(
  pending: Extract<PendingChoiceView, { kind: "divide_spell_damage" | "divide_counters" }>,
  board: BoardModel,
  state: VisibleState,
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
              amountStepper(index, amounts[index] ?? 0, pending.total),
            ],
          ),
        );
    return frame("pending-choice", `Divide ${pending.total} damage`, [
      onBoard
        ? h.div(
            [h.DataAttribute("testid", "pending-divide-aim"), h.Class("text-body text-mist")],
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
      submitButton("Assign", !ready),
    ]);
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
            amountStepper(item.id, amounts[item.id] ?? 0, pending.total),
          ],
        ),
      );
  return frame("pending-choice", `Divide ${pending.total} counters`, [
    onBoard
      ? h.div(
          [h.DataAttribute("testid", "pending-divide-counters-aim"), h.Class("text-body text-mist")],
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
    submitButton("Assign", !ready),
  ]);
}

function pilePickPrompt(
  pending: Extract<PendingChoiceView, { kind: "opponent_chooses_pile" | "choose_pile_for_hand" }>,
  tableId: string | null,
): Html {
  const pileBlock = (title: string, items: ReadonlyArray<ChoiceItem>, pile: 0 | 1): Html =>
    h.div(
      [h.Class("min-w-[180px] flex-1 rounded-panel bg-glass p-3")],
      [
        h.div([h.Class("mb-2 font-semibold text-body text-snow")], [title]),
        h.div(
          [h.Class("mb-3 flex flex-col gap-1 text-caption text-mist")],
          items.map((item) => h.span([], [item.label])),
        ),
        answerButton(
          pending,
          `prompt-pile-${pile}`,
          title,
          { kind: "opponent_pile", pile },
          pile === 0,
          tableId == null,
        ),
      ],
    );
  return h.div(
    [
      h.DataAttribute("testid", "pending-pile-aim"),
      h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
      h.Class(
        "pointer-events-auto fixed left-1/2 z-30 flex max-w-[min(100%-2rem,40rem)] -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [
      h.div([h.Class("pointer-events-none text-center font-semibold text-body text-snow")], ["Choose a pile"]),
      h.div(
        [h.Class("flex w-full flex-wrap justify-center gap-3")],
        [pileBlock("Pile A", pending.pile_a, 0), pileBlock("Pile B", pending.pile_b, 1)],
      ),
    ],
  );
}

function partitionPrompt(
  pending: Extract<PendingChoiceView, { kind: "partition_revealed" | "distribute_top" }>,
  board: BoardModel,
  state: VisibleState,
  tableId: string | null,
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
    return h.div(
      [
        h.DataAttribute("testid", "pending-partition-aim"),
        h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
        h.Class(
          "pointer-events-auto fixed left-1/2 z-30 flex max-h-[min(70vh,560px)] w-[min(92vw,720px)] -translate-x-1/2 flex-col gap-2 overflow-hidden rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-snow shadow-hud",
        ),
      ],
      [
        h.div([h.Class("shrink-0 font-semibold text-body")], ["Choose cards for Pile A"]),
        h.div(
          [
            h.DataAttribute("testid", "prompt-partition-lanes"),
            h.Class("flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto overscroll-contain"),
          ],
          [
            h.div([h.Class("shrink-0 text-caption text-mist")], ["Click a card to move it between Pile A and Pile B."]),
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
                    ? pileAItems.map((item) => arrangeLaneCard(item, state, pileAIds, false))
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
                    ? pileBItems.map((item) => arrangeLaneCard(item, state, [], false))
                    : [h.div([h.Class("self-center text-caption text-mist")], ["None"])],
                ),
              ],
            ),
          ],
        ),
        h.div([h.Class("flex shrink-0 gap-2")], [submitButton("Lock piles", false), cancelButton()]),
      ],
    );
  }

  return distributeTopLanesPrompt(pending, board, state, tableId);
}

function distributeTopLanesPrompt(
  pending: Extract<PendingChoiceView, { kind: "distribute_top" }>,
  board: BoardModel,
  state: VisibleState,
  tableId: string | null,
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
    const face = print
      ? cardArt(h, {
          print,
          size: "large",
          alt: "",
          className: "block aspect-[150/209] w-[120px] rounded-[6px] bg-morph-slate",
        })
      : h.div(
          [
            h.Class(
              "flex aspect-[150/209] w-[120px] items-center justify-center rounded-[6px] bg-morph-slate px-2 text-caption text-snow",
            ),
          ],
          [item.label],
        );
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

  return h.div(
    [
      h.DataAttribute("testid", "pending-distribute-aim"),
      h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
      h.Class(
        "pointer-events-auto fixed left-1/2 z-30 flex max-h-[min(70vh,560px)] w-[min(92vw,720px)] -translate-x-1/2 flex-col gap-2 overflow-hidden rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-snow shadow-hud",
      ),
    ],
    [
      h.div([h.Class("shrink-0 font-semibold text-body")], ["Distribute the revealed cards"]),
      h.div(
        [
          h.DataAttribute("testid", "prompt-distribute-lanes"),
          h.Class("flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto overscroll-contain"),
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
      h.div([h.Class("flex shrink-0 gap-2")], [submitButton("Distribute", !ready), cancelButton()]),
    ],
  );
}

function colorPickPrompt(
  pending: Extract<PendingChoiceView, { kind: "choose_color" | "choose_mana_color" }>,
  tableId: string | null,
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
  return h.div(
    [
      h.DataAttribute("testid", "pending-color-aim"),
      h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
      h.Class(
        "pointer-events-auto fixed left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [
      h.div([h.Class("pointer-events-none text-center font-semibold text-body text-snow")], [title]),
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
              h.span(
                [
                  h.DataAttribute("testid", `prompt-color-pip-${color.index}`),
                  h.Class(
                    "inline-flex shrink-0 items-center justify-center rounded-full shadow-[0_1px_2px_rgb(0_0_0/0.9)] transition-transform duration-150 ease-out group-hover:-translate-y-1",
                  ),
                  h.Style({
                    width: `${sizePx}px`,
                    height: `${sizePx}px`,
                    "background-color": costPipPlate(color.code),
                    color: "#111",
                    "font-size": `${Math.round(sizePx * 0.82)}px`,
                  }),
                ],
                [h.i([h.Class(`ms ms-${ms}`)], [])],
              ),
            ],
          );
        }),
      ),
    ],
  );
}

function stringPickPrompt(
  pending: Extract<PendingChoiceView, { kind: "choose_creature_type" | "choose_card_name" }>,
  board: BoardModel,
  state: VisibleState,
  tableId: string | null,
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
    return h.div(
      [
        h.DataAttribute("testid", "pending-card-name-aim"),
        h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
        h.Class(
          "pointer-events-auto fixed left-1/2 z-30 flex max-h-[min(70vh,560px)] w-[min(92vw,360px)] -translate-x-1/2 flex-col items-center gap-sm overflow-hidden rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
        ),
      ],
      [
        h.div(
          [h.Class("pointer-events-none shrink-0 text-center font-semibold text-body text-snow")],
          ["Name a card"],
        ),
        h.input([
          h.DataAttribute("testid", "prompt-name-input"),
          h.Placeholder("Card name"),
          h.Autofocus(true),
          h.AriaLabel("Card name"),
          h.Value(value),
          h.OnInput((v) => PromptStringSet({ value: v })),
          h.OnKeyDownPreventDefault((key) => {
            if (key !== "Enter" || !canSubmit) return Option.none();
            return Option.some(PromptSubmitted());
          }),
          h.Class("w-full shrink-0 rounded-hud bg-glass px-3 py-1 text-body text-snow"),
        ]),
        suggestions.length > 0
          ? h.div(
              [
                h.DataAttribute("testid", "prompt-name-suggestions"),
                h.Class("flex min-h-0 w-full flex-1 flex-col gap-1 overflow-y-auto"),
              ],
              suggestions.map((name, index) =>
                h.button(
                  [
                    h.Type("button"),
                    h.DataAttribute("testid", `prompt-name-suggestion-${index}`),
                    h.OnClick(PromptStringSet({ value: name })),
                    h.Class(
                      "cursor-pointer rounded-hud bg-glass px-3 py-1 text-left text-body text-snow hover:bg-glass-dim",
                    ),
                  ],
                  [name],
                ),
              ),
            )
          : null,
        submitButton("Name", !canSubmit),
      ].filter((v): v is Html => v !== null),
    );
  }
  return h.div(
    [
      h.DataAttribute("testid", "pending-creature-type-aim"),
      h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
      h.Class(
        "pointer-events-auto fixed left-1/2 z-30 flex max-h-[min(70vh,560px)] w-[min(92vw,360px)] -translate-x-1/2 flex-col items-center gap-sm overflow-hidden rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [
      h.div(
        [h.Class("pointer-events-none shrink-0 text-center font-semibold text-body text-snow")],
        ["Choose a creature type"],
      ),
      h.input([
        h.DataAttribute("testid", "prompt-type-filter"),
        h.Type("search"),
        h.Placeholder("Filter types…"),
        h.Autofocus(true),
        h.AriaLabel("Filter creature types"),
        h.Value(board.promptOptionFilter),
        h.OnInput((v) => PromptOptionFilterSet({ query: v })),
        h.Class("w-full shrink-0 rounded-hud bg-glass px-3 py-1 text-body text-snow"),
      ]),
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
                );
              });
            })(),
          ),
        ],
      ),
    ],
  );
}

function numberPickTitle(
  pending: Extract<
    PendingChoiceView,
    { kind: "may_draw_up_to" | "trade_secrets_caster_draw" | "pay_any_amount_of_mana" }
  >,
): string {
  if (pending.kind === "trade_secrets_caster_draw") return `Choose how many cards to draw (up to ${pending.max})`;
  if (pending.kind === "pay_any_amount_of_mana") return `Pay any amount of mana (up to ${pending.max})`;
  return `Draw up to ${pending.max}`;
}

function numberPickPrompt(
  pending: Extract<
    PendingChoiceView,
    { kind: "may_draw_up_to" | "trade_secrets_caster_draw" | "pay_any_amount_of_mana" }
  >,
  board: BoardModel,
  state: VisibleState,
  tableId: string | null,
): Html {
  if (pending.kind === "pay_any_amount_of_mana") {
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    const max = pending.max;
    const count = clampX(draft.kind === "number" ? draft.count : 0, 0, max);
    return h.div(
      [
        h.DataAttribute("testid", "pending-join-forces-aim"),
        h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
        h.Class(
          "pointer-events-auto fixed left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
        ),
      ],
      [
        h.div(
          [h.Class("pointer-events-none text-center font-semibold text-body text-snow")],
          [numberPickTitle(pending)],
        ),
        h.div(
          [h.Class("flex flex-wrap items-center justify-center gap-2")],
          [
            itemButton("Min", "prompt-number-min", PromptNumberSet({ count: 0 })),
            itemButton("−", "prompt-number-dec", PromptNumberSet({ count: count - 1 }), count <= 0),
            h.span(
              [
                h.DataAttribute("testid", "prompt-number-value"),
                h.Class("min-w-[2ch] text-center text-body font-semibold text-snow"),
              ],
              [String(count)],
            ),
            itemButton("+", "prompt-number-inc", PromptNumberSet({ count: count + 1 }), count >= max),
            itemButton("Max", "prompt-number-max", PromptNumberSet({ count: max })),
          ],
        ),
        submitButton(count === 0 ? "Pay 0 (decline)" : `Pay {${count}}`, tableId == null),
      ],
    );
  }
  const answerFor = (count: number): AnswerInput => ({ kind: "draw_count", count });
  return h.div(
    [
      h.DataAttribute("testid", "pending-draw-count-aim"),
      h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
      h.Class(
        "pointer-events-auto fixed left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [
      h.div(
        [h.Class("pointer-events-none text-center font-semibold text-body text-snow")],
        [numberPickTitle(pending)],
      ),
      h.div(
        [h.Class("flex flex-wrap justify-center gap-2")],
        Array.from({ length: pending.max + 1 }, (_, count) =>
          answerButton(
            pending,
            `prompt-number-${count}`,
            String(count),
            answerFor(count),
            count === pending.max,
            tableId == null,
          ),
        ),
      ),
    ],
  );
}

function destinationPickPrompt(
  pending: Extract<
    PendingChoiceView,
    { kind: "choose_countered_spell_destination" | "revealed_card_to_battlefield_or_hand" }
  >,
  state: VisibleState,
  tableId: string | null,
): Html {
  if (pending.kind === "choose_countered_spell_destination") {
    return h.div(
      [
        h.DataAttribute("testid", "pending-destination-aim"),
        h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
        h.Class(
          "pointer-events-auto fixed left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
        ),
      ],
      [
        h.div(
          [h.Class("pointer-events-none text-center font-semibold text-body text-snow")],
          ["Put the countered spell on top or bottom?"],
        ),
        h.div(
          [h.Class("flex flex-wrap justify-center gap-2")],
          [
            answerButton(
              pending,
              "prompt-destination-top",
              "Top",
              { kind: "top_or_bottom", top: true },
              true,
              tableId == null,
            ),
            answerButton(
              pending,
              "prompt-destination-bottom",
              "Bottom",
              { kind: "top_or_bottom", top: false },
              false,
              tableId == null,
            ),
          ],
        ),
      ],
    );
  }
  const print = choiceItemPrint(pending.item, state);
  const face = print
    ? cardArt(h, {
        print,
        size: "large",
        alt: "",
        className: "block aspect-[150/209] w-[120px] rounded-[6px] bg-morph-slate",
      })
    : h.div(
        [
          h.DataAttribute("testid", "prompt-revealed-face"),
          h.Class(
            "flex aspect-[150/209] w-[120px] items-center justify-center rounded-[6px] bg-morph-slate px-2 text-caption text-snow",
          ),
        ],
        [pending.item.label],
      );
  const faceEl =
    print !== "" ? h.div([h.DataAttribute("testid", "prompt-revealed-face"), h.Class("relative")], [face]) : face;
  return h.div(
    [
      h.DataAttribute("testid", "pending-revealed-destination-aim"),
      h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
      h.Class(
        "pointer-events-auto fixed left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [
      h.div(
        [h.Class("pointer-events-none text-center font-semibold text-body text-snow")],
        ["Put the revealed card onto the battlefield or into your hand?"],
      ),
      faceEl,
      h.div(
        [h.Class("flex flex-wrap justify-center gap-2")],
        [
          answerButton(
            pending,
            "prompt-destination-battlefield",
            "Battlefield",
            { kind: "revealed", choice: pending.item.id },
            true,
            tableId == null,
          ),
          answerButton(
            pending,
            "prompt-destination-hand",
            "Hand",
            { kind: "revealed", choice: null },
            false,
            tableId == null,
          ),
        ],
      ),
    ],
  );
}

function pendingChoicePrompt(
  pending: PendingChoiceView,
  state: VisibleState,
  board: BoardModel,
  tableId: string | null,
): Html | null {
  const id = FORMULATOR_FOR_KIND[pending.kind];
  switch (id) {
    case "cardPick":
      return cardPickForKind(pending, state, board, tableId);
    case "orderTriggers":
      if (pending.kind !== "order_triggers") return frame("pending-choice", pendingChoiceTitle(pending), []);
      return orderPrompt(pending, board);
    case "damageAssign":
      if (pending.kind !== "assign_combat_damage") return frame("pending-choice", pendingChoiceTitle(pending), []);
      return damageAssignPrompt(pending, state, board);
    case "yesNo":
      if (
        pending.kind !== "may_yes_no" &&
        pending.kind !== "dance_exile_more" &&
        pending.kind !== "trade_secrets_repeat"
      ) {
        return frame("pending-choice", pendingChoiceTitle(pending), []);
      }
      return yesNoPrompt(pending, tableId);
    case "payCost":
      if (
        pending.kind !== "pay_cost" &&
        pending.kind !== "pay_or_counter" &&
        pending.kind !== "pay_or_controller_draws" &&
        pending.kind !== "pay_echo_or_sacrifice" &&
        pending.kind !== "pay_recover_or_exile" &&
        pending.kind !== "sacrifice_unless_pay"
      ) {
        return frame("pending-choice", pendingChoiceTitle(pending), []);
      }
      return payCostPrompt(pending, tableId);
    case "modeList":
      if (pending.kind !== "choose_mode" && pending.kind !== "choose_trigger_modes") {
        return frame("pending-choice", pendingChoiceTitle(pending), []);
      }
      return modeListPrompt(pending, board, state, tableId);
    case "playerPick":
      if (pending.kind !== "choose_target_players" && pending.kind !== "choose_splitting_opponent") {
        return frame("pending-choice", pendingChoiceTitle(pending), []);
      }
      return playerPickPrompt(pending, state, board, tableId);
    case "divideTotal":
      if (pending.kind !== "divide_spell_damage" && pending.kind !== "divide_counters") {
        return frame("pending-choice", pendingChoiceTitle(pending), []);
      }
      return divideTotalPrompt(pending, board, state);
    case "pilePick":
      if (pending.kind !== "opponent_chooses_pile" && pending.kind !== "choose_pile_for_hand") {
        return frame("pending-choice", pendingChoiceTitle(pending), []);
      }
      return pilePickPrompt(pending, tableId);
    case "partition":
      if (pending.kind !== "partition_revealed" && pending.kind !== "distribute_top") {
        return frame("pending-choice", pendingChoiceTitle(pending), []);
      }
      return partitionPrompt(pending, board, state, tableId);
    case "colorPick":
      if (pending.kind !== "choose_color" && pending.kind !== "choose_mana_color") {
        return frame("pending-choice", pendingChoiceTitle(pending), []);
      }
      return colorPickPrompt(pending, tableId);
    case "stringPick":
      if (pending.kind !== "choose_creature_type" && pending.kind !== "choose_card_name") {
        return frame("pending-choice", pendingChoiceTitle(pending), []);
      }
      return stringPickPrompt(pending, board, state, tableId);
    case "numberPick":
      if (
        pending.kind !== "may_draw_up_to" &&
        pending.kind !== "trade_secrets_caster_draw" &&
        pending.kind !== "pay_any_amount_of_mana"
      ) {
        return frame("pending-choice", pendingChoiceTitle(pending), []);
      }
      return numberPickPrompt(pending, board, state, tableId);
    case "destinationPick":
      if (
        pending.kind !== "choose_countered_spell_destination" &&
        pending.kind !== "revealed_card_to_battlefield_or_hand"
      ) {
        return frame("pending-choice", pendingChoiceTitle(pending), []);
      }
      return destinationPickPrompt(pending, state, tableId);
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

export function promptsView(board: BoardModel, state: VisibleState, tableId: string | null): Html | null {
  if (board.xPrompt != null) return boardXPrompt(board.xPrompt);
  if (board.modalCast != null) return modalPrompt(board.modalCast);
  if (board.sacrificePick != null) {
    const choices = board.sacrificePick.action.sacrifice_choices ?? [];
    if (sacrificeCostObjectIds(choices, state) != null) {
      return h.div(
        [
          h.DataAttribute("testid", "sacrifice-cost-aim"),
          h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
          h.Class(
            "fixed left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud pointer-events-auto",
          ),
        ],
        [h.div([h.Class("pointer-events-none")], ["Click a permanent to sacrifice"]), cancelButton()],
      );
    }
    return costPickPrompt("sacrifice-pick", "Choose a permanent to sacrifice", choices, state, (id) =>
      SacrificeChosen({ objectId: id }),
    );
  }
  if (board.discardPick != null) {
    const choices = board.discardPick.action.discard_choices ?? [];
    const handIds = new Set(
      state.objects.filter((o) => o.zone === ZONE.Hand && o.owner === state.viewer).map((o) => o.id),
    );
    const onHand = choices.length > 0 && choices.every((id) => handIds.has(id));
    if (onHand) {
      return h.div(
        [
          h.DataAttribute("testid", "discard-cost-aim"),
          h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
          h.Class(
            "pointer-events-auto fixed left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
          ),
        ],
        [h.div([h.Class("pointer-events-none")], ["Click a card in your hand to discard"]), cancelButton()],
      );
    }
    return costPickPrompt("discard-pick", "Choose a card to discard", choices, state, (id) =>
      DiscardChosen({ ids: [id] }),
    );
  }
  if (board.gyExilePick != null) {
    const choices = board.gyExilePick.action.graveyard_exile_choices ?? [];
    const onPile = gyExileCostObjectIds(choices, state) != null;
    if (onPile) {
      const min = board.gyExilePick.action.graveyard_exile_min ?? 0;
      const max = board.gyExilePick.action.graveyard_exile_max ?? 0;
      const selected = board.gyExilePick.picks.graveyard_exile;
      const oneClick = max <= 1;
      const ready = !oneClick && selected.length >= min && selected.length <= max;
      const countLine = !oneClick
        ? h.div(
            [h.DataAttribute("testid", "gy-exile-cost-count"), h.Class("pointer-events-none text-caption text-mist")],
            [`${selected.length} / ${max} selected`],
          )
        : null;
      const actions: Html[] = [cancelButton()];
      if (!oneClick && min < max) {
        actions.unshift(
          h.button(
            [
              h.Type("button"),
              h.DataAttribute("testid", "prompt-submit"),
              h.OnClick(GyExileConfirmed()),
              h.Disabled(!ready),
              h.Class(
                ready
                  ? "cursor-pointer rounded-hud bg-llanowar px-3 py-1 text-body text-snow hover:bg-llanowar/90"
                  : "cursor-not-allowed rounded-hud bg-glass px-3 py-1 text-body text-mist",
              ),
            ],
            ["Exile"],
          ),
        );
      }
      return h.div(
        [
          h.DataAttribute("testid", "gy-exile-cost-aim"),
          h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
          h.Class(
            "pointer-events-auto fixed left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
          ),
        ],
        [
          h.div(
            [h.Class("pointer-events-none")],
            [oneClick ? "Click a card in the graveyard to exile" : "Click cards in the graveyard to exile"],
          ),
          countLine,
          h.div([h.Class("flex flex-wrap justify-center gap-2")], actions),
        ].filter((v): v is Html => v !== null),
      );
    }
    return costPickPrompt("gy-exile-pick", "Choose cards to exile from graveyard", choices, state, (id) =>
      GyExileChosen({ ids: [id] }),
    );
  }
  if (board.staged != null) {
    const targets = stagedPickTargets(board.staged, state);
    if (targets != null) {
      return targetPickPrompt(stagedTargetTitle(board.staged), targets, state);
    }
  }
  const pending = state.pending_choice;
  if (pending == null) return null;
  if (!shouldShowPendingChoice(state)) return null;
  return pendingChoicePrompt(pending, state, board, tableId);
}
