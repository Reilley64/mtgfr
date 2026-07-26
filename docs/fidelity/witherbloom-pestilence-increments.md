# Witherbloom Pestilence deck increments (2026-07-26)

Deck report: [witherbloom-pestilence.md](witherbloom-pestilence.md). This file is the sole
engine-capability backlog for this deck (ranked increments plus the concrete Witherbloom cards they
unblock).

From `docs/decklists/witherbloom_pestilence.md` (official Wizards `soc` precon; commander Dina,
Essence Brewer). After the documentation re-audit, three deck cards need engine work and one stays
on an explicit approximation. The real residuals are narrow: cast-time self-copy timing
(`Ominous Harvest`, `Plumb the Forbidden`), a mandatory graveyard-return choice
(`Witherbloom Command`), and the already-declared `Final Act` mode family that still depends on
surfaces the pool does not model.

### 1. `cast-time-self-copy-from-cast-context` — 2 cards, M

**Depends on:** none.
**Cards:** `ominous_harvest.toml`, `plumb_the_forbidden.toml`
**Sketch:** both cards still use the old resolution-time `Effect::CopyThisSpell` rider, but their
copies belong to cast-time context instead. `Ominous Harvest`'s Gravestorm should snapshot
"permanents put into a graveyard from the battlefield this turn" as the spell is cast, mint that
many copies immediately, and let those copies sit on the stack above the original. `Plumb the
Forbidden`'s "When you do, copy this spell for each creature sacrificed this way" should likewise
read the already-recorded `spell_sacrifice_count` from the cast and mint the copies before the
original begins resolving, not after the original draw/life step runs. The cleanest route is a
cast-trigger self-copy primitive alongside the existing storm / `copy_triggering_spell` family,
rather than stretching the resolution rider further.

**Slices:**
1. **Additional-cost copy timing (S).** Land a cast-time path that reads `spell_sacrifice_count`
   and mints Plumb's copies immediately after the cast is committed.
2. **Gravestorm timing (S).** Repoint Ominous Harvest from the same resolution rider to the
   cast-time path, snapshotting `permanents_died_this_turn` the same way Storm snapshots prior
   spells cast.

### 2. `mandatory-return-from-your-graveyard-to-hand` — 1 card, S

**Depends on:** none.
**Cards:** `witherbloom_command.toml`
**Sketch:** the current `may_return_from_graveyard` helper is optional-only, but Witherbloom
Command's first mode is mandatory whenever you have at least one land card in your graveyard. Add a
mandatory sibling (or an optionality flag) that:

- returns nothing when no legal card exists,
- forces a choice when one or more legal cards exist,
- keeps the current optional behavior unchanged for cards like `Witch of the Moors`.

That restores the printed "target player mills three cards, then you return a land card from your
graveyard to your hand" without papering over a real graveyard-resource choice.

### 3. `final-act-missing-mode-family` — 1 card, L — **DEFERRED APPROXIMATION**

**Depends on:** battle permanents and player-counter tracking (neither exists in the current pool).
**Cards:** `final_act.toml`
**Sketch:** `Final Act` is already honest about its residual: the engine can destroy all creatures,
destroy all planeswalkers, and exile all graveyards, but it cannot yet express "destroy all
battles" or "each opponent loses all counters." Keep the card in section B for Witherbloom rather
than promoting it to D: this deck does not falsify the existing premise that neither missing surface
is live yet. Once battles or player counters enter the faithful target set, land both missing
families and widen the authored modal card back to its printed five-mode "choose one or more."
