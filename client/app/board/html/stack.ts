// Stack overlay: right-edge card pile with staged ghost, dwell, hold timer, and expand.
//
// Legal stack targets are clickable while arrow-aiming (Counterspell-style). Hovering the overlay
// emits `StackDwellChanged` when the player has priority (dwell-suppresses helpless auto-resolve).
// Resting faces hide only for in-flight `kind: "stack"` objects — not for battlefield flights that
// share an ability's source permanent id.

import { Option } from "effect";
import type { Attribute, Html, HtmlBuilder } from "foldkit/html";
import { type FaceData, faceDataFrom, faceDataFromStackSource } from "~/card-render/frame";
import { cardTextFor } from "~/cardText";
import { button } from "~/ui/button";
import { cardFace } from "~/ui/card-face";
import type { ObjectView, VisibleState } from "~/wire/types";
import { formatMessage } from "../../domain/i18n/message";
import { aimingObjectIds, pendingStackGhost, stagedPickTargets } from "../action/targeting";
import {
  STACK_CARD_W,
  STACK_HORIZONTAL_MARGIN,
  STACK_STRIP_MIN_PEEK,
  STACK_VERTICAL_RESERVED,
  stackCardH,
  stackExpandAvailable,
  stackFullPerRow,
  stackPeekFor,
  stackPresentation,
  stackStripPeek,
} from "../geometry/stackLayout";
import { formatStackTargetSuffix, stackEntryTargets } from "../geometry/stackTargets";
import {
  InspectAuxHovered,
  type Message,
  StackCollapseClicked,
  StackDwellChanged,
  StackExpandClicked,
  TargetChosen,
} from "../messages";
import type { BoardModel } from "../submodel";

type StackItem = {
  row: number;
  source: number;
  imageName: string | null;
  print: string;
  cardId?: string;
  label: string;
  staged: boolean;
  /** The rendered face, or null for a tombstone the snapshot no longer carries an object for. */
  face: FaceData | null;
};

/** Hide a resting stack face only while a *stack* flight owns that object id.
 * Ability entries reuse the source permanent's id — a battlefield / from-stack flight for that
 * permanent must not blank the ability face (ETB triggers would otherwise show only the effect
 * caption). */
function hideStackRestingFace(board: BoardModel, source: number): boolean {
  const flight = board.flights.get(source);
  if (flight == null || flight.kind !== "stack") return false;
  // Any in-model stack flight still owns the face — including settled frames before FlightsSynced
  // drops it. Revealing HTML while the canvas flight is still painted reads as a short second ease.
  return true;
}

function stackItems(board: BoardModel, state: VisibleState, showGhost: boolean): StackItem[] {
  /** The catalog's words folded into a face once its lookup lands — as the hand bar does. */
  const withText = (face: FaceData, cardId: string | undefined, print: string): FaceData => {
    const text = cardTextFor(board.cardText, cardId, print);
    if (text == null) return face;
    return { ...face, typeLine: text.type_line, oracle: text.oracle, flavor: text.flavor };
  };
  const faceOf = (view: ObjectView): FaceData => withText(faceDataFrom(view), view.card_id, view.print ?? "");

  const items: StackItem[] = state.stack.map((entry, row) => {
    const object = state.objects.find((o) => o.id === entry.source);
    const label = formatMessage(entry.label);
    // Prefer the live object; fall back to entry-carried identity when `source` is a Moved
    // tombstone (sacrifice-as-cost) omitted from `objects`.
    const print = object?.print || entry.print || "";
    const name = object?.name || entry.name || null;
    const cardId = object?.card_id || entry.card_id || undefined;
    // A tombstone is gone from `objects`, so its own identity is all there is to draw a face from.
    const face =
      object != null
        ? faceOf(object)
        : print && entry.source_face != null
          ? withText(faceDataFromStackSource(entry.source_face, print, name ?? ""), cardId, print)
          : null;
    return {
      row,
      source: entry.source,
      imageName: entry.kind === "spell" ? label : name,
      print,
      cardId,
      label,
      staged: false,
      // An ability on the stack is the one sentence that prints it, not its source card's whole
      // text box; the flavor belongs to the card, so it goes with the rest of the card's words.
      face:
        face != null && entry.kind === "ability"
          ? { ...face, oracle: entry.ability_oracle || label, flavor: "" }
          : face,
    };
  });
  if (!showGhost) return items;

  // Local staged cast/activate wins over a pending ghost (both should not be live together).
  if (board.staged != null && stagedPickTargets(board.staged, state) === null) {
    const card = board.staged.card;
    items.push({
      row: state.stack.length,
      source: card.id,
      imageName: card.name,
      print: card.print ?? "",
      cardId: card.card_id,
      label: card.name,
      staged: true,
      face: faceOf(card),
    });
    return items;
  }

  const pending = pendingStackGhost(state);
  if (pending != null) {
    items.push({
      row: state.stack.length,
      source: pending.id,
      imageName: pending.name,
      print: pending.print ?? "",
      cardId: pending.card_id,
      label: pending.name,
      staged: true,
      face: faceOf(pending),
    });
  }
  return items;
}

function stackFace(
  opts: {
    row: number;
    source: number;
    imageName: string | null;
    print: string;
    cardId?: string;
    label: string;
    face: FaceData | null;
    isTop: boolean;
    staged?: boolean;
    legalTarget?: boolean;
    cardH: number;
    /** Caller-specific placement utilities reading the CSS vars in `style` (`--b`/`--x`/`--y`/`--z`). */
    positionClass: string;
    /** Placement data only (CSS variables); sizes come from `--stack-w`/`--card-h` on the container. */
    style: Record<string, string>;
  },
  h: HtmlBuilder<Message>,
): Html {
  const faceClass = [
    "group/stack-face absolute w-(--stack-w) rounded-game shadow-hand",
    "data-[legal-target=true]:cursor-pointer data-[legal-target=true]:ring-2 data-[legal-target=true]:ring-island-blue",
    "data-[staged=true]:ring-2 data-[staged=true]:ring-island-blue",
    opts.isTop ? "group-hover/stack:shadow-[0_0_16px_rgba(255,215,106,0.4)]" : "",
    opts.positionClass,
  ]
    .filter((v) => v !== "")
    .join(" ");

  // The whole printed card, not a crop of its art — the stack is where a player reads what is
  // about to resolve, so it shows the same rendered face the hand bar does.
  const cardBody: Html =
    opts.face && opts.print
      ? cardFace(h, {
          face: opts.face,
          width: STACK_CARD_W,
          height: opts.cardH,
          className: "block h-(--card-h) w-(--stack-w) rounded-game",
        })
      : h.div(
          [
            h.Class(
              "flex h-(--card-h) w-(--stack-w) items-center justify-center rounded-game bg-forest-hud px-1 text-center font-semibold text-caption text-seafoam",
            ),
          ],
          [opts.label],
        );

  const faceAttrs: Attribute<Message>[] = [
    h.Class(faceClass),
    h.Style(opts.style),
    h.DataAttribute("testid", `stack-face-${opts.row}`),
    h.Attribute("title", opts.imageName ?? opts.label),
  ];
  if (opts.staged) {
    faceAttrs.push(h.DataAttribute("staged", "true"));
  }
  if (opts.legalTarget) {
    faceAttrs.push(h.DataAttribute("legal-target", "true"));
    // Legal targets are real controls: click AND keyboard pick the target.
    faceAttrs.push(h.Role("button"));
    faceAttrs.push(h.Tabindex(0));
    faceAttrs.push(h.Attribute("aria-label", `Target: ${opts.imageName ?? opts.label}`));
    faceAttrs.push(h.OnClick(TargetChosen({ target: { kind: "object", id: opts.source } })));
    faceAttrs.push(
      h.OnKeyDownPreventDefault((key) => {
        if (key !== "Enter" && key !== " ") return Option.none();
        return Option.some(TargetChosen({ target: { kind: "object", id: opts.source } }));
      }),
    );
  }
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

  return h.div(faceAttrs, [cardBody]);
}

function holdBar(holdMs: number, holdPeak: number, show: boolean, h: HtmlBuilder<Message>): Html | null {
  if (!show || holdMs <= 0) return null;
  const total = Math.max(holdPeak, holdMs, 1);
  const pct = Math.min(100, (holdMs / total) * 100);
  return h.div(
    [
      h.DataAttribute("testid", "stack-hold-bar"),
      h.Class("pointer-events-none h-1.5 w-(--stack-w) overflow-hidden rounded-full bg-white/15"),
      h.Attribute("aria-hidden", "true"),
    ],
    [
      h.div(
        [
          h.Class("h-full w-(--w) rounded-full bg-vine transition-[width] duration-150 ease-linear"),
          h.Style({ "--w": `${pct}%` }),
        ],
        [],
      ),
    ],
  );
}

function pileCaption(state: VisibleState, showStaged: boolean, h: HtmlBuilder<Message>): Html | null {
  if (showStaged) {
    return h.div(
      [
        h.DataAttribute("testid", "stack-staged-hint"),
        h.Class("max-w-(--stack-w) text-center text-chip text-island-blue"),
      ],
      ["Choose a target"],
    );
  }
  const top = state.stack[state.stack.length - 1];
  if (top == null) return null;
  const target = formatStackTargetSuffix(stackEntryTargets(top), state);
  if (target === "") return null;
  return h.div(
    [h.DataAttribute("testid", "stack-top-caption"), h.Class("max-w-(--stack-w) text-center text-chip text-seafoam")],
    [h.div([], [target])],
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
  legalTargets: ReadonlySet<number>,
  h: HtmlBuilder<Message>,
): Html {
  const pileH = cardH + Math.max(0, items.length - 1) * peek;
  const holdMs = state.stack_hold_remaining_ms ?? 0;
  const holdPeak = board.stackHoldPeak;
  const showHold = holdMs > 0 && !showStaged;

  const faces = items
    .filter((item) => !hideStackRestingFace(board, item.source))
    .map((item) => {
      const isTop = item.row === items.length - 1;
      return stackFace(
        {
          row: item.row,
          source: item.source,
          imageName: item.imageName,
          print: item.print,
          cardId: item.cardId,
          label: item.label,
          face: item.face,
          isTop,
          staged: item.staged,
          legalTarget: !item.staged && legalTargets.has(item.source),
          cardH,
          positionClass: "bottom-(--b) left-0 z-(--z)",
          style: {
            "--b": `${item.row * peek}px`,
            "--z": String(item.row),
          },
        },
        h,
      );
    });

  const showMagnifier = stackExpandAvailable(items.length, peek);

  const pileAttrs: Attribute<Message>[] = [
    h.DataAttribute("testid", "stack-overlay"),
    h.Class("group/stack pointer-events-auto fixed top-1/2 right-4 z-20 h-(--pile-h) w-(--stack-w) -translate-y-1/2"),
    h.Style({
      "--stack-w": `${STACK_CARD_W}px`,
      "--card-h": `${cardH}px`,
      "--pile-h": `${pileH}px`,
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
          ? button(
              h,
              {
                testId: "stack-expand",
                onClick: StackExpandClicked(),
                variant: "ghost",
                class: "absolute -top-9 right-0 flex items-center gap-1 px-2 py-1 text-chip text-seafoam",
                ariaLabel: `Expand stack (${items.length} objects)`,
              },
              [`Expand · ${items.length}`],
            )
          : null,
      ],
    ),
    h.div(
      [h.Class("absolute top-full right-0 left-0 mt-sm flex flex-col items-center gap-sm")],
      [holdBar(holdMs, holdPeak, showHold, h), pileCaption(state, showStaged, h)].filter((v): v is Html => v !== null),
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
  legalTargets: ReadonlySet<number>,
  h: HtmlBuilder<Message>,
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
    .filter((item) => !hideStackRestingFace(board, item.source))
    .map((item) => {
      const col = item.row % perRow;
      const rowY = Math.floor(item.row / perRow);
      const isTop = item.row === n - 1;
      return stackFace(
        {
          row: item.row,
          source: item.source,
          imageName: item.imageName,
          print: item.print,
          cardId: item.cardId,
          label: item.label,
          face: item.face,
          isTop,
          staged: item.staged,
          legalTarget: !item.staged && legalTargets.has(item.source),
          cardH,
          positionClass: "top-(--y) left-(--x) z-(--z)",
          style: {
            "--x": `${col * hPeek}px`,
            "--y": `${rowY * cardH * 0.35}px`,
            "--z": String(item.row),
          },
        },
        h,
      );
    });

  const positionClass =
    mode === "full" ? "top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2" : "top-1/2 right-4 -translate-y-1/2";

  const stripAttrs: Attribute<Message>[] = [
    h.DataAttribute("testid", "stack-overlay-expanded"),
    h.Class(
      `group/stack pointer-events-auto fixed z-20 flex w-(--strip-cap) max-w-(--strip-max) flex-col items-center gap-sm ${positionClass}`,
    ),
    h.Style({
      "--stack-w": `${STACK_CARD_W}px`,
      "--card-h": `${cardH}px`,
      "--strip-cap": `${Math.min(viewportW - STACK_HORIZONTAL_MARGIN, stripW)}px`,
      "--strip-max": `${viewportW - STACK_HORIZONTAL_MARGIN}px`,
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
        button(
          h,
          {
            testId: "stack-collapse",
            onClick: StackCollapseClicked(),
            variant: "ghost",
            class: "hit-quiet px-2 py-1 text-chip",
            ariaLabel: "Collapse stack",
          },
          ["✕"],
        ),
      ],
    ),
    h.div(
      [
        h.Class("relative h-(--strip-h) w-(--strip-w)"),
        h.Style({ "--strip-w": `${stripW}px`, "--strip-h": `${stripH}px` }),
      ],
      faces,
    ),
    holdBar(holdMs, holdPeak, showHold, h),
    pileCaption(state, showStaged, h),
  ]);
}

/** Dwell suppresses helpless auto-resolve — only meaningful when the viewer has priority and
 * the stack is non-empty. Same policy as Solid stack-overlay `allowDwell`. */
function shouldEmitDwell(_board: BoardModel, state: VisibleState): boolean {
  if (state.stack.length === 0) return false;
  return state.can_act && state.priority === state.viewer;
}

/** Local staged aim or pending board-aim source that needs a stack ghost. */
function showStackGhost(board: BoardModel, state: VisibleState): boolean {
  if (board.staged != null && stagedPickTargets(board.staged, state) === null) return true;
  return pendingStackGhost(state) != null;
}

export function stackView(board: BoardModel, state: VisibleState, h: HtmlBuilder<Message>): Html | null {
  const showStaged = showStackGhost(board, state);
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
  const legalTargets = aimingObjectIds(board.staged, state.pending_choice, state);

  if (presentation === "pile") {
    return pileView(board, state, items, peek, cardH, showStaged, allowDwell, legalTargets, h);
  }
  return stripView(board, state, items, presentation, showStaged, allowDwell, legalTargets, h);
}
