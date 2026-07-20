// Fidelity-grind wave orchestration. See SKILL.md Phase 4.
// Copy to scratch space, replace {{WORKTREE}} (absolute worktree path), {{BRANCH}},
// {{BACKLOG_RANGE}} (e.g. "#167-#202"), {{BACKLOG_FILE}} (the deck's increments file,
// e.g. "docs/fidelity/<slug>-increments.md"), and {{SHARED}} (absolute path of the filled
// shared-context-template.md). Relaunch after committing each green wave.
export const meta = {
  name: 'fidelity-grind-wave',
  description: 'One self-planned wave of the fidelity backlog: a planner picks the next dependency-safe batch of increments, implements each TDD in sequence on the shared tree, then verifies adversarially.',
  phases: [
    { title: 'Plan', detail: 'planner selects the next batch + writes a brief per increment' },
    { title: 'Implement', detail: 'each selected increment, TDD, sequential on the shared tree' },
    { title: 'Verify', detail: 'full test/clippy/fmt, adversarial diff review, reconcile notes, update backlog' },
  ],
}

const ROOT = '{{WORKTREE}}'
const shared = '{{SHARED}}'
const BRIEF_DIR = shared.slice(0, shared.lastIndexOf('/'))

const PLAN_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['done', 'rationale', 'increments'],
  properties: {
    done: { type: 'boolean', description: 'true ONLY when every backlog increment in range and every per-card exotic is landed or provably ineligible (dead variant / absent subsystem) — i.e. no eligible work remains.' },
    rationale: { type: 'string', maxLength: 1200, description: 'Why this batch (or why done). TERSE — detailed reasoning belongs in the brief files.' },
    increments: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['num', 'title', 'effort', 'briefPath'],
        properties: {
          num: { type: 'string', description: 'Backlog number, or "exotic:<card>" for a per-card exotic' },
          title: { type: 'string' },
          effort: { type: 'string', enum: ['S', 'M', 'L', 'XL'] },
          briefPath: { type: 'string', description: 'Absolute path of the self-sufficient handoff brief this planner wrote' },
        },
      },
    },
  },
}

const VERIFY_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['green', 'testsPassed', 'newClippyWarnings', 'report', 'cardsFixed', 'stillApproximated'],
  properties: {
    green: { type: 'boolean', description: 'true iff cargo test passes AND cargo fmt --check clean AND no NEW clippy warnings vs the pre-wave git-stash baseline.' },
    testsPassed: { type: 'integer' },
    newClippyWarnings: { type: 'boolean' },
    report: { type: 'string' },
    cardsFixed: { type: 'array', items: { type: 'string' } },
    stillApproximated: { type: 'array', items: { type: 'string' }, description: 'Cards touched but left approximated, each with the unlanded prereq.' },
  },
}

phase('Plan')
const plan = await agent(
  `You are the PLANNER for one wave of the mtgfr engine fidelity grind. Repo root: ${ROOT} (branch {{BRANCH}}).\n\n` +
  `The backlog is {{BACKLOG_FILE}} only — increments {{BACKLOG_RANGE}}. There is no global FIDELITY_BACKLOG; deps may reference earlier increments in this same file (or already-landed code). Increments marked "**LANDED**" are DONE; do not repick them. VERIFY eligibility against the card TOMLs and the code, not backlog prose — "still blocked" lists go stale.\n\n` +
  `Grep to find where things live NOW; do not assume file layouts from older briefs. docs/agent-navigation.md maps the layout.\n\n` +
  `Your job: pick the NEXT batch of increments, then write a self-sufficient handoff brief for each.\n\n` +
  `Selection rules (in order):\n` +
  `1. An increment is eligible ONLY if (a) not already landed, (b) every "Depends on:" dep is landed, and (c) at least one real pool card in crates/cards/data/*.toml would become faithful or measurably closer (grep the approximates notes for a live consumer — flag-don't-force, never add a dead variant). IMPORTANT: an increment whose *Cards:* line names a deck card NOT YET on disk is STILL eligible — the increment AUTHORS that card (TOML + tests) as part of its own work; "card absent from the pool" is never a reason to skip an increment in a deck grind.\n` +
  `2. EVERY WAVE MUST CARRY ONE XL SLICE while any XL increment or L-sized new-mechanic exotic remains unlanded: pick exactly ONE and scope its brief to the NEXT UNBUILT STAGED SLICE (an XL gets a dated progress note, not a LANDED mark, until all slices are in). Put it LAST. Prefer finishing an in-progress XL over starting a new one. Alongside it, up to 3 independent S/M/exotic picks (or 1 L) touching clearly disjoint subsystems — riders never displace the XL; if in doubt about disjointness, drop the rider. Once ALL XLs are landed: pick up to 6 eligible S/M/exotics (an L may ride with up to 3 disjoint S/M; if no S/M remain, ONE L plus disjoint exotics).\n` +
  `3. Set done=true and increments=[] ONLY if nothing is eligible (everything landed, or every remainder is a documented dead variant / absent subsystem — explain in rationale).\n\n` +
  `NEVER paraphrase a card's oracle text, mana cost, or P/T from memory in a brief — the Xira grind's briefs got five cards' costs/text wrong and only live re-verification saved them. Quote text fetched from Scryfall this session, or instruct the implementer to fetch it; implementers must verify against live Scryfall regardless.\n\n` +
  `For each pick, WRITE a handoff brief to ${BRIEF_DIR}/wave-<num>-<slug>.md that a fresh agent can execute with no other context. Each brief must: name the backlog section, state the goal + design sketch, list the exact example cards (grep data/ for filenames + current approximates notes), name the files to touch, give a TDD order, and state the definition of done (named cards faithful or trimmed with a precise residual; cargo test/clippy/fmt green). Mandate the shared conventions in ${shared} and invoke the test-driven-development + card-dsl skills.\n\n` +
  `Batch AGGRESSIVELY on count; be conservative ONLY about eligibility and dependency-safety. Keep rationale under 1200 chars and ALWAYS emit the increments array.`,
  { label: 'plan next wave', phase: 'Plan', model: 'opus', agentType: 'general-purpose', schema: PLAN_SCHEMA })

if (plan.done || !plan.increments.length) {
  return { done: true, rationale: plan.rationale, verify: null }
}

phase('Implement')
const implResults = []
for (const inc of plan.increments) {
  const model = (inc.effort === 'L' || inc.effort === 'XL') ? 'opus' : 'sonnet'
  const res = await agent(
    `You are implementing ONE increment of the mtgfr fidelity backlog on the real working tree (branch {{BRANCH}}).\n` +
    `FIRST read ${shared} for shared constraints, then read ${inc.briefPath} and do exactly what it says.\n` +
    `Invoke the test-driven-development and card-dsl skills. TDD: failing engine test in crates/engine/tests/game.rs FIRST.\n` +
    `You MUST leave the tree with \`cargo test\` GREEN, \`cargo fmt\` applied, and NO NEW \`cargo clippy --all-targets\` warnings vs baseline — the next agent inherits this exact tree. Do not commit.\n` +
    `Return a terse summary: cards made faithful, cards left approximated (and the unlanded prereq), files touched, follow-ups noticed.`,
    { label: `#${inc.num} ${inc.title} (${inc.effort})`, phase: 'Implement', model, agentType: 'general-purpose' })
  implResults.push({ inc, summary: res })
}

phase('Verify')
const picked = plan.increments.map(i => `#${i.num} ${i.title}`).join(', ')
const summaries = implResults.map(r => `--- #${r.inc.num} ${r.inc.title} ---\n${r.summary}`).join('\n\n')
const verify = await agent(
  `You are the VERIFY + RECONCILE stage for one fidelity wave in ${ROOT} (branch {{BRANCH}}, current working tree).\n` +
  `Increments implemented this wave: ${picked}. Shared context: ${shared}.\n\n` +
  `Implementer summaries (do NOT trust them — verify against the code and diff):\n${summaries}\n\n` +
  `Do ALL of the following (verification-before-completion: evidence before any green claim; requesting-code-review for the adversarial diff pass; systematic-debugging if something is red and the cause is unclear):\n` +
  `1. Run \`cargo test --workspace\`, \`cargo clippy --all-targets\`, \`cargo fmt --check\`. Establish whether any clippy warning is NEW vs the pre-wave baseline via \`git stash\` comparison. If the tree is red/dirty or has a NEW warning, FIX it if small and safe; otherwise leave it buildable and report exactly what's wrong.\n` +
  `2. Adversarially review the wave's \`git diff\` for correctness vs the MTG Comprehensive Rules and project standards (guard-return-first, CardDef stays Copy, ponytail comments on shortcuts, Magic terminology). Fix real bugs if small; regression-test every fix.\n` +
  `3. Reconcile approximations: for every card the wave claims faithful, open its TOML and confirm the note was correctly removed or trimmed to only the residual gap. Enforce every convention in ${shared} (no faithful-asserting comments; bare oracle quotes above every abilities/effects block; array form; DSL_REFERENCE backfilled for any new TOML surface — diff types/de against it).\n` +
  `4. Update {{BACKLOG_FILE}} from the ACTUAL TOML diffs: LANDED marks with dated Landed:/Still blocked: lines; XL slices get dated progress notes instead. Do not write a global fidelity backlog.\n` +
  `5. Run \`just engine-cr-index\` so docs/CR_INDEX.md includes the wave's citations.\n` +
  `6. FRAME AUDIT every card TOML this wave added or touched: diff cost/type/P-T/legendary/verbatim-oracle against a live Scryfall fetch and fix EVERY mismatch — a wave once shipped a card modeled on stale mandatory oracle text (current text says "you may"), a behavioral bug invisible to its own tests. Zero frame problems is a green=true requirement.\n` +
  `7. After any edits, re-run \`cargo test\` and \`cargo fmt\` so the tree ends green and formatted.\n\n` +
  `Set green=true ONLY if tests pass, fmt is clean, and there are no NEW clippy warnings. Return the structured result honestly — no silent passes.`,
  { label: 'verify + reconcile + backlog', phase: 'Verify', model: 'opus', agentType: 'general-purpose', schema: VERIFY_SCHEMA })

return { done: false, picked, rationale: plan.rationale, implResults, verify }
