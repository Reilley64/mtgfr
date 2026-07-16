/**
 * @vitest-environment happy-dom
 *
 * ActionSession pulls ActionChrome → prompt-forms → card-preview, which registers
 * Solid delegated events on `window` at module load. Default vitest env is node.
 */
import { createRoot } from "solid-js";
import { describe, expect, it, vi } from "vitest";
import type { ActionView, ObjectView, VisibleState } from "~/api/generated";
import { useActionSession } from "~/controllers/action-session";
import { emptyCostPicks, useActionExecution } from "~/controllers/actionExecution";
import { fitCamera } from "~/lib/interaction";

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
      });
      execution.setXPrompt({ name: "Fireball", submit: () => {} });

      execution.setStaged(null); // ActionChrome staged-pick cancel path
      expect(execution.staged()).toBeNull();
      expect(execution.xPrompt()?.name).toBe("Fireball");

      execution.cancelActionState(); // Escape / session.cancel — clears everything
      expect(execution.xPrompt()).toBeNull();
      dispose();
    });
  });
});
