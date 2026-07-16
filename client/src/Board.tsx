// The Phase 3.5 board: an MTGA-style table. The battlefield renders on the canvas (two seats,
// lands + creatures rows, active seat highlighted); the hand is a DOM overlay you physically
// drag cards out of; casting auto-taps lands; targeted spells stage onto the stack and shoot a
// targeting arrow; combat is click-drag (creature → opponent avatar to attack, creature →
// attacker to block) confirmed with a button. Priority auto-advances server-side, so the client
// mostly just shows whose turn and priority it is.

import { useAtomMount, useAtomSet, useAtomValue } from "@effect/atom-solid";
import { useNavigate } from "@solidjs/router";
import * as Effect from "effect/Effect";
import * as Fiber from "effect/Fiber";
import * as Match from "effect/Match";
import { createEffect, createMemo, createSignal, For, Index, onCleanup, onMount, Show } from "solid-js";
import ActivationRadial from "~/ActivationRadial";
import type { ActionView, ObjectView, PlayerView, VisibleState, WireTarget } from "~/api/generated";
import { InspectDock } from "~/CardPreview";
import ConfirmDialog from "~/ConfirmDialog";
import { planCastClickResolution } from "~/controllers/actionExecution";
import { useActionSession } from "~/controllers/actionSession";
import { useCombatStaging } from "~/controllers/combatStaging";
import { setStackDwellFn, setTurnYieldFn, setYieldFn, submitIntentFn } from "~/controllers/intentAtoms";
import { isInteractiveControl, myChoice, PromptHost } from "~/controllers/promptHost";
import { useTableSurface } from "~/controllers/tableSurface";
import Hand, { type ActionDrop, HAND_BAR_H } from "~/Hand";
import { avatarPos, layout, PHASES, phaseOf, type RenderCard, STEP_NAMES, ZONE } from "~/layout";
import { autoTapPreviewIds } from "~/lib/actions";
import {
  draw,
  RESPONSE_COLOR,
  STACK_CARD_W,
  STACK_HORIZONTAL_MARGIN,
  STACK_OVERLAY_RIGHT,
  STACK_STRIP_MIN_PEEK,
  STACK_VERTICAL_RESERVED,
  shouldAutoCollapseStackExpand,
  stackCardH,
  stackExpandAvailable,
  stackFullPerRow,
  stackPeekFor,
  stackPresentation,
  stackStripPeek,
  stagingAimFrom,
  TARGET_COLOR,
} from "~/lib/boardDraw";
import { boardStatusSummary } from "~/lib/boardStatus";
import { worldToScreen } from "~/lib/camera";
import { cn } from "~/lib/cn";
import { ImageCache } from "~/lib/imageCache";
import { type PrimaryAction, resolveClick } from "~/lib/interaction";
import { projectManaTrays } from "~/lib/manaTrayProject";
import { type Outcome, outcome } from "~/lib/outcome";
import { playerLabel } from "~/lib/players";
import { PROMPT_ROW, PROMPT_TITLE } from "~/lib/promptForms";
import { type RadialOption, radialOptions } from "~/lib/radial";
import { imageUrlByPrint } from "~/lib/scryfall";
import { stackChrome } from "~/lib/stackResponse";
import { stagedTargetHint } from "~/lib/targetPrompt";
import { type Heat, heatOf, watchElapsed } from "~/lib/watch";
import ManaTray from "~/ManaTray";
import { connectedAtom, gameStreamFamily, tableId } from "~/net";
import { game, resetGame, resolvedFromStack, SPECTATOR_VIEWER, setReject, zoneMoves } from "~/store";
import { Button, Hud, Modal } from "~/ui";

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
  const cache = new ImageCache(() => setTick((t) => t + 1));
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
  const turnYielded = () => game.state?.turn_yielded ?? false;
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
  const [legendOpen, setLegendOpen] = createSignal(false);

  // Logical layout → TableSurface density overlay for hits; draw uses surface.drawnCards (tween + density).
  const cards = createMemo<RenderCard[]>(() => (game.state ? layout(game.state, me()) : []));
  const surface = useTableSurface({
    me,
    playerCount,
    cards,
    handBarH: HAND_BAR_H,
    zoneMoves,
    fromStack: resolvedFromStack,
    stackLength: () => game.state?.stack.length ?? 0,
    selectedId,
  });
  const { camera, size, setSize, hitCard, hitSeat, dragging, drawnCards, inspectPin, clearInspect, tryPinInspect } =
    surface;
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
    seedDrop: (seed) => surface.noteDropSeed(seed),
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
    stackChrome({
      spectating: spectating(),
      // Expand suspends arrow drawing only — staged still blocks Pass / Space / yield.
      staged: stackStagedCard() != null,
      yielded: yielded(),
      stackLen: game.state?.stack.length ?? 0,
      holdRemainingMs: game.state?.stack_hold_remaining_ms ?? 0,
      canAct: game.state?.can_act ?? false,
      viewer: me(),
      priority: game.state?.priority ?? -1,
      active: game.state?.active_player ?? -1,
      step: game.state?.step ?? -1,
      actions: game.state?.actions,
      manaSources: drawnCards(),
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

    const ctx = canvas.getContext("2d");
    if (!ctx) return;
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
      // Space/Enter = primary context bar when the stack is empty; on the stack, one-shot
      // pass_priority while you can act (Next is hidden — stack yield is the standing opt-out).
      if (
        (e.key === " " || e.key === "Enter") &&
        !inspectPin() &&
        !spectating() &&
        !promptOpen() &&
        yours() &&
        !isInteractiveControl(e.target)
      ) {
        e.preventDefault(); // Space must not scroll the page
        const stackLen = game.state?.stack.length ?? 0;
        if (stackLen > 0) {
          if (boardChrome().spaceOnStack === "pass_priority") {
            void act({ kind: "pass_priority", player: me() });
          }
          return;
        }
        runPrimaryAction();
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
      const arrowsNeedFrame = draw(ctx, {
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
      });
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
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerCancel}
        onWheel={onWheel}
        class="block cursor-grab touch-none bg-forest-floor"
      />
      <ManaTray trays={manaTrays()} />
      <Show when={!connected()}>
        <div class="fixed top-0 right-0 left-0 z-40 bg-reconnect-rust p-1.5 text-center font-semibold text-label text-white">
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
              <div class="fixed top-3 left-1/2 z-20 -translate-x-1/2 rounded-control bg-llanowar px-3 py-1 font-semibold text-label text-white tracking-[0.04em]">
                Spectating
              </div>
            </Show>
            <Show when={!spectating() && !eliminated()}>
              <PriorityContextBar
                action={primaryAction()}
                yours={yours()}
                chrome={boardChrome()}
                turnYielded={turnYielded()}
                showTurnYield={(game.state?.active_player ?? -1) !== me()}
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
                onClick={() => setConfirmConcede(true)}
                variant="ghost"
                class="fixed top-3 right-3 z-20"
              >
                Concede
              </Button>
              {/* The '?' legend toggle, parked just above the horizontal context bar. */}
              <Button
                type="button"
                aria-label="Board legend"
                aria-expanded={legendOpen()}
                onClick={() => setLegendOpen((o) => !o)}
                style={{ "--b": `${HAND_BAR_H + 58}px` }}
                variant="ghost"
                hitQuiet
                class="fixed right-[10px] bottom-(--b) z-25 px-[11px] py-[5px]"
              >
                ?
              </Button>
              <Show when={legendOpen()}>
                <LegendPanel onClose={() => setLegendOpen(false)} />
              </Show>
              <Show when={hintVisible()}>
                <HintStrip onDismiss={dismissHint} />
              </Show>
            </Show>
            <StackOverlay
              state={state()}
              staged={stackStagedCard()}
              showPileStaged={arrowAiming()}
              allowDwell={boardChrome().allowDwell}
              viewportW={size().x}
              viewportH={size().y}
              expanded={stackExpanded()}
              onExpand={() => setStackExpanded(true)}
              onCollapse={() => setStackExpanded(false)}
              onHoverCard={(c) => surface.setAuxHover("stack", c)}
              onDwell={setDwell}
            />
            <LogPanel />
            <PromptHost me={me()} state={state()} onAnswer={act} />
          </>
        )}
      </Show>
      <Show when={!spectating() && !eliminated()}>
        <Hand
          viewer={me()}
          hiddenId={stagedCard()?.id ?? null}
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

// ── Small helpers ────────────────────────────────────────────────────────────────────

function targetLabel(target: WireTarget, name: (id: number) => string, players: PlayerView[]): string {
  return target.kind === "object" ? name(target.id) : playerLabel(players, target.player);
}
function Connecting() {
  return (
    <div class="fixed inset-0 flex items-center justify-center">
      <Hud class="animate-breathe text-center">Connecting to the table…</Hud>
    </div>
  );
}

// ── Turn banner + player life orbs ─────────────────────────────────────────────────

function TurnBanner(props: { me: number; state: VisibleState }) {
  const s = () => props.state;
  const yourTurn = () => s().active_player === props.me;
  const current = () => phaseOf(s().step);
  // Show the specific step name under the band when it's more precise than the band label
  // (e.g. Combat → "Declare Attackers"); a single-step band like Main 1 adds nothing.
  const stepDetail = () => {
    const band = PHASES[current()];
    const name = STEP_NAMES[s().step] ?? String(s().step);
    return band && band.steps.length > 1 && band.name !== name ? name : null;
  };
  return (
    <Hud class="fixed top-[10px] left-1/2 z-20 flex -translate-x-1/2 flex-col items-center gap-[5px] rounded-panel border border-hud-edge px-lg py-sm shadow-hud">
      <div class={cn("font-bold text-turn-ember", yourTurn() && "text-turn-mint")}>
        {yourTurn() ? "Your turn" : `${playerLabel(s().players, s().active_player)}'s turn`}
      </div>
      <div class="flex gap-1">
        <For each={PHASES}>
          {(band, i) => {
            const state = () => (i() < current() ? "past" : i() === current() ? "now" : "future");
            return (
              <div class={phaseSegment(state(), yourTurn())}>
                {band.name}
                <Show when={i() === current() && stepDetail()}>
                  {(d) => <div class="mt-px text-micro opacity-85">{d()}</div>}
                </Show>
              </div>
            );
          }}
        </For>
      </div>
      <PriorityWatch me={props.me} state={props.state} />
    </Hud>
  );
}

// Visual escalation for the slowpoke, per `heatOf`'s thresholds.
const HEAT_INK: Record<Heat, string> = {
  sage: "text-watch-sage",
  ember: "text-turn-ember",
  flare: "text-watch-flare",
};

// Names whose priority it is with a live elapsed timer — so you can shame whoever's dawdling.
// Client-local clock (the engine is deterministic and carries no wall time).
function PriorityWatch(props: { me: number; state: VisibleState }) {
  const holder = () => props.state.priority;
  const [elapsed, setElapsed] = createSignal(0);
  // Priority changing hands restarts the shame clock: the effect re-runs, its cleanup interrupts
  // the counting fiber, and a fresh `watchElapsed` starts again from 1. There is no wall-clock
  // read here — the tick sleeps on the Effect Clock, which is what makes `heatOf`'s escalation
  // testable (lib/watch.test.ts) instead of a thirty-second wait.
  createEffect(() => {
    holder(); // tracked: a new holder restarts the count
    setElapsed(0);
    const fiber = Effect.runFork(watchElapsed(setElapsed));
    onCleanup(() => Effect.runFork(Fiber.interrupt(fiber)));
  });

  const yours = () => holder() === props.me;
  return (
    <div class={cn("font-semibold text-caption", HEAT_INK[heatOf(elapsed())], yours() && "text-turn-mint")}>
      {yours() ? "You have priority" : `Waiting on ${playerLabel(props.state.players, holder())}`}
      {/* Suppressed below 10s — a 1s "· 1s" flicker reads as noise, not signal. */}
      <Show when={elapsed() >= 10}> · {elapsed()}s</Show>
    </div>
  );
}

// ── Priority context bar: Next / Pass / yields (always bottom-right, above stack z) ─

function PriorityContextBar(props: {
  action: PrimaryAction;
  yours: boolean;
  chrome: ReturnType<typeof stackChrome>;
  turnYielded: boolean;
  /** Hidden on your own turn — turn yield means "until I become active". */
  showTurnYield: boolean;
  reject: string | null;
  staged: string | null;
  /** The staged spell may target a player, so the hint must mention their life orb. */
  stagedPlayers: boolean;
  onRun: () => void;
  onPass: () => void;
  onArmStackYield: () => void;
  onTurnYield: (enabled: boolean) => void;
  /** Clear a staged targeting cast (arrow mode). */
  onCancelTarget: (() => void) | null;
}) {
  const showNext = () => !(props.chrome.hideControlsPass && props.action.kind === "pass");
  return (
    <div
      style={{ "--b": `${HAND_BAR_H + 10}px` }}
      class="fixed right-[10px] bottom-(--b) z-25 flex flex-col items-end gap-sm"
    >
      <div class="flex flex-row-reverse flex-wrap items-center justify-end gap-sm">
        {/* Primary first in reverse row so it sits rightmost (Arena-style pass cluster). */}
        <Show when={showNext()}>
          <Button
            type="button"
            disabled={!props.yours}
            onClick={props.onRun}
            variant="game"
            class={props.action.kind !== "pass" ? "shadow-glow" : undefined}
          >
            {props.action.label}
          </Button>
        </Show>
        <Show when={props.chrome.pass}>
          <Button type="button" onClick={props.onPass} variant="game">
            Pass
          </Button>
        </Show>
        <Show when={props.chrome.stackYieldArm}>
          <Button type="button" onClick={props.onArmStackYield} variant="game-quiet">
            Auto-pass stack
          </Button>
        </Show>
        <Show when={props.chrome.stackYieldArmed}>
          <Button type="button" disabled variant="game-yielded">
            Auto-pass stack
          </Button>
        </Show>
        {/* Arena-style pass-turn rocker: icon + sliding switch, not a form checkbox. */}
        <Show when={props.showTurnYield}>
          <button
            type="button"
            role="switch"
            aria-checked={props.turnYielded}
            aria-label="Auto-pass until my turn"
            title="Auto-pass until my turn"
            onClick={() => props.onTurnYield(!props.turnYielded)}
            class={cn(
              "flex h-[42px] items-center rounded-game border border-white/12 bg-[#141c18f0] px-3",
              "transition-colors",
              props.turnYielded && "border-priority-gold/50",
            )}
          >
            <span
              class={cn(
                "relative h-[22px] w-[40px] shrink-0 rounded-full transition-colors",
                props.turnYielded ? "bg-priority-gold" : "bg-tapped-out",
              )}
            >
              <span
                class={cn(
                  "absolute top-[2px] left-[2px] flex size-[18px] items-center justify-center rounded-full",
                  "bg-snow font-bold text-[#1a221e] text-[10px] leading-none shadow-press transition-transform",
                  props.turnYielded && "translate-x-[18px] bg-[#1a221e] text-priority-gold",
                )}
                aria-hidden="true"
              >
                ≫
              </span>
            </span>
          </button>
        </Show>
        <Show when={props.onCancelTarget}>
          <Button type="button" onClick={() => props.onCancelTarget?.()} variant="game-quiet">
            Cancel
          </Button>
        </Show>
      </div>
      <Show when={props.staged}>
        <div class="max-w-[280px] text-right text-caption text-caution-amber">
          {props.staged}: click a highlighted {props.stagedPlayers ? "card or life orb" : "card"}
        </div>
      </Show>
      <Show when={props.reject}>
        <div class="text-burn-red text-caption">{props.reject}</div>
      </Show>
    </div>
  );
}

// ── Discoverability: hint strip + legend panel (findings: undiscoverable interaction grammar) ──

// A dismissible "✕" that adds no chrome of its own — used by the hint strip and legend panel.
const QUIET_CLOSE = cn("hit-quiet cursor-pointer border-none bg-transparent p-0 text-label text-lichen leading-none");

function HintStrip(props: { onDismiss: () => void }) {
  return (
    <Hud
      style={{ "--b": `${HAND_BAR_H + 10}px` }}
      // Lichen, not prose ink: this is metadata about the interface (DESIGN.md §6).
      class="fixed bottom-(--b) left-1/2 z-20 flex -translate-x-1/2 items-center gap-md text-lichen"
    >
      <span>Drag a card to play · Click a permanent to activate · Alt to inspect · Space to pass · Esc to cancel</span>
      <button type="button" aria-label="Dismiss hint" onClick={props.onDismiss} class={QUIET_CLOSE}>
        ✕
      </button>
    </Hud>
  );
}

// One canonical entry per badge/dot/outline the canvas draws, in the same colors drawCard/dot/badge
// use (ATTACK_STROKE/BLOCK_STROKE below, and the dot()/badge() calls in drawCard) — kept as data so
// this list is easy to audit against the canvas, though it's a literal copy, not a shared reference
// (those consts live further down the file, after this component).
const LEGEND_ITEMS: { color: string; shape: "dot" | "badge" | "outline"; label: string }[] = [
  { color: "#e8b24a", shape: "badge", label: "Summoning sick" },
  { color: "#7a3b13", shape: "dot", label: "Goaded" },
  { color: "#0c1412", shape: "dot", label: "Keyword / ability (Mana font)" },
  { color: "#55cc99", shape: "badge", label: "Prepared (P)" },
  { color: "#e9b84a", shape: "dot", label: "Commander" },
  { color: "#2f7d46", shape: "badge", label: "+1/+1 counters" },
  { color: "#8f2f2f", shape: "badge", label: "Marked damage" },
  { color: "#f4efe2", shape: "badge", label: "Power / toughness / loyalty" },
  { color: "#FF5555", shape: "outline", label: "Attacking" },
  { color: "#66FF99", shape: "outline", label: "Blocking" },
  { color: "rgba(0,0,0,0.45)", shape: "badge", label: "Dimmed — not usable at instant speed" },
  { color: RESPONSE_COLOR, shape: "badge", label: "Bright — usable at instant speed" },
];

function LegendPanel(props: { onClose: () => void }) {
  return (
    <Hud style={{ "--b": `${HAND_BAR_H + 92}px` }} class="fixed right-[10px] bottom-(--b) z-21 w-[220px]">
      <div class="mb-1.5 flex items-center justify-between">
        <span class="font-bold">Board legend</span>
        <button type="button" aria-label="Close legend" onClick={props.onClose} class={QUIET_CLOSE}>
          ✕
        </button>
      </div>
      <For each={LEGEND_ITEMS}>
        {(item) => (
          <div class="my-1 flex items-center gap-sm">
            {/* The swatch's colour is canvas paint, so it arrives as data (a CSS variable) and the
                classes read it — the same colours drawCard() uses, not a second encoding of them. */}
            <span style={{ "--c": item.color }} class={legendSwatch(item.shape)} />
            <span>{item.label}</span>
          </div>
        )}
      </For>
    </Hud>
  );
}
function legendSwatch(shape: "dot" | "badge" | "outline") {
  const base = "inline-block h-[14px] w-[14px] shrink-0";
  if (shape === "dot") return `${base} rounded-full border border-[#1a1a1a] bg-(--c)`;
  if (shape === "badge") return `${base} rounded-[3px] border border-[#1a1a1a] bg-(--c)`;
  return `${base} rounded-[3px] border-2 border-(--c)`;
}

// ── Stack overlay, game log, prompt modals, pile overlay ───────────────────────────

/** Right-edge / expanded / full stack presentation. Pass / yield live on the priority context bar. */
function StackOverlay(props: {
  state: VisibleState;
  /** Local preview of a hand card awaiting a target — visual top when shown. */
  staged: ObjectView | null;
  /** Pile shows the staged ghost only while arrow aiming (suspended in expand/full). */
  showPileStaged: boolean;
  /** From shared boardChrome — do not recompute with a divergent staged/mana policy. */
  allowDwell: boolean;
  viewportW: number;
  viewportH: number;
  expanded: boolean;
  onExpand: () => void;
  onCollapse: () => void;
  onHoverCard: (card: { name: string; cardId?: string; print?: string } | null) => void;
  onDwell: (dwelling: boolean) => void;
}) {
  const names = createMemo(() => new Map(props.state.objects.map((o) => [o.id, o.name])));
  const name = (id: number) => names().get(id) ?? `#${id}`;
  const byId = createMemo(
    () => new Map(props.state.objects.map((o) => [o.id, { print: o.print ?? "", cardId: o.card_id || undefined }])),
  );
  const [holdMs, setHoldMs] = createSignal(props.state.stack_hold_remaining_ms ?? 0);
  const [holdTotal, setHoldTotal] = createSignal(0);
  const [stackHover, setStackHover] = createSignal(false);
  const [dwelling, setDwelling] = createSignal(false);
  const [resolveFlash, setResolveFlash] = createSignal(false);
  createEffect(() => {
    const serverMs = props.state.stack_hold_remaining_ms ?? 0;
    if (serverMs <= 0) {
      setHoldMs(0);
      setHoldTotal(0);
      return;
    }
    setHoldTotal((t) => Math.max(t, serverMs));
    setHoldMs(serverMs);
    const started = performance.now();
    const id = window.setInterval(() => {
      const left = Math.max(0, serverMs - (performance.now() - started));
      setHoldMs(left);
      if (left <= 0) window.clearInterval(id);
    }, 100);
    onCleanup(() => window.clearInterval(id));
  });
  createEffect((prev: number | undefined) => {
    const len = props.state.stack.length;
    if (prev !== undefined && len < prev) {
      setResolveFlash(true);
      const t = window.setTimeout(() => setResolveFlash(false), 220);
      onCleanup(() => window.clearTimeout(t));
    }
    return len;
  });
  const holdPct = () => {
    const total = holdTotal();
    if (total <= 0) return 0;
    return Math.min(100, (holdMs() / total) * 100);
  };
  const cardH = () => Math.round(stackCardH());
  const visualCount = () => props.state.stack.length + (props.staged ? 1 : 0);
  const peek = () => stackPeekFor(visualCount(), props.viewportH, STACK_VERTICAL_RESERVED);
  const presentation = () =>
    stackPresentation({
      count: visualCount(),
      expandedOpen: props.expanded,
      viewportW: props.viewportW,
      viewportH: props.viewportH,
    });
  // Auto-collapse when both expand thresholds clear (or stack empties) — not while staged.
  createEffect(() => {
    if (
      !shouldAutoCollapseStackExpand({
        expanded: props.expanded,
        count: visualCount(),
        peek: peek(),
        staged: props.staged != null,
      })
    ) {
      return;
    }
    props.onCollapse();
  });
  const pileH = () => cardH() + Math.max(0, visualCount() - 1) * peek();
  const top = () => props.state.stack[props.state.stack.length - 1];
  const showMagnifier = () => stackExpandAvailable(visualCount(), peek());
  let hoveredRow: number | null = null;
  const endDwell = () => {
    if (!dwelling()) return;
    setDwelling(false);
    props.onDwell(false);
  };
  const clearHover = (row: number) => {
    if (hoveredRow !== row) return;
    hoveredRow = null;
    props.onHoverCard(null);
    endDwell();
  };
  const leaveStack = () => {
    setStackHover(false);
    if (hoveredRow !== null) clearHover(hoveredRow);
    endDwell();
  };
  const hoverEntry = (row: number, imageName: string | null, meta: { cardId?: string; print?: string }) => {
    if (!imageName) return;
    hoveredRow = row;
    props.onHoverCard({ name: imageName, cardId: meta.cardId, print: meta.print });
    if (props.allowDwell) {
      setDwelling(true);
      props.onDwell(true);
    }
  };
  onMount(() => {
    const onHide = () => {
      leaveStack();
    };
    const onVis = () => {
      if (document.hidden) leaveStack();
    };
    window.addEventListener("pagehide", onHide);
    document.addEventListener("visibilitychange", onVis);
    onCleanup(() => {
      window.removeEventListener("pagehide", onHide);
      document.removeEventListener("visibilitychange", onVis);
      leaveStack();
    });
  });

  const stackFace = (opts: {
    row: number;
    imageName: string | null;
    print: string;
    cardId?: string;
    label: string;
    isTop: boolean;
    staged?: boolean;
    style: Record<string, string>;
  }) => (
    // biome-ignore lint/a11y/noStaticElementInteractions: hover reveals art / dwell
    <div
      onMouseEnter={() => hoverEntry(opts.row, opts.imageName, { cardId: opts.cardId, print: opts.print })}
      style={opts.style}
      class={cn(
        "absolute animate-stack-in rounded-game shadow-[0_4px_14px_rgb(0_0_0/0.55)]",
        opts.staged && "ring-(--target) ring-2",
        opts.isTop && holdMs() > 0 && stackHover() && "shadow-[0_0_16px_rgba(255,215,106,0.4)]",
      )}
    >
      <Show
        when={opts.imageName}
        fallback={
          <div
            style={{ "--h": `${cardH()}px`, "--w": `${STACK_CARD_W}px` }}
            class="flex h-(--h) w-(--w) items-center justify-center rounded-game bg-[rgb(14_26_20/0.95)] px-1 text-center font-semibold text-caption text-seafoam"
          >
            {opts.label}
          </div>
        }
      >
        {(n) => <img src={imageUrlByPrint(opts.print)} alt={n()} width={STACK_CARD_W} class="block rounded-game" />}
      </Show>
    </div>
  );

  const pileBody = () => (
    <div
      style={{ "--r": `${STACK_OVERLAY_RIGHT}px`, "--w": `${STACK_CARD_W}px`, height: `${pileH()}px` }}
      class={cn("fixed top-1/2 right-(--r) z-15 w-(--w) -translate-y-1/2", resolveFlash() && "animate-stack-resolve")}
    >
      {/* biome-ignore lint/a11y/noStaticElementInteractions: hover drives hold UI / dwell */}
      <div class="relative h-full w-full" onMouseEnter={() => setStackHover(true)} onMouseLeave={leaveStack}>
        <Index each={props.state.stack}>
          {(entry, row) => {
            onCleanup(() => clearHover(row));
            createEffect((prev: string | undefined) => {
              const identity = `${entry().kind}:${entry().source}`;
              if (prev !== undefined && prev !== identity) clearHover(row);
              return identity;
            });
            const imageName = () => (entry().kind === "spell" ? entry().label : (names().get(entry().source) ?? null));
            const meta = byId().get(entry().source) ?? { print: "", cardId: undefined };
            const isTop = () => row === props.state.stack.length - 1 && !(props.staged && props.showPileStaged);
            return stackFace({
              row,
              imageName: imageName(),
              print: meta.print,
              cardId: meta.cardId,
              label: entry().label,
              isTop: isTop(),
              style: {
                "--w": `${STACK_CARD_W}px`,
                width: `${STACK_CARD_W}px`,
                bottom: `${row * peek()}px`,
                "z-index": String(row),
                left: "0",
              },
            });
          }}
        </Index>
        <Show when={props.showPileStaged ? props.staged : null}>
          {(card) =>
            stackFace({
              row: props.state.stack.length,
              imageName: card().name,
              print: card().print ?? "",
              cardId: card().card_id || undefined,
              label: card().name,
              isTop: true,
              staged: true,
              style: {
                "--w": `${STACK_CARD_W}px`,
                "--target": TARGET_COLOR,
                width: `${STACK_CARD_W}px`,
                bottom: `${props.state.stack.length * peek()}px`,
                "z-index": String(props.state.stack.length),
                left: "0",
              },
            })
          }
        </Show>
        <Show when={showMagnifier()}>
          <Button
            type="button"
            aria-label={`Expand stack (${visualCount()} objects)`}
            onClick={props.onExpand}
            variant="ghost"
            class="absolute -top-9 right-0 flex items-center gap-1 px-2 py-1 text-chip text-seafoam"
          >
            Expand · {visualCount()}
          </Button>
        </Show>
      </div>
      <div class="absolute top-full right-0 left-0 mt-sm flex flex-col items-center gap-sm">
        <Show when={stackHover() && holdMs() > 0 && !props.staged}>
          <div
            style={{ "--w": `${STACK_CARD_W}px` }}
            class="h-1.5 w-(--w) overflow-hidden rounded-full bg-white/15"
            aria-hidden="true"
          >
            <div
              style={{ width: `${holdPct()}%` }}
              class="h-full rounded-full bg-priority-gold transition-[width] duration-100 ease-linear"
            />
          </div>
        </Show>
        <Show when={props.staged && props.showPileStaged}>
          <div
            style={{ "--w": `${STACK_CARD_W}px`, "--target": TARGET_COLOR }}
            class="max-w-(--w) text-center text-(--target) text-chip"
          >
            Choose a target
          </div>
        </Show>
        <Show when={!props.staged && top()}>
          {(t) => (
            <div style={{ "--w": `${STACK_CARD_W}px` }} class="max-w-(--w) text-center text-chip text-seafoam">
              <Show when={t().kind === "ability"}>
                <div class="font-semibold">{t().label}</div>
              </Show>
              <Show when={t().target}>
                {(target) => <div>→ {targetLabel(target(), name, props.state.players)}</div>}
              </Show>
            </div>
          )}
        </Show>
      </div>
    </div>
  );

  const stripOrFullBody = () => {
    const mode = presentation();
    const items = () => {
      const list = props.state.stack.map((entry, row) => {
        const meta = byId().get(entry.source) ?? { print: "", cardId: undefined };
        return {
          row,
          imageName: entry.kind === "spell" ? entry.label : (names().get(entry.source) ?? null),
          print: meta.print,
          cardId: meta.cardId,
          label: entry.label,
          staged: false as boolean,
        };
      });
      if (props.staged) {
        list.push({
          row: props.state.stack.length,
          imageName: props.staged.name,
          print: props.staged.print ?? "",
          cardId: props.staged.card_id || undefined,
          label: props.staged.name,
          staged: true,
        });
      }
      return list;
    };
    const n = () => items().length;
    const hPeek = () =>
      mode === "full" ? STACK_STRIP_MIN_PEEK : Math.max(STACK_STRIP_MIN_PEEK, stackStripPeek(n(), props.viewportW));
    const perRow = () => (mode === "full" ? stackFullPerRow(props.viewportW) : n());
    const rows = () => Math.ceil(n() / perRow());
    const stripW = () => {
      const cols = Math.min(n(), perRow());
      return STACK_CARD_W + Math.max(0, cols - 1) * hPeek();
    };
    const stripH = () => cardH() + Math.max(0, rows() - 1) * (cardH() * 0.35);
    return (
      <div
        class={cn(
          "fixed z-15 flex flex-col items-center gap-sm",
          mode === "full" ? "top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2" : "top-1/2 right-4 -translate-y-1/2",
          resolveFlash() && "animate-stack-resolve",
        )}
        style={{
          width:
            mode === "full" ? `${Math.min(props.viewportW - STACK_HORIZONTAL_MARGIN, stripW())}px` : `${stripW()}px`,
          "max-width": `${props.viewportW - STACK_HORIZONTAL_MARGIN}px`,
        }}
      >
        <div class="flex w-full items-center justify-between gap-sm">
          <span class="text-chip text-seafoam">
            Stack · {visualCount()}
            {mode === "full" ? " · full" : ""}
          </span>
          <Button
            type="button"
            aria-label="Collapse stack"
            onClick={props.onCollapse}
            variant="ghost"
            class="px-2 py-1 text-chip"
          >
            ✕
          </Button>
        </div>
        {/* biome-ignore lint/a11y/noStaticElementInteractions: hover drives hold UI / dwell */}
        <div
          class="relative"
          style={{ width: `${stripW()}px`, height: `${stripH()}px` }}
          onMouseEnter={() => setStackHover(true)}
          onMouseLeave={leaveStack}
        >
          <For each={items()}>
            {(item) => {
              const col = () => item.row % perRow();
              const rowY = () => Math.floor(item.row / perRow());
              const isTop = () => item.row === n() - 1;
              onCleanup(() => clearHover(item.row));
              return stackFace({
                row: item.row,
                imageName: item.imageName,
                print: item.print,
                cardId: item.cardId,
                label: item.label,
                isTop: isTop(),
                staged: item.staged,
                style: {
                  "--w": `${STACK_CARD_W}px`,
                  "--target": TARGET_COLOR,
                  width: `${STACK_CARD_W}px`,
                  left: `${col() * hPeek()}px`,
                  top: `${rowY() * cardH() * 0.35}px`,
                  "z-index": String(item.row),
                },
              });
            }}
          </For>
        </div>
        <Show when={stackHover() && holdMs() > 0 && !props.staged}>
          <div
            style={{ "--w": `${STACK_CARD_W}px` }}
            class="h-1.5 w-(--w) overflow-hidden rounded-full bg-white/15"
            aria-hidden="true"
          >
            <div
              style={{ width: `${holdPct()}%` }}
              class="h-full rounded-full bg-priority-gold transition-[width] duration-100 ease-linear"
            />
          </div>
        </Show>
        <Show when={!props.staged && top()}>
          {(t) => (
            <div class="max-w-[280px] text-center text-chip text-seafoam">
              <Show when={t().kind === "ability"}>
                <div class="font-semibold">{t().label}</div>
              </Show>
              <Show when={t().target}>
                {(target) => <div>→ {targetLabel(target(), name, props.state.players)}</div>}
              </Show>
            </div>
          )}
        </Show>
      </div>
    );
  };

  return (
    <Show when={visualCount() > 0}>
      <Show when={presentation() === "pile"} fallback={stripOrFullBody()}>
        {pileBody()}
      </Show>
    </Show>
  );
}

// The end of the game, said out loud. Before this, a winner simply kept priority over a board of
// faded avatars and nobody was told anything. "Keep watching" dismisses it — an eliminated player
// stays at the table to see how it finishes — and "Back to your decks" is the board's only exit.
function ResultOverlay(props: { outcome: Outcome; onWatch: () => void; onLeave: () => void }) {
  const headline = () =>
    Match.value(props.outcome).pipe(
      Match.withReturnType<string>(),
      Match.discriminatorsExhaustive("kind")({
        won: () => "You win",
        lost: (o) => (o.winner === null ? "You're eliminated" : `${seatName(o.winner)} wins`),
        over: (o) => (o.winner === null ? "Nobody wins" : `${seatName(o.winner)} wins`),
        playing: () => "", // never rendered — the overlay is gated on kind !== "playing"
      }),
    );
  // A player who lost while the game continues is out, but the game isn't over; say which it is.
  const detail = () =>
    Match.value(props.outcome).pipe(
      Match.withReturnType<string>(),
      Match.discriminatorsExhaustive("kind")({
        won: () => "Last player standing.",
        lost: (o) => (o.winner === null ? "The game continues without you." : "You were eliminated."),
        over: () => "The game is over.",
        playing: () => "",
      }),
    );
  const stillRunning = () => props.outcome.kind === "lost" && props.outcome.winner === null;
  return (
    // z 55: above a pending decision's prompt (50) — once the game is over, nothing outranks it.
    <div class="fixed inset-0 z-55 flex items-center justify-center bg-black/70">
      <Modal class="flex max-w-[420px] flex-col items-center gap-lg text-center">
        <div class="font-bold text-title">{headline()}</div>
        <div class="text-label text-lichen">{detail()}</div>
        <div class="flex gap-md">
          <Button type="button" onClick={props.onWatch} variant="ghost">
            {stillRunning() ? "Keep watching" : "Stay on the board"}
          </Button>
          <Button type="button" onClick={props.onLeave}>
            Back to your decks
          </Button>
        </div>
      </Modal>
    </div>
  );
}

/** How a seat is named everywhere else on this board (the turn banner, the priority watch). */
const seatName = (seat: number) => playerLabel(game.state?.players ?? [], seat);

function LogPanel() {
  let panel: HTMLDivElement | undefined;
  const lines = () => game.log.slice(-30);
  // New lines land at the bottom, but a scroll container stays pinned at the top as it grows — so
  // once the log outgrows its 150px, the line you actually want to read is the one below the fold.
  // Follow it. (Tracking `lines()`, not `game.log.length`, which stops changing once the log caps.)
  createEffect(() => {
    lines();
    if (panel) panel.scrollTop = panel.scrollHeight;
  });
  return (
    <Show when={game.log.length > 0}>
      <Hud
        ref={panel}
        role="log"
        aria-live="polite"
        style={{ "--b": `${HAND_BAR_H + 10}px` }}
        class="fixed bottom-(--b) left-[72px] z-10 max-h-[150px] w-[300px] overflow-y-auto"
      >
        <For each={lines()}>
          {(l) => (
            <div class={cn("text-caption", l.auto ? "flex items-start gap-xs text-snow-mint" : "text-mist")}>
              <Show when={l.auto}>
                <span class="mt-px shrink-0 rounded-full bg-auto-moss px-[5px] py-px font-bold text-micro text-snow-mint tracking-[0.06em]">
                  AUTO
                </span>
              </Show>
              <span>{l.text}</span>
            </div>
          )}
        </For>
      </Hud>
    </Show>
  );
}

function PileOverlay(props: { cards: ObjectView[]; onClose: () => void }) {
  return (
    // Click-outside-to-dismiss is a redundant shortcut; the Close button below is the keyboard path.
    // biome-ignore lint/a11y/noStaticElementInteractions: backdrop, not a control
    // biome-ignore lint/a11y/useKeyWithClickEvents: same
    <div
      onClick={(e) => {
        if (e.target === e.currentTarget) props.onClose(); // a click on the modal itself isn't "outside"
      }}
      class="fixed inset-0 z-29 bg-black/50"
    >
      <Modal class="fixed top-[45%] left-1/2 z-30 max-w-[520px] -translate-x-1/2 -translate-y-1/2">
        <div class={PROMPT_TITLE}>Pile ({props.cards.length})</div>
        <div class="flex flex-wrap gap-xs">
          <For each={props.cards}>
            {(c) => <img src={imageUrlByPrint(c.print ?? "")} alt={c.name} width={90} class="rounded-md" />}
          </For>
        </div>
        <div class={cn(PROMPT_ROW, "mt-sm")}>
          <Button type="button" onClick={props.onClose} variant="ghost">
            Close
          </Button>
        </div>
      </Modal>
    </div>
  );
}

// ── Shared class strings ─────────────────────────────────────────────────────────────

// A single band in the phase track: past bands read as done, the current one lights up (green on
// your turn, amber on an opponent's), future bands stay faint. Phase Fern on the future band's
// fill computes to ~6.5:1, clearing the 4.5:1 contrast floor while staying quieter than past/now's
// Snow Mint (~16.6:1).
function phaseSegment(state: "past" | "now" | "future", yourTurn: boolean): string {
  return cn(
    // Fixed equal width — sized for the longest step detail ("First Strike Damage" at text-micro).
    "w-[7.5rem] rounded-[7px] border border-transparent px-md py-1 text-center font-semibold text-caption",
    "bg-[rgb(24_34_30/0.6)] text-phase-fern", // future: the resting band
    state === "past" && "bg-[rgb(40_55_48/0.9)] text-snow-mint",
    state === "now" && "text-snow-mint",
    state === "now" && yourTurn && "border-phase-mint bg-[rgb(60_150_95/0.9)]",
    state === "now" && !yourTurn && "border-phase-ember bg-[rgb(150_95_55/0.9)]",
  );
}
