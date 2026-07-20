# Prismari Artistry — Secrets of Strixhaven (soc, 2026)

Frozen target list for the soc precon pool.

**Commander:** Rootha, Mastering the Moment (Izzet / UR legendary creature, soc #8)

Sources:
- Official Wizards decklists: https://magic.wizards.com/en/news/announcements/secrets-of-strixhaven-commander-decklists
- Card Game Base (per-deck): https://cardgamebase.com/prismari-artistry-precon-decklist-spoilers/
- Scryfall set `soc`: https://scryfall.com/sets/soc

## Commander (1)
- 1 x Rootha, Mastering the Moment

## Creatures (25)
- 1 x Archmage Emeritus
- 1 x Brazen Borrower
- 1 x Brudiclad, Telchor Engineer
- 1 x Curiosity Crafter
- 1 x Dirgur Focusmage
- 1 x Faerie Mastermind
- 1 x Galazeth Prismari
- 1 x Goldspan Dragon
- 1 x Harmonic Prodigy
- 1 x Inspired Skypainter
- 1 x Leitmotif Composer
- 1 x Manaform Hellkite
- 1 x Mirrorwing Dragon
- 1 x Muddle, the Ever-Changing
- 1 x Plargg and Nassari
- 1 x Prismari Pianist
- 1 x Redoubled Stormsinger
- 1 x Renegade Bull
- 1 x Rionya, Fire Dancer
- 1 x Rootha, Mercurial Artist
- 1 x Solemn Simulacrum
- 1 x Stormcatch Mentor
- 1 x Storm-Kiln Artist
- 1 x Thunderclap Drake
- 1 x Veyran, Voice of Duality

## Instants / Sorceries (29)
Instants (10):
- 1 x Abrade
- 1 x Arcane Denial
- 1 x Big Score
- 1 x Chaos Warp
- 1 x Dig Through Time
- 1 x Magma Opus
- 1 x Prismari Charm
- 1 x Prismari Command
- 1 x Reality Shift
- 1 x Resculpt

Sorceries (19):
- 1 x Abstract Performance
- 1 x Aether Gale
- 1 x Blasphemous Act
- 1 x Chain Reaction
- 1 x Creative Technique
- 1 x Dance with Calamity
- 1 x Deep Analysis
- 1 x Expressive Iteration
- 1 x Furygale Flocking
- 1 x Mana Geyser
- 1 x Replication Technique
- 1 x Rite of Replication
- 1 x Rousing Refrain
- 1 x Surge to Victory
- 1 x Throes of Chaos
- 1 x Treasure Cruise
- 1 x Twinflame
- 1 x Volcanic Salvo
- 1 x Volcanic Torrent

## Artifacts (6)
- 1 x Arcane Signet
- 1 x Cursed Mirror
- 1 x Fellwar Stone
- 1 x Lightning Greaves
- 1 x Sol Ring
- 1 x Talisman of Creativity

## Enchantments (1)
- 1 x Determined Iteration

## Planeswalkers (0)
- (none)

## Lands (38)
- 1 x Cascade Bluffs
- 1 x Coastal Peak
- 1 x Command Tower
- 1 x Exotic Orchard
- 1 x Fabled Passage
- 1 x Ferrous Lake
- 1 x Frostboil Snarl
- 1 x Hall of Oracles
- 1 x Molten Tributary
- 1 x Mystic Sanctuary
- 1 x Path of Ancestry
- 1 x Prismari Campus
- 1 x Reliquary Tower
- 1 x Restless Spire
- 1 x Scorched Geyser
- 1 x Shivan Reef
- 1 x Spectacle Summit
- 1 x Study Hall
- 1 x Sulfur Falls
- 1 x Temple of Epiphany
- 1 x Temple of the False God
- 1 x Terramorphic Expanse
- 1 x Turbulent Springs
- 7 x Mountain
- 8 x Island

## Totals
Commander 1 + Creatures 25 + Instants/Sorceries 29 + Artifacts 6 + Enchantments 1 + Planeswalkers 0 + Lands 38 = **100**.
Distinct card names: 87 (85 unique singles + Mountain + Island).

## Implementation risks
- **Copy / token-copy effects (hardest):** Rite of Replication (kicker → 5 copies), Replication Technique, Twinflame, Determined Iteration (copies a token at end step), Cursed Mirror, Brudiclad (transform all your tokens into copies), Rionya (temporary token copies of a creature), Mirrorwing Dragon (opponents copy spells). Needs a real "copy of object" primitive with copiable characteristics + ETB re-resolution.
- **Magecraft trigger-doubling:** Veyran and Harmonic Prodigy double magecraft/instant-sorcery triggers; Archmage Emeritus / Storm-Kiln Artist / Stormcatch Mentor all fire on each cast. Trigger multiplication + ordering is engine-level, not per-card.
- **X spells + big-mana / cost payment:** Volcanic Salvo (cost reduced by power of two creatures), Magma Opus, Mana Geyser, Rousing Refrain, Chain Reaction. X on the stack and cost modification.
- Also: Chaos Warp (shuffle + reveal, needs injected randomness), Aether Gale / Resculpt / Reality Shift bounce-and-replace.
