// ActionSession — deep module at the seam between Board gestures and WireIntent.
//
// In Foldkit, session state lives in BoardModel (staged, xPrompt, modalCast, cost picks) and the
// session verbs are Board messages. This module re-exports the pure planners + types so callers
// have one import surface, matching the Solid `action-session.tsx` shape.

export {
  buildTakeActionIntent,
  type CostPickState,
  type CostPicks,
  emptyCostPicks,
  findCastActionForObject,
  type HandDropPlan,
  type ModalCast,
  planCastClickResolution,
  planCostPipeline,
  planHandDrop,
  planRunAction,
  type RunActionPlan,
  type StagedAction,
  settleSacrificePick,
  stagedCastSubmission,
  usedCostPick,
  type XPromptState,
} from "./execution";
export { advance, type ModalStep, modeAvailable } from "./modal";
export {
  askFor,
  onBoard,
  stagedPickTargets,
  stagedTargetTitle,
  type TargetMode,
  targetMode,
} from "./targeting";
