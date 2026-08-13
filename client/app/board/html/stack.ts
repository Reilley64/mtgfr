// Stack overlay: right-edge compact fan with staged ghost, dwell, hold timer, and expansion.
//
// Legal stack targets are clickable while arrow-aiming (Counterspell-style). Hovering the overlay
// emits `StackDwellChanged` when the player has priority (dwell-suppresses helpless auto-resolve).
// Resting faces hide only for in-flight `kind: "stack"` objects — not for battlefield flights that
// share an ability's source permanent id.

import { Option } from "effect";
import type { Attribute, Html, HtmlBuilder } from "foldkit/html";
import { BLANK_FACE, type FaceData, faceDataFrom, faceDataFromStackSource } from "~/card-render/frame";
import { cardTextFor } from "~/cardText";
import { button } from "~/ui/button";
import { cardFace } from "~/ui/card-face";
import type { ObjectView, VisibleState } from "~/wire/types";
import { formatMessage } from "../../domain/i18n/message";
import { aimingObjectIds, pendingStackGhost, stagedPickTargets } from "../action/targeting";
import {
  stackExpandedLayout,
  stackFanLayout,
  stackFanPlacement,
  stackOverflowBadgeLayout,
  stackPresentation,
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
  kind: string;
  /** Production entry identity. Local staged/pending faces deliberately have no authoritative id. */
  entryId?: bigint;
  source?: number;
  imageName: string | null;
  print: string;
  cardId?: string;
  label: string;
  printedSentences: readonly string[];
  staged: boolean;
  /** The rendered face for every stack entry, including metadata-free tombstones. */
  face: FaceData;
  accessibleDescription?: string;
};

/** Hide a resting stack face only while a *stack* flight owns that object id.
 * Ability entries reuse the source permanent's id — a battlefield / from-stack flight for that
 * permanent must not blank the ability face (ETB triggers would otherwise show only the effect
 * caption). */
function hideStackRestingFace(board: BoardModel, item: StackItem): boolean {
  if (item.kind !== "spell" || item.source == null) return false;
  const flight = board.flights.get(item.source);
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
    return {
      ...face,
      typeLine: text.type_line,
      oracle: text.oracle,
      flavor: text.flavor,
    };
  };
  const faceOf = (view: ObjectView): FaceData => withText(faceDataFrom(view), view.card_id, view.print ?? "");

  const items: StackItem[] = state.stack.map((entry, row) => {
    const object = entry.source == null ? undefined : state.objects.find((o) => o.id === entry.source);
    const label = formatMessage(entry.label);
    // Prefer the live object; fall back to entry-carried identity when `source` is a Moved
    // tombstone (sacrifice-as-cost) omitted from `objects`.
    const print = object?.print || entry.print || "";
    const name = object?.name || entry.name || null;
    // Source-less entries must not trigger catalog/card-default inference from an otherwise explicit card id.
    const cardId = entry.source == null ? undefined : object?.card_id || entry.card_id || undefined;
    // A tombstone is gone from `objects`, so its own identity is all there is to draw a face from.
    // When that identity is unavailable, public stack text still deserves a neutral card face.
    const baseFace =
      object != null
        ? faceOf(object)
        : entry.source_face != null
          ? withText(faceDataFromStackSource(entry.source_face, print, name ?? label), cardId, print)
          : entry.source == null
            ? { ...BLANK_FACE, print, name: name ?? label }
            : withText({ ...BLANK_FACE, print, name: name ?? label }, cardId, print);
    const spellFace =
      entry.active_face_text == null
        ? baseFace
        : {
            ...baseFace,
            typeLine: entry.active_face_text.type_line,
            oracle: entry.active_face_text.oracle,
            flavor: entry.active_face_text.flavor,
          };
    const printedSentences = entry.printed_sentences ?? [];
    const abilityOracle = entry.ability_oracle || printedSentences.join("\n") || label;
    return {
      row,
      kind: entry.kind,
      entryId: entry.entry_id,
      ...(entry.source == null ? {} : { source: entry.source }),
      imageName: entry.kind === "spell" ? label : name,
      print,
      cardId,
      label,
      printedSentences,
      staged: false,
      // An ability on the stack is the one sentence that prints it, not its source card's whole
      // text box; the flavor belongs to the card, so it goes with the rest of the card's words.
      face: entry.kind === "ability" ? { ...baseFace, oracle: abilityOracle, flavor: "" } : spellFace,
      accessibleDescription:
        entry.kind === "ability" ? [name, ...printedSentences, abilityOracle].filter(Boolean).join(": ") : undefined,
    };
  });
  if (!showGhost) return items;

  // Local staged cast/activate wins over a pending ghost (both should not be live together).
  if (board.staged != null && stagedPickTargets(board.staged, state) === null) {
    const card = board.staged.card;
    items.push({
      row: state.stack.length,
      kind: "spell",
      source: card.id,
      imageName: card.name,
      print: card.print ?? "",
      cardId: card.card_id,
      label: card.name,
      printedSentences: [],
      staged: true,
      face: faceOf(card),
    });
    return items;
  }

  const pending = pendingStackGhost(state);
  if (pending != null) {
    items.push({
      row: state.stack.length,
      kind: "spell",
      source: pending.id,
      imageName: pending.name,
      print: pending.print ?? "",
      cardId: pending.card_id,
      label: pending.name,
      printedSentences: [],
      staged: true,
      face: faceOf(pending),
    });
  }
  return items;
}

function stackFace(
  opts: {
    row: number;
    entryId?: bigint;
    source?: number;
    imageName: string | null;
    print: string;
    cardId?: string;
    label: string;
    face: FaceData;
    accessibleDescription?: string;
    printedSentences: readonly string[];
    isTop: boolean;
    staged?: boolean;
    legalTarget?: boolean;
    expandOnActivate?: boolean;
    cardW: number;
    cardH: number;
    /** Caller-specific placement utilities reading the CSS vars in `style` (`--x`/`--y`/`--rotation`/`--z`). */
    positionClass: string;
    /** Placement data only (CSS variables); sizes come from `--stack-w`/`--card-h` on the container. */
    style: Record<string, string>;
  },
  h: HtmlBuilder<Message>,
): Html {
  const faceClass = [
    "group/stack-face pointer-events-auto absolute w-(--stack-w) rounded-game shadow-hand",
    "data-[legal-target=true]:cursor-pointer data-[legal-target=true]:ring-2 data-[legal-target=true]:ring-island-blue",
    "data-[staged=true]:ring-2 data-[staged=true]:ring-island-blue",
    opts.isTop ? "group-hover/stack:shadow-[0_0_16px_rgba(255,215,106,0.4)]" : "",
    opts.positionClass,
  ]
    .filter((v) => v !== "")
    .join(" ");

  // The whole printed card, not a crop of its art — the stack is where a player reads what is
  // about to resolve, so it shows the same rendered face the hand bar does.
  const cardBody = cardFace(h, {
    face: opts.face,
    accessibleDescription: opts.accessibleDescription,
    width: opts.cardW,
    height: opts.cardH,
    className: "block h-(--card-h) w-(--stack-w) rounded-game",
  });

  const accessibleParts: string[] = [];
  for (const part of [opts.imageName, opts.label, ...opts.printedSentences]) {
    if (part == null || accessibleParts.includes(part)) continue;
    accessibleParts.push(part);
  }
  const accessibleLabel = accessibleParts.join(" ");
  const isLegalTarget = opts.legalTarget && opts.source != null;
  const faceAttrs: Attribute<Message>[] = [
    h.Class(faceClass),
    h.Style(opts.style),
    h.Key(opts.entryId == null ? `local-${opts.row}` : String(opts.entryId)),
    h.DataAttribute("testid", `stack-face-${opts.row}`),
    ...(opts.entryId == null ? [] : [h.DataAttribute("stack-entry-id", String(opts.entryId))]),
    ...(opts.cardId == null ? [] : [h.DataAttribute("inspect-card-id", opts.cardId)]),
    h.Attribute("title", opts.imageName ?? opts.label),
    h.Role(isLegalTarget ? "button" : "group"),
    h.Attribute("aria-label", isLegalTarget ? `Target: ${accessibleLabel}` : accessibleLabel),
  ];
  if (opts.staged) {
    faceAttrs.push(h.DataAttribute("staged", "true"));
  }
  if (isLegalTarget) {
    faceAttrs.push(h.DataAttribute("legal-target", "true"));
    // Legal targeting takes precedence over compact-fan expansion for both pointer and keyboard.
    faceAttrs.push(h.Tabindex(0));
    faceAttrs.push(h.OnClick(TargetChosen({ target: { kind: "object", id: opts.source } })));
    faceAttrs.push(
      h.OnKeyDownPreventDefault((key) => {
        if (key !== "Enter" && key !== " ") return Option.none();
        return Option.some(TargetChosen({ target: { kind: "object", id: opts.source } }));
      }),
    );
  } else if (opts.expandOnActivate) {
    faceAttrs.push(h.Role("button"));
    faceAttrs.push(h.Tabindex(0));
    faceAttrs.push(h.OnClick(StackExpandClicked()));
    faceAttrs.push(
      h.OnKeyDownPreventDefault((key) => {
        if (key !== "Enter" && key !== " ") return Option.none();
        return Option.some(StackExpandClicked());
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

function compactFanView(
  board: BoardModel,
  state: VisibleState,
  items: StackItem[],
  showStaged: boolean,
  allowDwell: boolean,
  legalTargets: ReadonlySet<number>,
  h: HtmlBuilder<Message>,
): Html {
  const layout = stackFanLayout(board.viewport, items.length);
  const visibleItems = items.slice(layout.visibleFrom);
  const overflowBadge = stackOverflowBadgeLayout(layout);
  const holdMs = state.stack_hold_remaining_ms ?? 0;
  const holdPeak = board.stackHoldPeak;
  const showHold = holdMs > 0 && !showStaged;

  const faces = visibleItems
    .filter((item) => !hideStackRestingFace(board, item))
    .map((item) => {
      const placement = stackFanPlacement(layout, item.row);
      if (placement == null) return null;
      return stackFace(
        {
          row: item.row,
          entryId: item.entryId,
          source: item.source,
          imageName: item.imageName,
          print: item.print,
          cardId: item.cardId,
          label: item.label,
          face: item.face,
          accessibleDescription: item.accessibleDescription,
          printedSentences: item.printedSentences,
          isTop: item.row === items.length - 1,
          staged: item.staged,
          legalTarget: !item.staged && item.source != null && legalTargets.has(item.source),
          expandOnActivate: layout.hiddenCount > 0,
          cardW: layout.cardW,
          cardH: layout.cardH,
          positionClass: "top-0 left-0 z-(--z) translate-x-(--x) translate-y-(--y) rotate-(--rotation)",
          style: {
            "--x": `${placement.x}px`,
            "--y": `${placement.y}px`,
            "--rotation": `${placement.rotation}deg`,
            "--z": String(item.row),
          },
        },
        h,
      );
    })
    .filter((face): face is Html => face !== null);

  const fanAttrs: Attribute<Message>[] = [
    h.DataAttribute("testid", "stack-overlay"),
    h.DataAttribute("presentation", "compact"),
    h.Class("group/stack pointer-events-none fixed inset-0 z-20"),
    h.Style({
      "--stack-w": `${layout.cardW}px`,
      "--card-h": `${layout.cardH}px`,
      "--fan-w": `${layout.fanW}px`,
      "--fan-left": `${layout.left}px`,
      "--fan-top": `${layout.top}px`,
      "--fan-bottom": `${layout.top + layout.cardH}px`,
      "--badge-left": `${overflowBadge.left}px`,
      "--badge-top": `${overflowBadge.top}px`,
      "--badge-w": `${overflowBadge.width}px`,
      "--badge-h": `${overflowBadge.height}px`,
    }),
  ];
  if (allowDwell) {
    fanAttrs.push(h.OnMouseEnter(StackDwellChanged({ dwelling: true })));
    fanAttrs.push(h.OnMouseLeave(StackDwellChanged({ dwelling: false })));
  }

  return h.div(fanAttrs, [
    ...faces,
    layout.hiddenCount > 0
      ? button(
          h,
          {
            testId: "stack-expand",
            onClick: StackExpandClicked(),
            variant: "ghost",
            class:
              "pointer-events-auto fixed top-(--badge-top) left-(--badge-left) z-10 h-(--badge-h) w-(--badge-w) px-2 py-1 text-chip text-seafoam",
            ariaLabel: `Show ${layout.hiddenCount} older stack objects`,
          },
          [`+${layout.hiddenCount}`],
        )
      : null,
    h.div(
      [
        h.Class(
          "pointer-events-none fixed top-(--fan-bottom) left-(--fan-left) mt-sm flex w-(--fan-w) flex-col items-center gap-sm",
        ),
      ],
      [holdBar(holdMs, holdPeak, showHold, h), pileCaption(state, showStaged, h)].filter(
        (value): value is Html => value !== null,
      ),
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
  const n = items.length;
  const layout = stackExpandedLayout({ presentation: mode, viewport: board.viewport, count: n });
  const holdMs = state.stack_hold_remaining_ms ?? 0;
  const holdPeak = board.stackHoldPeak;
  const showHold = holdMs > 0 && !showStaged;

  const faces = items
    .filter((item) => !hideStackRestingFace(board, item))
    .map((item) => {
      const col = item.row % layout.perRow;
      const rowY = Math.floor(item.row / layout.perRow);
      const isTop = item.row === n - 1;
      return stackFace(
        {
          row: item.row,
          entryId: item.entryId,
          source: item.source,
          imageName: item.imageName,
          print: item.print,
          cardId: item.cardId,
          label: item.label,
          face: item.face,
          accessibleDescription: item.accessibleDescription,
          printedSentences: item.printedSentences,
          isTop,
          staged: item.staged,
          legalTarget: !item.staged && item.source != null && legalTargets.has(item.source),
          cardW: layout.cardW,
          cardH: layout.cardH,
          positionClass: "top-(--y) left-(--x) z-(--z)",
          style: {
            "--x": `${col * layout.peek}px`,
            "--y": `${rowY * layout.rowStride}px`,
            "--z": String(item.row),
          },
        },
        h,
      );
    });

  const stripAttrs: Attribute<Message>[] = [
    h.DataAttribute("testid", "stack-overlay-expanded"),
    h.Class(
      "group/stack pointer-events-auto fixed top-(--expanded-top) left-(--expanded-left) z-20 flex w-(--strip-w) flex-col items-center gap-(--expanded-gap)",
    ),
    h.Style({
      "--stack-w": `${layout.cardW}px`,
      "--card-h": `${layout.cardH}px`,
      "--strip-w": `${layout.stripW}px`,
      "--strip-h": `${layout.stripH}px`,
      "--expanded-left": `${layout.left}px`,
      "--expanded-top": `${layout.top}px`,
      "--expanded-header-h": `${layout.headerH}px`,
      "--expanded-gap": `${layout.gap}px`,
      "--expanded-peek": `${layout.peek}px`,
      "--expanded-row-stride": `${layout.rowStride}px`,
    }),
  ];
  if (allowDwell) {
    stripAttrs.push(h.OnMouseEnter(StackDwellChanged({ dwelling: true })));
    stripAttrs.push(h.OnMouseLeave(StackDwellChanged({ dwelling: false })));
  }

  return h.div(stripAttrs, [
    h.div(
      [h.Class("flex h-(--expanded-header-h) w-full items-center justify-between gap-sm")],
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
        h.Style({ "--strip-w": `${layout.stripW}px`, "--strip-h": `${layout.stripH}px` }),
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

  const presentation = stackPresentation({
    count: items.length,
    expandedOpen: board.stackExpand,
    viewportW: board.viewport.width,
    viewportH: board.viewport.height,
  });
  const allowDwell = shouldEmitDwell(board, state);
  const legalTargets = aimingObjectIds(board.staged, state.pending_choice, state);

  if (presentation === "pile") {
    return compactFanView(board, state, items, showStaged, allowDwell, legalTargets, h);
  }
  return stripView(board, state, items, presentation, showStaged, allowDwell, legalTargets, h);
}
