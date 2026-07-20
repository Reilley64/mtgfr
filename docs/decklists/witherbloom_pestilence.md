# Witherbloom Pestilence — Secrets of Strixhaven (soc, 2026)

Frozen target list for the soc precon pool.

**Commander:** Dina, Essence Brewer (Golgari / BG legendary creature)

Sources:
- Official Wizards decklists: https://magic.wizards.com/en/news/announcements/secrets-of-strixhaven-commander-decklists
- Card Game Base (per-deck): https://cardgamebase.com/witherbloom-pestilence-precon-decklist-spoilers/
- Scryfall set `soc`: https://scryfall.com/sets/soc

## Commander (1)
- 1 x Dina, Essence Brewer

## Creatures (39)
- 1 x Beledros Witherbloom
- 1 x Blood Artist
- 1 x Bloodghast
- 1 x Blossoming Bogbeast
- 1 x Creakwood Liege
- 1 x Defiling Daemogoth
- 1 x Dina, Soul Steeper
- 1 x Eccentric Pestfinder
- 1 x Elvish Mystic
- 1 x Gilded Goose
- 1 x Gorma, the Gullet
- 1 x Gyome, Master Chef
- 1 x Haywire Mite
- 1 x Jadar, Ghoulcaller of Nephalia
- 1 x Mazirek, Kraul Death Priest
- 1 x Merchant of Venom
- 1 x Morbid Opportunist
- 1 x Mycoloth
- 1 x Nether Traitor
- 1 x Ohran Frostfang
- 1 x Ophiomancer
- 1 x Pawn of Ulamog
- 1 x Pest Rescuer
- 1 x Priest of Forgotten Gods
- 1 x Ribtruss Roaster
- 1 x Sakura-Tribe Elder
- 1 x Smothering Abomination
- 1 x Springbloom Druid
- 1 x Stensian Sanguinist
- 1 x Teacher's Pest
- 1 x Tendershoot Dryad
- 1 x Umbral Collar Zealot
- 1 x Veinwitch Coven
- 1 x Viscera Seer
- 1 x Wight of the Reliquary
- 1 x Witch of the Moors
- 1 x Woe Strider
- 1 x Yahenni, Undying Partisan
- 1 x Zulaport Cutthroat

## Instants / Sorceries (16)
Instants (5):
- 1 x Assassin's Trophy
- 1 x Infernal Grasp
- 1 x Mortality Spear
- 1 x Plumb the Forbidden
- 1 x Witherbloom Charm

Sorceries (11):
- 1 x Casualties of War
- 1 x Culling Ritual
- 1 x Cultivate
- 1 x Deadly Brew
- 1 x Final Act
- 1 x Immoral Bargain
- 1 x Night's Whisper
- 1 x Ominous Harvest
- 1 x Pest Infestation
- 1 x Toxic Deluge
- 1 x Witherbloom Command

## Artifacts (2)
- 1 x Arcane Signet
- 1 x Sol Ring

## Enchantments (5)
- 1 x Awakening Zone
- 1 x Blight Mound
- 1 x Feral Appetite
- 1 x Moldervine Reclamation
- 1 x Trudge Garden

## Planeswalkers (0)
- (none)

## Lands (37)
- 1 x Bojuka Bog
- 1 x Command Tower
- 1 x Exotic Orchard
- 1 x Fabled Passage
- 1 x Festering Thicket
- 1 x Grim Backwoods
- 1 x Haunted Mire
- 1 x High Market
- 1 x Llanowar Wastes
- 1 x Necroblossom Snarl
- 1 x Path of Ancestry
- 1 x Study Hall
- 1 x Temple of Malady
- 1 x Terramorphic Expanse
- 1 x Titan's Grave
- 1 x Turbulent Fen
- 1 x Twilight Mire
- 1 x Vernal Fen
- 1 x Viridescent Bog
- 1 x Witherbloom Campus
- 1 x Woodland Cemetery
- 8 x Forest
- 8 x Swamp

## Totals
Commander 1 + Creatures 39 + Instants/Sorceries 16 + Artifacts 2 + Enchantments 5 + Planeswalkers 0 + Lands 37 = **100**.
Distinct card names: 86 (84 unique singles + Forest + Swamp).

## Implementation risks
- **Stacked simultaneous death triggers (hardest):** Blood Artist, Zulaport Cutthroat, Dina/Dina Soul Steeper, Mazirek (counters on each sacrifice), Morbid Opportunist, Veinwitch Coven, Bastion-style drains. Mass sacrifice creates many triggers that must resolve/order correctly (APNAP-style ordering, "dies" seeing the board as it was).
- **X-cost / choice-heavy board wipes:** Toxic Deluge (X life-payment −X/−X), Culling Ritual (X ramp/destroy), Casualties of War (multi-target modal destruction), Final Act. X on stack + multi-target selection.
- **Recursion & recurring triggers:** Bloodghast (returns on land drop), Nether Traitor (haste recursion), Ophiomancer (upkeep token if none), Beledros Witherbloom (pay 10 life to untap lands — big-mana engine + life as resource). Graveyard-to-battlefield replacement/return timing.
- Also: sac outlets everywhere (Viscera Seer, Woe Strider, Priest of Forgotten Gods, Yahenni) — need a clean free-sacrifice framework.
