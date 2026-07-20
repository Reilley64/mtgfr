/**
 * @vitest-environment happy-dom
 *
 * ActionSession pulls ActionChrome → prompt-forms → card-preview, which registers
 * Solid delegated events on `window` at module load. Default vitest env is node.
 */
import { createRoot } from "solid-js";
import { describe, expect, it, vi } from "vitest";
import { useActionSession } from "~/controllers/action-session";
import { emptyCostPicks, useActionExecution } from "~/controllers/actionExecution";
import { fitCamera } from "~/lib/interaction";
import type { ActionView, ObjectView, VisibleState } from "~/wire/types";

const mkAction = (over: Partial<ActionView> = {}): ActionView => ({
  id: 1,
  kind: "cast",
  label: "Bolt",
  needs_target: true,
  object: 7,
  section: "hand",
  ...over,
});

const card = (id: number): ObjectView =>
  ({
    id,
    name: `Card ${id}`,
    mana_cost: { has_x: false },
  }) as ObjectView;

function sessionDeps(state: VisibleState | null = null, act = vi.fn(async () => true)) {
  return {
    me: () => 0,
    act,
    getState: () => state,
    camera: () => fitCamera({ x: 800, y: 600 }, 2, 210),
    size: () => ({ x: 800, y: 600 }),
    handBarH: 210,
    setReject: vi.fn(),
    seedDrop: vi.fn(),
    clearPlayOrigin: vi.fn(),
    onHintUsed: vi.fn(),
  };
}

describe("ActionSession / ActionChrome cancel contract", () => {
  it("exposes play/aim/cancel/overlay/Chrome without an execution escape hatch", () => {
    createRoot((dispose) => {
      const session = useActionSession(sessionDeps());
      expect(session.play).toBeTypeOf("function");
      expect(session.aim).toBeTypeOf("function");
      expect(session.cancel).toBeTypeOf("function");
      expect(session.playObjectCast).toBeTypeOf("function");
      expect(session.overlay).toBeTypeOf("function");
      expect(session.Chrome).toBeTypeOf("function");
      expect(session).not.toHaveProperty("execution");
      dispose();
    });
  });

  it("staged-pick cancel via setStaged(null) leaves X prompt alone", () => {
    // Regression lock for review: ActionChrome staged-pick onCancel must unstage only —
    // full cancelActionState would wipe an unrelated X prompt if both were somehow live.
    createRoot((dispose) => {
      const execution = useActionExecution(sessionDeps());
      execution.setStaged({
        card: card(7),
        action: mkAction(),
        picks: emptyCostPicks(),
        preferPick: true,
        playOrigin: { x: 0, y: 0 },
        playOriginScreen: { x: 100, y: 200 },
      });
      execution.setXPrompt({
        name: "Fireball",
        minX: 0,
        maxX: 1,
        xCost: { generic: 0, colored: [0, 0, 0, 0, 0], has_x: true, x_symbols: 1 },
        submit: () => {},
      });

      execution.cancelStagedOnly(); // ActionChrome staged-pick cancel path
      expect(execution.staged()).toBeNull();
      expect(execution.xPrompt()?.name).toBe("Fireball");

      execution.cancelActionState(); // Escape / session.cancel — clears everything
      expect(execution.xPrompt()).toBeNull();
      dispose();
    });
  });

  it("opens X prompt with the action's bounded cost", async () => {
    const xAction = mkAction({
      has_x: true,
      min_x: 0,
      max_x: 3,
      x_cost: { generic: 1, colored: [0, 0, 0, 0, 0], has_x: true, x_symbols: 2 },
    });
    const state = { actions: [xAction], objects: [card(7)] } as VisibleState;
    createRoot((dispose) => {
      const execution = useActionExecution(sessionDeps(state));

      void execution.takeCastAction(xAction, null);

      expect(execution.xPrompt()).toMatchObject({
        name: "Card 7",
        minX: 0,
        maxX: 3,
        xCost: xAction.x_cost,
      });
      dispose();
    });
  });
});
