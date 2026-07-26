# Fidelity report — Frank Horrigan

Source: https://archidekt.com/decks/24351870/frank_horrigan (Archidekt deck 24351870, fetched
2026-07-26). 79 unique non-basic cards + 2 basics (100 cards total).
Commander: **Agent Frank Horrigan**. Backlog: [frank-horrigan-increments.md](frank-horrigan-increments.md).

A Golgari infect/toxic/poison deck built around proliferate and +1/+1 counters. Its core
mechanic — **poison counters on players** — does not exist anywhere in the engine at intake
(`grep -ri 'poison\|infect\|toxic\|rad'` over `crates/engine/src/` returns nothing but Toxic
Deluge's name), so this grind's centre of gravity is a new player-counter subsystem rather than
a spread of small card-shaped gaps.

Intake counts: 28 in pool (all faithful, 0 approximated) / 53 new.

## A. In pool, faithful at intake (28)

Every deck card already in the pool is fully faithful — none carries an `approximates` note.

- [x] Arcane Signet
- [x] Assassin's Trophy
- [x] Beast Within
- [x] Bojuka Bog
- [x] Command Tower
- [x] Cultivate
- [x] Forest (basic)
- [x] Forgotten Ancient
- [x] Golgari Rot Farm
- [x] Golgari Signet
- [x] Hardened Scales
- [x] Kami of Whispered Hopes
- [x] Kodama's Reach
- [x] Lightning Greaves
- [x] Llanowar Elves
- [x] Llanowar Wastes
- [x] Nature's Lore
- [x] Necroblossom Snarl
- [x] Phyrexian Arena
- [x] Putrefy
- [x] Rampant Growth
- [x] Rogue's Passage
- [x] Sol Ring
- [x] Swamp (basic)
- [x] Temple of Malady
- [x] Three Visits
- [x] Viridescent Bog
- [x] Woodland Cemetery

## B. In pool, approximated at intake (0)

None.

## C. New, expressible today (18)

Authored in the pure-authoring pass (2026-07-26), no engine change needed. All 18 carry engine
tests and pass the frame audit against a fresh Scryfall fetch.

- [x] Atomize
- [x] Birds of Paradise
- [x] Branching Evolution
- [x] Cathedral Acolyte
- [x] Contagion Clasp
- [x] Corpsejack Menace
- [x] Dark Ritual
- [x] Deathcap Glade
- [x] Drown in Ichor
- [x] Evolution Sage
- [x] Farseek
- [x] Heroic Intervention
- [x] Karn's Bastion
- [x] Swiftfoot Boots
- [x] Tainted Wood
- [x] Talisman of Resilience
- [x] Thirsting Roots
- [x] Unnatural Restoration

Cathedral Acolyte was expected to need increment #2's counter filter axis, but
`StaticEffect::Anthem` carries its own `has_counters` field, so it authored cleanly. #2 is still
needed for Inspiring Call, whose *count* of matching creatures drives a draw.

The pass also surfaced one engine bug, fixed here with a regression test:
`invalidate_characteristics_cache` handled `Event::CountersPlaced` but not its kind-keyed sibling
`Event::KindCountersPlaced`, so a -1/-1 counter left a stale cached toughness (power read
correctly, toughness did not) whenever the SBA sweep had already warmed the cache. This blocks
increments #3, #20 slice 2, and #24, all of which mint -1/-1 counters.

## D. New, needs engine work (35)

Ranked increments live in [frank-horrigan-increments.md](frank-horrigan-increments.md); the
increment that unblocks each card is in parentheses.

- [x] Agent Frank Horrigan (#1) — built 2026-07-26, faithful (#17 cleared the proliferate residual 2026-07-27)
- [x] Alpha Deathclaw (#12) — built 2026-07-26, faithful
- [x] Bilious Skulldweller (#20 slice 3) — built 2026-07-27, faithful
- [x] Blightbelly Rat (#20 slice 3) — built 2026-07-27, faithful (#17 cleared the proliferate residual same day)
- [x] Bloated Contaminator (#20 slice 3) — built 2026-07-27, faithful (#17 cleared the proliferate residual same day)
- [x] Bloatfly Swarm (#21, #22) — built 2026-07-27, faithful
- [x] Cankerbloom (#10) — built 2026-07-26, faithful (#17 cleared the proliferate residual 2026-07-27)
- [x] Contagion Engine (#3) — built 2026-07-26, faithful (#17 cleared the proliferate residual 2026-07-27)
- [x] Contaminant Grafter (#20 slices 3–4) — built 2026-07-27, faithful
- [x] Everflowing Chalice (#11) — built 2026-07-26, faithful
- [x] Feral Ghoul (#21) — built 2026-07-27, faithful
- [x] Garruk, Cursed Huntsman (#13a, #13b) — built 2026-07-27, faithful (#13b landed emblems and the −6 the same day)
- [x] Garruk, Primal Hunter (#4) — built 2026-07-26, faithful
- [x] Glistening Sphere (#20 slice 4) — built 2026-07-27, faithful
- [x] Ichor Rats (#20 slices 1–2) — built 2026-07-26, faithful
- [x] Infectious Bite (#7, #20 slice 1) — built 2026-07-27, faithful
- [x] Infectious Inquiry (#20 slice 1) — built 2026-07-26, faithful
- [ ] Innkeeper's Talent (#2, #17, #19) — built 2026-07-26; still approximated (L3's replacement models as of #19, but keys off the recipient's side rather than the placing player; L2 ward misses the Class itself because level is a scalar, not CR 717.2 counters)
- [x] Inspiring Call (#2) — built 2026-07-26, faithful
- [x] Lily Bowen, Raging Grandma (#14) — built 2026-07-27, faithful
- [x] Necrogen Communion (#20 slice 3) — built 2026-07-27, faithful
- [x] Overgrown Tomb (#6) — built 2026-07-26, faithful
- [x] Phyresis (#20 slice 2) — built 2026-07-26, faithful
- [x] Phyresis Outbreak (#20 slice 5) — built 2026-07-27, faithful
- [x] Phyrexian Swarmlord (#20 slices 2, 4) — built 2026-07-27, faithful
- [x] Plague Stinger (#20 slice 2) — built 2026-07-26, faithful
- [x] Power Fist (#15) — built 2026-07-27, faithful
- [x] Rampaging Yao Guai (#9) — built 2026-07-26, faithful
- [x] Scheming Aspirant (#18) — built 2026-07-27, faithful
- [x] Undergrowth Stadium (#5) — built 2026-07-26, faithful
- [x] Venerated Rotpriest (#20 slices 1, 3) — built 2026-07-27, faithful
- [ ] Vorinclex, Monstrous Raider (#19) — built 2026-07-27; still approximated (both clauses key off the recipient's side, not the placing player, so counters you put on an opponent's permanent take the wrong clause; CR 616.1 ordering unoffered)
- [ ] Vraska, Betrayal's Sting (#8, #16, #25, #20 slices 1, 4) — built 2026-07-27; still approximated ({B/P} is a real pip as of #8, but which way it was paid isn't persisted, so Compleated never applies (#16); −2 Treasure mode omitted (#25))
- [x] Vraska's Fall (#20 slice 1) — built 2026-07-26, faithful
- [x] Winding Constrictor (#19) — built 2026-07-27, faithful

## Observability re-audit

The mandatory re-audit found **nine** pool-absence claims this deck falsifies. Each is folded
into the increment that clears it. Seven are cleared as of 2026-07-27; two remain (both on #19).

| Claim | Falsified by | Increment |
|---|---|---|
| ~~`final_act.toml:13,22` — "each opponent loses all counters" dropped; "this pool tracks no player-level counters"~~ **cleared 2026-07-27** | the whole infect/rad suite | #23 (landed) — the fifth mode is restored; only "destroy all battles" remains dropped |
| ~~`types/effect/shared.rs:1035` — proliferate "can't yet add a time counter to a suspended card"~~ **cleared 2026-07-27** | 9 proliferate sources + 3 planeswalkers + Innkeeper's Talent | #17 (landed) — the note understated it: proliferate also omitted **players**, **loyalty**, and **Class level**. Players and loyalty now ship; the exile-store kinds (time/suspend) remain the residual |
| ~~`types/effect/shared.rs:1070` — "grow this slot array when a future card needs another named kind"~~ **cleared 2026-07-27** | poison/rad | #20 slices 1 + #21 (landed) — the prescribed remedy could not work (the slot array lives on `Permanent`, poison/rad live on players); a separate `PlayerCounterKind` shipped instead |
| `characteristics.rs:1811` — CR 616.1 ordering "documented rather than offered as a choice" because every pool replacement is the affected player's own adder | Vorinclex, Monstrous Raider | #19 (landed) — a *halving* owned by an opponent breaks all three premises at once; the note is rewritten on `Game::replaced_counters` and the ordering is **still** unoffered |
| ~~`characteristics.rs:1807` — `counters_after_replacements(object, base)`~~ **cleared 2026-07-27** | Winding Constrictor, Vorinclex, Innkeeper's Talent L3 | #19 (landed) — `Game::replaced_counters` now keys on a `CounterRecipient` (permanent or player) and an any-kind axis; the residual is the *placing* player, not the recipient |
| ~~`characteristics.rs:1100` — CR 704.5r ±1/±1 annihilation SBA "unobservable today (no pool card puts both kinds on one creature)"~~ **cleared 2026-07-26** | Contagion Clasp and Contagion Engine place real `-1/-1` counters onto a deck full of `+1/+1` counters | #24 (landed) |
| ~~`triggers.rs:2946` — "no pool Class gates one of those triggers"~~ **cleared 2026-07-26** | Innkeeper's Talent | #2 (landed) — the audit's "read at exactly **one** site" was itself wrong: `min_level` is read at four, and only `keyword_anthem_static_grants` was missing the gate |
| ~~`promise_of_loyalty.toml:3` — "unobservable while every attack target is a player (planeswalker defenders unmodeled)"~~ **cleared 2026-07-27** | the three planeswalkers | #13a (landed) — the parenthetical was already false about its own engine; attack declaration resolves every attack to its defending player whether the declared target was that player or their planeswalker, so the card was always faithful. A note fix, not a code fix. Two `DSL_REFERENCE.md` rows (`counter_scaled_attack_tax`, `cant_be_attacked_by`) carried the same stale claim and were corrected with it |
| ~~`types/mana.rs:223` — single-kicker only, "grow those from a real card that needs one"~~ **cleared 2026-07-26** | Everflowing Chalice | #11 (landed) |

`ozolith_the_shattered_spire.toml:10` is a near miss kept on watch: its over-broad
`counter_replacement` shape is still harmless *today* only because level counters route around
`counters_after_replacements` — it flips to a live bug the moment #19 widens that function past
+1/+1.

