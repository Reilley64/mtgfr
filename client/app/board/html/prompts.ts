// Engine `pending_choice` prompts, plus pre-submit cost/modal/X pickers owned by the board.
//
// High-traffic pending_choice kinds get faithful interactive forms (card multi-pick, trigger
// ordering, combat damage assignment, etc.) mapped through `choiceIntent`. Remaining kinds keep a
// banner + single-click item list so the pipeline still flows.

import { type Html, html } from "foldkit/html";
import {
  cardPickReady,
  chooseTargetIsCardPick,
  damageAssignReady,
  initPromptDraft,
  isFaithfulPromptKind,
} from "~/choice";
import { imageUrlByPrint } from "~/deck-builder/scryfall";
import type { ChoiceItem, PendingChoiceView, VisibleState, WireIntent, WireTarget } from "~/wire/types";
import { modeAvailable } from "../action/modal";
import { objectName, playerSeatLabel, stagedPickTargets, stagedTargetTitle } from "../action/targeting";
import { seatColor } from "../geometry/layout";
import {
  CancelActionClicked,
  DiscardChosen,
  GyExileChosen,
  type Message,
  ModalModesChosen,
  ModalModeToggled,
  PendingChoiceAnswered,
  PromptCardToggled,
  PromptDamageSet,
  PromptDeclined,
  PromptOrderMoved,
  PromptSubmitted,
  SacrificeChosen,
  TargetChosen,
  XSubmitted,
} from "../messages";
import type { BoardModel } from "../submodel";

const h = html<Message>();

function itemButton(label: string, testId: string, onClick: Message): Html {
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", testId),
      h.OnClick(onClick),
      h.Class("group relative cursor-pointer rounded-hud border-0 bg-transparent p-0"),
    ],
    [
      h.span(
        [
          h.Class(
            "block rounded-hud bg-glass px-3 py-1 text-body text-snow transition-transform duration-150 ease-out group-hover:-translate-y-1 hover:bg-glass-dim",
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

function choiceItemArt(item: ChoiceItem, state: VisibleState): string {
  if (item.print) return imageUrlByPrint(item.print, "normal");
  const obj = state.objects.find((o) => o.id === item.id);
  return obj?.print ? imageUrlByPrint(obj.print, "normal") : "";
}

function cardPickButton(item: ChoiceItem, state: VisibleState, picked: ReadonlyArray<number>, ordered: boolean): Html {
  const selected = picked.includes(item.id);
  const pickOrder = picked.indexOf(item.id);
  const artUrl = choiceItemArt(item, state);
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
      artUrl
        ? h.img([
            h.Src(artUrl),
            h.Alt(""),
            h.Draggable(false),
            h.Class("block aspect-[150/209] w-[120px] rounded-[6px] bg-morph-slate"),
          ])
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
  const ready = cardPickReady(pending, picked);
  const body: Html[] = [];
  if (config.hint) body.push(h.div([h.Class("text-caption text-mist")], [config.hint]));
  body.push(
    h.div(
      [h.Class("flex flex-wrap justify-center gap-2")],
      items.map((item) => cardPickButton(item, state, picked, config.ordered ?? false)),
    ),
  );
  body.push(
    h.div(
      [h.Class("flex flex-wrap gap-2")],
      [
        submitButton(config.submitLabel, !ready),
        config.declineLabel != null
          ? itemButton(config.declineLabel, "prompt-decline", PromptDeclined())
          : h.span([], []),
      ],
    ),
  );
  return frame("pending-choice", config.title, body);
}

function orderPrompt(pending: Extract<PendingChoiceView, { kind: "order_triggers" }>, board: BoardModel): Html {
  const draft = board.promptDraft;
  const order = draft?.kind === "order" ? draft.order : pending.labels.map((_, i) => i);
  const rows = order.map((effectIndex, pos) =>
    h.div(
      [h.DataAttribute("testid", `prompt-order-${pos}`), h.Class("flex items-center gap-2")],
      [
        h.button(
          [
            h.Type("button"),
            h.Disabled(pos === 0),
            h.OnClick(PromptOrderMoved({ pos, delta: -1 })),
            h.Class("rounded-hud bg-glass px-2 py-1 text-body disabled:opacity-40"),
          ],
          ["↑"],
        ),
        h.button(
          [
            h.Type("button"),
            h.Disabled(pos === order.length - 1),
            h.OnClick(PromptOrderMoved({ pos, delta: 1 })),
            h.Class("rounded-hud bg-glass px-2 py-1 text-body disabled:opacity-40"),
          ],
          ["↓"],
        ),
        h.span([h.Class("text-body")], [pending.labels[effectIndex] ?? ""]),
      ],
    ),
  );
  return frame("pending-choice", "Order these triggers — the last one resolves first", [
    h.div([h.Class("flex flex-col gap-1")], rows),
    submitButton("Submit", false),
  ]);
}

function damageAssignPrompt(
  pending: Extract<PendingChoiceView, { kind: "assign_combat_damage" }>,
  state: VisibleState,
  board: BoardModel,
): Html {
  const draft = board.promptDraft ?? initPromptDraft(pending, state);
  const amounts = draft.kind === "damage" ? draft.amounts : {};
  const power = state.objects.find((o) => o.id === pending.source)?.power ?? 0;
  const assigned = Object.values(amounts).reduce((s, n) => s + n, 0);
  const ready = damageAssignReady(pending, draft, state);
  const rows = pending.items.map((it) =>
    h.div(
      [h.Class("flex items-center gap-2")],
      [
        h.span([h.Class("w-28 truncate text-body")], [it.label]),
        h.input([
          h.Type("number"),
          h.Min("0"),
          h.DataAttribute("testid", `prompt-damage-${it.id}`),
          h.Value(String(amounts[it.id] ?? 0)),
          h.OnInput((value) => PromptDamageSet({ id: it.id, amount: Number.parseInt(value, 10) || 0 })),
          h.Class("w-16 rounded-hud bg-glass px-2 py-1 text-body text-snow"),
        ]),
      ],
    ),
  );
  return frame("pending-choice", `Divide ${power} damage among blockers`, [
    ...rows,
    h.div(
      [h.Class(assigned === power ? "text-assign-clover" : "text-caution-amber")],
      [`assigned ${assigned} / ${power}`],
    ),
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
  const artUrl = obj?.print ? imageUrlByPrint(obj.print, "normal") : "";
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
      artUrl
        ? h.img([
            h.Src(artUrl),
            h.Alt(""),
            h.Draggable(false),
            h.Class("block aspect-[150/209] w-[150px] rounded-[9px] bg-morph-slate"),
          ])
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
  const options: Html[] = [];
  for (let x = prompt.minX; x <= prompt.maxX; x++) {
    options.push(itemButton(`X = ${x}`, `x-prompt-${x}`, XSubmitted({ x })));
  }
  return frame("x-prompt", `Choose X for ${prompt.name}`, [
    h.div([h.Class("flex flex-wrap gap-2")], options),
    cancelButton(),
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

function pendingChoiceLabel(pending: PendingChoiceView): string {
  if ("label" in pending && typeof pending.label === "string" && pending.label !== "") return pending.label;
  return `Choose (${pending.kind})`;
}

function pendingChoiceItems(pending: PendingChoiceView): ReadonlyArray<ChoiceItem> {
  if ("items" in pending && Array.isArray(pending.items)) return pending.items;
  return [];
}

function pendingChoiceIntent(pending: PendingChoiceView, itemId: number): WireIntent | null {
  switch (pending.kind) {
    case "may_yes_no":
      return { kind: "answer_may", player: pending.player, yes: itemId === 1 };
    case "choose_mode":
      return { kind: "choose_mode", player: pending.player, mode: itemId };
    case "choose_ability_targets":
    case "choose_spell_targets":
    case "choose_copy_target":
      return { kind: "choose_targets", player: pending.player, targets: [{ kind: "object", id: itemId }] };
    default:
      return null;
  }
}

function answerButton(testId: string, label: string, intent: WireIntent, primary: boolean): Html {
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", testId),
      h.OnClick(PendingChoiceAnswered({ intent })),
      h.Class("group relative cursor-pointer rounded-hud border-0 bg-transparent p-0"),
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

function faithfulPendingChoice(pending: PendingChoiceView, state: VisibleState, board: BoardModel): Html | null {
  switch (pending.kind) {
    case "order_triggers":
      return orderPrompt(pending, board);
    case "assign_combat_damage":
      return damageAssignPrompt(pending, state, board);
    case "search_library":
      return cardPickPrompt(pending, pending.items, state, board, {
        title: "Search your library",
        submitLabel: "Choose",
        declineLabel: "Fail to find",
      });
    case "scry":
      return cardPickPrompt(pending, pending.items, state, board, {
        title: `Scry ${pending.items.length}`,
        hint: "Click cards to keep on top, in that order — the rest go to the bottom of your library.",
        submitLabel: "Done",
        ordered: true,
      });
    case "surveil":
      return cardPickPrompt(pending, pending.items, state, board, {
        title: `Surveil ${pending.items.length}`,
        hint: "Click cards to keep on top, in that order — the rest go to your graveyard.",
        submitLabel: "Done",
        ordered: true,
      });
    case "select_from_top":
      return cardPickPrompt(pending, pending.items, state, board, {
        title: `Select up to ${pending.up_to} from the top`,
        hint: "Click cards to take — the rest go to the bottom.",
        submitLabel: "Done",
      });
    case "discard":
      return cardPickPrompt(pending, pending.items, state, board, {
        title: `Discard ${pending.count} card${pending.count === 1 ? "" : "s"}`,
        submitLabel: "Discard",
      });
    case "sacrifice_edict":
      return cardPickPrompt(pending, pending.items, state, board, {
        title: pending.keep_one ? "Choose permanents to sacrifice (keep one)" : "Choose a permanent to sacrifice",
        submitLabel: "Sacrifice",
      });
    case "choose_own_sacrifices":
      return cardPickPrompt(pending, pending.items, state, board, {
        title: `Choose ${pending.count} permanent${pending.count === 1 ? "" : "s"} to sacrifice`,
        submitLabel: "Sacrifice",
      });
    case "proliferate":
      return cardPickPrompt(pending, pending.items, state, board, {
        title: "Proliferate — choose any number",
        submitLabel: "Proliferate",
      });
    default:
      return null;
  }
}

function stubPendingChoice(
  pending: PendingChoiceView,
  state: VisibleState,
  board: BoardModel,
  tableId: string | null,
): Html {
  const items = pendingChoiceItems(pending);
  if (pending.kind === "choose_target" && chooseTargetIsCardPick(items)) {
    return cardPickPrompt(pending, pending.items, state, board, {
      title: pending.label,
      submitLabel: "Choose",
    });
  }
  const yesNo = pending.kind === "may_yes_no";
  const buttons: Html[] = [];
  if (yesNo) {
    if (tableId != null) {
      buttons.push(
        answerButton("prompt-yes", "Yes", { kind: "answer_may", player: pending.player, yes: true }, true),
        answerButton("prompt-no", "No", { kind: "answer_may", player: pending.player, yes: false }, false),
      );
    }
  } else if (pending.kind === "choose_mode" && "labels" in pending) {
    for (let i = 0; i < pending.labels.length; i++) {
      const label = pending.labels[i] ?? `Mode ${i}`;
      if (tableId == null) continue;
      buttons.push(
        answerButton(`prompt-mode-${i}`, label, { kind: "choose_mode", player: pending.player, mode: i }, false),
      );
    }
  } else {
    for (const item of items) {
      const intent = pendingChoiceIntent(pending, item.id);
      if (intent == null || tableId == null) continue;
      buttons.push(answerButton(`prompt-item-${item.id}`, item.label, intent, false));
    }
  }
  const title = pendingChoiceLabel(pending);
  return frame("pending-choice", title === "" ? `Engine prompt (${pending.kind})` : title, [
    h.div([h.Class("flex flex-col gap-1")], buttons),
    !isFaithfulPromptKind(pending.kind)
      ? h.div([h.Class("text-caption text-mist")], ["Limited UI — pick an option"])
      : h.span([], []),
  ]);
}

function pendingChoicePrompt(
  pending: PendingChoiceView,
  state: VisibleState,
  board: BoardModel,
  tableId: string | null,
): Html {
  const faithful = faithfulPendingChoice(pending, state, board);
  if (faithful != null) return faithful;
  return stubPendingChoice(pending, state, board, tableId);
}

export function promptsView(board: BoardModel, state: VisibleState, tableId: string | null): Html | null {
  if (board.xPrompt != null) return boardXPrompt(board.xPrompt);
  if (board.modalCast != null) return modalPrompt(board.modalCast);
  if (board.sacrificePick != null) {
    return costPickPrompt(
      "sacrifice-pick",
      "Choose a permanent to sacrifice",
      board.sacrificePick.action.sacrifice_choices ?? [],
      state,
      (id) => SacrificeChosen({ objectId: id }),
    );
  }
  if (board.discardPick != null) {
    return costPickPrompt(
      "discard-pick",
      "Choose a card to discard",
      board.discardPick.action.discard_choices ?? [],
      state,
      (id) => DiscardChosen({ ids: [id] }),
    );
  }
  if (board.gyExilePick != null) {
    return costPickPrompt(
      "gy-exile-pick",
      "Choose cards to exile from graveyard",
      board.gyExilePick.action.graveyard_exile_choices ?? [],
      state,
      (id) => GyExileChosen({ ids: [id] }),
    );
  }
  if (board.staged != null) {
    const targets = stagedPickTargets(board.staged, state);
    if (targets != null) {
      return targetPickPrompt(stagedTargetTitle(board.staged), targets, state);
    }
  }
  if (state.pending_choice != null) return pendingChoicePrompt(state.pending_choice, state, board, tableId);
  return null;
}

export { isFaithfulPromptKind };
