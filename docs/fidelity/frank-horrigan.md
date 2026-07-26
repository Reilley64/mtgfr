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

- [ ] Agent Frank Horrigan (#1) — built 2026-07-26; still approximated (proliferate can't choose players, #17)
- [x] Alpha Deathclaw (#12) — built 2026-07-26, faithful
- [x] Bilious Skulldweller (#20 slice 3) — built 2026-07-27, faithful
- [ ] Blightbelly Rat (#20 slice 3) — built 2026-07-27; still approximated (proliferate can't choose players, #17)
- [ ] Bloated Contaminator (#20 slice 3) — built 2026-07-27; still approximated (proliferate can't choose players, #17)
- [ ] Bloatfly Swarm (#21, #22)
- [x] Cankerbloom (#10) — built 2026-07-26; still approximated (proliferate can't choose players, #17)
- [ ] Contagion Engine (#3) — built 2026-07-26; still approximated (proliferate can't choose players, #17)
- [ ] Contaminant Grafter (#20 slices 3–4)
- [x] Everflowing Chalice (#11) — built 2026-07-26, faithful
- [ ] Feral Ghoul (#21)
- [ ] Garruk, Cursed Huntsman (#13)
- [x] Garruk, Primal Hunter (#4) — built 2026-07-26, faithful
- [ ] Glistening Sphere (#20 slice 4)
- [x] Ichor Rats (#20 slices 1–2) — built 2026-07-26, faithful
- [x] Infectious Bite (#7, #20 slice 1) — built 2026-07-27, faithful
- [x] Infectious Inquiry (#20 slice 1) — built 2026-07-26, faithful
- [ ] Innkeeper's Talent (#2, #17, #19) — built 2026-07-26; still approximated (L3 counter-doubling needs #19; L2 ward misses the Class itself because level is a scalar, not CR 717.2 counters)
- [x] Inspiring Call (#2) — built 2026-07-26, faithful
- [x] Lily Bowen, Raging Grandma (#14) — built 2026-07-27, faithful
- [x] Necrogen Communion (#20 slice 3) — built 2026-07-27, faithful
- [x] Overgrown Tomb (#6) — built 2026-07-26, faithful
- [x] Phyresis (#20 slice 2) — built 2026-07-26, faithful
- [ ] Phyresis Outbreak (#20 slice 5)
- [ ] Phyrexian Swarmlord (#20 slices 2, 4)
- [x] Plague Stinger (#20 slice 2) — built 2026-07-26, faithful
- [x] Power Fist (#15) — built 2026-07-27, faithful
- [x] Rampaging Yao Guai (#9) — built 2026-07-26, faithful
- [ ] Scheming Aspirant (#18)
- [x] Undergrowth Stadium (#5) — built 2026-07-26, faithful
- [ ] Venerated Rotpriest (#20 slices 1, 3)
- [ ] Vorinclex, Monstrous Raider (#19)
- [ ] Vraska, Betrayal's Sting (#8, #16, #25, #20 slices 1, 4)
- [x] Vraska's Fall (#20 slice 1) — built 2026-07-26, faithful
- [ ] Winding Constrictor (#19)

## Observability re-audit

The mandatory re-audit found **nine** pool-absence claims this deck falsifies. Each is folded
into the increment that clears it.

| Claim | Falsified by | Increment |
|---|---|---|
| `final_act.toml:13,22` — "each opponent loses all counters" dropped; "this pool tracks no player-level counters" | the whole infect/rad suite | #23 |
| `types/effect/shared.rs:1035` — proliferate "can't yet add a time counter to a suspended card" | 9 proliferate sources + 3 planeswalkers + Innkeeper's Talent | #17 (the note understates it: proliferate also omits **players**, **loyalty**, and **Class level** counters) |
| `types/effect/shared.rs:1070` — "grow this slot array when a future card needs another named kind" | poison/rad | #20 slice 1 — the prescribed remedy *cannot work*: the slot array lives on `Permanent`, and poison/rad live on players |
| `characteristics.rs:1811` — CR 616.1 ordering "documented rather than offered as a choice" because every pool replacement is the affected player's own adder | Vorinclex, Monstrous Raider | #19 — a *halving* owned by an opponent breaks all three premises at once |
| `characteristics.rs:1807` — `counters_after_replacements(object, base)` | Winding Constrictor, Vorinclex, Innkeeper's Talent L3 | #19 — the `ObjectId` signature gives "or on **you**" no call site, and the +1/+1-only scope silently skips every other kind |
| ~~`characteristics.rs:1100` — CR 704.5r ±1/±1 annihilation SBA "unobservable today (no pool card puts both kinds on one creature)"~~ **cleared 2026-07-26** | Contagion Clasp and Contagion Engine place real `-1/-1` counters onto a deck full of `+1/+1` counters | #24 (landed) |
| ~~`triggers.rs:2946` — "no pool Class gates one of those triggers"~~ **cleared 2026-07-26** | Innkeeper's Talent | #2 (landed) — the audit's "read at exactly **one** site" was itself wrong: `min_level` is read at four, and only `keyword_anthem_static_grants` was missing the gate |
| `promise_of_loyalty.toml:3` — "unobservable while every attack target is a player (planeswalker defenders unmodeled)" | the three planeswalkers | #13 — the parenthetical was already false about its own engine (`Defender::Planeswalker` exists); a note fix, not a code fix |
| ~~`types/mana.rs:223` — single-kicker only, "grow those from a real card that needs one"~~ **cleared 2026-07-26** | Everflowing Chalice | #11 (landed) |

`ozolith_the_shattered_spire.toml:10` is a near miss kept on watch: its over-broad
`counter_replacement` shape is still harmless *today* only because level counters route around
`counters_after_replacements` — it flips to a live bug the moment #19 widens that function past
+1/+1.

