# Fidelity report — Silverquill Influence (Secrets of Strixhaven)

Source: `docs/decklists/silverquill_influence.md` (official Wizards `soc` precon list, cross-checked
against live pool files and the current engine trigger/combat paths). Commander:
**Killian, Decisive Mentor**. Backlog:
[silverquill-influence-increments.md](silverquill-influence-increments.md).

Intake counts: 56 faithful / 0 approximated / 0 expressible / 28 needing engine work.
The classifier returned 84 A / 0 B / 0 missing, but the Silverquill re-audit demoted 28 cards to
D once the deck's real observers were considered: `Animate Dead` makes controller-vs-owner trigger
bugs observable, the goad package falsifies the current goad-plus-tax shortcut, and two Aura-move
helpers plus Darksteel Mutation still sit on real rules gaps.

## A. In pool, faithful at intake (56)

- [ ] Angelic Destiny
- [ ] Anguished Unmaking
- [ ] Animate Dead
- [ ] Arcane Lighthouse
- [ ] Arcane Signet
- [ ] Armored Skyhunter
- [ ] Bojuka Bog
- [ ] Caves of Koilos
- [ ] Chains of Custody
- [ ] Changing Loyalty
- [ ] Command Tower
- [ ] Desolate Mire
- [ ] Eclipsed Steppe
- [ ] Eidolon of Countless Battles
- [ ] Eldrazi Conscription
- [ ] Exotic Orchard
- [ ] Fabled Passage
- [ ] Fallen Ideal
- [ ] Fellwar Stone
- [ ] Fetid Heath
- [ ] Flickering Ward
- [ ] Forum Filibuster
- [ ] Forum of Amity
- [ ] Fracture
- [ ] Inkshield
- [ ] Intermediate Chirography
- [ ] Isolated Chapel
- [ ] Killian, Ink Duelist
- [ ] Land Tax
- [ ] Path of Ancestry
- [ ] Promise of Loyalty
- [ ] Raffine's Guidance
- [ ] Sage's Reverie
- [ ] Screams from Within
- [ ] Secret Rendezvous
- [ ] Sentinel's Eyes
- [ ] Shadrix Silverquill
- [ ] Sheltered by Ghosts
- [ ] Shielded by Faith
- [ ] Shineshadow Snarl
- [ ] Silverquill Campus
- [ ] Sol Ring
- [ ] Songbirds' Blessing
- [ ] Spirit Mantle
- [ ] Study Hall
- [ ] Sunlit Marsh
- [ ] Talisman of Hierarchy
- [ ] Temple of Silence
- [ ] Terramorphic Expanse
- [ ] Tomik, Wielder of Law
- [ ] Transcendent Envoy
- [ ] Turbulent Moor
- [ ] Umbral Expanse
- [ ] Vanishing Verse
- [ ] War Room
- [ ] Winds of Rath

Notes on cards deliberately kept in A despite intake `ponytail:` / unusual-risk prompts:

- **Flickering Ward** — the current Aura-legality posture already makes its CR 702.16e rider a
  non-issue for this deck; the warning is explanatory, not a live residual.
- **Promise of Loyalty** — its old "planeswalker defenders unmodeled" ponytail is stale now that
  combat records attacks on planeswalkers against their controller; the printed rider is already
  represented.
- **Shineshadow Snarl** — automatic reveal/no-reveal collapse is behaviorally exact here: revealing
  is strictly upside and no Silverquill card punishes showing the land.
- **Winds of Rath** — the printed "can't be regenerated" text is still a no-op only because no
  Silverquill card or pool card in this interaction space grants regeneration.

## B. In pool, approximated at intake (0)

None.

Every live Silverquill gap that survived the re-audit is now large enough to need engine work, so
the residual cards move straight to D instead of sitting in B with a papered-over note.

## C. New, expressible today (0)

None. All 84 Silverquill nonbasics are already in the pool.

## D. In pool, not yet faithful; needs engine work (28)

These cards are scripted, but Silverquill makes their remaining engine gaps observable. Each card
points at the increment(s) that clear it.

- [x] Ajani's Chosen — #1, #3 (both landed)
- [x] Archon of Sun's Grace — #1 (landed)
- [x] Breena, the Demagogue — #1 (landed)
- [ ] Coercive Impetus — #2
- [x] Combat Calligrapher — #1 (landed)
- [x] Darksteel Mutation — #4 (landed)
- [x] Defacing Duskmage — #1, #6 (both landed)
- [x] Doomwake Giant — #1 (landed)
- [x] Eiganjo Dynastorian — #1 (landed)
- [x] Eriette of the Charmed Apple — #1 (landed)
- [x] Firemane Commando — #1 (landed)
- [ ] Ghostly Prison — #2
- [x] Gift of Immortality — #3
- [ ] Ghoulish Impetus — #2
- [x] Hateful Eidolon — #1 (landed)
- [ ] Herald of Amity — #1 landed; still blocked on #5
- [x] Keen Duelist — #1 (landed)
- [ ] Killian, Decisive Mentor — #1 landed; still blocked on #2
- [x] Kor Spiritdancer — #1 (landed)
- [x] Mangara, the Diplomat — #1 (landed)
- [ ] Martial Impetus — #2
- [ ] Nils, Discipline Enforcer — #1 landed; still blocked on #2
- [ ] Parasitic Impetus — #2
- [x] Pearl-Ear, Imperial Advisor — #1 (landed)
- [ ] Redemption Arc — #2
- [x] Scriv, the Obligator — #1 (landed)
- [x] Sram, Senior Edificer — #1 (landed)
- [x] Starfield Mystic — #1 (landed)

## Observability re-audit

### 1. `Animate Dead` falsifies owner-based battlefield trigger scans

Silverquill is the first deck here whose own in-pool card (`Animate Dead`) routinely puts
opponent-owned permanents under your control, and many trigger families still compute "you" /
"opponent" and trigger ownership from `owner_of(id)` instead of `controller_of(id)`. That breaks
controller-scoped triggers on reanimated opponent-owned Silverquill permanents — the deck's own
watchers are enough to make this visible.

Affected Silverquill cards: Ajani's Chosen, Archon of Sun's Grace, Breena, Combat Calligrapher,
Defacing Duskmage, Doomwake Giant, Eiganjo Dynastorian, Eriette of the Charmed Apple, Firemane
Commando, Hateful Eidolon, Herald of Amity, Keen Duelist, Killian, Decisive Mentor, Kor
Spiritdancer, Mangara, Nils, Pearl-Ear, Scriv, Sram, Starfield Mystic. This is increment #1.

Checked and deliberately kept A: `Animate Dead` and `Changing Loyalty` are the observers / enablers
that make the controller bug visible on the reanimated creature; their own core reanimation text is
not the broken seam.

### 2. Silverquill falsifies the current goad-plus-tax shortcut

`combat.rs` still carries a note that no pool card exercises "goad + an unpayable tax" at once.
Silverquill falsifies that directly: the deck combines `Ghostly Prison` and `Nils, Discipline
Enforcer` with `Killian, Decisive Mentor`, all four Impetus Auras, and `Redemption Arc`. Today the
engine auto-pays taxes and still forces a goaded creature to attack even when the controller cannot
actually pay, which is wrong for the printed "if able" rider. This is increment #2.

### 3. The old "no pool Aura re-attaches" claim is stale; the live gap is narrower

Silverquill already ships several Auras that move without being cast: `Ajani's Chosen`, `Gift of
Immortality`, `Ghoulish Impetus`, `Screams from Within`, and `Shielded by Faith`. The broad
`types/card.rs` claim is therefore stale. `Shielded by Faith`'s main move path already re-checks
legality correctly, so the surviving problem is narrower: Ajani's Chosen's token-retarget helper
and Gift's delayed self-return still attach without a fresh legality gate. That narrower bug is
increment #3.

### 4. `Promise of Loyalty`'s ponytail rationale is stale

The card file still says the "or planeswalkers you control" clause is unobservable because every
attack target is a player. That is no longer true of the engine: attacks on planeswalkers are
resolved against that planeswalker's controller in combat, which already matches how Promise's
restriction is checked. The note should be cleaned up later, but there is no remaining Silverquill
fidelity gap here, so the card stays in A.

### 5. `Herald of Amity`'s ponytail is half stale

The file's "random order" half is no longer a gap — the bottomed pile is already randomized through
the injected operation RNG. The live residual is only the cast timing: the chosen Aura is granted a
later free-cast permission this turn, rather than being cast during Herald's ETB resolution. That
surviving issue is increment #5.

### 6. Darksteel Mutation's decklist warning is real

Silverquill's own `Doomwake Giant` proves the current type layer is still incomplete for Darksteel
Mutation. The engine unions added card types onto the printed line and replaces creature subtypes,
but it does not yet express "loses all other card types" — so mutating an enchantment creature
leaves it an enchantment creature artifact instead of exactly an artifact creature Insect. That is
increment #4.
