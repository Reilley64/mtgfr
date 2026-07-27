# Unlimited Edition (`2ed`) — fidelity report

292 unique cards. Source of truth: Scryfall `set:2ed unique:cards`.
Engine backlog for section D: [`2ed-increments.md`](2ed-increments.md).

| Section | Meaning | Count |
| --- | --- | ---: |
| A | In the pool, faithful | 28 |
| B | In the pool, approximated | 0 |
| C | New, expressible with today's DSL | 115 |
| D | New, needs engine work | 145 |
| — | Out of scope (flag-don't-force) | 4 |

## A. In the pool, faithful

Already scripted with no `approximates` field. Re-frame-audited as part of this grind.

- [x] **Ancestral Recall** {U} · Instant
- [x] **Animate Dead** {1}{B} · Enchantment — Aura
- [x] **Birds of Paradise** {G} · Creature — Bird
- [x] **Braingeyser** {X}{U}{U} · Sorcery
- [x] **Channel** {G}{G} · Sorcery
- [x] **Dark Ritual** {B} · Instant
- [x] **Dragon Whelp** {2}{R}{R} · Creature — Dragon
- [x] **Earthquake** {X}{R} · Sorcery
- [x] **Forest** — · Basic Land — Forest
- [x] **Howling Mine** {2} · Artifact
- [x] **Illusionary Mask** {2} · Artifact
- [x] **Island** — · Basic Land — Island
- [x] **Lightning Bolt** {R} · Instant
- [x] **Llanowar Elves** {G} · Creature — Elf Druid
- [x] **Mountain** — · Basic Land — Mountain
- [x] **Plains** — · Basic Land — Plains
- [x] **Raise Dead** {B} · Sorcery
- [x] **Resurrection** {2}{W}{W} · Sorcery
- [x] **Savannah Lions** {W} · Creature — Cat
- [x] **Serra Angel** {3}{W}{W} · Creature — Angel
- [x] **Sol Ring** {1} · Artifact
- [x] **Swamp** — · Basic Land — Swamp
- [x] **Swords to Plowshares** {W} · Instant
- [x] **Terror** {1}{B} · Instant
- [x] **Unsummon** {U} · Instant
- [x] **Wheel of Fortune** {2}{R} · Sorcery
- [x] **White Knight** {W}{W} · Creature — Human Knight
- [x] **Grizzly Bears** {1}{G} · Creature — Bear — renamed from the misprinted key `"Grizzly Bear"` in this grind

## B. In the pool, approximated

Empty — no 2ed card already in the pool carries an `approximates` field.

## C. New, expressible with today's DSL

Authorable in Phase 3 with no engine change. TDD each one.

- [x] **Air Elemental** {3}{U}{U} · Creature — Elemental
- [x] **Armageddon** {3}{W} · Sorcery
- [x] **Bad Moon** {1}{B} · Enchantment
- [x] **Badlands** — · Land — Swamp Mountain
- [x] **Bayou** — · Land — Swamp Forest
- [x] **Black Knight** {B}{B} · Creature — Human Knight
- [x] **Black Lotus** {0} · Artifact
- [x] **Black Ward** {W} · Enchantment — Aura
- [x] **Blessing** {W}{W} · Enchantment — Aura
- [x] **Blue Ward** {W} · Enchantment — Aura
- [x] **Celestial Prism** {3} · Artifact
- [x] **Control Magic** {2}{U}{U} · Enchantment — Aura
- [x] **Counterspell** {U}{U} · Instant
- [x] **Craw Wurm** {4}{G}{G} · Creature — Wurm
- [x] **Crusade** {W}{W} · Enchantment
- [x] **Death Ward** {W} · Instant
- [x] **Demonic Tutor** {1}{B} · Sorcery
- [x] **Disenchant** {1}{W} · Instant
- [x] **Drudge Skeletons** {1}{B} · Creature — Skeleton
- [x] **Dwarven Demolition Team** {2}{R} · Creature — Dwarf
- [x] **Dwarven Warriors** {2}{R} · Creature — Dwarf Warrior
- [x] **Earth Elemental** {3}{R}{R} · Creature — Elemental
- [x] **Elvish Archers** {1}{G} · Creature — Elf Archer
- [x] **Fear** {B}{B} · Enchantment — Aura
- [x] **Fire Elemental** {3}{R}{R} · Creature — Elemental
- [x] **Firebreathing** {R} · Enchantment — Aura
- [x] **Flight** {U} · Enchantment — Aura
- [x] **Fog** {G} · Instant
- [x] **Frozen Shade** {2}{B} · Creature — Shade
- [x] **Giant Growth** {G} · Instant
- [x] **Giant Spider** {3}{G} · Creature — Spider
- [x] **Goblin Balloon Brigade** {R} · Creature — Goblin Warrior
- [x] **Granite Gargoyle** {2}{R} · Creature — Gargoyle
- [x] **Gray Ogre** {2}{R} · Creature — Ogre
- [x] **Green Ward** {W} · Enchantment — Aura
- [x] **Hill Giant** {3}{R} · Creature — Giant
- [x] **Holy Armor** {W} · Enchantment — Aura
- [x] **Holy Strength** {W} · Enchantment — Aura
- [x] **Howl from Beyond** {X}{B} · Instant
- [x] **Hurloon Minotaur** {1}{R}{R} · Creature — Minotaur
- [x] **Hurricane** {X}{G} · Sorcery
- [x] **Ice Storm** {2}{G} · Sorcery
- [x] **Icy Manipulator** {4} · Artifact
- [x] **Ironroot Treefolk** {4}{G} · Creature — Treefolk
- [x] **Jayemdae Tome** {4} · Artifact — Book
- [x] **Jump** {U} · Instant
- [x] **Lance** {W} · Enchantment — Aura
- [x] **Ley Druid** {2}{G} · Creature — Human Druid
- [x] **Living Wall** {4} · Artifact Creature — Wall
- [x] **Mahamoti Djinn** {4}{U}{U} · Creature — Djinn
- [x] **Merfolk of the Pearl Trident** {U} · Creature — Merfolk
- [x] **Mons's Goblin Raiders** {R} · Creature — Goblin
- [x] **Mox Emerald** {0} · Artifact
- [x] **Mox Jet** {0} · Artifact
- [x] **Mox Pearl** {0} · Artifact
- [x] **Mox Ruby** {0} · Artifact
- [x] **Mox Sapphire** {0} · Artifact
- [x] **Nevinyrral's Disk** {4} · Artifact
- [x] **Northern Paladin** {2}{W}{W} · Creature — Human Knight
- [x] **Obsianus Golem** {6} · Artifact Creature — Golem
- [x] **Orcish Artillery** {1}{R}{R} · Creature — Orc Warrior
- [x] **Orcish Oriflamme** {3}{R} · Enchantment
- [x] **Pearled Unicorn** {2}{W} · Creature — Unicorn
- [x] **Pestilence** {2}{B}{B} · Enchantment
- [x] **Phantasmal Forces** {3}{U} · Creature — Illusion
- [x] **Phantom Monster** {3}{U} · Creature — Illusion
- [x] **Plateau** — · Land — Mountain Plains
- [x] **Prodigal Sorcerer** {2}{U} · Creature — Human Wizard Sorcerer
- [x] **Psionic Blast** {2}{U} · Instant
- [x] **Red Ward** {W} · Enchantment — Aura
- [x] **Regeneration** {1}{G} · Enchantment — Aura
- [x] **Regrowth** {1}{G} · Sorcery
- [x] **Roc of Kher Ridges** {3}{R} · Creature — Bird
- [x] **Rod of Ruin** {4} · Artifact
- [x] **Royal Assassin** {1}{B}{B} · Creature — Human Assassin
- [x] **Savannah** — · Land — Forest Plains
- [x] **Scathe Zombies** {2}{B} · Creature — Zombie
- [x] **Scrubland** — · Land — Plains Swamp
- [x] **Scryb Sprites** {G} · Creature — Faerie
- [x] **Sedge Troll** {2}{R} · Creature — Troll
- [x] **Sengir Vampire** {3}{B}{B} · Creature — Vampire
- [x] **Shatter** {1}{R} · Instant
- [x] **Shivan Dragon** {4}{R}{R} · Creature — Dragon
- [x] **Sinkhole** {B}{B} · Sorcery
- [x] **Steal Artifact** {2}{U}{U} · Enchantment — Aura
- [x] **Stone Rain** {2}{R} · Sorcery
- [x] **Stream of Life** {X}{G} · Sorcery
- [x] **Taiga** — · Land — Mountain Forest
- [x] **The Hive** {5} · Artifact
- [x] **Tranquility** {2}{G} · Sorcery
- [x] **Tropical Island** — · Land — Forest Island
- [x] **Tundra** — · Land — Plains Island
- [x] **Tunnel** {R} · Instant
- [x] **Twiddle** {U} · Instant
- [x] **Underground Sea** — · Land — Island Swamp
- [x] **Unholy Strength** {B} · Enchantment — Aura
- [x] **Uthden Troll** {2}{R} · Creature — Troll
- [x] **Verduran Enchantress** {1}{G}{G} · Creature — Human Druid
- [x] **Volcanic Island** — · Land — Island Mountain
- [x] **Wall of Air** {1}{U}{U} · Creature — Wall
- [x] **Wall of Bone** {2}{B} · Creature — Skeleton Wall
- [x] **Wall of Brambles** {2}{G} · Creature — Plant Wall
- [x] **Wall of Fire** {1}{R}{R} · Creature — Wall
- [x] **Wall of Ice** {2}{G} · Creature — Wall
- [x] **Wall of Stone** {1}{R}{R} · Creature — Wall
- [x] **Wall of Swords** {3}{W} · Creature — Wall
- [x] **Wall of Water** {1}{U}{U} · Creature — Wall
- [x] **Wall of Wood** {G} · Creature — Wall
- [x] **War Mammoth** {3}{G} · Creature — Elephant
- [x] **Water Elemental** {3}{U}{U} · Creature — Elemental
- [x] **Weakness** {B} · Enchantment — Aura
- [x] **Web** {G} · Enchantment — Aura
- [x] **White Ward** {W} · Enchantment — Aura
- [x] **Will-o'-the-Wisp** {B} · Creature — Spirit
- [x] **Wrath of God** {2}{W}{W} · Sorcery

## D. New, needs engine work

Blocked on the numbered increments in [`2ed-increments.md`](2ed-increments.md).

- [ ] **Animate Artifact** {3}{U} · Enchantment — Aura — increment 2
- [ ] **Animate Wall** {W} · Enchantment — Aura — increment 24
- [ ] **Ankh of Mishra** {2} · Artifact — increment 58
- [ ] **Aspect of Wolf** {1}{G} · Enchantment — Aura — increment 2, 25
- [ ] **Balance** {1}{W} · Sorcery — increment 43
- [ ] **Basalt Monolith** {3} · Artifact — increment 7
- [ ] **Benalish Hero** {W} · Creature — Human Soldier — increment 14
- [ ] **Berserk** {G} · Instant — increment 45
- [ ] **Black Vise** {1} · Artifact — increment 25
- [ ] **Blaze of Glory** {W} · Instant — increment 11
- [x] **Blue Elemental Blast** {U} · Instant — increment 9
- [x] **Bog Wraith** {3}{B} · Creature — Wraith — increment 3
- [x] **Burrowing** {R} · Enchantment — Aura — increment 3
- [ ] **Camouflage** {G} · Instant — increment 48
- [ ] **Castle** {3}{W} · Enchantment — increment 40
- [ ] **Chaoslace** {R} · Instant — increment 15
- [ ] **Circle of Protection: Black** {1}{W} · Enchantment — increment 4, 5
- [ ] **Circle of Protection: Blue** {1}{W} · Enchantment — increment 4, 5
- [ ] **Circle of Protection: Green** {1}{W} · Enchantment — increment 4, 5
- [ ] **Circle of Protection: Red** {1}{W} · Enchantment — increment 4, 5
- [ ] **Circle of Protection: White** {1}{W} · Enchantment — increment 4, 5
- [ ] **Clockwork Beast** {6} · Artifact Creature — Beast — increment 28
- [ ] **Clone** {3}{U} · Creature — Shapeshifter — increment 12
- [ ] **Cockatrice** {3}{G}{G} · Creature — Cockatrice — increment 21
- [ ] **Consecrate Land** {W} · Enchantment — Aura — increment 35
- [ ] **Conservator** {4} · Artifact — increment 4
- [ ] **Conversion** {2}{W}{W} · Enchantment — increment 8
- [ ] **Copper Tablet** {2} · Artifact — increment 60
- [ ] **Copy Artifact** {1}{U} · Enchantment — increment 12
- [ ] **Creature Bond** {1}{U} · Enchantment — Aura — increment 62
- [ ] **Crystal Rod** {1} · Artifact — increment 10
- [ ] **Cursed Land** {2}{B}{B} · Enchantment — Aura — increment 61
- [ ] **Cyclopean Tomb** {4} · Artifact — increment 8, 28
- [x] **Deathgrip** {B}{B} · Enchantment — increment 9
- [ ] **Deathlace** {B} · Instant — increment 15
- [ ] **Demonic Hordes** {3}{B}{B}{B} · Creature — Demon — increment 20, 41
- [ ] **Dingus Egg** {4} · Artifact — increment 59
- [ ] **Disintegrate** {X}{R} · Sorcery — increment 34
- [ ] **Disrupting Scepter** {3} · Artifact — increment 56
- [ ] **Drain Life** {X}{1}{B} · Sorcery — increment 54
- [ ] **Drain Power** {U}{U} · Sorcery — increment 23, 49
- [ ] **Earthbind** {R} · Enchantment — Aura — increment 44
- [ ] **Evil Presence** {B} · Enchantment — Aura — increment 8
- [ ] **False Orders** {R} · Instant — increment 11
- [ ] **Farmstead** {W}{W}{W} · Enchantment — Aura — increment 36
- [ ] **Fastbond** {G} · Enchantment — increment 29
- [ ] **Feedback** {2}{U} · Enchantment — Aura — increment 61
- [ ] **Fireball** {X}{R} · Sorcery — increment 53
- [ ] **Flashfires** {3}{R} · Sorcery — increment 51
- [ ] **Force of Nature** {2}{G}{G}{G}{G} · Creature — Elemental — increment 20
- [ ] **Forcefield** {3} · Artifact — increment 4
- [ ] **Fork** {R}{R} · Instant — increment 13
- [ ] **Fungusaur** {3}{G} · Creature — Fungus Dinosaur — increment 63
- [ ] **Gaea's Liege** {3}{G}{G}{G} · Creature — Avatar — increment 1, 2, 8
- [ ] **Gauntlet of Might** {4} · Artifact — increment 19
- [ ] **Glasses of Urza** {1} · Artifact — increment 31
- [ ] **Gloom** {2}{B} · Enchantment — increment 67
- [x] **Goblin King** {1}{R}{R} · Creature — Goblin — increment 3
- [ ] **Guardian Angel** {X}{W} · Instant — increment 4
- [ ] **Healing Salve** {W} · Instant — increment 4
- [ ] **Helm of Chatzuk** {1} · Artifact — increment 14
- [ ] **Hypnotic Specter** {1}{B}{B} · Creature — Specter — increment 17
- [ ] **Instill Energy** {G} · Enchantment — Aura — increment 7
- [ ] **Invisibility** {U}{U} · Enchantment — Aura — increment 11
- [ ] **Iron Star** {1} · Artifact — increment 10
- [ ] **Ironclaw Orcs** {1}{R} · Creature — Orc — increment 11
- [ ] **Island Sanctuary** {1}{W} · Enchantment — increment 65
- [ ] **Ivory Cup** {1} · Artifact — increment 10
- [ ] **Jade Monolith** {4} · Artifact — increment 6
- [ ] **Jade Statue** {4} · Artifact — increment 57
- [ ] **Juggernaut** {4} · Artifact Creature — Juggernaut — increment 11
- [ ] **Karma** {2}{W}{W} · Enchantment — increment 1
- [ ] **Keldon Warlord** {2}{R}{R} · Creature — Human Barbarian — increment 1, 2
- [ ] **Kormus Bell** {4} · Artifact — increment 8
- [ ] **Kudzu** {1}{G}{G} · Enchantment — Aura — increment 37
- [ ] **Library of Leng** {1} · Artifact — increment 33
- [ ] **Lich** {B}{B}{B}{B} · Enchantment — increment 22, 47
- [x] **Lifeforce** {G}{G} · Enchantment — increment 9
- [ ] **Lifelace** {G} · Instant — increment 15
- [ ] **Lifetap** {U}{U} · Enchantment — increment 19
- [ ] **Living Artifact** {G} · Enchantment — Aura — increment 22, 28
- [ ] **Living Lands** {3}{G} · Enchantment — increment 8
- [x] **Lord of Atlantis** {U}{U} · Creature — Merfolk — increment 3
- [ ] **Lord of the Pit** {4}{B}{B}{B} · Creature — Demon — increment 20
- [ ] **Lure** {1}{G}{G} · Enchantment — Aura — increment 11
- [ ] **Magical Hack** {U} · Instant — increment 16
- [ ] **Mana Flare** {2}{R} · Enchantment — increment 19
- [ ] **Mana Short** {2}{U} · Instant — increment 23
- [ ] **Mana Vault** {1} · Artifact — increment 7
- [ ] **Manabarbs** {3}{R} · Enchantment — increment 19
- [ ] **Meekstone** {1} · Artifact — increment 7
- [ ] **Mesa Pegasus** {1}{W} · Creature — Pegasus — increment 14
- [ ] **Mind Twist** {X}{B} · Sorcery — increment 17
- [ ] **Natural Selection** {G} · Instant — increment 55
- [ ] **Nether Shadow** {B}{B} · Creature — Spirit — increment 39
- [ ] **Nettling Imp** {2}{B} · Creature — Imp — increment 26
- [ ] **Nightmare** {5}{B} · Creature — Nightmare Horse — increment 1, 2
- [ ] **Paralyze** {B} · Enchantment — Aura — increment 7
- [ ] **Personal Incarnation** {3}{W}{W}{W} · Creature — Avatar Incarnation — increment 6
- [ ] **Phantasmal Terrain** {U}{U} · Enchantment — Aura — increment 8
- [ ] **Pirate Ship** {4}{U} · Creature — Human Pirate — increment 24
- [ ] **Plague Rats** {2}{B} · Creature — Rat — increment 1, 2
- [ ] **Power Leak** {1}{U} · Enchantment — Aura — increment 4
- [ ] **Power Sink** {X}{U} · Instant — increment 23
- [ ] **Power Surge** {R}{R} · Enchantment — increment 1
- [ ] **Psychic Venom** {1}{U} · Enchantment — Aura — increment 19
- [ ] **Purelace** {W} · Instant — increment 15
- [ ] **Raging River** {R}{R} · Enchantment — increment 48
- [x] **Red Elemental Blast** {R} · Instant — increment 9
- [ ] **Reverse Damage** {1}{W}{W} · Instant — increment 4, 5
- [ ] **Righteousness** {W} · Instant — increment 52
- [ ] **Rock Hydra** {X}{R}{R} · Creature — Hydra — increment 4, 28
- [ ] **Sacrifice** {B} · Instant — increment 46
- [ ] **Samite Healer** {1}{W} · Creature — Human Cleric — increment 4
- [ ] **Scavenging Ghoul** {3}{B} · Creature — Zombie — increment 28
- [ ] **Sea Serpent** {5}{U} · Creature — Serpent — increment 24
- [x] **Shanodin Dryads** {G} · Creature — Nymph Dryad — increment 3
- [ ] **Simulacrum** {1}{B} · Instant — increment 22
- [ ] **Siren's Call** {U} · Instant — increment 26
- [ ] **Sleight of Mind** {U} · Instant — increment 16
- [ ] **Smoke** {R}{R} · Enchantment — increment 7
- [ ] **Soul Net** {1} · Artifact — increment 10
- [ ] **Spell Blast** {X}{U} · Instant — increment 30
- [ ] **Stasis** {1}{U} · Enchantment — increment 7
- [ ] **Stone Giant** {2}{R}{R} · Creature — Giant — increment 42
- [ ] **Sunglasses of Urza** {3} · Artifact — increment 32
- [ ] **Thicket Basilisk** {3}{G}{G} · Creature — Basilisk — increment 21
- [ ] **Thoughtlace** {U} · Instant — increment 15
- [ ] **Throne of Bone** {1} · Artifact — increment 10
- [ ] **Timber Wolves** {G} · Creature — Wolf — increment 14
- [ ] **Time Vault** {2} · Artifact — increment 7, 18
- [ ] **Time Walk** {1}{U} · Sorcery — increment 18
- [ ] **Timetwister** {2}{U} · Sorcery — increment 38
- [ ] **Tsunami** {3}{G} · Sorcery — increment 51
- [ ] **Two-Headed Giant of Foriys** {4}{R} · Creature — Giant — increment 11
- [ ] **Vesuvan Doppelganger** {3}{U}{U} · Creature — Shapeshifter — increment 12
- [ ] **Veteran Bodyguard** {3}{W}{W} · Creature — Human — increment 6
- [ ] **Volcanic Eruption** {X}{U}{U}{U} · Sorcery — increment 1
- [ ] **Wanderlust** {2}{G} · Enchantment — Aura — increment 61
- [ ] **Warp Artifact** {B}{B} · Enchantment — Aura — increment 61
- [ ] **Wild Growth** {G} · Enchantment — Aura — increment 64
- [ ] **Winter Orb** {2} · Artifact — increment 7
- [ ] **Wooden Sphere** {1} · Artifact — increment 10
- [ ] **Word of Command** {B}{B} · Instant — increment 49
- [ ] **Zombie Master** {1}{B}{B} · Creature — Zombie — increment 66

## Out of scope

Flagged, not forced. These are not increments — they are card mechanics this game
deliberately does not model.

- [ ] **Chaos Orb** {2} · Artifact — physical dexterity (CR 713) — no digital analogue
- [ ] **Contract from Below** {B} · Sorcery — ante (CR 407) — not supported
- [ ] **Darkpact** {B}{B}{B} · Sorcery — ante (CR 407) — not supported
- [ ] **Demonic Attorney** {1}{B}{B} · Sorcery — ante (CR 407) — not supported

