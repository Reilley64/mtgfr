# Fidelity report — Heavenly Inferno (Commander 2011, Kaalia)

Source: https://archidekt.com/decks/2209172/heavenly_inferno_commander_2011
Deck: Mardu (WBR), commander **Kaalia of the Vast**. 79 unique names; 4 basic land types
dropped → **75 non-basic cards** to make faithful.

Legend: A = in pool, faithful · B = in pool, approximated · C = new, expressible today ·
D = new, needs engine work (increments landed; backlog file removed).

## A. In pool, faithful (22)

- [x] Anger
- [x] Armillary Sphere
- [x] Barren Moor
- [x] Bojuka Bog
- [x] Boros Garrison
- [x] Command Tower
- [x] Darksteel Ingot
- [x] Death by Dragons
- [x] Diabolic Tutor
- [x] Evolving Wilds
- [x] Forgotten Cave
- [x] Lightning Greaves
- [x] Path to Exile
- [x] Rakdos Carnarium
- [x] Rakdos Signet
- [x] Rupture Spire
- [x] Serra Angel
- [x] Sol Ring
- [x] Soul Snare
- [x] Terminate
- [x] Vow of Duty
- [x] Vow of Lightning

## B. In pool, approximated (0)

None. Soul Snare and the two Vows carried an `"or a planeswalker you control"` residual on the
premise that no planeswalker could be attacked. That premise was wrong — Quintorius, History
Chaser is a planeswalker permanent in the pool, and the real gap was the combat model: every
attack target was a `PlayerId`, so a creature could never be declared as attacking a planeswalker.
This grind built **planeswalker-as-attack-defender** into the engine (`AttackTarget`, CR 508.1a
declaration legality, CR 306.9 loyalty-removal on combat damage, trample/lifelink over a walker),
after which all three clauses are enforced via the defending-player mapping and became faithful.

## C. New, expressible today (27)

Lands & rocks:
- [x] Akoum Refuge — enters tapped, ETB gain 1 life, `{T}: B or R` (produces dual).
- [x] Orzhov Basilica — Karoo (copy Boros Garrison / Rakdos Carnarium pattern), `{T}: W B`.
- [x] Secluded Steppe — enters tapped, `{T}: W`, cycling `{W}` (copy Forgotten Cave).
- [x] Boros Signet — `{1},{T}: R W`.
- [x] Orzhov Signet — `{1},{T}: W B`.

Creatures & spells:
- [x] Akroma's Vengeance — `destroy_all` (artifact/creature/enchantment union) + `cycling {3}`.
- [x] Akroma, Angel of Fury — `uncounterable`, flying/trample + two `protection` keywords
      (white, blue), `{R}: +1/0`, morph (all supported; cf. questing_phelddagrif, altered_ego).
- [x] Angel of Despair — flying, ETB `destroy_target` permanent.
- [x] Boros Guildmage — `{1}{R}:` haste EOT / `{1}{W}:` first strike EOT (pump w/ keywords).
- [x] Duergar Hedge-Mage — ETB `controls_lands_with_subtype` (2+ Mountains → may destroy artifact;
      2+ Plains → may destroy enchantment).
- [x] Earthquake — `{X}{R}` `damage_each_creature` (without_flying) + `damage_each_player`, amount `x`.
- [x] Evincar's Justice — buyback `{3}`, 2 damage to each creature and each player.
- [x] Fallen Angel — flying, `Sacrifice a creature:` +2/+1 EOT.
- [x] Furnace Whelp — flying, `{R}: +1/0`.
- [x] Gwyllion Hedge-Mage — ETB 2+ Plains → may mint a Kithkin; 2+ Swamps → may -1/-1 counter
      (needed the D-23 cache-invalidation fix; landed in the same Phase-4 wave).
- [x] Mortify — `destroy_target` creature-or-enchantment.
- [x] Oni of Wild Places — haste, upkeep `return_to_hand` a red creature you control.
- [x] Oros, the Avenger — flying, combat-damage-to-player trigger, may pay `{2}{W}` →
      3 damage to each nonwhite creature *(landed with D-6's color filter on `damage_each_creature`)*.
- [x] Orzhov Guildmage — `{2}{W}:` target player +1 life / `{2}{B}:` each player loses 1.
- [x] Shattered Angel — flying, opponent-land-ETB → may gain 3 life.
- [x] Wrecking Ball — `destroy_target` creature-or-land.

*(Oros is listed C but rides D-6's color filter; authored in the wave that lands it. Righteous
Cause moved to D-20 — its "whenever a creature attacks" watch has no trigger timing today.
Bladewing the Risen (D-21) and Pyrohemia (D-22) moved to D during Phase 3 authoring — each needs
engine work a pure script can't reach. Gwyllion Hedge-Mage was also flagged (D-23), but that was a
one-line P/T-cache bug, fixed in the same Phase-4 wave, so it stays faithful in C.)*

## D. New, needs engine work (27) → increments landed; backlog file removed

Grouped by the increment that unblocks them. **Every increment that gates a card has landed** —
every card below is in the pool, faithful except **Stranglehold**, which carries an `approximates`
naming exactly what it drops. The live drive then filed four more increments that gate no card —
#24–#27, each an advertisement the gate could only reject — and the post-drive observability pass
filed #28–#29, which retired Archangel of Strife's `approximates` (and the same as-enters shortcut
in Flickering Ward, Patchwork Banner, and Voice of All, which had never declared it). All landed
here.

- **D-1 Kaalia put-from-hand tapped & attacking (subtype-gated):** ~~Kaalia of the Vast~~
  *(commander)* — done (attack trigger puts an Angel/Demon/Dragon from hand tapped and attacking
  the same defender).
- **D-2 `cast_from_hand` ETB condition + controller/color mass filters:** ~~Dread Cacodemon~~,
  ~~Reiver Demon~~ — done (`cast_from_hand` intervening-if on the ETB trigger + the D-6 filters).
- **D-3 Resolution-scoped count formulas ("for each … this way"):** ~~Congregate~~,
  ~~Syphon Mind~~, ~~Syphon Flesh~~, ~~Malfegor~~ — done (a resolution-frame count each step writes
  and a later step reads as an `Amount`).
- **D-4 Protection from a chosen color (choose-as-enters / until-EOT):** ~~Voice of All~~,
  ~~Mother of Runes~~, ~~Bathe in Light~~ — done (chosen color stored per object, read by the
  protection checks).
- **D-5 Radiance targeting (shares-a-color batch):** ~~Bathe in Light~~, ~~Cleansing Beam~~ — done
  (one `Game::radiance_batch` helper expanding at resolution: the target plus every other
  battlefield creature sharing a color with it; `TargetSpec` untouched).
- **D-6 Color-filtered mass damage/destroy (nonwhite / nonartifact-nonblack):** ~~Oros the
  Avenger~~, ~~Reiver Demon~~ — done (color/controller predicates on the mass-effect target set).
- **D-7 Multikicker:** ~~Comet Storm~~, ~~Lightkeeper of Emeria~~ — done (a repeatable kicker count
  paid at cast, read as an `Amount`; wire field `multikicker_count`).
- **D-8 Kicked/main-phase conditional extra target & amount:** ~~Orim's Thunder~~,
  ~~Return to Dust~~, ~~Sulfurous Blast~~ — done (a was-kicked / is-main-phase condition gating an
  extra target and a larger amount).
- **D-9 Vivid land (any-color via charge-counter removal) & storage land (X-counter cost):**
  ~~Vivid Meadow~~, ~~Molten Slagheap~~ — done with **zero engine changes** (charge-counter removal
  as an activation cost and the storage land's X-counters-for-X-mana shape already existed).
- **D-10 Activation-count sacrifice trigger:** ~~Dragon Whelp~~ — done (per-turn activation count
  watched by an end-step sacrifice trigger).
- **D-11 Global combat-static anthems:** ~~Avatar of Slaughter~~ (all creatures double strike +
  must attack), ~~Razorjaw Oni~~ (black creatures can't block), ~~Basandra~~ (no spells in combat)
  — done (table-wide statics reaching every player, not just the controller).
- **D-12 Per-player war/peace choice anthem:** ~~Archangel of Strife~~ — done and faithful (#28
  keys each answer to the asking Archangel, #29 makes the choice as it enters, CR 614.12).
- **D-13 Conditional per-opponent attack/cast lockout:** ~~Angelic Arbiter~~ — done (per-opponent
  attacked/cast-this-turn flags gating the other action).
- **D-14 Choose-others' attackers/blockers:** ~~Master Warcraft~~ — done (turn-scoped attack/block
  declarer overrides + a `cast_only_before_attackers` timing window).
- **D-15 Search-denial + extra-turn skip static:** ~~Stranglehold~~ — done, **approximated** (the
  search-denial half is faithful; the extra-turn skip is dropped — nothing in the pool grants an
  extra turn and the engine has no extra-turn machinery).
- **D-16 Random reanimate from an opponent's graveyard:** ~~Tariel, Reckoner of Souls~~ — done
  (injected-RNG pick from an opponent's graveyard, onto the battlefield under your control).
- **D-17 Join forces:** ~~Mana-Charged Dragon~~ — done (the existing join-forces payment fan-out
  reused from a new `attacks_or_blocks` trigger).
- **D-18 `intimidate` keyword:** ~~Vow of Malice~~ — done (evasion checked in `can_block`).
- **D-19 Land with morph:** ~~Zoetic Cavern~~ — done (face-down `kind` override so a hidden land
  resolves as a 2/2, morph's turn-face-up gate split from manifest's, face-down mana-tap guards).
- **D-20 "Whenever a creature attacks" watch-others trigger (any attacker, any defender):**
  ~~Righteous Cause~~ — done (a per-attacker trigger that isn't scoped to its own controller).
- **D-21 ETB reanimate-a-Dragon + all-controllers subtype pump until EOT:** ~~Bladewing the Risen~~
  — done (ETB reanimate of a target Dragon permanent card from your graveyard, plus a new
  board-wide `pump_each_creature_until_end_of_turn` that drops the you-control gate).
- **D-22 End-step no-creatures self-sac + any-player "beginning of the end step" timing:**
  ~~Pyrohemia~~ — done (the every-player end-step trigger already existed; the new piece is a
  `no_creatures_on_battlefield` condition checked both at placement and on resolution).
- **D-23 `put_counters` -1/-1 misses the P/T cache invalidation:** ~~Gwyllion Hedge-Mage~~ — done
  (added `KindCountersPlaced` to `invalidate_characteristics_cache`; Gwyllion is faithful in C).

## Observability re-audit

Grepped `approximates` / `ponytail:` across `crates/cards/data/` and `crates/engine/src/` for
pool-absence claims the incoming cards falsify. The three former section-B residuals cited a
"no planeswalker in pool" premise that was already false — Quintorius, History Chaser is a
planeswalker permanent — and their true blocker was the combat model (attack targets were always
a `PlayerId`). Building planeswalker-as-attack-defender falsified and retired all three; they are
now faithful in section A. No other residual is falsified by this deck (no card here is the first
of a previously-absent permanent type).

A second pass over this deck's own `approximates` found the reverse problem: Archangel of Strife
declared its "as this creature enters" shortcut while Flickering Ward, Patchwork Banner, and Voice
of All shared it in silence. Increments #28–#29 retired the shortcut for all four rather than
adding three more `approximates` — "as ~ enters" is now a real CR 614.12 replacement effect
(`timing = "as_enters"`), and each Archangel keys its own war/peace answers.
