# Fidelity report — Deathdancer Xira (Magic Online Theme Decks)

Source: https://archidekt.com/decks/2209179/deathdancer_xira_magic_online_theme_decks
(Archidekt deck 2209179, fetched 2026-07-17). 75 unique non-basic cards + basics.
Commander: **Xira Arien**. Backlog increments: #167–#202.

Intake counts: 9 faithful / 1 approximated / 39 expressible / 26 needing engine work.

**Final state (2026-07-18): 75/75 cards in the pool, all fully faithful — 0 residuals.**
Animate Dead is a real Aura reanimator (#199) whose printed enchant self-rewrite is modeled
literally: attaching records the reanimated object as the Aura's only legal host, and the
CR 704.5m sweep holds it to exactly that object (its intervening "if it's on the battlefield"
is regression-tested too). All 36 increments #167–#202 landed across ten green waves; the
deck ships as the in-app precon **Deathdancer Xira (id -7)**.

## A. In pool, faithful at intake (9)

- [x] Anger
- [x] Elvish Visionary
- [x] Krosan Tusker
- [x] Lightning Bolt
- [x] Lightning Greaves
- [x] Rupture Spire
- [x] Sakura-Tribe Elder
- [x] Terramorphic Expanse
- [x] Wirewood Guardian

## B. In pool, approximated at intake (1)

- [x] Animate Dead — intake note (now fully closed): "typed as an enchantment, not an aura
  kind, since the aura resolution path needs an existing battlefield host to attach to; the
  printed \"loses/gains enchant ability\" self-rewrite is modeled implicitly as staying
  attached rather than as a literal ability rewrite" → #199, then the rewrite modeled
  literally (`enchant_rewrite_host` consulted by the CR 704.5m sweep) — no `approximates`
  remains

## C. New, expressible today (39)

Authored in the pure-authoring pass, no engine change needed. Cards marked (†#N) have a
pending correctness increment that adds regression coverage around them — the DSL expresses
the card today; the increment fixes an engine-global gap their presence makes observable.

- [x] Abyssal Gatekeeper (†#183)
- [x] Ancient Grudge
- [x] Barren Moor
- [x] Brawn (†#186)
- [x] Carrion Feeder
- [x] Chartooth Cougar
- [x] Civic Wayfinder
- [x] Darigaaz's Caldera
- [x] Elven Cache
- [x] Eternal Witness
- [x] Explosive Vegetation
- [x] Fires of Yavimaya
- [x] Flametongue Kavu (†#180 creature-type lexicon)
- [x] Forgotten Cave
- [x] Genesis
- [x] Ghost Quarter
- [x] Golgari Rot Farm
- [x] Golgari Signet
- [x] Gruul Signet
- [x] Gruul Turf
- [x] Harmonize
- [x] Hissing Iguanar (†#184, †#187)
- [x] Jund Panorama
- [x] Keldon Vandals (†#181)
- [x] Kodama's Reach
- [x] Penumbra Bobcat (†#183)
- [x] Phyrexian Arena (†#185)
- [x] Putrefy
- [x] Rakdos Carnarium
- [x] Rakdos Signet
- [x] Recollect
- [x] Resounding Thunder
- [x] Rootbreaker Wurm (†#186)
- [x] Savage Lands
- [x] Skullclamp (†#183)
- [x] Terminate
- [x] Tranquil Thicket
- [x] Xira Arien
- [x] Yavimaya Elder (†#183)

Notes: Edge of Autumn's "four or fewer lands" gate is expressible as the negated
`you_control_lands at_least = 5` condition (Court Hussar idiom) — the card sits in D only
for its nonmana cycling cost. Searches reveal nothing in this model (tracked Land Tax
idiom), which matches every printed "reveal it" search here.

## D. New, needs engine work (26)

- [x] All Hallow's Eve → #202
- [x] Anarchist → #167
- [x] Ashes to Ashes → #168, #173
- [x] Avatar of Woe → #170, #171
- [x] Buried Alive → #172
- [x] Cauldron Dance → #179, #197
- [x] Chain Lightning → #191
- [x] Constant Mists → #178
- [x] Dread Return → #189
- [x] Edge of Autumn → #190
- [x] Golgari Grave-Troll → #174, #200
- [x] Golgari Thug → #200 (faithful; dies-tuck + Dredge 4)
- [x] Grim Harvest → #188
- [x] Life from the Loam → #175, #200
- [x] Massacre → #192
- [x] Nezumi Graverobber // Nighteyes the Desecrator → #176, #201 (faithful both faces; flip renders on the wire)
- [x] Reaping the Graves → #198
- [x] Shambling Shell → #200 (faithful; sac-for-counter + Dredge 3)
- [x] Shriekmaw → #168, #170
- [x] Stinkweed Imp → #193, #200
- [x] Terror → #168
- [x] Twisted Abomination → #174
- [x] Vampiric Dragon → #194 (faithful; turn-scoped damaged-by death-watch)
- [x] Werebear → #171
- [x] Wickerbough Elder → #195
- [x] Wild Mongrel → #177, #196

## Observability re-audit (falsified pool-absence claims)

19 claims falsified by this deck (171 checked), each folded into the increment that clears it:

1. `effect.rs:967` + `effects.rs:3013` — "no pool card grants a regeneration shield" /
   SBA-destroy never consults shields → Twisted Abomination, Golgari Grave-Troll — #174.
2. `triggers.rs:107` — Dies keyed off `MovedToGraveyard` with no source zone; "no pool card
   has both a Dies trigger and is a discard target" → Wild Mongrel discards, dredge mills,
   Buried Alive entombs dies-trigger creatures — #183.
3. `triggers.rs:114` — simultaneous-with-player-loss deaths suppress other players'
   death-watch; "no pool card cares" → Hissing Iguanar — #184.
4. `triggers.rs:3188` — two controllers triggering off one event "untested-but-correct" →
   Hissing Iguanar + any opponent's dies trigger — regression test rides #184.
5. `effect.rs:2423` — storm copies as resolution rider; "no pool storm card would see the
   difference" → Reaping the Graves (targeted storm) — #198.
6. `effect.rs:1884` — ReturnThisToHand "finds the source wherever it now lives"; "no pool
   card contests the graveyard in between" → Nezumi Graverobber exiles from graveyards — #182.
7. `label.rs:1035` — SacrificeSelfUnlessPay label renders generic pips only → Keldon
   Vandals' Echo {2}{R} — #181.
8. `filter.rs:550` — ColorFilter positive-only → Terror/Shriekmaw "nonblack" — #168.
9. `filter.rs:643` — `noncreature` single-bool exclusion → "nonartifact" needs the
   `exclude: TypeSet` generalization the comment names — #168.
10. `smothering_abomination.toml:5` — "Devoid is flavor" → Terror/Shriekmaw can target the
    colorless Abomination only if devoid really zeroes colors — #169.
11. `filter.rs:648` — colors derived from cost pips only → Wild Mongrel makes color runtime
    state — #196.
12. `effect.rs:3133` — no condition combinators → Massacre's "opponent controls a Plains AND
    you control a Swamp" — #192.
13. `combat.rs:660` + `combat.rs:668` — prevented combat damage tramples through; "no pool
    trampler meets a protected/self-shielding blocker" → Brawn grants all your creatures
    trample (vs pool Phantom Centaur / Flickering Ward hosts) — #186.
14. `spawn.rs:151` — exile-instead-of-graveyard rider skips Dies observability; "no pool card
    watches this death" → Hissing Iguanar watches every creature death — #187.
15. `apply.rs:1987` — a departing player's pending triggers/choices aren't purged; "no pool
    card lets a player die with those outstanding" → Phyrexian Arena's upkeep drain — #185.
16. `stack.rs:1364` — CREATURE_TYPES is the pool's own types → deck prints Kavu, Imp, Plant,
    Avatar, Treefolk, Mutant (and pool Krosan Tusker's Boar was already missing) — #180.
17. `effect.rs:3933` — "add an `at_most` sibling when a card appears" → Edge of Autumn is
    that card, but the landed negate composition already expresses "four or fewer"; no new
    arm needed (noted here so the ponytail can be re-pointed or the sibling added in #190's
    wave — planner's choice).

Borderline, checked and deliberately not flagged: sequential-kill batching vs Abyssal
Gatekeeper (CR-simultaneous, current model correct); `mana.rs:188`/`cast.rs:227` buyback
claims stay literally true (Constant Mists is new work, #178, not a falsified claim);
Genesis' fixed-cost optional-trigger pay path (verify targets work during authoring — Rubinia
precedent says reclassify C→D if TDD exposes a gap).

## Engine bugs found by the grind (beyond the planned increments)

- Anarchist shipped modeled on stale mandatory oracle text ("return" vs current "you may
  return") — caught by the wave-1 frame audit, which became a hard verify-gate requirement
  for every later wave.
- The new combat-damage-to-creature trigger panicked when a blocker's controller lost in the
  same SBA sweep as the damage (`owner_of` on a removed object) — found and regression-tested
  during #193.
- Vampiric Dragon's damaged-by death watch fired once per damage *instance* instead of once
  per death — caught by the wave-9 verify stage, deduped with a regression test.
- Mill was proven never-bugged for Dies triggers (its own `Event::Milled` path) and locked in
  with a regression test while fixing the real discard/entomb hole (#183).
- Cycle actions projected `sacrifice_choices: None`, so Edge of Autumn's sacrifice-cycling
  was unpayable from the client — fixed at the snapshot layer during Phase 5 client catch-up
  with a projection regression test.
