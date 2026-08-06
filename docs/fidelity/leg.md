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

- [x] **All Hallow's Eve** {2}{B}{B} · Sorcery — scream-counter countdown ticks in the `Step::Upkeep`
      arm and the `on_expiry` payload runs after the graveyard move
- [x] **Chain Lightning** {R} · Sorcery — the reflexive `{R}{R}` is offered to the damaged player or
      the damaged permanent's controller, and the copy mints under that payer
- [x] **Concordant Crossroads** {G} · World Enchantment — faithful as of increment 2
- [x] **Fallen Angel** {3}{B}{B} · Creature — Angel
- [x] **Land Tax** {W} · Enchantment — "up to three" and the single shuffle both hold
- [x] **Rubinia Soulsinger** {2}{G}{W}{U} · Legendary Creature — Faerie — the steal reverts as a
      state-based check on all three legs: source leaves, you lose control of it, or it untaps
- [x] **Xira Arien** {B}{R}{G} · Legendary Creature — Insect Wizard

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

- [x] **Abomination** {3}{B}{B} · Creature — Horror — increment 105
- [x] **Adventurers' Guildhouse** — · Land — increment 3 (slices 1, 2, 3, 4)
- [x] **Aerathi Berserker** {2}{R}{R}{R} · Creature — Human Berserker — increment 1, 119
- [x] **Aisling Leprechaun** {G} · Creature — Faerie — increment 14
- [x] **Akron Legionnaire** {6}{W}{W} · Creature — Giant Soldier — faithful as of increment 10
- [x] **Al-abara's Carpet** {5} · Artifact — increment 12
- [x] **Alchor's Tomb** {4} · Artifact — faithful as of increment 96
- [x] **Angelic Voices** {2}{W}{W} · Enchantment — increment 13
- [x] **Angus Mackenzie** {G}{W}{U} · Legendary Creature — Human Cleric — increment 111
- [x] **Anti-Magic Aura** {2}{U} · Enchantment — Aura — increment 15
- [x] **Arboria** {2}{G}{G} · World Enchantment — increment 2, 16
- [x] **Arcades Sabboth** {2}{G}{G}{W}{W}{U}{U} · Legendary Creature — Elder Dragon — increment 13
- [x] **Arena of the Ancients** {3} · Artifact — increment 6 (absorbed 17)
- [x] **Avoid Fate** {G} · Instant — increment 18
- [x] **Ayesha Tanaka** {W}{W}{U}{U} · Legendary Creature — Human Artificer — increment 103
      (approximated: while attacking, her banding does not take over the division of a blocker's
      combat damage — CR 702.22k)
- [x] **Backdraft** {1}{R} · Instant — increment 19, 135, 176
      (approximated: the chooser is a target, so this Backdraft is uncastable when no player has
      cast a sorcery rather than castable and blank — increment 216)
- [x] **Backfire** {U} · Enchantment — Aura — increment 20
- [x] **Bartel Runeaxe** {3}{B}{R}{G} · Legendary Creature — Giant Warrior — increment 15
- [x] **Beasts of Bogardan** {4}{R} · Creature — Beast — increment 13
- [x] **Black Mana Battery** {4} · Artifact — increment 11
- [x] **Blazing Effigy** {1}{R} · Creature — Elemental — increment 19
- [x] **Blood Lust** {1}{R} · Instant — increment 21
- [x] **Blue Mana Battery** {4} · Artifact — increment 11
- [x] **Boris Devilboon** {3}{B}{R} · Legendary Creature — Zombie Wizard — increment 97
- [x] **Brine Hag** {2}{U}{U} · Creature — Hag — increment 22
- [x] **Bronze Horse** {7} · Artifact Creature — Horse — increment 12
- [x] **Cathedral of Serra** — · Land — increment 3 (slices 1, 2, 3)
- [x] **Caverns of Despair** {2}{R}{R} · World Enchantment — increment 2, 23
- [x] **Chains of Mephistopheles** {1}{B} · Enchantment — increment 24
- [x] **Chromium** {2}{W}{W}{U}{U}{B}{B} · Legendary Creature — Elder Dragon — increment 1, 119
- [x] **Clergy of the Holy Nimbus** {W} · Creature — Human Cleric — increments 25, 128
- [x] **Cocoon** {G} · Enchantment — Aura — increment 26
- [x] **Cosmic Horror** {3}{B}{B}{B} · Creature — Horror — increment 98
- [x] **Craw Giant** {3}{G}{G}{G}{G} · Creature — Giant — increment 1, 119
- [x] **Crevasse** {2}{R} · Enchantment — increment 4
- [x] **Crimson Manticore** {2}{R}{R} · Creature — Manticore — increment 8
- [x] **Cyclopean Mummy** {1}{B} · Creature — Zombie — increment 109
- [x] **D'Avenant Archer** {2}{W} · Creature — Human Soldier Archer — increment 8
- [x] **Deadfall** {2}{G} · Enchantment — increment 4
- [x] **Demonic Torment** {2}{B} · Enchantment — Aura — increment 95
- [x] **Divine Intervention** {6}{W}{W} · Enchantment — increment 27
- [x] **Dream Coat** {U} · Enchantment — Aura — increment 28 (approximated: only a single colour can
      be chosen, not a set — increment 191)
- [x] **Dwarven Song** {R} · Instant — increment 96, 129
- [x] **Elder Land Wurm** {4}{W}{W}{W} · Creature — Dragon Wurm — increment 5
- [x] **Elder Spawn** {4}{U}{U}{U} · Creature — Spawn — increment 29, 106
- [x] **Elven Riders** {3}{G}{G} · Creature — Elf — faithful as of increment 9
- [x] **Enchanted Being** {1}{W}{W} · Creature — Human — increment 12
- [x] **Enchantment Alteration** {U} · Instant — increment 30
- [x] **Energy Tap** {U} · Sorcery — increment 31
- [x] **Equinox** {W} · Enchantment — Aura — increment 32 (approximated: "would destroy" is predicted
      from the spell's script, so a destroy behind a modal or conditional branch is missed —
      increment 192)
- [x] **Eureka** {2}{G}{G} · Sorcery — increment 33
- [x] **Evil Eye of Orms-by-Gore** {4}{B} · Creature — Eye — faithful as of increments 9 and 10
- [x] **Feint** {R} · Instant — increment 34
- [x] **Field of Dreams** {U} · World Enchantment — faithful as of increment 35
- [x] **Firestorm Phoenix** {4}{R}{R} · Creature — Phoenix — increment 36
- [x] **Flash Counter** {1}{U} · Instant — increment 105
- [x] **Floral Spuzzem** {3}{G} · Creature — Elemental — increment 37
- [x] **Forethought Amulet** {5} · Artifact — increment 38
- [x] **Frost Giant** {3}{R}{R}{R} · Creature — Giant — increment 1, 119
- [x] **Gabriel Angelfire** {3}{G}{G}{W}{W} · Legendary Creature — Angel — increment 1, 39
- [x] **Gaseous Form** {2}{U} · Enchantment — Aura — increment 95
- [x] **Gauntlets of Chaos** {5} · Artifact — increment 40
- [x] **Giant Slug** {1}{B} · Creature — Slug — increment 41
- [x] **Giant Turtle** {1}{G}{G} · Creature — Turtle — increment 42
- [x] **Glyph of Delusion** {U} · Instant — increment 43, 131, 132 (approximated: the two granted
      abilities are modeled by the glyph counter)
- [x] **Glyph of Destruction** {R} · Instant — increment 43, 134
- [x] **Glyph of Doom** {B} · Instant — increment 43
- [x] **Glyph of Life** {W} · Instant — increment 43
- [x] **Glyph of Reincarnation** {G} · Instant — increment 43, 133
- [x] **Gosta Dirk** {3}{W}{W}{U}{U} · Legendary Creature — Human Warrior — increment 4
- [x] **Gravity Sphere** {2}{R} · World Enchantment — increment 2, 44
- [x] **Great Wall** {2}{W} · Enchantment — increment 4
- [x] **Greater Realm of Preservation** {1}{W} · Enchantment — increment 12
- [x] **Green Mana Battery** {4} · Artifact — increment 11
- [x] **Halfdane** {1}{W}{U}{B} · Legendary Creature — Shapeshifter — increment 22
- [x] **Hammerheim** — · Legendary Land — increment 5
- [x] **Hazezon Tamar** {4}{R}{G}{W} · Legendary Creature — Human Warrior — increment 45
- [x] **Heaven's Gate** {W} · Instant — increment 96, 129
- [x] **Hellfire** {2}{B}{B}{B} · Sorcery — increment 46
- [x] **Horn of Deafening** {4} · Artifact — increment 94
- [x] **Hunding Gjornersen** {3}{W}{U}{U} · Legendary Creature — Human Warrior — increment 1, 119
- [x] **Ichneumon Druid** {1}{G}{G} · Creature — Human Druid — increment 101
- [x] **Imprison** {B} · Enchantment — Aura — increment 47
- [x] **In the Eye of Chaos** {2}{U} · World Enchantment — increment 2, 48
- [x] **Indestructible Aura** {W} · Instant — increment 94
- [x] **Infernal Medusa** {3}{B}{B} · Creature — Gorgon — increment 100
- [x] **Infinite Authority** {W}{W}{W} · Enchantment — Aura — increment 49
- [x] **Invoke Prejudice** {U}{U}{U}{U} · Enchantment — increment 48
- [x] **Ivory Guardians** {4}{W}{W} · Creature — Giant Cleric — increment 13
- [x] **Johan** {3}{R}{G}{W} · Legendary Creature — Human Wizard — increment 50
- [x] **Juxtapose** {3}{U} · Sorcery — increment 40; residual carried to increment 124
- [x] **Karakas** — · Legendary Land — increment 6
- [x] **Kismet** {3}{W} · Enchantment — faithful as of increment 51
- [x] **Knowledge Vault** {4} · Artifact — increment 52
- [x] **Kry Shield** {2} · Artifact — increment 94
- [x] **Lady Caleria** {3}{G}{G}{W}{W} · Legendary Creature — Elf Archer — increment 8
- [x] **Lady Evangela** {W}{U}{B} · Legendary Creature — Human Cleric — increment 94
- [x] **Land Equilibrium** {2}{U}{U} · Enchantment — increment 53
- [x] **Land's Edge** {1}{R}{R} · World Enchantment — increment 2, 25
- [x] **Lesser Werewolf** {3}{B} · Creature — Werewolf — increment 8, 54
- [x] **Life Matrix** {4} · Artifact — increment 55
- [x] **Living Plane** {2}{G}{G} · World Enchantment — increment 2, 44
- [x] **Livonya Silone** {2}{R}{R}{G}{G} · Legendary Creature — Human Warrior — increment 6, 7
- [x] **Lord Magnus** {3}{G}{W}{W} · Legendary Creature — Human Druid — increment 4
- [x] **Mana Matrix** {6} · Artifact — increment 105
- [x] **Marble Priest** {5} · Artifact Creature — Cleric — increment 12, 56
- [x] **Marhault Elsdragon** {3}{R}{R}{G} · Legendary Creature — Elf Warrior — increment 1, 119
- [x] **Master of the Hunt** {2}{G}{G} · Creature — Human — increment 123
- [x] **Mirror Universe** {6} · Artifact — faithful as of increment 57
- [x] **Moat** {2}{W}{W} · Enchantment — increment 107
- [x] **Mold Demon** {5}{B}{B} · Creature — Fungus Demon — increment 106
- [x] **Mountain Stronghold** — · Land — increment 3 (slices 1, 2, 3, 4)
- [x] **Nebuchadnezzar** {3}{U}{B} · Legendary Creature — Human Wizard — increment 58
- [x] **Nether Void** {3}{B} · World Enchantment — increment 2, 48
- [x] **Nicol Bolas** {2}{U}{U}{B}{B}{R}{R} · Legendary Creature — Elder Dragon — increment 112
- [x] **North Star** {4} · Artifact — increment 59 (approximated: the relaxation is spent by
      the next spell its controller casts rather than by one they pick, and it widens the five
      colors into each other but not colorless {C} pips)
- [x] **Nova Pentacle** {4} · Artifact — increment 60
- [x] **Osai Vultures** {1}{W} · Creature — Bird — increment 102
- [x] **Part Water** {X}{X}{U} · Sorcery — increment 61
- [x] **Pendelhaven** — · Legendary Land — increment 62
- [x] **Petra Sphinx** {2}{W}{W}{W} · Creature — Sphinx — increment 58
- [x] **Pit Scorpion** {2}{B} · Creature — Scorpion — increment 99; residual carried to increment 118
- [x] **Presence of the Master** {3}{W} · Enchantment — increment 108
- [x] **Primordial Ooze** {R} · Creature — Ooze — increment 63
- [x] **Psionic Entity** {4}{U} · Creature — Illusion — increment 110
- [x] **Psychic Purge** {U} · Sorcery — increment 64
- [x] **Puppet Master** {U}{U}{U} · Enchantment — Aura — increment 65
- [x] **Quagmire** {2}{B} · Enchantment — increment 4
- [x] **Quarum Trench Gnomes** {3}{R} · Creature — Gnome — increment 66
- [x] **Rabid Wombat** {2}{G}{G} · Creature — Wombat — increment 67
- [x] **Radjan Spirit** {3}{G} · Creature — Spirit — increment 5
- [x] **Rapid Fire** {3}{W} · Instant — increment 1, 119
- [x] **Rasputin Dreamweaver** {4}{W}{U} · Legendary Creature — Human Wizard — increment 68
- [x] **Recall** {X}{X}{U} · Sorcery — increment 69
- [x] **Red Mana Battery** {4} · Artifact — increment 11
- [x] **Reincarnation** {1}{G}{G} · Instant — increment 70
- [x] **Relic Bind** {2}{U} · Enchantment — Aura — increment 20
- [x] **Remove Enchantments** {W} · Instant — increment 71
- [x] **Reset** {U}{U} · Instant — faithful as of increment 72
- [x] **Revelation** {G} · World Enchantment — faithful as of increment 35
- [x] **Reverberation** {2}{U}{U} · Instant — increment 19
- [x] **Ring of Immortals** {5} · Artifact — increment 18
- [x] **Rohgahh of Kher Keep** {2}{B}{B}{R}{R} · Legendary Creature — Kobold — increment 73
- [x] **Rust** {G} · Instant — increment 103
- [x] **Sea Kings' Blessing** {U} · Instant — increment 96, 129
- [x] **Seafarer's Quay** — · Land — increment 3 (slices 1, 2, 3, 4)
- [x] **Seeker** {2}{W}{W} · Enchantment — Aura — faithful as of increment 9
- [x] **Sentinel** {4} · Artifact Creature — Shapeshifter — increment 8, 22, 54
- [x] **Serpent Generator** {6} · Artifact — increment 97
- [x] **Shelkin Brownie** {1}{G} · Creature — Ouphe — increment 3, 5
- [x] **Shimian Night Stalker** {3}{B}{B} · Creature — Nightstalker — increment 60
- [x] **Silhouette** {1}{U} · Instant — increment 12, 130
- [x] **Spectral Cloak** {U}{U} · Enchantment — Aura — increment 117
- [x] **Spirit Shackle** {B}{B} · Enchantment — Aura — increment 102
- [x] **Spiritual Sanctuary** {2}{W}{W} · Enchantment — increment 116
- [x] **Stangg** {4}{R}{G} · Legendary Creature — Human Warrior — increment 74
- [x] **Storm World** {R} · World Enchantment — increment 2, 75
- [x] **Subdue** {G} · Instant — increment 94
- [x] **Sword of the Ages** {6} · Artifact — increment 76
- [x] **Sylvan Library** {1}{G} · Enchantment — increment 77
- [x] **Sylvan Paradise** {G} · Instant — increment 96, 129
- [x] **Syphon Soul** {2}{B} · Sorcery — increment 113
- [x] **Takklemaggot** {2}{B}{B} · Enchantment — Aura
- [x] **Telekinesis** {U}{U} · Instant — increment 79
- [x] **Teleport** {U}{U}{U} · Instant — increment 114
- [x] **Tetsuo Umezawa** {U}{B}{R} · Legendary Creature — Human Archer — increment 8, 15
- [x] **The Abyss** {3}{B} · World Enchantment — increment 2, 80
- [x] **The Tabernacle at Pendrell Vale** — · Legendary Land — increment 81
- [x] **The Wretched** {3}{B}{B} · Creature — Demon — increment 104
- [x] **Time Elemental** {2}{U} · Creature — Elemental — increment 82
- [x] **Tolaria** — · Legendary Land — increment 3, 5
- [x] **Tor Wauki** {2}{B}{B}{R} · Legendary Creature — Human Archer — increment 8
- [x] **Touch of Darkness** {B} · Instant — increment 96, 129
- [x] **Transmutation** {1}{B} · Instant — increment 22
- [x] **Triassic Egg** {4} · Artifact — increment 83
- [x] **Undertow** {2}{U} · Enchantment — increment 4
- [x] **Underworld Dreams** {B}{B}{B} · Enchantment — increment 115
- [x] **Unholy Citadel** — · Land — increment 3 (slices 1, 2, 3, 4)
- [x] **Ur-Drago** {3}{U}{U}{B}{B} · Legendary Creature — Elemental — increment 4
- [x] **Urborg** — · Legendary Land — increment 5
- [x] **Vampire Bats** {B} · Creature — Bat — faithful as of increment 84
- [x] **Venarian Gold** {X}{U}{U} · Enchantment — Aura — increment 26
- [x] **Visions** {W} · Sorcery — increment 85, 150
- [x] **Voodoo Doll** {6} · Artifact — increment 86
- [x] **Wall of Caltrops** {1}{W} · Creature — Wall — increment 49, 87
- [x] **Wall of Dust** {2}{R} · Creature — Wall — increment 42
- [x] **Wall of Putrid Flesh** {2}{B} · Creature — Wall — increment 12
- [x] **Wall of Shadows** {1}{B}{B} · Creature — Wall — increment 12, 88
- [x] **Wall of Tombstones** {1}{B} · Creature — Wall — increment 22
- [x] **Wall of Vapor** {3}{U} · Creature — Wall — increment 12
- [x] **Wall of Wonder** {2}{U}{U} · Creature — Wall — increment 89
- [x] **Whirling Dervish** {G}{G} · Creature — Human Monk — increment 90
- [x] **White Mana Battery** {4} · Artifact — increment 11
- [x] **Willow Satyr** {2}{G}{G} · Creature — Satyr — increment 6
- [x] **Winds of Change** {R} · Sorcery — faithful as of increment 91
- [x] **Winter Blast** {X}{G} · Sorcery — increment 61, 92
- [x] **Wolverine Pack** {2}{G}{G} · Creature — Wolverine — increment 1, 119
- [x] **Wood Elemental** {3}{G} · Creature — Elemental — increment 93

## Out of scope

Flagged, not forced. These are not increments — they are card mechanics this game
deliberately does not model.

- [ ] **Falling Star** {2}{R} · Sorcery — physical dexterity (CR 713, the Chaos Orb family) — no digital analogue
- [ ] **Rebirth** {3}{G}{G}{G} · Sorcery — ante (CR 407) — not supported
- [ ] **Tempest Efreet** {1}{R}{R}{R} · Creature — Efreet — ante (CR 407) — not supported
