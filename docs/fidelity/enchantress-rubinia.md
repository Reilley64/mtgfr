# Fidelity report — Enchantress Rubinia (Magic Online Theme Decks)

Source: https://archidekt.com/decks/2209180/enchantress_rubinia_magic_online_theme_decks
(Archidekt deck 2209180, fetched 2026-07-16). 72 unique non-basic cards + basics.
Commander: **Rubinia Soulsinger**. Backlog increments: #135–#166 (all landed 2026-07-16/17).

**Final state (2026-07-17): 72/72 cards in the pool, all fully faithful — 0 residuals.**
Intake counts were 8 faithful / 0 approximated / 29 expressible / 35 needing
engine work; three cards were reclassified C→D mid-grind when TDD exposed real engine gaps
(#164 cache invalidation, #165 ordered-trigger targets, #166 noncreature-host Aura legality).

## A. In pool, faithful at intake (8)

- [x] Azorius Signet
- [x] Elvish Visionary
- [x] Ghostly Prison
- [x] Kami of Ancient Law
- [x] Rampant Growth
- [x] Sakura-Tribe Elder
- [x] Swords to Plowshares
- [x] Terramorphic Expanse

## B. In pool, approximated at intake (0)

None.

## C. Authored in the pure-authoring pass (26)

- [x] Auramancer · Azorius Chancery · Bant Panorama · Borderland Ranger · Coastal Tower ·
  Dismantling Blow · Elfhame Palace · Jungle Barrier · Jungle Lion · Man-o'-War ·
  Merfolk Looter · Noble Templar · Oblivion Ring · Overwhelming Intellect · Raven Familiar ·
  Resurrection · Seal of Cleansing · Seaside Citadel · Selesnya Sanctuary · Selesnya Signet ·
  Shoreline Ranger · Simic Growth Chamber · Simic Signet · Wirewood Guardian · Wood Elves —
  all fully faithful
- [x] Looter il-Kor — fully faithful (2026-07-17): triggers on any damage dealt to an opponent
  via `timing = "deals_damage_to_opponent"`, closing its intake residual

## D. Landed via engine increments (38)

- [x] Armadillo Cloak (#151) · Azorius Guildmage (#146) · Capsize (#149) · Coiling Oracle (#141)
  · Compulsive Research (#144) · Concordant Crossroads (#138, World rule ponytail) · Condemn
  (#135+#136) · Confiscate (#166) · Copy Enchantment (#158) · Court Hussar (#157) · Decree of
  Justice (#145) · Empyrial Armor (#137) · Enlightened Tutor (#140) · Fact or Fiction (#161) ·
  Faith's Fetters (#155) · Fertile Ground (#152) · Hinder (#160) · Krosan Tusker (#145) ·
  Mirari's Wake (#152) · Miren, the Moaning Well (#136) · Mistmeadow Witch (#147) · Moment's
  Peace (#150) · Momentary Blink (#147) · Mulldrifter (#148) · Phantom Centaur (#159) · Prison
  Term (#155+#156) · Questing Phelddagrif (#154) · Relic of Progenitus (#142) · Rhystic Study
  (#153) · Rubinia Soulsinger (#162) · Rupture Spire (#143) · Sterling Grove (#139+#140) ·
  Stonecloaker (#165) · Temporal Spring (#135) · Treva's Ruins (#143) · Willbender (#163) ·
  Yavimaya Enchantress (#164) — all fully faithful
- [x] Illusionary Mask (#163 slices 1–3) — fully faithful (2026-07-17): the printed "mana you
  spent on {X} could pay its cost" test is modeled exactly (`Cost::payable_from_multiset` over
  the activation payment's spent-mana multiset, CR 107.3), closing the last residual; the CR 615
  flip-on-interaction replacement was already modeled

## Observability re-audit (falsified pool-absence claims — all closed)

1. "no pool Aura re-attaches" → Prison Term (#156). 2. "no morph card is in the pool" →
Willbender/Illusionary Mask (#163). 3. cycling/hand activations resolved off-stack → Azorius
Guildmage + five cycling cards (#146). 4. "no pool card counters a flashback/escape spell" →
Hinder (#160, countered-flashback exile fork). 5. `OpponentSplitsExilePiles` hardcoded
next-in-APNAP → Fact or Fiction's controller-chosen opponent (#161, Abstract Performance fixed
too). 6. Chaos Warp's fused tuck → standalone `tuck_permanent_into_library` (#135).

## Engine bugs found by the grind's TDD (beyond the planned increments)

- Cross-owner count anthems went stale (owner-scoped cache invalidation) — #164.
- Ordered simultaneous targeted triggers silently fizzled (`target: None`) — #165.
- The CR 704.5m Aura-legality SBA swept enchant-permanent Auras off noncreature hosts — #166.
- Treva's Ruins-style flat mana arrays produced ALL listed colors instead of a choice
  (`of_colors` was never emitted by `AddMana`) — caught by the wave-5 verify stage.
- A blocking Phantom Centaur missed its prevention shield (fifth damage choke) — caught by the
  wave-10 verify stage.
- A self-sacrificing mana source (Treasure) panicked the tapped-for-mana watch — fixed in #152.
