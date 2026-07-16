# Quandrix Unlimited — Secrets of Strixhaven (soc, 2026)

Frozen target list for the soc precon pool.

**Commander:** Zimone, Infinite Analyst (Simic / GU legendary creature, soc #10)

Sources:
- Official Wizards decklists: https://magic.wizards.com/en/news/announcements/secrets-of-strixhaven-commander-decklists
- Card Game Base (per-deck): https://cardgamebase.com/quandrix-unlimited-precon-decklist-spoilers/
- Scryfall set `soc`: https://scryfall.com/sets/soc

## Commander (1)
- 1 x Zimone, Infinite Analyst

## Creatures (27)
- 1 x Altered Ego
- 1 x Benevolent Hydra
- 1 x Deekah, Fractal Theorist
- 1 x Elusive Otter
- 1 x Forgotten Ancient
- 1 x Goldvein Hydra
- 1 x Guardian Augmenter
- 1 x Hangarback Walker
- 1 x Hydroid Krasis
- 1 x Ingenious Prodigy
- 1 x Kami of Whispered Hopes
- 1 x Kinetic Ooze
- 1 x Lifeblood Hydra
- 1 x Nev, the Practical Dean
- 1 x Owlin Spiralmancer
- 1 x Primo, the Unbounded
- 1 x Primordial Hydra
- 1 x Quandrix Apprentice
- 1 x Steelbane Hydra
- 1 x Stonecoil Serpent
- 1 x Striding Shotcaller
- 1 x Tanazir Quandrix
- 1 x The Goose Mother
- 1 x Troyan, Gutsy Explorer
- 1 x Yavimaya Bloomsage
- 1 x Zimone, All-Questioning
- 1 x Zimone, Quandrix Prodigy

## Instants / Sorceries (24)
Instants (15):
- 1 x Beast Within
- 1 x Biomass Mutation
- 1 x Commander's Insight
- 1 x Decisive Denial
- 1 x Eureka Moment
- 1 x Nexus Mentality
- 1 x Perplexing Test
- 1 x Pull from Tomorrow
- 1 x Quandrix Charm
- 1 x Quandrix Command
- 1 x Rapid Hybridization
- 1 x Silkguard
- 1 x Stroke of Genius
- 1 x Tyvar's Stand
- 1 x Zimone's Hypothesis

Sorceries (9):
- 1 x Animist's Awakening
- 1 x Curse of the Swine
- 1 x Entrancing Melody
- 1 x Expansion Algorithm
- 1 x Nature's Lore
- 1 x Open the Way
- 1 x Oversimplify
- 1 x Primal Might
- 1 x Three Visits

## Artifacts (7)
- 1 x Arcane Signet
- 1 x Astral Cornucopia
- 1 x Brass Infiniscope
- 1 x Elementalist's Palette
- 1 x Fractal Harness
- 1 x Ozolith, the Shattered Spire
- 1 x Sol Ring

## Enchantments (4)
- 1 x Hardened Scales
- 1 x Lattice Library
- 1 x Mana Bloom
- 1 x Unbound Flourishing

## Planeswalkers (0)
- (none)

## Lands (37)
- 1 x Alchemist's Refuge
- 1 x Command Tower
- 1 x Exotic Orchard
- 1 x Fabled Passage
- 1 x Flooded Grove
- 1 x Hinterland Harbor
- 1 x Opal Palace
- 1 x Oran-Rief, the Vastwood
- 1 x Overflowing Basin
- 1 x Paradox Gardens
- 1 x Path of Ancestry
- 1 x Quandrix Campus
- 1 x Rain-Slicked Copse
- 1 x Reliquary Tower
- 1 x Rogue's Passage
- 1 x Sodden Verdure
- 1 x Study Hall
- 1 x Tangled Islet
- 1 x Temple of Mystery
- 1 x Temple of the False God
- 1 x Terramorphic Expanse
- 1 x Turbulent Wilderness
- 1 x Vineglimmer Snarl
- 1 x Yavimaya Coast
- 6 x Forest
- 7 x Island

## Totals
Commander 1 + Creatures 27 + Instants/Sorceries 24 + Artifacts 7 + Enchantments 4 + Planeswalkers 0 + Lands 37 = **100**.
Distinct card names: 89 (87 unique singles + Forest + Island).

## Implementation risks
- **X-spell doubling / copying (hardest):** Unbound Flourishing (doubles X on the stack AND copies noncreature spells with X / triggers on cast), Zimone Infinite Analyst (first X-spell each turn costs {1} less per counter, then grows). X value modification and spell-copy interaction is the deck's core and the trickiest engine work.
- **+1/+1 counter replacement stacking:** Hardened Scales, Ozolith the Shattered Spire, Kami of Whispered Hopes, Forgotten Ancient — counter-adding replacement effects that must compound correctly (multiple replacement effects, player-chosen order).
- **X creatures / clones entering with counters:** Hydroid Krasis, Primordial Hydra, Goldvein/Lifeblood/Steelbane/Benevolent Hydra, Stonecoil Serpent, Hangarback Walker, Altered Ego (clone entering with N +1/+1 counters). ETB-with-counters interacts with the doublers above.
- Also: Biomass Mutation / Tanazir (set or double P/T — layer-dependent), Curse of the Swine / Oversimplify / Perplexing Test (exile-and-replace-with-tokens), Beast Within (create 3/3 for opponent). Copy + layer + token-generation heavy.
