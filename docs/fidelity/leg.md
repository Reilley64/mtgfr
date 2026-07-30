# Legends (`leg`) — fidelity report

310 unique cards. Source of truth: Scryfall `set:leg unique:cards`.
Engine backlog for section D: [`leg-increments.md`](leg-increments.md).

| Section | Meaning | Count |
| --- | --- | ---: |
| A | In the pool, faithful | 7 |
| B | In the pool, approximated | 0 |
| C | New, expressible with today's DSL | 102 |
| D | New, needs engine work | 198 |
| — | Out of scope (flag-don't-force) | 3 |

Every card already in the pool carries `leg` in its `sets` array; the intake diff found no
second-side misses.

## A. In the pool, faithful

Already scripted with no `approximates` field. Re-frame-audited as part of this grind.

- [ ] **All Hallow's Eve** {2}{B}{B} · Sorcery
- [ ] **Chain Lightning** {R} · Sorcery
- [x] **Concordant Crossroads** {G} · World Enchantment — faithful as of increment 2
- [ ] **Fallen Angel** {3}{B}{B} · Creature — Angel
- [ ] **Land Tax** {W} · Enchantment
- [ ] **Rubinia Soulsinger** {2}{G}{W}{U} · Legendary Creature — Faerie
- [ ] **Xira Arien** {B}{R}{G} · Legendary Creature — Insect Wizard

## B. In the pool, approximated

None. Concordant Crossroads was the only entry and is faithful as of increment 2.

### Observability re-audit

Every `approximates` in `crates/cards/data/` and every `ponytail:` in `crates/engine/src/`
and `crates/cards/src/` was re-read against the incoming set. Two claims fell:

- **Concordant Crossroads** — "World supertype … is the only World card in the pool, so the
  rule has nothing to interact with yet." Legends prints **eleven** World enchantments
  (Arboria, Caverns of Despair, Field of Dreams, Gravity Sphere, In the Eye of Chaos,
  Land's Edge, Living Plane, Nether Void, Revelation, Storm World, The Abyss). CR 704.5k now
  has plenty to bite on. Moves to real work as **increment 2**.
- **Spurnmage Advocate** — "only 'attacking' is modeled — no `blocking` filter axis exists
  yet on `PermanentFilter`." The note describes a card that does not exist. Spurnmage Advocate
  reads "{T}: Return two target cards from an opponent's graveyard to their hand. Destroy target
  attacking creature." — a {W} 1/1 Human Nomad with a plain `attacking` target and no "or
  blocking" clause. The pool TOML was authored against invented text (the wrong cost, P/T and
  subtypes, an exile-as-cost first clause, and Labyrinth of Skophos' remove-from-combat as the
  second). **Increment 8** cleared the bogus note and corrected the front matter; the body is
  now **increment 120**, carrying a precise `approximates` until it lands.

Everything else held. The `reveal.rs` `matched_dest` trio, the CR 613 timestamp note in
`characteristics.rs`, the text-changer composition note in `types/object.rs`, and the
single-kicker notes in `types/mana.rs` all survive: no Legends card exercises them.

## C. New, expressible with today's DSL

Authored in Phase 3 with no engine change, TDD, over a nine-batch fan-out. A further 39 cards
were classified here at intake and reclassified to section D once authoring reached them —
each is listed there against the increment that blocks it.

**Mana Drain** is the wave's only `approximates`: the delayed {C} is scheduled for the
controller's next precombat main phase, so casting it during your own precombat main pays out
next turn rather than in this turn's postcombat main phase.

- [x] **Acid Rain** {3}{U} · Sorcery
- [x] **Active Volcano** {R} · Instant
- [x] **Adun Oakenshield** {B}{R}{G} · Legendary Creature — Human Knight
- [x] **Alabaster Potion** {X}{W}{W} · Instant
- [x] **Amrou Kithkin** {W}{W} · Creature — Kithkin
- [x] **Axelrod Gunnarson** {4}{B}{B}{R}{R} · Legendary Creature — Giant
- [x] **Azure Drake** {3}{U} · Creature — Drake
- [x] **Barbary Apes** {1}{G} · Creature — Ape
- [x] **Barktooth Warbeard** {4}{B}{R}{R} · Legendary Creature — Human Warrior
- [x] **Blight** {B}{B} · Enchantment — Aura
- [x] **Boomerang** {U}{U} · Instant
- [x] **Carrion Ants** {2}{B}{B} · Creature — Insect
- [x] **Cat Warriors** {1}{G}{G} · Creature — Cat Warrior
- [x] **Cleanse** {2}{W}{W} · Sorcery
- [x] **Crimson Kobolds** {0} · Creature — Kobold
- [x] **Crookshank Kobolds** {0} · Creature — Kobold
- [x] **Dakkon Blackblade** {2}{W}{U}{U}{B} · Legendary Creature — Human Warrior
- [x] **Darkness** {B} · Instant
- [x] **Devouring Deep** {2}{U} · Creature — Fish
- [x] **Disharmony** {2}{R} · Instant
- [x] **Divine Offering** {1}{W} · Instant
- [x] **Divine Transformation** {2}{W}{W} · Enchantment — Aura
- [x] **Durkwood Boars** {4}{G} · Creature — Boar
- [x] **Emerald Dragonfly** {1}{G} · Creature — Insect
- [x] **Eternal Warrior** {R} · Enchantment — Aura
- [x] **Fire Sprites** {1}{G} · Creature — Faerie
- [x] **Flash Flood** {U} · Instant
- [x] **Force Spike** {U} · Instant
- [x] **Fortified Area** {1}{W}{W} · Enchantment
- [x] **Ghosts of the Damned** {1}{B}{B} · Creature — Spirit
- [x] **Giant Strength** {R}{R} · Enchantment — Aura
- [x] **Great Defender** {W} · Instant
- [x] **Greed** {3}{B} · Enchantment
- [x] **Gwendlyn Di Corci** {U}{B}{B}{R} · Legendary Creature — Human Rogue
- [x] **Headless Horseman** {2}{B} · Creature — Zombie Knight
- [x] **Hell Swarm** {B} · Instant
- [x] **Hell's Caretaker** {3}{B} · Creature — Horror
- [x] **Holy Day** {W} · Instant
- [x] **Hornet Cobra** {1}{G}{G} · Creature — Snake
- [x] **Horror of Horrors** {3}{B}{B} · Enchantment
- [x] **Hyperion Blacksmith** {1}{R}{R} · Creature — Human Artificer
- [x] **Immolation** {R} · Enchantment — Aura
- [x] **Jacques le Vert** {1}{R}{G}{W} · Legendary Creature — Human Warrior
- [x] **Jasmine Boreal** {3}{G}{W} · Legendary Creature — Human
- [x] **Jedit Ojanen** {4}{W}{W}{U} · Legendary Creature — Cat Warrior
- [x] **Jerrard of the Closed Fist** {3}{R}{G}{G} · Legendary Creature — Human Knight
- [x] **Jovial Evil** {2}{B} · Sorcery
- [x] **Kasimir the Lone Wolf** {4}{W}{U} · Legendary Creature — Human Warrior
- [x] **Keepers of the Faith** {1}{W}{W} · Creature — Human Cleric
- [x] **Kei Takahashi** {2}{G}{W} · Legendary Creature — Human Cleric
- [x] **Killer Bees** {1}{G}{G} · Creature — Insect
- [x] **Kobold Drill Sergeant** {1}{R} · Creature — Kobold Soldier
- [x] **Kobold Overlord** {1}{R} · Creature — Kobold
- [x] **Kobold Taskmaster** {1}{R} · Creature — Kobold
- [x] **Kobolds of Kher Keep** {0} · Creature — Kobold
- [x] **Lady Orca** {5}{B}{R} · Legendary Creature — Demon
- [x] **Life Chisel** {4} · Artifact
- [x] **Lifeblood** {2}{W}{W} · Enchantment
- [x] **Lost Soul** {1}{B}{B} · Creature — Spirit Minion
- [x] **Mana Drain** {U}{U} · Instant
- [x] **Moss Monster** {3}{G}{G} · Creature — Elemental
- [x] **Mountain Yeti** {2}{R}{R} · Creature — Yeti
- [x] **Palladia-Mors** {2}{R}{R}{G}{G}{W}{W} · Legendary Creature — Elder Dragon
- [x] **Pavel Maliki** {4}{B}{R} · Legendary Creature — Human
- [x] **Pixie Queen** {2}{G}{G} · Creature — Faerie
- [x] **Planar Gate** {6} · Artifact
- [x] **Pradesh Gypsies** {2}{G} · Creature — Human Nomad
- [x] **Princess Lucrezia** {3}{U}{U}{B} · Legendary Creature — Human Wizard
- [x] **Pyrotechnics** {4}{R} · Sorcery
- [x] **Raging Bull** {2}{R} · Creature — Ox
- [x] **Ragnar** {G}{W}{U} · Legendary Creature — Human Cleric
- [x] **Ramirez DePietro** {3}{U}{B}{B} · Legendary Creature — Human Pirate
- [x] **Ramses Overdark** {2}{U}{U}{B}{B} · Legendary Creature — Human Assassin
- [x] **Relic Barrier** {2} · Artifact
- [x] **Remove Soul** {1}{U} · Instant
- [x] **Righteous Avengers** {4}{W} · Creature — Human Soldier
- [x] **Riven Turnbull** {5}{U}{B} · Legendary Creature — Human Advisor
- [x] **Segovian Leviathan** {4}{U} · Creature — Leviathan
- [x] **Shield Wall** {1}{W} · Instant
- [x] **Sir Shandlar of Eberyn** {4}{G}{W} · Legendary Creature — Human Knight
- [x] **Sivitri Scarzam** {5}{U}{B} · Legendary Creature — Human
- [x] **Sol'kanar the Swamp King** {2}{U}{B}{R} · Legendary Creature — Demon
- [x] **Spinal Villain** {2}{R} · Creature — Beast
- [x] **Spirit Link** {W} · Enchantment — Aura
- [x] **Storm Seeker** {3}{G} · Instant
- [x] **Sunastian Falconer** {3}{R}{G} · Legendary Creature — Human Shaman
- [x] **The Brute** {1}{R} · Enchantment — Aura
- [x] **The Lady of the Mountain** {4}{R}{G} · Legendary Creature — Giant
- [x] **Thunder Spirit** {1}{W}{W} · Creature — Elemental Spirit
- [x] **Tobias Andrion** {3}{W}{U} · Legendary Creature — Human Advisor
- [x] **Torsten Von Ursus** {3}{G}{G}{W} · Legendary Creature — Human Soldier
- [x] **Tuknir Deathlock** {R}{R}{G}{G} · Legendary Creature — Human Wizard
- [x] **Tundra Wolves** {W} · Creature — Wolf
- [x] **Typhoon** {2}{G} · Sorcery
- [x] **Untamed Wilds** {2}{G} · Sorcery
- [x] **Vaevictis Asmadi** {2}{B}{B}{R}{R}{G}{G} · Legendary Creature — Elder Dragon
- [x] **Walking Dead** {1}{B} · Creature — Zombie
- [x] **Wall of Earth** {1}{R} · Creature — Wall
- [x] **Wall of Heat** {2}{R} · Creature — Wall
- [x] **Wall of Light** {2}{W} · Creature — Wall
- [x] **Wall of Opposition** {3}{R}{R} · Creature — Wall
- [x] **Zephyr Falcon** {1}{U} · Creature — Bird

## D. New, needs engine work

Blocked on the numbered increments in [`leg-increments.md`](leg-increments.md). A checked box
means the blocking increment has landed and the card is scripted; the increment number stays so
the record shows what unblocked it.

- [ ] **Abomination** {3}{B}{B} · Creature — Horror — increment 105
- [ ] **Adventurers' Guildhouse** — · Land — increment 3
- [x] **Aerathi Berserker** {2}{R}{R}{R} · Creature — Human Berserker — increment 1; residual — increment 119
- [ ] **Aisling Leprechaun** {G} · Creature — Faerie — increment 14
- [ ] **Akron Legionnaire** {6}{W}{W} · Creature — Giant Soldier — increment 10
- [ ] **Al-abara's Carpet** {5} · Artifact — increment 12
- [ ] **Alchor's Tomb** {4} · Artifact — increment 96
- [ ] **Angelic Voices** {2}{W}{W} · Enchantment — increment 13
- [ ] **Angus Mackenzie** {G}{W}{U} · Legendary Creature — Human Cleric — increment 111
- [ ] **Anti-Magic Aura** {2}{U} · Enchantment — Aura — increment 15
- [ ] **Arboria** {2}{G}{G} · World Enchantment — increment 2, 16
- [ ] **Arcades Sabboth** {2}{G}{G}{W}{W}{U}{U} · Legendary Creature — Elder Dragon — increment 13
- [x] **Arena of the Ancients** {3} · Artifact — increment 6 (absorbed 17)
- [ ] **Avoid Fate** {G} · Instant — increment 18
- [ ] **Ayesha Tanaka** {W}{W}{U}{U} · Legendary Creature — Human Artificer — increment 103
- [ ] **Backdraft** {1}{R} · Instant — increment 19
- [ ] **Backfire** {U} · Enchantment — Aura — increment 20
- [ ] **Bartel Runeaxe** {3}{B}{R}{G} · Legendary Creature — Giant Warrior — increment 15
- [ ] **Beasts of Bogardan** {4}{R} · Creature — Beast — increment 13
- [x] **Black Mana Battery** {4} · Artifact — increment 11
- [ ] **Blazing Effigy** {1}{R} · Creature — Elemental — increment 19
- [ ] **Blood Lust** {1}{R} · Instant — increment 21
- [x] **Blue Mana Battery** {4} · Artifact — increment 11
- [ ] **Boris Devilboon** {3}{B}{R} · Legendary Creature — Zombie Wizard — increment 97
- [ ] **Brine Hag** {2}{U}{U} · Creature — Hag — increment 22
- [ ] **Bronze Horse** {7} · Artifact Creature — Horse — increment 12
- [x] **Cathedral of Serra** — · Land — increment 3 (slice 1); residual — increment 3 slices 2, 3
- [ ] **Caverns of Despair** {2}{R}{R} · World Enchantment — increment 2, 23
- [ ] **Chains of Mephistopheles** {1}{B} · Enchantment — increment 24
- [x] **Chromium** {2}{W}{W}{U}{U}{B}{B} · Legendary Creature — Elder Dragon — increment 1; residual — increment 119
- [ ] **Clergy of the Holy Nimbus** {W} · Creature — Human Cleric — increment 25
- [ ] **Cocoon** {G} · Enchantment — Aura — increment 26
- [x] **Cosmic Horror** {3}{B}{B}{B} · Creature — Horror — increment 98
- [x] **Craw Giant** {3}{G}{G}{G}{G} · Creature — Giant — increment 1; residual — increment 119
- [x] **Crevasse** {2}{R} · Enchantment — increment 4
- [x] **Crimson Manticore** {2}{R}{R} · Creature — Manticore — increment 8
- [ ] **Cyclopean Mummy** {1}{B} · Creature — Zombie — increment 109
- [x] **D'Avenant Archer** {2}{W} · Creature — Human Soldier Archer — increment 8
- [x] **Deadfall** {2}{G} · Enchantment — increment 4
- [ ] **Demonic Torment** {2}{B} · Enchantment — Aura — increment 95
- [ ] **Divine Intervention** {6}{W}{W} · Enchantment — increment 27
- [ ] **Dream Coat** {U} · Enchantment — Aura — increment 28
- [ ] **Dwarven Song** {R} · Instant — increment 96
- [ ] **Elder Land Wurm** {4}{W}{W}{W} · Creature — Dragon Wurm — increment 5
- [ ] **Elder Spawn** {4}{U}{U}{U} · Creature — Spawn — increment 29
- [ ] **Elven Riders** {3}{G}{G} · Creature — Elf — increment 9
- [ ] **Enchanted Being** {1}{W}{W} · Creature — Human — increment 12
- [ ] **Enchantment Alteration** {U} · Instant — increment 30
- [ ] **Energy Tap** {U} · Sorcery — increment 31
- [ ] **Equinox** {W} · Enchantment — Aura — increment 32
- [ ] **Eureka** {2}{G}{G} · Sorcery — increment 33
- [ ] **Evil Eye of Orms-by-Gore** {4}{B} · Creature — Eye — increment 9, 10
- [ ] **Feint** {R} · Instant — increment 34
- [ ] **Field of Dreams** {U} · World Enchantment — increment 2, 35
- [ ] **Firestorm Phoenix** {4}{R}{R} · Creature — Phoenix — increment 36
- [ ] **Flash Counter** {1}{U} · Instant — increment 105
- [ ] **Floral Spuzzem** {3}{G} · Creature — Elemental — increment 37
- [ ] **Forethought Amulet** {5} · Artifact — increment 38
- [x] **Frost Giant** {3}{R}{R}{R} · Creature — Giant — increment 1; residual — increment 119
- [ ] **Gabriel Angelfire** {3}{G}{G}{W}{W} · Legendary Creature — Angel — increment 1, 39
- [ ] **Gaseous Form** {2}{U} · Enchantment — Aura — increment 95
- [ ] **Gauntlets of Chaos** {5} · Artifact — increment 40
- [ ] **Giant Slug** {1}{B} · Creature — Slug — increment 41
- [ ] **Giant Turtle** {1}{G}{G} · Creature — Turtle — increment 42
- [ ] **Glyph of Delusion** {U} · Instant — increment 43
- [ ] **Glyph of Destruction** {R} · Instant — increment 43
- [ ] **Glyph of Doom** {B} · Instant — increment 43
- [ ] **Glyph of Life** {W} · Instant — increment 43
- [ ] **Glyph of Reincarnation** {G} · Instant — increment 43
- [x] **Gosta Dirk** {3}{W}{W}{U}{U} · Legendary Creature — Human Warrior — increment 4
- [ ] **Gravity Sphere** {2}{R} · World Enchantment — increment 2, 44
- [x] **Great Wall** {2}{W} · Enchantment — increment 4
- [ ] **Greater Realm of Preservation** {1}{W} · Enchantment — increment 12
- [x] **Green Mana Battery** {4} · Artifact — increment 11
- [ ] **Halfdane** {1}{W}{U}{B} · Legendary Creature — Shapeshifter — increment 22
- [ ] **Hammerheim** — · Legendary Land — increment 5
- [ ] **Hazezon Tamar** {4}{R}{G}{W} · Legendary Creature — Human Warrior — increment 45
- [ ] **Heaven's Gate** {W} · Instant — increment 96
- [ ] **Hellfire** {2}{B}{B}{B} · Sorcery — increment 46
- [ ] **Horn of Deafening** {4} · Artifact — increment 94
- [x] **Hunding Gjornersen** {3}{W}{U}{U} · Legendary Creature — Human Warrior — increment 1; residual — increment 119
- [ ] **Ichneumon Druid** {1}{G}{G} · Creature — Human Druid — increment 101
- [ ] **Imprison** {B} · Enchantment — Aura — increment 47
- [ ] **In the Eye of Chaos** {2}{U} · World Enchantment — increment 2, 48
- [ ] **Indestructible Aura** {W} · Instant — increment 94
- [ ] **Infernal Medusa** {3}{B}{B} · Creature — Gorgon — increment 100
- [ ] **Infinite Authority** {W}{W}{W} · Enchantment — Aura — increment 49
- [ ] **Invoke Prejudice** {U}{U}{U}{U} · Enchantment — increment 48
- [ ] **Ivory Guardians** {4}{W}{W} · Creature — Giant Cleric — increment 13
- [ ] **Johan** {3}{R}{G}{W} · Legendary Creature — Human Wizard — increment 50
- [ ] **Juxtapose** {3}{U} · Sorcery — increment 40
- [x] **Karakas** — · Legendary Land — increment 6
- [ ] **Kismet** {3}{W} · Enchantment — increment 51
- [ ] **Knowledge Vault** {4} · Artifact — increment 52
- [ ] **Kry Shield** {2} · Artifact — increment 94
- [x] **Lady Caleria** {3}{G}{G}{W}{W} · Legendary Creature — Elf Archer — increment 8
- [ ] **Lady Evangela** {W}{U}{B} · Legendary Creature — Human Cleric — increment 94
- [ ] **Land Equilibrium** {2}{U}{U} · Enchantment — increment 53
- [ ] **Land's Edge** {1}{R}{R} · World Enchantment — increment 2, 25
- [ ] **Lesser Werewolf** {3}{B} · Creature — Werewolf — increment 8, 54
- [ ] **Life Matrix** {4} · Artifact — increment 55
- [ ] **Living Plane** {2}{G}{G} · World Enchantment — increment 2, 44
- [ ] **Livonya Silone** {2}{R}{R}{G}{G} · Legendary Creature — Human Warrior — increment 6, 7
- [x] **Lord Magnus** {3}{G}{W}{W} · Legendary Creature — Human Druid — increment 4
- [ ] **Mana Matrix** {6} · Artifact — increment 105
- [ ] **Marble Priest** {5} · Artifact Creature — Cleric — increment 12, 56
- [x] **Marhault Elsdragon** {3}{R}{R}{G} · Legendary Creature — Elf Warrior — increment 1; residual — increment 119
- [ ] **Master of the Hunt** {2}{G}{G} · Creature — Human — increment 3
- [ ] **Mirror Universe** {6} · Artifact — increment 57
- [ ] **Moat** {2}{W}{W} · Enchantment — increment 107
- [ ] **Mold Demon** {5}{B}{B} · Creature — Fungus Demon — increment 106
- [ ] **Mountain Stronghold** — · Land — increment 3
- [ ] **Nebuchadnezzar** {3}{U}{B} · Legendary Creature — Human Wizard — increment 58
- [ ] **Nether Void** {3}{B} · World Enchantment — increment 2, 48
- [ ] **Nicol Bolas** {2}{U}{U}{B}{B}{R}{R} · Legendary Creature — Elder Dragon — increment 112
- [ ] **North Star** {4} · Artifact — increment 59
- [ ] **Nova Pentacle** {4} · Artifact — increment 60
- [ ] **Osai Vultures** {1}{W} · Creature — Bird — increment 102
- [ ] **Part Water** {X}{X}{U} · Sorcery — increment 61
- [ ] **Pendelhaven** — · Legendary Land — increment 62
- [ ] **Petra Sphinx** {2}{W}{W}{W} · Creature — Sphinx — increment 58
- [x] **Pit Scorpion** {2}{B} · Creature — Scorpion — increment 99; residual carried to increment 118
- [ ] **Presence of the Master** {3}{W} · Enchantment — increment 108
- [ ] **Primordial Ooze** {R} · Creature — Ooze — increment 63
- [ ] **Psionic Entity** {4}{U} · Creature — Illusion — increment 110
- [ ] **Psychic Purge** {U} · Sorcery — increment 64
- [ ] **Puppet Master** {U}{U}{U} · Enchantment — Aura — increment 65
- [x] **Quagmire** {2}{B} · Enchantment — increment 4
- [ ] **Quarum Trench Gnomes** {3}{R} · Creature — Gnome — increment 66
- [ ] **Rabid Wombat** {2}{G}{G} · Creature — Wombat — increment 67
- [ ] **Radjan Spirit** {3}{G} · Creature — Spirit — increment 5
- [x] **Rapid Fire** {3}{W} · Instant — increment 1; residual — increment 119
- [ ] **Rasputin Dreamweaver** {4}{W}{U} · Legendary Creature — Human Wizard — increment 68
- [ ] **Recall** {X}{X}{U} · Sorcery — increment 69
- [x] **Red Mana Battery** {4} · Artifact — increment 11
- [ ] **Reincarnation** {1}{G}{G} · Instant — increment 70
- [ ] **Relic Bind** {2}{U} · Enchantment — Aura — increment 20
- [ ] **Remove Enchantments** {W} · Instant — increment 71
- [ ] **Reset** {U}{U} · Instant — increment 72
- [ ] **Revelation** {G} · World Enchantment — increment 2, 35
- [ ] **Reverberation** {2}{U}{U} · Instant — increment 19
- [ ] **Ring of Immortals** {5} · Artifact — increment 18
- [ ] **Rohgahh of Kher Keep** {2}{B}{B}{R}{R} · Legendary Creature — Kobold — increment 73
- [ ] **Rust** {G} · Instant — increment 103
- [ ] **Sea Kings' Blessing** {U} · Instant — increment 96
- [ ] **Seafarer's Quay** — · Land — increment 3
- [ ] **Seeker** {2}{W}{W} · Enchantment — Aura — increment 9
- [ ] **Sentinel** {4} · Artifact Creature — Shapeshifter — increment 8, 22
- [ ] **Serpent Generator** {6} · Artifact — increment 97
- [ ] **Shelkin Brownie** {1}{G} · Creature — Ouphe — increment 3, 5
- [ ] **Shimian Night Stalker** {3}{B}{B} · Creature — Nightstalker — increment 60
- [ ] **Silhouette** {1}{U} · Instant — increment 12
- [ ] **Spectral Cloak** {U}{U} · Enchantment — Aura — increment 117
- [ ] **Spirit Shackle** {B}{B} · Enchantment — Aura — increment 102
- [ ] **Spiritual Sanctuary** {2}{W}{W} · Enchantment — increment 116
- [ ] **Stangg** {4}{R}{G} · Legendary Creature — Human Warrior — increment 74
- [ ] **Storm World** {R} · World Enchantment — increment 2, 75
- [ ] **Subdue** {G} · Instant — increment 94
- [ ] **Sword of the Ages** {6} · Artifact — increment 76
- [ ] **Sylvan Library** {1}{G} · Enchantment — increment 77
- [ ] **Sylvan Paradise** {G} · Instant — increment 96
- [ ] **Syphon Soul** {2}{B} · Sorcery — increment 113
- [ ] **Takklemaggot** {2}{B}{B} · Enchantment — Aura — increment 78
- [ ] **Telekinesis** {U}{U} · Instant — increment 79
- [x] **Teleport** {U}{U}{U} · Instant — increment 114
- [ ] **Tetsuo Umezawa** {U}{B}{R} · Legendary Creature — Human Archer — increment 8, 15
- [ ] **The Abyss** {3}{B} · World Enchantment — increment 2, 80
- [ ] **The Tabernacle at Pendrell Vale** — · Legendary Land — increment 81
- [ ] **The Wretched** {3}{B}{B} · Creature — Demon — increment 104
- [ ] **Time Elemental** {2}{U} · Creature — Elemental — increment 82
- [ ] **Tolaria** — · Legendary Land — increment 3, 5
- [x] **Tor Wauki** {2}{B}{B}{R} · Legendary Creature — Human Archer — increment 8
- [ ] **Touch of Darkness** {B} · Instant — increment 96
- [ ] **Transmutation** {1}{B} · Instant — increment 22
- [ ] **Triassic Egg** {4} · Artifact — increment 83
- [x] **Undertow** {2}{U} · Enchantment — increment 4
- [x] **Underworld Dreams** {B}{B}{B} · Enchantment — increment 115
- [ ] **Unholy Citadel** — · Land — increment 3
- [x] **Ur-Drago** {3}{U}{U}{B}{B} · Legendary Creature — Elemental — increment 4
- [ ] **Urborg** — · Legendary Land — increment 5
- [ ] **Vampire Bats** {B} · Creature — Bat — increment 84
- [ ] **Venarian Gold** {X}{U}{U} · Enchantment — Aura — increment 26
- [ ] **Visions** {W} · Sorcery — increment 85
- [ ] **Voodoo Doll** {6} · Artifact — increment 86
- [ ] **Wall of Caltrops** {1}{W} · Creature — Wall — increment 49, 87
- [ ] **Wall of Dust** {2}{R} · Creature — Wall — increment 42
- [ ] **Wall of Putrid Flesh** {2}{B} · Creature — Wall — increment 12
- [ ] **Wall of Shadows** {1}{B}{B} · Creature — Wall — increment 12, 88
- [ ] **Wall of Tombstones** {1}{B} · Creature — Wall — increment 22
- [ ] **Wall of Vapor** {3}{U} · Creature — Wall — increment 12
- [ ] **Wall of Wonder** {2}{U}{U} · Creature — Wall — increment 89
- [ ] **Whirling Dervish** {G}{G} · Creature — Human Monk — increment 90
- [x] **White Mana Battery** {4} · Artifact — increment 11
- [x] **Willow Satyr** {2}{G}{G} · Creature — Satyr — increment 6
- [ ] **Winds of Change** {R} · Sorcery — increment 91
- [ ] **Winter Blast** {X}{G} · Sorcery — increment 61, 92
- [x] **Wolverine Pack** {2}{G}{G} · Creature — Wolverine — increment 1; residual — increment 119
- [ ] **Wood Elemental** {3}{G} · Creature — Elemental — increment 93

## Out of scope

Flagged, not forced. These are not increments — they are card mechanics this game
deliberately does not model.

- [ ] **Falling Star** {2}{R} · Sorcery — physical dexterity (CR 713, the Chaos Orb family) — no digital analogue
- [ ] **Rebirth** {3}{G}{G}{G} · Sorcery — ante (CR 407) — not supported
- [ ] **Tempest Efreet** {1}{R}{R}{R} · Creature — Efreet — ante (CR 407) — not supported
