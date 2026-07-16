# Fidelity report — Enchantress Rubinia (Magic Online Theme Decks)

Source: https://archidekt.com/decks/2209180/enchantress_rubinia_magic_online_theme_decks
(Archidekt deck 2209180, fetched 2026-07-16). 72 unique non-basic cards + basics.
Commander: **Rubinia Soulsinger**. Backlog increments for section D: #135–#163.

## A. In pool, faithful (8)

- [x] Azorius Signet
- [x] Elvish Visionary
- [x] Ghostly Prison
- [x] Kami of Ancient Law
- [x] Rampant Growth
- [x] Sakura-Tribe Elder
- [x] Swords to Plowshares
- [x] Terramorphic Expanse

## B. In pool, approximated (0)

None — every other deck card is new to the pool.

## C. New, expressible today (27)

- [ ] Auramancer — ETB optional `return_from_graveyard_to_hand`, `{ card_in_graveyard = { whose = "yours", filter = "enchantment" } }`
- [ ] Azorius Chancery — `enters_tapped`, ETB targeted self-land bounce, `{T}: Add {W}{U}` via `add_mana`
- [ ] Bant Panorama — `{T}: Add {C}` + sac-fetch `{ basic_land_with_subtype = ["Forest", "Plains", "Island"] }`
- [ ] Borderland Ranger — ETB optional `search_library` basic land to hand
- [ ] Coastal Tower — tapped dual
- [ ] Confiscate — `enchant = {}` (any permanent) + `control_attached`
- [ ] Dismantling Blow — `[cost.additional.kicker]` + `destroy_target` + draw `{ if_kicked = 2, else = 0 }`
- [ ] Elfhame Palace — tapped dual
- [ ] Jungle Barrier — defender, ETB draw
- [ ] Jungle Lion — `cant_block`
- [ ] Looter il-Kor — shadow; damage trigger approximated (see note in TOML)
- [ ] Man-o'-War — ETB bounce target creature
- [ ] Merfolk Looter — `{T}`: draw then discard
- [ ] Noble Templar — vigilance + Plainscycling via `hand_ability`
- [ ] Oblivion Ring — ETB `exile_until_source_leaves`, `{ types = "nonland", other = true }`
- [ ] Overwhelming Intellect — `counter_target_spell` filter `"creature"` + draw `"target_mana_value"`
- [ ] Raven Familiar — `[echo]` + ETB `look_at_top` 3/keep 1/rest bottom
- [ ] Resurrection — `reanimate_to_battlefield` from your graveyard
- [ ] Seal of Cleansing — sac-self: destroy artifact or enchantment
- [ ] Seaside Citadel — tapped; three `taps_self` `add_mana` abilities (G / W / U)
- [ ] Selesnya Sanctuary — karoo (as Azorius Chancery)
- [ ] Selesnya Signet — `{1}, {T}: Add {G}{W}`
- [ ] Shoreline Ranger — flying + Islandcycling via `hand_ability`
- [ ] Simic Growth Chamber — karoo
- [ ] Simic Signet — `{1}, {T}: Add {G}{U}`
- [ ] Wirewood Guardian — Forestcycling via `hand_ability`
- [ ] Wood Elves — ETB `search_library` `{ land_with_subtype = ["Forest"] }` to battlefield untapped

## D. New, needs engine work (37)

- [ ] Armadillo Cloak — #151 enchanted-deals-damage lifegain
- [ ] Azorius Guildmage — #146 counter target activated ability (cycling/hand activations on the stack)
- [ ] Capsize — #149 buyback
- [ ] Coiling Oracle — #141 reveal-top route (land→battlefield, else→hand)
- [ ] Compulsive Research — #144 discard-unless-land
- [ ] Concordant Crossroads — #138 global anthem scope
- [ ] Condemn — #135 library-tuck primitive + #136 toughness amounts
- [ ] Copy Enchantment — #158 enter-as-copy enchantment
- [ ] Court Hussar — #157 mana-spent tracking
- [ ] Decree of Justice — #145 cycling triggers
- [ ] Empyrial Armor — #137 cards-in-hand amount
- [ ] Enlightened Tutor — #140 library-top search
- [ ] Fact or Fiction — #161 pile split
- [ ] Faith's Fetters — #155 attached can't-attack/block
- [ ] Fertile Ground — #152 tapped-for-mana triggers
- [ ] Hinder — #160 counter-with-tuck
- [ ] Illusionary Mask — #163 morph/face-down (slice 3)
- [ ] Krosan Tusker — #145 cycling triggers
- [ ] Mirari's Wake — #152 tapped-for-mana triggers
- [ ] Miren, the Moaning Well — #136 toughness amounts
- [ ] Mistmeadow Witch — #147 flicker (delayed slice)
- [ ] Moment's Peace — #150 fog
- [ ] Momentary Blink — #147 flicker (immediate slice)
- [ ] Mulldrifter — #148 evoke
- [ ] Phantom Centaur — #159 phantom prevention
- [ ] Prison Term — #155 attached can't-attack/block + #156 aura re-attach
- [ ] Questing Phelddagrif — #154 opponent-gift riders
- [ ] Relic of Progenitus — #142 target-player graveyard exile
- [ ] Rhystic Study — #153 rhystic punisher
- [ ] Rubinia Soulsinger — #162 conditioned control duration (commander)
- [ ] Rupture Spire — #143 ETB sacrifice-unless
- [ ] Sterling Grove — #139 noncreature keyword anthem + #140 library-top search
- [ ] Temporal Spring — #135 library-tuck primitive
- [ ] Treva's Ruins — #143 ETB sacrifice-unless
- [ ] Willbender — #163 morph/face-down (slices 1–2)
- [ ] Yavimaya Enchantress — #164 cross-owner anthem cache invalidation (reclassified from C in
  wave B: the TOML is expressible, but the characteristics cache invalidates owner-scoped, so an
  any-controller `per_permanent` count anthem goes stale when an opponent's enchantment
  enters/leaves)
- [ ] Stonecloaker — #165 ordered-trigger targets (reclassified from C in wave C: its two
  simultaneous targeted ETB triggers both fizzle — `choose_order` places ordered triggers with no
  target)

## Observability re-audit (falsified pool-absence claims)

Claims justified by pool absence that this deck falsifies — folded into the increments above:

1. `types/card.rs` "no pool Aura re-attaches" — **Prison Term** moves while attached (→ #156).
2. `types/card.rs` "no morph card is in the pool" — **Willbender**, **Illusionary Mask** (→ #163).
3. `cast.rs` cycling/hand activations resolved off-stack ("no pool card responds to a cycling
   activation") — **Azorius Guildmage** counters activated abilities; the deck ships five cycling
   cards (→ #146).
4. `effects.rs` "no pool card counters a flashback/escape spell" (countered flashback must exile,
   CR 702.34e) — **Hinder** + **Moment's Peace**/**Momentary Blink** (→ #160).
5. `types/effect.rs` `OpponentSplitsExilePiles` hardcodes next-in-APNAP opponent — **Fact or
   Fiction**'s controller chooses the opponent in multiplayer (→ #161).
6. `types/effect.rs` Chaos Warp's fused tuck ("split it only when a second card wants just the
   tuck half") — **Condemn**, **Temporal Spring**, **Hinder** (→ #135).

Near-misses that survive (no action): control-override timestamps (Rubinia + Confiscate pairing
still unreachable as until-EOT + aura), `discard_land` cost vs Compulsive Research's *effect*-time
choice, own-black-spell-vs-pro-black targeting (no black spells in this deck),
reanimation/blink counter-on-entry choke (no enters-with-counters static ships in this deck).
