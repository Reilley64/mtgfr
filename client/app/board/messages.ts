import { Schema as S } from "effect";
import { m } from "foldkit/message";
import type {
  ActionView as ActionViewT,
  CatalogCard as CatalogCardT,
  WireIntent as WireIntentT,
  WireTarget as WireTargetT,
} from "~/wire/types";

const CanvasPoint = { x: S.Number, y: S.Number };
const WireTarget: S.Schema<WireTargetT> = S.Any;
const WireIntent: S.Schema<WireIntentT> = S.Any;
const ActionView: S.Schema<ActionViewT> = S.Any;
const CatalogCard: S.Schema<CatalogCardT | null> = S.Any;
const FlightPhase = S.Union([S.Literal("flying"), S.Literal("settled")]);
const FlightKind = S.Union([S.Literal("battlefield"), S.Literal("stack"), S.Literal("from-stack")]);
const CardFlight = S.Struct({
  id: S.Number,
  print: S.String,
  name: S.String,
  x: S.Number,
  y: S.Number,
  scale: S.Number,
  targetX: S.Number,
  targetY: S.Number,
  targetScale: S.Number,
  phase: FlightPhase,
  kind: FlightKind,
  fromCardId: S.optional(S.Number),
});

export const ArtLoaded = m("ArtLoaded");
export const BoardPointerDown = m("BoardPointerDown", CanvasPoint);
export const BoardPointerMove = m("BoardPointerMove", CanvasPoint);
export const BoardPointerUp = m("BoardPointerUp", CanvasPoint);
export const FlightsSynced = m("FlightsSynced", { now: S.Number, flights: S.Array(CardFlight) });

/** User activated a hand/command/graveyard/exile bar action (click / Enter / Space / drop above threshold). */
export const HandActionActivated = m("HandActionActivated", {
  action: ActionView,
  /** Screen drop point; if omitted, submodel uses viewport center. */
  x: S.optional(S.Number),
  y: S.optional(S.Number),
});

/** Window-captured hand-bar drag started on a playable tile. */
export const HandDragStarted = m("HandDragStarted", {
  action: ActionView,
  name: S.String,
  print: S.String,
  manaCost: S.Any,
  kind: S.optional(S.String),
  x: S.Number,
  y: S.Number,
});
/** Hand drag ghost follows the cursor. */
export const HandDragMoved = m("HandDragMoved", { x: S.Number, y: S.Number });
/** Hand drag released — plays when above the hand-bar threshold. */
export const HandDragEnded = m("HandDragEnded", { x: S.Number, y: S.Number });
/** Hand or radial wedge hover — drives auto-tap payment preview on the battlefield. */
export const HandActionHovered = m("HandActionHovered", { actionId: S.NullOr(S.Number) });

/** Primary board button click (Next / confirm attackers / confirm blockers). */
export const PrimaryClicked = m("PrimaryClicked");
/** One-shot Resolve card (pass_priority while stack owns priority). */
export const PassClicked = m("PassClicked");
/** Pre-game mulligan: keep the current opening hand. */
export const KeepHandClicked = m("KeepHandClicked");
/** Pre-game mulligan: shuffle and redraw to the next hand size. */
export const MulliganClicked = m("MulliganClicked");
/** Arm Resolve stack (stack yield). */
export const StackYieldArmed = m("StackYieldArmed");
/** Toggle Arena End Turn / Turn Yield rocker. */
export const TurnYieldToggled = m("TurnYieldToggled", { enabled: S.Boolean });
/** Escape / Cancel — clear staged/xPrompt/modal/cost picks. */
export const CancelActionClicked = m("CancelActionClicked");

/** Commander (or any object) cast from board click. */
export const CommanderCastClicked = m("CommanderCastClicked", { objectId: S.Number });
/** Board click on a legal target of the staged action. */
export const TargetChosen = m("TargetChosen", { target: WireTarget });

/** Modal step answers. */
export const ModalModesChosen = m("ModalModesChosen", { chosen: S.Array(S.Number) });
export const ModalTargetChosen = m("ModalTargetChosen", { target: WireTarget });

/** Choose-X stepper draft adjusted (Min / − / + / Max). */
export const XDraftSet = m("XDraftSet", { x: S.Number });
/** X cost submitted. */
export const XSubmitted = m("XSubmitted", { x: S.Number });

/** Cost-pick answers. */
export const SacrificeChosen = m("SacrificeChosen", { objectId: S.Number });
export const DiscardChosen = m("DiscardChosen", { ids: S.Array(S.Number) });
export const GyExileChosen = m("GyExileChosen", { ids: S.Array(S.Number) });
/** Confirm a multi-card local gy-exile cost draft (`gyExilePick.picks.graveyard_exile`). */
export const GyExileConfirmed = m("GyExileConfirmed");

/** Combat staging drops (drag creature onto opponent seat or attacker card). */
export const CombatAttackerDropped = m("CombatAttackerDropped", {
  attackerId: S.Number,
  defenderSeat: S.NullOr(S.Number),
});
export const CombatBlockerDropped = m("CombatBlockerDropped", {
  blockerId: S.Number,
  attackerId: S.NullOr(S.Number),
});
export const CombatCancelAttacker = m("CombatCancelAttacker", { attackerId: S.Number });
export const CombatCancelBlocker = m("CombatCancelBlocker", { blockerId: S.Number });

/** Engine `pending_choice` answer — carries a fully-formed `WireIntent` (answer_may /
 * choose_targets / etc.). Board update folds this into `SubmitIntent` so the pending-choice
 * pipeline is Board Messages → OutMessages, not a direct Command cast. */
export const PendingChoiceAnswered = m("PendingChoiceAnswered", { intent: WireIntent });

/** Interactive pending-choice draft: toggle a card in/out of the pick set. */
export const PromptCardToggled = m("PromptCardToggled", { id: S.Number });
/** Submit the current prompt draft as a pending-choice answer. */
export const PromptSubmitted = m("PromptSubmitted");
/** Decline / fail-to-find paths where the engine allows skipping. */
export const PromptDeclined = m("PromptDeclined");
/** Reorder stacked triggers (delta -1 = up, +1 = down). */
export const PromptOrderMoved = m("PromptOrderMoved", { pos: S.Number, delta: S.Number });
/** Click-to-place reorder: first click picks a row, second click inserts it at that index. */
export const PromptOrderRowClicked = m("PromptOrderRowClicked", { pos: S.Number });
/** Set combat damage assigned to a blocker. */
export const PromptDamageSet = m("PromptDamageSet", { id: S.Number, amount: S.Number });
/** Type into a free-text prompt (naming a card). */
export const PromptStringSet = m("PromptStringSet", { value: S.String });
/** Filter searchable card-pick prompts (library search) by name. */
export const PromptCardFilterSet = m("PromptCardFilterSet", { query: S.String });
/** Filter closed option lists (creature types) by name. */
export const PromptOptionFilterSet = m("PromptOptionFilterSet", { query: S.String });
/** Set a numeric pending-choice draft (join-forces mana amount, etc.). */
export const PromptNumberSet = m("PromptNumberSet", { count: S.Number });
/** Toggle a trigger-mode choice while assembling a pending-choice answer. */
export const PromptModeChoiceToggled = m("PromptModeChoiceToggled", {
  index: S.Number,
  target: S.NullOr(WireTarget),
});
/** Assign a card to a partition bucket (or clear it). */
export const PromptPartitionSet = m("PromptPartitionSet", {
  id: S.Number,
  bucket: S.NullOr(S.String),
});
/** Toggle a modal mode while picking choose..chooseMax modes before cast. */
export const ModalModeToggled = m("ModalModeToggled", { index: S.Number });

/** Stack overlay hover/dwell changed — folds into `SetStackDwell` OutMessage. */
export const StackDwellChanged = m("StackDwellChanged", { dwelling: S.Boolean });
/** Expand the stack overlay into a horizontal strip. */
export const StackExpandClicked = m("StackExpandClicked");
/** Collapse the expanded stack overlay back to the pile. */
export const StackCollapseClicked = m("StackCollapseClicked");

/** Activation radial: arm a wedge on pointer down. */
export const RadialWedgeArmed = m("RadialWedgeArmed", { index: S.Number });
/** Activation radial: release on a wedge index, or null for the dismiss scrim. */
export const RadialWedgeReleased = m("RadialWedgeReleased", { index: S.NullOr(S.Number) });
/** Activation radial: hover highlight (null clears). */
export const RadialWedgeHovered = m("RadialWedgeHovered", { index: S.NullOr(S.Number) });
/** Activation radial: keyboard confirm of a wedge. */
export const RadialOptionPicked = m("RadialOptionPicked", { index: S.Number });
/** Activation radial: explicit dismiss (Escape / cancel). */
export const RadialDismissed = m("RadialDismissed");

// ── Inspect (Alt-pin card preview) ────────────────────────────────────────────
/** Alt key held: pin the face-up card under the cursor / hand-stack aux hover (Solid parity). */
export const AltDown = m("AltDown");
/** Alt key released: dismiss the inspect dock. */
export const AltUp = m("AltUp");
/** Hand or stack DOM overlay hover — preferred over canvas hit when Alt-pinning. */
export const InspectAuxHovered = m("InspectAuxHovered", {
  source: S.Union([S.Literal("hand"), S.Literal("stack")]),
  card: S.NullOr(
    S.Struct({
      name: S.String,
      cardId: S.optional(S.String),
      print: S.optional(S.String),
    }),
  ),
});
/** Catalog lookup returned for the current inspect pin. */
export const InspectCardFetched = m("InspectCardFetched", { card: CatalogCard });
/** Catalog name suggestions for `choose_card_name` typeahead. */
export const CardNameSuggestionsFetched = m("CardNameSuggestionsFetched", {
  query: S.String,
  names: S.Array(S.String),
});
/** Toggle DFC face in the inspect overlay. */
export const InspectFlipFace = m("InspectFlipFace");
/** Dismiss inspect overlay (Escape / backdrop click). */
export const InspectDismissed = m("InspectDismissed");

// ── Pile overlay (GY / exile expand) ──────────────────────────────────────────
/** Clicked a pile card: open the pile overlay for the given zone + owner. */
export const PileExpanded = m("PileExpanded", { zone: S.Number, owner: S.Number });
/** Close the pile overlay (Close button / Escape). */
export const PileOverlayClosed = m("PileOverlayClosed");

// ── Concede ───────────────────────────────────────────────────────────────────
/** Concede button pressed: open confirmation dialog. */
export const ConcedeClicked = m("ConcedeClicked");
/** Concede cancelled: dismiss confirmation dialog. */
export const ConcedeCancelled = m("ConcedeCancelled");
/** Concede confirmed: submit concede intent + dismiss dialog. */
export const ConcedeConfirmed = m("ConcedeConfirmed");

// ── Game result ───────────────────────────────────────────────────────────────
/** Result overlay "Watch / Stay" button: dismiss the result banner and stay on board. */
export const ResultSeen = m("ResultSeen");
/** Result overlay "Back to your decks" button: navigate home. */
export const LeaveGame = m("LeaveGame");

// ── Global keyboard shortcuts ─────────────────────────────────────────────────
/** Space pressed: acts like PrimaryClicked or PassClicked depending on state. */
export const KeyboardSpacePressed = m("KeyboardSpacePressed");
/** Enter pressed: toggle End Turn / Turn Yield rocker. */
export const KeyboardEnterPressed = m("KeyboardEnterPressed");
/** Escape pressed: dismiss inspect → dismiss radial → cancel action → close pile. */
export const KeyboardEscape = m("KeyboardEscape");

/** Dismiss the coaching hint strip (persisted to localStorage). */
export const HintDismissed = m("HintDismissed");
/** Auto-hide hint after 12s or first hand drop (session-only). */
export const HintAutoHidden = m("HintAutoHidden");
/** Toggle table sound preference (`mtgfr.sound`). */
export const SoundToggled = m("SoundToggled");
/** Priority-watch shame clock tick (whole seconds). */
export const PriorityElapsed = m("PriorityElapsed", { seconds: S.Number });
/** Toggle the board legend panel. */
export const LegendToggled = m("LegendToggled");

export const Message = S.Union([
  ArtLoaded,
  BoardPointerDown,
  BoardPointerMove,
  BoardPointerUp,
  FlightsSynced,
  HandActionActivated,
  HandDragStarted,
  HandDragMoved,
  HandDragEnded,
  HandActionHovered,
  PrimaryClicked,
  PassClicked,
  KeepHandClicked,
  MulliganClicked,
  StackYieldArmed,
  TurnYieldToggled,
  CancelActionClicked,
  CommanderCastClicked,
  TargetChosen,
  ModalModesChosen,
  ModalTargetChosen,
  XDraftSet,
  XSubmitted,
  SacrificeChosen,
  DiscardChosen,
  GyExileChosen,
  GyExileConfirmed,
  CombatAttackerDropped,
  CombatBlockerDropped,
  CombatCancelAttacker,
  CombatCancelBlocker,
  PendingChoiceAnswered,
  PromptCardToggled,
  PromptSubmitted,
  PromptDeclined,
  PromptOrderMoved,
  PromptOrderRowClicked,
  PromptDamageSet,
  PromptStringSet,
  PromptCardFilterSet,
  PromptOptionFilterSet,
  PromptNumberSet,
  PromptModeChoiceToggled,
  PromptPartitionSet,
  ModalModeToggled,
  StackDwellChanged,
  StackExpandClicked,
  StackCollapseClicked,
  RadialWedgeArmed,
  RadialWedgeReleased,
  RadialWedgeHovered,
  RadialOptionPicked,
  RadialDismissed,
  AltDown,
  AltUp,
  InspectAuxHovered,
  InspectCardFetched,
  CardNameSuggestionsFetched,
  InspectFlipFace,
  InspectDismissed,
  PileExpanded,
  PileOverlayClosed,
  ConcedeClicked,
  ConcedeCancelled,
  ConcedeConfirmed,
  ResultSeen,
  LeaveGame,
  KeyboardSpacePressed,
  KeyboardEnterPressed,
  KeyboardEscape,
  HintDismissed,
  HintAutoHidden,
  SoundToggled,
  PriorityElapsed,
  LegendToggled,
]);
export type Message = typeof Message.Type;
