# Silverquill Influence — Secrets of Strixhaven (soc, 2026)

Frozen target list for the soc precon pool. Orzhov (W/B) Commander precon built around
Auras, enchantments, and goad.

**Sources**
- Official Wizards decklist: <https://magic.wizards.com/en/news/announcements/secrets-of-strixhaven-commander-decklists>
- EDHREC precon: <https://edhrec.com/precon/silverquill-influence>
- Card Game Base: <https://cardgamebase.com/silverquill-influence-precon-decklist-spoilers/>

EDHREC and Card Game Base agree card-for-card; new `soc` cards spot-checked against the
Scryfall MCP (set `soc`, released 2026-04-24).

**Commander:** Killian, Decisive Mentor

---

## Commander (1)

| Count | Card |
|------:|------|
| 1 | Killian, Decisive Mentor |

## Creatures (25)

| Count | Card |
|------:|------|
| 1 | Ajani's Chosen |
| 1 | Archon of Sun's Grace |
| 1 | Armored Skyhunter |
| 1 | Breena, the Demagogue |
| 1 | Combat Calligrapher |
| 1 | Defacing Duskmage |
| 1 | Doomwake Giant |
| 1 | Eidolon of Countless Battles |
| 1 | Eiganjo Dynastorian |
| 1 | Eriette of the Charmed Apple |
| 1 | Firemane Commando |
| 1 | Hateful Eidolon |
| 1 | Herald of Amity |
| 1 | Keen Duelist |
| 1 | Killian, Ink Duelist |
| 1 | Kor Spiritdancer |
| 1 | Mangara, the Diplomat |
| 1 | Nils, Discipline Enforcer |
| 1 | Pearl-Ear, Imperial Advisor |
| 1 | Scriv, the Obligator |
| 1 | Shadrix Silverquill |
| 1 | Sram, Senior Edificer |
| 1 | Starfield Mystic |
| 1 | Tomik, Wielder of Law |
| 1 | Transcendent Envoy |

## Instants (4)

| Count | Card |
|------:|------|
| 1 | Anguished Unmaking |
| 1 | Fracture |
| 1 | Inkshield |
| 1 | Vanishing Verse |

## Sorceries (3)

| Count | Card |
|------:|------|
| 1 | Promise of Loyalty |
| 1 | Secret Rendezvous |
| 1 | Winds of Rath |

## Artifacts (4)

| Count | Card |
|------:|------|
| 1 | Arcane Signet |
| 1 | Fellwar Stone |
| 1 | Sol Ring |
| 1 | Talisman of Hierarchy |

## Enchantments (26)

| Count | Card |
|------:|------|
| 1 | Angelic Destiny |
| 1 | Animate Dead |
| 1 | Chains of Custody |
| 1 | Changing Loyalty |
| 1 | Coercive Impetus |
| 1 | Darksteel Mutation |
| 1 | Eldrazi Conscription |
| 1 | Fallen Ideal |
| 1 | Flickering Ward |
| 1 | Forum Filibuster |
| 1 | Ghostly Prison |
| 1 | Ghoulish Impetus |
| 1 | Gift of Immortality |
| 1 | Intermediate Chirography |
| 1 | Land Tax |
| 1 | Martial Impetus |
| 1 | Parasitic Impetus |
| 1 | Raffine's Guidance |
| 1 | Redemption Arc |
| 1 | Sage's Reverie |
| 1 | Screams from Within |
| 1 | Sentinel's Eyes |
| 1 | Sheltered by Ghosts |
| 1 | Shielded by Faith |
| 1 | Songbirds' Blessing |
| 1 | Spirit Mantle |

## Planeswalkers (0)

None.

## Lands (37)

| Count | Card |
|------:|------|
| 1 | Arcane Lighthouse |
| 1 | Bojuka Bog |
| 1 | Caves of Koilos |
| 1 | Command Tower |
| 1 | Desolate Mire |
| 1 | Eclipsed Steppe |
| 1 | Exotic Orchard |
| 1 | Fabled Passage |
| 1 | Fetid Heath |
| 1 | Forum of Amity |
| 1 | Isolated Chapel |
| 1 | Path of Ancestry |
| 1 | Shineshadow Snarl |
| 1 | Silverquill Campus |
| 1 | Study Hall |
| 1 | Sunlit Marsh |
| 1 | Temple of Silence |
| 1 | Terramorphic Expanse |
| 1 | Turbulent Moor |
| 1 | Umbral Expanse |
| 1 | War Room |
| 8 | Plains |
| 8 | Swamp |

---

## Group totals

| Group | Count |
|-------|------:|
| Commander | 1 |
| Creatures | 25 |
| Instants | 4 |
| Sorceries | 3 |
| Artifacts | 4 |
| Enchantments | 26 |
| Planeswalkers | 0 |
| Lands | 37 |
| **Grand total** | **100** |

---

## Unusual cards (hard to implement faithfully)

The whole deck is Aura-centric, so the *engine's* Aura attach/detach, control-change,
and "enchanted creature" tracking gets exercised hard. Specific pain points:

- **Layer-dependent Auras (type/PT/ability rewrites).** `Darksteel Mutation` (becomes
  Insect, loses abilities, indestructible), `Eldrazi Conscription` (+10/+10, trample,
  annihilator), `Angelic Destiny` (type-change + flying/first strike + return-on-death),
  `Sage's Reverie`, `Fallen Ideal`, `Spirit Mantle`. These are exactly the layer
  interactions the engine-core-and-event-model spec chose *not* to model — flag before pulling any of them in.
- **Control-changing effects.** `Changing Loyalty` (Aura that steals a creature) and
  Scriv/Killian's goad on opponents' creatures assume a real multiplayer control/attack
  model — Phase 4 territory, not the 1v1 engine.
- **Copy / "prepared" mechanic.** `Eiganjo Dynastorian` is a DFC-style *Prepare* card
  (cast a copy of its spell face `Replenish`) — needs copy-on-stack support.
- **Reanimation / replacement on death.** `Animate Dead` (reanimate + static debuff),
  `Gift of Immortality` and `Shielded by Faith` (indestructible + return-to-battlefield
  replacement), `Screams from Within`.
- **Global board wipes with conditions.** `Winds of Rath` (destroy all non-enchanted
  creatures) and `Doomwake Giant` (constellation -1/-1 to all opponents' creatures)
  need clean SBA + enchantment-ETB triggers.
- **No planeswalkers, no counters-doubling, no true copy-permanent effects** — those
  categories are absent, which is the one bit of good news.
