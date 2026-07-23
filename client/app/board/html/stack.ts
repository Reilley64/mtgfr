// Stack overlay: right-edge card-art pile with staged ghost, dwell, hold timer, and expand.
//
// Faithful minimum of Solid `stack-overlay.tsx`. Aim arrows into stack objects stay on canvas
// fidelity-grind backlog. Hovering the overlay emits `StackDwellChanged` when the player has
// priority (dwell-suppresses helpless auto-resolve).

import { type Attribute, type Html, html } from "foldkit/html";
import { buttonClass } from "~/ui/buttonClass";
import { cardArt } from "~/ui/card-art";
import type { PlayerView, StackObjectView, VisibleState } from "~/wire/types";
import { stagedPickTargets } from "../action/targeting";
import {
  STACK_CARD_W,
  STACK_HORIZONTAL_MARGIN,
  STACK_OVERLAY_RIGHT,
  STACK_STRIP_MIN_PEEK,
  STACK_VERTICAL_RESERVED,
  stackCardH,
  stackExpandAvailable,
  stackFullPerRow,
  stackPeekFor,
  stackPresentation,
  stackStripPeek,
  TARGET_COLOR,
} from "../geometry/stackLayout";
import {
  InspectAuxHovered,
  type Message,
  StackCollapseClicked,
  StackDwellChanged,
  StackExpandClicked,
} from "../messages";
import type { BoardModel } from "../submodel";

const h = html<Message>();

type StackItem = {
  row: number;
  source: number;
  imageName: string | null;
  print: string;
  cardId?: string;
  label: string;
  staged: boolean;
};

function objectMeta(state: VisibleState, source: number): { print: string; name: string | null; cardId?: string } {
  const obj = state.objects.find((o) => o.id === source);
  return { print: obj?.print ?? "", name: obj?.name ?? null, cardId: obj?.card_id };
}

function targetLabel(target: StackObjectView["target"], state: VisibleState): string {
  if (target == null) return "";
  if (target.kind === "player") {
    const name =
      state.players.find((p: PlayerView) => p.player === target.player)?.username ?? `Seat ${target.player + 1}`;
    return ` → ${name}`;
  }
  const obj = state.objects.find((o) => o.id === target.id);
  return obj ? ` → ${obj.name}` : "";
}

function stackItems(board: BoardModel, state: VisibleState, showStaged: boolean): StackItem[] {
  const items: StackItem[] = state.stack.map((entry, row) => {
    const meta = objectMeta(state, entry.source);
    return {
      row,
      source: entry.source,
      imageName: entry.kind === "spell" ? entry.label : meta.name,
      print: meta.print,
      cardId: meta.cardId,
      label: entry.label,
      staged: false,
    };
  });
  if (showStaged && board.staged != null) {
    const card = board.staged.card;
    items.push({
      row: state.stack.length,
      source: card.id,
      imageName: card.name,
      print: card.print ?? "",
      cardId: card.card_id,
      label: card.name,
      staged: true,
    });
  }
  return items;
}

function stackFace(opts: {
  row: number;
  imageName: string | null;
  print: string;
  cardId?: string;
  label: string;
  isTop: boolean;
  staged?: boolean;
  cardH: number;
  style: Record<string, string>;
}): Html {
  const faceClass = [
    "absolute rounded-game shadow-hand",
    opts.staged ? "ring-2" : "",
    opts.isTop ? "group-hover/stack:shadow-[0_0_16px_rgba(255,215,106,0.4)]" : "",
  ]
    .filter((v) => v !== "")
    .join(" ");

  const faceStyle: Record<string, string> = { ...opts.style };
  if (opts.staged) faceStyle["--tw-ring-color"] = TARGET_COLOR;

  const art: Html =
    opts.imageName && opts.print
      ? cardArt(h, {
          print: opts.print,
          size: "large",
          alt: opts.imageName,
          className: "block rounded-game",
          style: { width: `${STACK_CARD_W}px`, height: `${opts.cardH}px` },
        })
      : h.div(
          [
            h.Class(
              "flex items-center justify-center rounded-game bg-forest-hud px-1 text-center font-semibold text-caption text-seafoam",
            ),
            h.Style({ width: `${STACK_CARD_W}px`, height: `${opts.cardH}px` }),
          ],
          [opts.label],
        );

  const faceAttrs: Attribute<Message>[] = [
    h.Class(faceClass),
    h.Style(faceStyle),
    h.DataAttribute("testid", `stack-face-${opts.row}`),
    h.Attribute("title", opts.imageName ?? opts.label),
  ];
  // Solid stack overlay: hover a face → Alt-inspect aux for that card.
  if (opts.imageName) {
    faceAttrs.push(
      h.OnMouseEnter(
        InspectAuxHovered({
          source: "stack",
          card: {
            name: opts.imageName,
            ...(opts.cardId ? { cardId: opts.cardId } : {}),
            ...(opts.print ? { print: opts.print } : {}),
          },
        }),
      ),
    );
    faceAttrs.push(h.OnMouseLeave(InspectAuxHovered({ source: "stack", card: null })));
  }

  return h.div(faceAttrs, [art]);
}

function holdBar(holdMs: number, holdPeak: number, show: boolean): Html | null {
  if (!show || holdMs <= 0) return null;
  const total = Math.max(holdPeak, holdMs, 1);
  const pct = Math.min(100, (holdMs / total) * 100);
  return h.div(
    [
      h.DataAttribute("testid", "stack-hold-bar"),
      h.Class(
        "pointer-events-none h-1.5 overflow-hidden rounded-full bg-white/15 opacity-0 transition-opacity duration-150 group-hover/stack:opacity-100",
      ),
      h.Style({ width: `${STACK_CARD_W}px` }),
      h.Attribute("aria-hidden", "true"),
    ],
    [
      h.div(
        [
          h.Class("h-full rounded-full bg-vine transition-[width] duration-150 ease-linear"),
          h.Style({ width: `${pct}%` }),
        ],
        [],
      ),
    ],
  );
}

function pileCaption(state: VisibleState, showStaged: boolean): Html | null {
  if (showStaged) {
    return h.div(
      [
        h.DataAttribute("testid", "stack-staged-hint"),
        h.Class("max-w-full text-center text-chip"),
        h.Style({ color: TARGET_COLOR, maxWidth: `${STACK_CARD_W}px` }),
      ],
      ["Choose a target"],
    );
  }
  const top = state.stack[state.stack.length - 1];
  if (top == null) return null;
  const target = top.target != null ? targetLabel(top.target, state) : "";
  const ability = top.kind === "ability" ? top.label : "";
  if (ability === "" && target === "") return null;
  return h.div(
    [
      h.DataAttribute("testid", "stack-top-caption"),
      h.Class("max-w-full text-center text-chip text-seafoam"),
      h.Style({ maxWidth: `${STACK_CARD_W}px` }),
    ],
    [
      ability !== "" ? h.div([h.Class("font-semibold")], [ability]) : null,
      target !== "" ? h.div([], [target]) : null,
    ].filter((v): v is Html => v !== null),
  );
}

function pileView(
  board: BoardModel,
  state: VisibleState,
  items: StackItem[],
  peek: number,
  cardH: number,
  showStaged: boolean,
  allowDwell: boolean,
): Html {
  const pileH = cardH + Math.max(0, items.length - 1) * peek;
  const holdMs = state.stack_hold_remaining_ms ?? 0;
  const holdPeak = board.stackHoldPeak;
  const showHold = holdMs > 0 && !showStaged;

  const faces = items
    .filter((item) => !board.hideCardIds.has(item.source))
    .map((item) => {
      const isTop = item.row === items.length - 1;
      return stackFace({
        row: item.row,
        imageName: item.imageName,
        print: item.print,
        cardId: item.cardId,
        label: item.label,
        isTop,
        staged: item.staged,
        cardH,
        style: {
          width: `${STACK_CARD_W}px`,
          bottom: `${item.row * peek}px`,
          "z-index": String(item.row),
          left: "0",
        },
      });
    });

  const showMagnifier = stackExpandAvailable(items.length, peek);

  const pileAttrs: Attribute<Message>[] = [
    h.DataAttribute("testid", "stack-overlay"),
    h.Class("group/stack pointer-events-auto fixed top-1/2 z-20 -translate-y-1/2"),
    h.Style({
      right: `${STACK_OVERLAY_RIGHT}px`,
      width: `${STACK_CARD_W}px`,
      height: `${pileH}px`,
    }),
  ];
  if (allowDwell) {
    pileAttrs.push(h.OnMouseEnter(StackDwellChanged({ dwelling: true })));
    pileAttrs.push(h.OnMouseLeave(StackDwellChanged({ dwelling: false })));
  }

  return h.div(pileAttrs, [
    h.div(
      [h.Class("relative h-full w-full")],
      [
        ...faces,
        showMagnifier
          ? h.button(
              [
                h.Type("button"),
                h.DataAttribute("testid", "stack-expand"),
                h.OnClick(StackExpandClicked()),
                h.Class(
                  buttonClass(
                    "ghost",
                    "absolute -top-9 right-0 flex items-center gap-1 px-2 py-1 text-chip text-seafoam",
                  ),
                ),
                h.Attribute("aria-label", `Expand stack (${items.length} objects)`),
              ],
              [`Expand · ${items.length}`],
            )
          : null,
      ],
    ),
    h.div(
      [h.Class("absolute top-full right-0 left-0 mt-sm flex flex-col items-center gap-sm")],
      [holdBar(holdMs, holdPeak, showHold), pileCaption(state, showStaged)].filter((v): v is Html => v !== null),
    ),
  ]);
}

function stripView(
  board: BoardModel,
  state: VisibleState,
  items: StackItem[],
  mode: "expanded" | "full",
  showStaged: boolean,
  allowDwell: boolean,
): Html {
  const viewportW = board.viewport.width;
  const n = items.length;
  const hPeek = mode === "full" ? STACK_STRIP_MIN_PEEK : Math.max(STACK_STRIP_MIN_PEEK, stackStripPeek(n, viewportW));
  const perRow = mode === "full" ? stackFullPerRow(viewportW) : n;
  const rows = Math.ceil(n / perRow);
  const cardH = stackCardH();
  const cols = Math.min(n, perRow);
  const stripW = STACK_CARD_W + Math.max(0, cols - 1) * hPeek;
  const stripH = cardH + Math.max(0, rows - 1) * (cardH * 0.35);
  const holdMs = state.stack_hold_remaining_ms ?? 0;
  const holdPeak = board.stackHoldPeak;
  const showHold = holdMs > 0 && !showStaged;

  const faces = items
    .filter((item) => !board.hideCardIds.has(item.source))
    .map((item) => {
      const col = item.row % perRow;
      const rowY = Math.floor(item.row / perRow);
      const isTop = item.row === n - 1;
      return stackFace({
        row: item.row,
        imageName: item.imageName,
        print: item.print,
        cardId: item.cardId,
        label: item.label,
        isTop,
        staged: item.staged,
        cardH,
        style: {
          width: `${STACK_CARD_W}px`,
          left: `${col * hPeek}px`,
          top: `${rowY * cardH * 0.35}px`,
          "z-index": String(item.row),
        },
      });
    });

  const positionClass =
    mode === "full" ? "top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2" : "top-1/2 right-4 -translate-y-1/2";

  const stripAttrs: Attribute<Message>[] = [
    h.DataAttribute("testid", "stack-overlay-expanded"),
    h.Class(`group/stack pointer-events-auto fixed z-20 flex flex-col items-center gap-sm ${positionClass}`),
    h.Style({
      width: `${Math.min(viewportW - STACK_HORIZONTAL_MARGIN, stripW)}px`,
      maxWidth: `${viewportW - STACK_HORIZONTAL_MARGIN}px`,
    }),
  ];
  if (allowDwell) {
    stripAttrs.push(h.OnMouseEnter(StackDwellChanged({ dwelling: true })));
    stripAttrs.push(h.OnMouseLeave(StackDwellChanged({ dwelling: false })));
  }

  return h.div(stripAttrs, [
    h.div(
      [h.Class("flex w-full items-center justify-between gap-sm")],
      [
        h.span([h.Class("text-chip text-seafoam")], [`Stack · ${n}${mode === "full" ? " · full" : ""}`]),
        h.button(
          [
            h.Type("button"),
            h.DataAttribute("testid", "stack-collapse"),
            h.OnClick(StackCollapseClicked()),
            h.Class(buttonClass("ghost", "px-2 py-1 text-chip")),
            h.Attribute("aria-label", "Collapse stack"),
          ],
          ["✕"],
        ),
      ],
    ),
    h.div([h.Class("relative"), h.Style({ width: `${stripW}px`, height: `${stripH}px` })], faces),
    holdBar(holdMs, holdPeak, showHold),
    pileCaption(state, showStaged),
  ]);
}

/** Dwell suppresses helpless auto-resolve — only meaningful when the viewer has priority and
 * the stack is non-empty. Same policy as Solid stack-overlay `allowDwell`. */
function shouldEmitDwell(_board: BoardModel, state: VisibleState): boolean {
  if (state.stack.length === 0) return false;
  return state.can_act && state.priority === state.viewer;
}

export function stackView(board: BoardModel, state: VisibleState): Html | null {
  const showStaged = board.staged != null && stagedPickTargets(board.staged, state) === null;
  const items = stackItems(board, state, showStaged);
  if (items.length === 0) return null;

  const peek = stackPeekFor(items.length, board.viewport.height, STACK_VERTICAL_RESERVED);
  const presentation = stackPresentation({
    count: items.length,
    expandedOpen: board.stackExpand,
    viewportW: board.viewport.width,
    viewportH: board.viewport.height,
  });
  const allowDwell = shouldEmitDwell(board, state);
  const cardH = stackCardH();

  if (presentation === "pile") {
    return pileView(board, state, items, peek, cardH, showStaged, allowDwell);
  }
  return stripView(board, state, items, presentation, showStaged, allowDwell);
}
