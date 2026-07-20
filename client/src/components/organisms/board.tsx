// The Phase 3.5 board: an MTGA-style table. The battlefield renders on the canvas (two seats,
// lands + creatures rows, active seat highlighted); the hand is a DOM overlay you physically
// drag cards out of; casting auto-taps lands; targeted spells stage onto the stack and shoot a
// targeting arrow; combat is click-drag (creature → opponent avatar to attack, creature →
// attacker to block) confirmed with a button. Priority auto-advances server-side, so the client
// mostly just shows whose turn and priority it is.

import { useAtomMount, useAtomSet, useAtomValue } from "@effect/atom-solid";
import { useNavigate } from "@solidjs/router";
import * as Effect from "effect/Effect";
import * as Match from "effect/Match";
import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { Button } from "~/components/atoms";
import ActivationRadial from "~/components/molecules/activation-radial";
import { InspectDock } from "~/components/molecules/card-preview";
import ConfirmDialog from "~/components/molecules/confirm-dialog";
import Hand, { type ActionDrop, HAND_BAR_H } from "~/components/molecules/hand";
import ManaTray from "~/components/molecules/mana-tray";
import { HintStrip, LegendPanel } from "~/components/organisms/board-discoverability";
import { Connecting, LogPanel, PileOverlay, ResultOverlay } from "~/components/organisms/board-overlays";
import { PriorityContextBar } from "~/components/organisms/priority-context-bar";
import { StackOverlay } from "~/components/organisms/stack-overlay";
import { TurnBanner } from "~/components/organisms/turn-chrome";
import { useActionSession } from "~/controllers/action-session";
import { planCastClickResolution } from "~/controllers/actionExecution";
import { useCombatStaging } from "~/controllers/combatStaging";
import { setStackDwellFn, setTurnYieldFn, setYieldFn, submitIntentFn } from "~/controllers/intentAtoms";
import { usePlayMotion } from "~/controllers/playMotion";
import { isInteractiveControl, myChoice, PromptHost } from "~/controllers/prompt-host";
import { useTableSurface } from "~/controllers/tableSurface";
import { avatarPos, layout, type RenderCard, ZONE } from "~/layout";
import { autoTapPreviewIds } from "~/lib/actions";
import {
  draw,
  emptyArrowAnimState,
  STACK_VERTICAL_RESERVED,
  stackAimOrigin,
  stackPeekFor,
  stagingAimFrom,
} from "~/lib/boardDraw";
import { boardStatusSummary } from "~/lib/boardStatus";
import { worldToScreen } from "~/lib/camera";
import { cn } from "~/lib/cn";
import { preloadDecksIntoCache } from "~/lib/deckImagePreload";
import { sharedImageCache } from "~/lib/imageCache";
import { resolveClick } from "~/lib/interaction";
import * as lobbyClient from "~/lib/lobbyClient";
import { projectManaTrays } from "~/lib/manaTrayProject";
import { type Outcome, outcome } from "~/lib/outcome";
import { playerLabel } from "~/lib/players";
import { stackInFromDelta } from "~/lib/playOrigin";
import { type RadialOption, radialOptions } from "~/lib/radial";
import { boardChromeFromState } from "~/lib/stackResponse";
import {
  isSoundEnabled,
  playTableFeelDamage,
  playTableFeelLand,
  playTableFeelResolve,
  playTableFeelStack,
  setSoundEnabled,
} from "~/lib/tableAudio";
import { stagedTargetHint } from "~/lib/targetPrompt";
import { connectedAtom, gameStreamFamily, tableId } from "~/net";
import { foldProvenance, game, lastTableFeelBatch, resetGame, SPECTATOR_VIEWER, setReject } from "~/store";
import type { ActionView, ObjectView } from "~/wire/types";

export { humanReason, rejectMessageFor } from "~/controllers/reject";

type Vec = { x: number; y: number };

export default function Board() {
  let canvas!: HTMLCanvasElement;
  const navigate = useNavigate();
  const me = createMemo(() => {
    const v = game.state?.viewer;
    if (v === undefined || v === SPECTATOR_VIEWER) return 0;
    return v;
  });
  // Memoized (not a plain function): its value only changes when the actual seat count changes, so
  // TableSurface's auto-fit doesn't refire on every unrelated state delta (life totals, taps, …).
  const playerCount = createMemo(() => game.state?.players.length ?? 4);
  const opponents = () => (game.state?.players ?? []).map((p) => p.player).filter((s) => s !== me());
  const [tick, setTick] = createSignal(0);
  const cache = sharedImageCache;
  // Mana ability glyphs paint via canvas fillText — redraw once the face settles (ok or fail).
  void document.fonts.load("14px Mana").then(
    () => setTick((t) => t + 1),
    () => setTick((t) => t + 1),
  );

  // Interaction state.
  const [cursor, setCursor] = createSignal<Vec>({ x: 0, y: 0 });
  // A targeted action staged onto the stack, awaiting its target. Carries the action so the eventual
  // submission is `take_action { id, target }` — the action id is threaded through the staged state.
  const [expand, setExpand] = createSignal<{ zone: number; owner: number } | null>(null);
  // Stream + intent submission as atoms (ADR 0019). Mounting the per-table stream atom here ties
  // its fiber to this component's lifetime — it runs while Board is alive and is interrupted on
  // unmount. `connected` reads the stream's health for the reconnect banner. The intent setters run
  // the submission Effects (error folding lives inside the fn bodies).
  useAtomMount(() => gameStreamFamily(tableId()));
  const connected = useAtomValue(() => connectedAtom); // stream health, for the reconnect banner
  const act = useAtomSet(() => submitIntentFn, { mode: "promise" });
  const sendYield = useAtomSet(() => setYieldFn, { mode: "promise" });
  const sendTurnYield = useAtomSet(() => setTurnYieldFn, { mode: "promise" });
  const sendStackDwell = useAtomSet(() => setStackDwellFn, { mode: "promise" });
  // The viewer's "don't care" state is the server's flag, carried on every frame
  // (VisibleState.yielded) — no client mirror to drift when the server clears it mid-drive.
  const yielded = () => game.state?.yielded ?? false;
  // One-shot arm only (ADR 0027); server clears when the stack empties — no chrome cancel.
  const armStackYield = () => {
    if (yielded()) return;
    void sendYield({ enabled: true });
  };
  const setTurnYield = (enabled: boolean) => void sendTurnYield({ enabled });
  const setDwell = (dwelling: boolean) => void sendStackDwell({ dwelling });
  const [selectedId, setSelectedId] = createSignal<number | null>(null);
  /** Hovered action id — resolve `auto_tap` from the live action list so previews stay fresh. */
  const [hoverActionId, setHoverActionId] = createSignal<number | null>(null);
  const setHoverAction = (action: ActionView | null) => setHoverActionId(action?.id ?? null);
  const paymentPreviewIds = createMemo(() => {
    const id = hoverActionId();
    if (id == null) return autoTapPreviewIds(null);
    const action = game.state?.actions?.find((a) => a.id === id) ?? null;
    return autoTapPreviewIds(action);
  });
  // Clear radial payment preview whenever selection changes or closes (unmount skips mouseleave).
  createEffect((prev: number | null | undefined) => {
    const id = selectedId();
    if (prev !== undefined && id !== prev) setHoverActionId(null);
    return id;
  });

  // ── Discoverability: dismissible hint strip + '?' legend panel (finding: the interaction grammar
  // — drag/Alt/Space/Esc, badge/dot/outline meanings — is undiscoverable). The hint hides itself once
  // the viewer either dismisses it explicitly (persisted, so it stays gone next session) or completes
  // their first real hand drag-drop this session (they've demonstrably found it).
  const HINT_DISMISSED_KEY = "mtgfr.hintDismissed";
  const [hintDismissed, setHintDismissed] = createSignal(localStorage.getItem(HINT_DISMISSED_KEY) === "1");
  const [hintAutoHidden, setHintAutoHidden] = createSignal(false);
  const hintVisible = () => !hintDismissed() && !hintAutoHidden();
  const dismissHint = () => {
    localStorage.setItem(HINT_DISMISSED_KEY, "1");
    setHintDismissed(true);
  };
  // Fade the coaching strip after a beat so it doesn't sit on the life orb forever.
  createEffect(() => {
    if (!hintVisible()) return;
    const t = window.setTimeout(() => setHintAutoHidden(true), 12_000);
    onCleanup(() => window.clearTimeout(t));
  });
  const [legendOpen, setLegendOpen] = createSignal(false);
  const [soundOn, setSoundOn] = createSignal(isSoundEnabled());
  const toggleSound = () => {
    const next = !soundOn();
    setSoundEnabled(next);
    setSoundOn(next);
  };

  // Table-feel cues: one per kind per delta (ADR 0036). Flights decorate the same moments;
  // provenance flags cover opponents and prefers-reduced-motion snaps.
  createEffect((prevSeq: number | undefined) => {
    const seq = game.seq;
    if (prevSeq !== undefined && seq !== prevSeq) {
      const batch = lastTableFeelBatch();
      if (batch.land) playTableFeelLand();
      if (batch.stack) playTableFeelStack();
      if (batch.resolve) playTableFeelResolve();
      if (batch.damage) playTableFeelDamage();
    }
    return seq;
  });

  // Logical layout → TableSurface density overlay for hits; draw uses surface.drawnCards (tween + density).
  const cards = createMemo<RenderCard[]>(() => (game.state ? layout(game.state, me()) : []));
  const provenance = () => foldProvenance();
  // PlayMotion needs TableSurface's camera; TableSurface needs ownedIds. Assign the ref after
  // both hooks so seedEntrances can skip play-owned ids without a Solid bridge signal.
  let playMotionRef: { ownedIds: () => ReadonlySet<number> } | null = null;
  const surface = useTableSurface({
    me,
    playerCount,
    cards,
    handBarH: HAND_BAR_H,
    zoneMoves: () => provenance().zoneMoves,
    fromStack: () => provenance().resolvedFromStack,
    fromStackExit: () => provenance().leftStackToPile,
    tokenCreators: () => provenance().tokenCreators,
    zonePileEntrances: () => provenance().zonePileEntrances,
    stackObjectIds: () => provenance().priorStackObjectIds,
    stackLength: () => game.state?.stack.length ?? 0,
    selectedId,
    flightOwnedIds: () => playMotionRef?.ownedIds() ?? new Set(),
  });
  const { camera, size, setSize, hitCard, hitSeat, dragging, drawnCards, inspectPin, clearInspect, tryPinInspect } =
    surface;

  const playMotion = usePlayMotion({
    camera,
    size,
    cards,
    stackLength: () => game.state?.stack.length ?? 0,
    stackSourceIds: () => new Set((game.state?.stack ?? []).map((s) => s.source)),
    objectIds: () => new Set((game.state?.objects ?? []).map((o) => o.id)),
    landPlays: () => provenance().landPlayFrom,
    fromStack: () => provenance().resolvedFromStack,
    fromStackExit: () => provenance().leftStackToPile,
    stackEntrances: () => provenance().stackEntrances,
    zoneMoves: () => provenance().zoneMoves,
    reducedMotion: () => typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches,
    onTick: () => setTick((t) => t + 1),
  });
  playMotionRef = playMotion;

  /** Hand-card id → screen origin for stack DOM play-in; remapped to spell id on spell_cast. */
  const stackScreenByCard = new Map<number, { x: number; y: number }>();
  const [stackInDeltas, setStackInDeltas] = createSignal(new Map<number, { dx: number; dy: number }>());

  createEffect(() => {
    game.seq;
    const cam = camera();
    const sz = size();
    const live = new Set((game.state?.stack ?? []).map((s) => s.source));
    const next = new Map(stackInDeltas());
    // Drop play-in deltas for objects that left the stack so they can't revive on remount.
    for (const id of [...next.keys()]) {
      if (!live.has(id)) next.delete(id);
    }
    const owned = playMotion.ownedIds();
    const stackLen = game.state?.stack.length ?? 0;
    const peek = stackPeekFor(stackLen, sz.y, STACK_VERTICAL_RESERVED);
    for (const [spell, meta] of provenance().stackEntrances) {
      if (next.has(spell)) continue;
      // Canvas flight owns hand→stack; skip CSS stack-in deltas for those ids (ADR 0035).
      if (owned.has(spell) || owned.has(meta.from)) continue;
      let fromScreen = stackScreenByCard.get(meta.from);
      if (!fromScreen) {
        const a = avatarPos(meta.controller, me(), playerCount());
        fromScreen = worldToScreen(cam, a.x, a.y);
      } else {
        stackScreenByCard.delete(meta.from);
      }
      const idx = game.state?.stack.findIndex((s) => s.source === spell) ?? -1;
      const n = idx >= 0 ? idx + 1 : stackLen;
      const to = stackAimOrigin(sz.x, sz.y, Math.max(1, n), peek);
      next.set(spell, stackInFromDelta(fromScreen, to));
    }
    setStackInDeltas(next);
  });

  /** Prefer density/tween positions (fanned members) over collapsed layout faces. */
  const byId = (id: number) => drawnCards().find((c) => c.id === id) ?? cards().find((c) => c.id === id);

  const playerName = (seat: number) => playerLabel(game.state?.players ?? [], seat);

  const session = useActionSession({
    me,
    act,
    getState: () => game.state,
    camera,
    size,
    handBarH: HAND_BAR_H,
    setReject,
    seedDrop: (cardId, _world, screen, flight) => {
      stackScreenByCard.set(cardId, screen);
      const obj = game.state?.objects.find((o) => o.id === cardId);
      playMotion.spawnFromHand({
        cardId,
        print: obj?.print ?? "",
        name: obj?.name ?? "",
        screen,
        kind: flight,
      });
    },
    clearPlayOrigin: (cardId) => {
      stackScreenByCard.delete(cardId);
      playMotion.cancel(cardId);
    },
    onHintUsed: () => setHintAutoHidden(true),
  });
  /** Arrow aiming (not pick-modal / not expanded stack): stack ghost + canvas target arrow. */
  const [stackExpanded, setStackExpanded] = createSignal(false);
  const arrowAiming = createMemo(() => {
    if (stackExpanded()) return false;
    const o = session.overlay();
    return o.mode.kind === "arrow" && !o.staged?.preferPick;
  });
  /** Staged card for stack chrome (pile ghost or expand/full rightmost) — arrow may be suspended. */
  const stackStagedCard = createMemo(() => {
    const o = session.overlay();
    if (o.mode.kind !== "arrow" || o.staged?.preferPick) return null;
    return o.staged?.card ?? null;
  });
  const stagedCard = createMemo(() => session.overlay().staged?.card ?? null);
  const stagedObjects = () => session.overlay().objects;
  const stagedPlayers = () => session.overlay().players;

  const combat = useCombatStaging({
    me,
    step: () => game.state?.step ?? -1,
    activePlayer: () => game.state?.active_player ?? -1,
    spectating: () => game.state?.viewer === SPECTATOR_VIEWER,
    opponents,
    declaredAttackers: () => game.state?.combat.attackers ?? [],
    declaredBlocks: () => game.state?.combat.blocks ?? [],
    attackersDeclared: () => game.state?.combat.attackers_declared ?? false,
    blockersDeclared: () => game.state?.combat.blockers_declared?.includes(me()) ?? false,
    requiredAttacks: () => game.state?.actions?.find((a) => a.kind === "declare_attackers")?.required_attacks ?? [],
    hitSeat,
    hitCard,
    act,
  });
  const {
    attackers,
    blocks,
    combatStep,
    onCombatDrop,
    cancelAttacker,
    cancelBlocker,
    clearCombat,
    primaryAction,
    runPrimaryAction,
  } = combat;

  const step = () => game.state?.step ?? -1;
  // A spectator (no seat) watches read-only: the server sends `viewer === SPECTATOR_VIEWER` and
  // rejects any intent from them, so the UI just suppresses the hand and every action affordance.
  const spectating = () => game.state?.viewer === SPECTATOR_VIEWER;
  /** Shared Next/Pass/yield/dwell policy for bar, keyboard, canvas, and stack overlay. */
  const boardChrome = createMemo(() =>
    boardChromeFromState(game.state, {
      // Expand suspends arrow drawing only — staged still blocks Pass / Space / yield.
      staged: stackStagedCard() != null,
      manaSources: drawnCards(),
      // Attack (N) pending — End Turn must not compete (would auto-pass and seal empty).
      pendingAttackers: primaryAction().kind === "confirm-attackers" && attackers().length > 0,
    }),
  );
  // Won, lost, or still in it. An eliminated player's intents are rejected by the server, so their
  // hand and controls come down with them — they keep the board to watch the rest of the game.
  const result = createMemo<Outcome>(() => outcome(game.state?.players ?? [], game.state?.viewer ?? SPECTATOR_VIEWER));
  const eliminated = () => result().kind === "lost" || result().kind === "won";
  /** The overlay is dismissed to keep watching; it never comes back for the same game. */
  const [resultSeen, setResultSeen] = createSignal(false);
  const [confirmConcede, setConfirmConcede] = createSignal(false);

  onMount(() => {
    resetGame(); // M4: a fresh Board mount is a new table — drop the last game's state/seq/log
    // The delta stream runs off `useAtomMount(gameStreamFamily(...))` above; its frames fold into
    // the store, its status flips `connected`, and its terminal errors set the reject line.

    const unsubCache = cache.subscribe(() => setTick((t) => t + 1));
    onCleanup(unsubCache);

    // Warm every seated deck's art (owned decks + public precons). Library search then hits cache.
    void lobbyClient.lobbyState(tableId()).then((view) => {
      if (!view) return;
      const ids = view.seats.flatMap((s) => (s.deck_id != null ? [s.deck_id] : []));
      Effect.runFork(preloadDecksIntoCache(ids, cache));
    });

    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    // Arrow draw-on births live with this paint loop (not module globals) so scene paint stays pure.
    const arrowAnim = emptyArrowAnimState();
    const resize = () => {
      const dpr = window.devicePixelRatio || 1;
      canvas.width = window.innerWidth * dpr;
      canvas.height = window.innerHeight * dpr;
      canvas.style.width = `${window.innerWidth}px`;
      canvas.style.height = `${window.innerHeight}px`;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      const s = { x: window.innerWidth, y: window.innerHeight };
      setSize(s);
      // Camera fit itself is owned by TableSurface (guarded by userMoved) — this just keeps the
      // canvas backing store matched to the new viewport, whether or not the fit re-runs.
    };
    window.addEventListener("resize", resize);
    onCleanup(() => window.removeEventListener("resize", resize));
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (inspectPin()) {
          clearInspect();
          return;
        }
        if (selectedId() !== null) {
          setSelectedId(null);
          return;
        }
        cancelInteraction();
        setLegendOpen(false);
        setExpand(null); // the pile overlay is dismissable by mouse only otherwise
        setStackExpanded(false);
      }
      if (e.key === "Alt") {
        e.preventDefault();
        tryPinInspect();
      }
      // Space = one priority pass (Next / Resolve card). Enter = End Turn while active (ADR 0037).
      if (e.key === "Enter" && !inspectPin() && !promptOpen() && !isInteractiveControl(e.target)) {
        const chrome = boardChrome();
        if (chrome.showEndTurn) {
          e.preventDefault();
          setTurnYield(!chrome.turnYielded);
          return;
        }
        if (chrome.showTurnYield) {
          e.preventDefault();
          setTurnYield(!chrome.turnYielded);
          return;
        }
      }
      if (e.key === " " && !inspectPin() && !promptOpen() && yours() && !isInteractiveControl(e.target)) {
        e.preventDefault(); // Space must not scroll the page
        const binding = boardChrome().space;
        if (binding === "pass_priority") {
          void act({ kind: "pass_priority", player: me() });
          return;
        }
        if (binding === "primary") {
          runPrimaryAction();
        }
      }
    };
    const onKeyUp = (e: KeyboardEvent) => {
      if (e.key === "Alt") clearInspect();
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("keyup", onKeyUp);
    onCleanup(() => window.removeEventListener("keydown", onKey));
    onCleanup(() => window.removeEventListener("keyup", onKeyUp));
    resize();

    createEffect(() => {
      tick();
      // Tracked, not used: `resize` clears the backing store and only re-fits the camera while the
      // player hasn't taken manual control. Once they have, nothing else here changes on a resize,
      // so without this dependency a panned board stays blank until the next delta arrives.
      size();
      const count = playerCount();
      const players = game.state?.players ?? [];
      const avatarScreenPositions: Record<number, Vec> = {};
      const cam = camera();
      for (const p of players) {
        const a = avatarPos(p.player, me(), count);
        avatarScreenPositions[p.player] = worldToScreen(cam, a.x, a.y);
      }
      const chrome = boardChrome();
      const arrowsNeedFrame = draw(
        ctx,
        {
          cam,
          cards: drawnCards(),
          cache,
          me: me(),
          active: game.state?.active_player ?? -1,
          priority: game.state?.priority ?? -1,
          viewer: me(),
          count,
          players,
          combat: game.state?.combat ?? {
            attackers: [],
            blocks: [],
            attackers_declared: false,
            blockers_declared: [],
          },
          attackers: attackers(),
          blocks: blocks(),
          // Aiming only while the arrow is the asking mode — the picker brings its own modal, and an
          // arrow drawn under it would point at a board the player can no longer click.
          aiming: arrowAiming(),
          aimFrom: stagingAimFrom(
            size().x,
            size().y,
            game.state?.stack.length ?? 0,
            arrowAiming(),
            stackPeekFor((game.state?.stack.length ?? 0) + (arrowAiming() ? 1 : 0), size().y, STACK_VERTICAL_RESERVED),
          ),
          targetObjects: stagedObjects(),
          targetPlayers: stagedPlayers(),
          canvasDrag: dragging(),
          cursor: cursor(),
          avatarScreenPositions,
          stepIdx: step(),
          selectedId: selectedId(),
          paymentObjects: paymentPreviewIds(),
          stackResponseFocus: chrome.focus,
          responseObjects: chrome.brightIds,
          flights: playMotion.flights(),
          hideCardIds: playMotion.hideCardIds(),
        },
        arrowAnim,
      );
      // Combat/target arrows draw-on over ~180ms; without a follow-up paint they freeze as a stub
      // on the source card (staged attackers looked like they pointed at the creature).
      if (arrowsNeedFrame) {
        const id = requestAnimationFrame(() => setTick((t) => t + 1));
        onCleanup(() => cancelAnimationFrame(id));
      }
    });
  });

  const cancelInteraction = () => {
    session.cancel();
    clearCombat();
  };

  const onHandDrop = (d: ActionDrop) => {
    if (spectating()) return;
    setHoverActionId(null);
    session.play(d.action, { x: d.x, y: d.y });
  };

  // ── Canvas pointer: pan, click, or a combat drag from a creature ───────────────────
  const onPointerDown = (e: PointerEvent) => {
    canvas.setPointerCapture(e.pointerId);
    surface.pointerDown(e.clientX, e.clientY, {
      combatStep: combatStep(),
      me: me(),
      aimSeats: [...stagedPlayers()],
    });
    if (dragging()) setCursor({ x: e.clientX, y: e.clientY });
  };
  const onPointerMove = (e: PointerEvent) => {
    const effect = surface.pointerMove(e.clientX, e.clientY);
    if (effect?.kind === "drag" || stagedCard()) setCursor({ x: e.clientX, y: e.clientY });
  };
  const onPointerUp = (e: PointerEvent) => {
    canvas.releasePointerCapture(e.pointerId);
    Match.value(surface.pointerUp(e.clientX, e.clientY)).pipe(
      Match.discriminatorsExhaustive("kind")({
        "aim-seat": (fx) => {
          // Expand suspends targeting completion as well as the arrow draw.
          if (stackExpanded()) return;
          session.aim({ kind: "player", player: fx.seat });
        },
        "combat-drop": (fx) => onCombatDrop(fx.card, fx.x, fx.y),
        click: (fx) => onClickCard(fx.card),
        "clear-selection": () => setSelectedId(null),
        none: () => {},
      }),
    );
  };
  const onPointerCancel = () => {
    surface.pointerCancel();
  };
  const onWheel = (e: WheelEvent) => {
    e.preventDefault();
    surface.zoomAt(e.clientX, e.clientY, e.deltaY < 0 ? 1.1 : 1 / 1.1);
  };

  const onClickCard = (card: RenderCard) => {
    const r = resolveClick(game.state, me(), card, {
      spectating: spectating(),
      staged: stagedCard(),
      stagedTargets: stagedObjects(),
      attackers: attackers(),
      blocks: blocks(),
    });
    Match.value(r).pipe(
      Match.discriminatorsExhaustive("kind")({
        expand: (res) => setExpand({ zone: res.zone, owner: res.owner }),
        cast: (res) => {
          const plan = planCastClickResolution(stagedCard() != null, res);
          if (plan.kind === "complete-staged-target") {
            if (stackExpanded()) return;
            session.aim(plan.target);
          } else if (plan.kind === "cast-commander") session.playObjectCast(plan.card, plan.target);
        },
        "cancel-attacker": (res) => cancelAttacker(res.id),
        "cancel-blocker": (res) => cancelBlocker(res.id),
        intent: (res) => void act(res.intent),
        select: (res) => setSelectedId((cur) => (cur === res.id ? null : res.id)),
        none: () => setSelectedId(null),
      }),
    );
  };

  const selectedCard = () => {
    const id = selectedId();
    if (id == null) return null;
    // Prefer density/tween pose (fanned member); fall back to the cluster face that owns id.
    return byId(id) ?? cards().find((c) => c.clusterMembers.includes(id)) ?? null;
  };
  createEffect(() => {
    const id = selectedId();
    if (id == null) return;
    const obj = game.state?.objects.find((o) => o.id === id);
    if (!obj || obj.zone !== ZONE.Battlefield) setSelectedId(null);
  });
  const selectedRadial = createMemo(() => {
    const id = selectedId();
    const c = selectedCard();
    if (id == null || !c) return [] as RadialOption[];
    // Use the selected object id — `c` may be the cluster face when falling back for pose.
    return radialOptions(id, game.state?.actions, c.tapsForMana, c.tapped, game.state?.can_act ?? false);
  });
  const selectedScreen = createMemo(() => {
    const c = selectedCard();
    if (!c) return null;
    const s = worldToScreen(camera(), c.x + c.w / 2, c.y + c.h / 2);
    return s;
  });
  const manaTrays = createMemo(() => projectManaTrays(game.state?.players ?? [], me(), playerCount(), camera()));
  // DOM hit targets over canvas life orbs — for AT and for Playwright targeting/combat aims.
  const lifeOrbs = createMemo(() => {
    const players = game.state?.players ?? [];
    const count = playerCount();
    const cam = camera();
    const zoom = Math.max(0.5, cam.zoom);
    const size = Math.round(56 * zoom);
    return players.map((p) => {
      const a = avatarPos(p.player, me(), count);
      const s = worldToScreen(cam, a.x, a.y);
      return {
        seat: p.player,
        life: p.life,
        lost: p.lost,
        name: playerLabel(players, p.player),
        x: s.x,
        y: s.y,
        size,
      };
    });
  });
  // Screen-space markers over battlefield permanents — pointer-events none so canvas keeps
  // hit-testing; Playwright reads boxes to aim combat/targeting gestures at real cards.
  const bfMarkers = createMemo(() => {
    const cam = camera();
    return drawnCards()
      .filter((c) => c.zone === ZONE.Battlefield && c.pile === 0)
      .map((c) => {
        const tl = worldToScreen(cam, c.x, c.y);
        const br = worldToScreen(cam, c.x + c.w, c.y + c.h);
        return {
          id: c.id,
          kind: c.kind,
          controller: c.controller,
          owner: c.owner,
          tapped: c.tapped,
          summoningSick: c.summoningSick,
          hasHaste: c.hasHaste,
          x: (tl.x + br.x) / 2,
          y: (tl.y + br.y) / 2,
          w: Math.max(8, Math.abs(br.x - tl.x)),
          h: Math.max(8, Math.abs(br.y - tl.y)),
        };
      });
  });
  const onLifeOrbClick = (seat: number) => {
    if (spectating() || eliminated()) return;
    // Complete a staged player-target aim (same path as a canvas seat hit).
    if (arrowAiming() && stagedPlayers().has(seat)) {
      session.aim({ kind: "player", player: seat });
    }
  };
  const lifeOrbInteractive = () => arrowAiming() && stagedPlayers().size > 0;
  const onRadialPick = (opt: RadialOption) => {
    const id = selectedId();
    const aimFrom = selectedScreen() ?? undefined;
    setHoverActionId(null);
    setSelectedId(null);
    if (opt.kind === "tap_for_mana" && id != null) {
      void act({ kind: "tap_for_mana", player: me(), object: id });
      return;
    }
    if (opt.kind === "action") session.play(opt.action, aimFrom);
  };

  const yours = () => game.state?.priority === me();
  const promptOpen = () => game.state != null && myChoice(game.state, me()) != null;

  // Spoken summary of the canvas (live region only — the canvas stays an unlabeled pointer
  // surface so AT isn't told it's a static image). Uses wire viewer, not layout `me()`.
  const boardAria = createMemo(() => boardStatusSummary(game.state, game.state?.viewer ?? SPECTATOR_VIEWER));

  const expandCards = createMemo<ObjectView[]>(() => {
    const e = expand();
    if (!e || !game.state) return [];
    return game.state.objects.filter((o) => o.zone === e.zone && o.owner === e.owner);
  });

  return (
    <>
      <div class="sr-only" aria-live="polite">
        {boardAria()}
      </div>
      <canvas
        ref={canvas}
        data-testid="board-canvas"
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerCancel}
        onWheel={onWheel}
        class="block cursor-grab touch-none bg-forest-floor"
      />
      <ManaTray trays={manaTrays()} />
      <div class="pointer-events-none fixed inset-0 z-[15]">
        <For each={bfMarkers()}>
          {(m) => (
            <div
              data-testid={`bf-card-${m.id}`}
              data-card-kind={m.kind}
              data-controller={m.controller}
              data-owner={m.owner}
              data-tapped={m.tapped ? "1" : "0"}
              data-summoning-sick={m.summoningSick ? "1" : "0"}
              data-has-haste={m.hasHaste ? "1" : "0"}
              style={{
                left: `${m.x}px`,
                top: `${m.y}px`,
                width: `${m.w}px`,
                height: `${m.h}px`,
                transform: "translate(-50%, -50%)",
              }}
              class="absolute"
            />
          )}
        </For>
      </div>
      <div class="pointer-events-none fixed inset-0 z-[16]">
        <For each={lifeOrbs()}>
          {(orb) => (
            <button
              type="button"
              data-testid={`life-orb-${orb.seat}`}
              data-life={orb.life}
              data-lost={orb.lost ? "1" : "0"}
              aria-label={`${orb.name}, ${orb.life} life`}
              disabled={orb.lost || !lifeOrbInteractive() || !stagedPlayers().has(orb.seat)}
              onClick={() => onLifeOrbClick(orb.seat)}
              // Only capture pointers while aiming at players — otherwise these orbs sit over the
              // hand bar / battlefield and steal drag-to-play and combat gestures.
              class={cn(
                "absolute rounded-full border-0 bg-transparent",
                lifeOrbInteractive() && stagedPlayers().has(orb.seat) ? "pointer-events-auto" : "pointer-events-none",
              )}
              style={{
                left: `${orb.x}px`,
                top: `${orb.y}px`,
                width: `${orb.size}px`,
                height: `${orb.size}px`,
                transform: "translate(-50%, -50%)",
              }}
            />
          )}
        </For>
      </div>
      <Show when={!connected()}>
        <div
          data-testid="board-reconnecting"
          class="fixed top-0 right-0 left-0 z-40 bg-reconnect-rust p-sm text-center font-semibold text-label text-snow"
        >
          Reconnecting…
        </div>
      </Show>
      <InspectDock
        pin={inspectPin()}
        modifiersFor={(objectId) => {
          const obj = game.state?.objects.find((o) => o.id === objectId);
          if (!obj || obj.zone !== ZONE.Battlefield) return [];
          return obj.modifiers ?? [];
        }}
        onDismiss={clearInspect}
      />
      <Show when={selectedScreen()}>
        {(pos) => (
          <ActivationRadial
            x={pos().x}
            y={pos().y}
            zoom={camera().zoom}
            options={selectedRadial()}
            onPick={onRadialPick}
            onDismiss={() => {
              setHoverActionId(null);
              setSelectedId(null);
            }}
            onHoverAction={setHoverAction}
          />
        )}
      </Show>
      <Show when={game.state} fallback={<Connecting />}>
        {(state) => (
          <>
            <TurnBanner me={me()} state={state()} />
            <Show when={spectating()}>
              <div class="fixed top-md left-1/2 z-20 -translate-x-1/2 rounded-control bg-llanowar px-md py-xs font-semibold text-label text-snow-mint tracking-[0.04em]">
                Spectating
              </div>
            </Show>
            <Show when={!spectating() && !eliminated()}>
              <PriorityContextBar
                action={primaryAction()}
                yours={yours()}
                chrome={boardChrome()}
                reject={game.reject}
                // Coaching + Cancel stay while expand suspends only the arrow draw.
                staged={stackStagedCard() ? stagedTargetHint(session.overlay().staged) : null}
                stagedPlayers={stagedPlayers().size > 0}
                onRun={runPrimaryAction}
                onPass={() => void act({ kind: "pass_priority", player: me() })}
                onArmStackYield={armStackYield}
                onTurnYield={setTurnYield}
                onCancelTarget={stackStagedCard() ? () => session.cancel() : null}
              />
              {/* Quitting the table. Conceding is a real game action (CR 104.3a), not a navigation:
                  it eliminates the seat so the other three stop waiting on it. The result overlay
                  comes up afterwards and offers the way back to the deck manager. */}
              <Button
                type="button"
                data-testid="board-concede"
                onClick={() => setConfirmConcede(true)}
                variant="ghost"
                class="fixed top-md right-md z-20"
              >
                Concede
              </Button>
            </Show>
            {/* Legend + Sound — top-left. Sound is for everyone on the stream (table feel); legend for seated. */}
            <div class="fixed top-md left-md z-25 flex items-center gap-xs">
              <Show when={!spectating() && !eliminated()}>
                <Button
                  type="button"
                  aria-label="Board legend"
                  aria-expanded={legendOpen()}
                  onClick={() => setLegendOpen((o) => !o)}
                  variant="ghost"
                  hitQuiet
                  class="px-md py-xs"
                >
                  ?
                </Button>
              </Show>
              <Button
                type="button"
                data-testid="board-sound-toggle"
                aria-label={soundOn() ? "Mute sound" : "Unmute sound"}
                aria-pressed={soundOn()}
                onClick={toggleSound}
                variant="ghost"
                hitQuiet
                class="px-md py-xs text-caption"
              >
                {soundOn() ? "Sound" : "Muted"}
              </Button>
            </div>
            <Show when={!spectating() && !eliminated() && legendOpen()}>
              <LegendPanel onClose={() => setLegendOpen(false)} />
            </Show>
            <StackOverlay
              state={state()}
              staged={stackStagedCard()}
              returningStaged={session.overlay().returningStaged}
              stagedPlayIn={null}
              stagedReturn={(() => {
                const s = session.overlay().returningStaged;
                if (!s) return null;
                const sz = size();
                const peek = stackPeekFor(state().stack.length + 1, sz.y, STACK_VERTICAL_RESERVED);
                const from = stackAimOrigin(sz.x, sz.y, state().stack.length + 1, peek);
                const hand =
                  document.querySelector(`[data-testid="hand-card-${s.card.id}"]`)?.getBoundingClientRect() ?? null;
                const to = hand ? { x: hand.left + hand.width / 2, y: hand.top + hand.height / 2 } : s.playOriginScreen;
                // Return keyframes end at translate(dx,dy) — delta from stack rest to hand.
                return stackInFromDelta(to, from);
              })()}
              showPileStaged={(() => {
                const staged = stackStagedCard();
                if (!arrowAiming() || staged == null) return false;
                return !playMotion.hideCardIds().has(staged.id);
              })()}
              hideFaceIds={playMotion.hideCardIds()}
              allowDwell={boardChrome().allowDwell}
              viewportW={size().x}
              viewportH={size().y}
              expanded={stackExpanded()}
              entranceDeltas={stackInDeltas()}
              onExpand={() => setStackExpanded(true)}
              onCollapse={() => setStackExpanded(false)}
              onHoverCard={(c) => surface.setAuxHover("stack", c)}
              onDwell={setDwell}
            />
            {/* Left chrome column: hint above log so they never fight for the same inset. */}
            <div
              style={{ "--b": `${HAND_BAR_H + 10}px` }}
              class="fixed bottom-(--b) left-md z-20 flex max-w-[min(420px,46vw)] flex-col items-start gap-sm"
            >
              <Show when={!spectating() && !eliminated() && hintVisible()}>
                <HintStrip onDismiss={dismissHint} />
              </Show>
              <LogPanel />
            </div>
            <PromptHost me={me()} state={state()} onAnswer={act} />
          </>
        )}
      </Show>
      <Show when={!spectating() && !eliminated()}>
        <Hand
          viewer={me()}
          hiddenId={stagedCard()?.id ?? session.overlay().returningStaged?.card.id ?? null}
          flyingIds={playMotion.handHidden()}
          onHoverCard={(c) => surface.setAuxHover("hand", c)}
          onHoverAction={setHoverAction}
          onDrop={onHandDrop}
        />
      </Show>
      <ConfirmDialog
        open={confirmConcede()}
        title="Concede the game?"
        body="You're out for good, and the other players carry on without you."
        confirmLabel="Concede"
        danger
        onConfirm={() => {
          setConfirmConcede(false);
          act({ kind: "concede", player: me() });
        }}
        onCancel={() => setConfirmConcede(false)}
      />
      <Show when={result().kind !== "playing" && !resultSeen()}>
        <ResultOverlay
          outcome={result()}
          onWatch={() => setResultSeen(true)}
          onLeave={() => navigate("/", { replace: true })}
        />
      </Show>
      <session.Chrome playerName={playerName} />
      <Show when={expand()}>
        <PileOverlay cards={expandCards()} onClose={() => setExpand(null)} />
      </Show>
    </>
  );
}
