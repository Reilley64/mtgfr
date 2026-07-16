# Lorehold Spirit — Secrets of Strixhaven (soc, 2026)

Frozen target list for the soc precon pool.

**Commander:** Quintorius, History Chaser (Boros / RW **Legendary Planeswalker**, soc #7 — "Quintorius, History Chaser can be your commander")

Note: among the five soc precon face commanders, this is the only planeswalker.

Sources:
- Official Wizards decklists: https://magic.wizards.com/en/news/announcements/secrets-of-strixhaven-commander-decklists
- Card Game Base (per-deck): https://cardgamebase.com/lorehold-spirit-precon-decklist-spoilers/
- Scryfall set `soc`: https://scryfall.com/sets/soc

## Commander (1)
- 1 x Quintorius, History Chaser  *(planeswalker commander)*

## Creatures (36)
- 1 x Angel of Indemnity
- 1 x Anger
- 1 x Ao, the Dawn Sky
- 1 x Atsushi, the Blazing Sky
- 1 x Augusta, Order Returned
- 1 x Balefire Liege
- 1 x Claim Jumper
- 1 x Conspiracy Theorist
- 1 x Containment Construct
- 1 x Drumbellower
- 1 x Excava, the Risen Past
- 1 x Guardian of Faith
- 1 x Guardian Scalelord
- 1 x Hofri Ghostforge
- 1 x Kami of Ancient Law
- 1 x Karmic Guide
- 1 x Kirol, History Buff
- 1 x Laelia, the Blade Reforged
- 1 x Lorehold Archivist
- 1 x Millikin
- 1 x Moonshaker Cavalry
- 1 x Naktamun Lorespinner
- 1 x Quintorius, Field Historian
- 1 x Quintorius, Loremaster
- 1 x Relic Retriever
- 1 x Remorseful Cleric
- 1 x Selfless Spirit
- 1 x Serra Paragon
- 1 x Skyclave Apparition
- 1 x Spirit of Resilience
- 1 x Squee, Goblin Nabob
- 1 x Sun Titan
- 1 x Teshar, Ancestor's Apostle
- 1 x Vanguard of the Restless
- 1 x Venerable Warsinger
- 1 x White Orchid Phantom

## Instants / Sorceries (12)
Instants (3):
- 1 x Lorehold Charm
- 1 x Path to Exile
- 1 x Swords to Plowshares

Sorceries (9):
- 1 x Ceaseless Conflict
- 1 x Faithless Looting
- 1 x Fateful Tempest
- 1 x Rip Apart
- 1 x Secret Rendezvous
- 1 x Seize the Spoils
- 1 x Sevinne's Reclamation
- 1 x Tragic Arrogance
- 1 x Wave of Reckoning

## Artifacts (10)
- 1 x Arcane Signet
- 1 x Archaeomancer's Map
- 1 x Bitterthorn, Nissa's Animus
- 1 x Currency Converter
- 1 x Fellwar Stone
- 1 x Mind Stone
- 1 x Patchwork Banner
- 1 x Perpetual Timepiece
- 1 x Sol Ring
- 1 x Staff of the Storyteller

## Enchantments (4)
- 1 x Advanced Reconstruction
- 1 x Monologue Tax
- 1 x Primary Research
- 1 x Tocasia's Welcome

## Planeswalkers (0 in the 99; commander is a planeswalker)
- (the only planeswalker is the commander, Quintorius, History Chaser)

## Lands (37)
- 1 x Battlefield Forge
- 1 x Clifftop Retreat
- 1 x Command Tower
- 1 x Emeria, the Sky Ruin
- 1 x Exotic Orchard
- 1 x Fabled Passage
- 1 x Fields of Strife
- 1 x Furycalm Snarl
- 1 x Glittering Massif
- 1 x Lorehold Campus
- 1 x Lotus Field
- 1 x Mistveil Plains
- 1 x Radiant Summit
- 1 x Rugged Prairie
- 1 x Sacred Peaks
- 1 x Study Hall
- 1 x Sunscorched Divide
- 1 x Temple of Triumph
- 1 x Terramorphic Expanse
- 1 x Turbulent Steppe
- 11 x Plains
- 6 x Mountain

## Totals
Commander 1 + Creatures 36 + Instants/Sorceries 12 + Artifacts 10 + Enchantments 4 + Planeswalkers 0 + Lands 37 = **100**.
Distinct card names: 85 (83 unique singles + Plains + Mountain).

## Implementation risks
- **Planeswalker commander (hardest, engine-level):** Quintorius, History Chaser has loyalty, a +1 and a −4 loyalty ability, and a triggered ability "whenever one or more cards leave your graveyard, create a Spirit." Requires planeswalker support (loyalty counters, activate once/turn, can be attacked/damaged) AND a "cards leave graveyard" zone-change trigger — the whole deck keys off that trigger.
- **Reanimation / recursion loops:** Sun Titan, Karmic Guide, Serra Paragon, Sevinne's Reclamation, Teshar (free artifact/1-drop recursion loops), Squee/Anger from graveyard. ETB reanimation chains and potential infinite loops need loop detection / a "leaves graveyard" hook that feeds Quintorius.
- **Death-replacement effect:** Hofri Ghostforge replaces your creatures dying with "return as a token with haste" (replacement effect + copy). Layer/replacement handling.
- Also: Tragic Arrogance / Tocasia's Welcome / Wave of Reckoning (opponent- or you-choose sacrifice, damage = counters), Skyclave Apparition (exile + delayed token) — choice-heavy resolutions.
