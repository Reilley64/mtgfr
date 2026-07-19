# Fidelity report — Political Puppets (Commander 2011)

Source: https://archidekt.com/decks/2209176 (Archidekt deck 2209176, fetched 2026-07-18).
72 unique non-basic cards + basics. Commander: **Zedruu the Greathearted**. Backlog
increments: #203–#231 ([political-puppets-increments.md](political-puppets-increments.md)).

Intake counts: 12 faithful / 0 approximated / 18 expressible / 42 needing engine work.
(The classification pass marked 21 expressible; the observability re-audit demoted 3 of
those to D — see "C→D demotions" below.)

**Final state (2026-07-19): 72/72 cards in the pool — 68 fully faithful, 4 with the one
standing pool-wide planeswalker residual (Soul Snare and the three Vows, noted inline
below and in each card's `approximates`).**

## A. In pool, faithful at intake (12)

- [x] Azorius Chancery
- [x] Azorius Guildmage
- [x] Chaos Warp
- [x] Command Tower
- [x] Court Hussar
- [x] Fellwar Stone
- [x] Flametongue Kavu
- [x] Ghostly Prison
- [x] Lightning Greaves
- [x] Prison Term
- [x] Sol Ring
- [x] Terramorphic Expanse

## B. In pool, approximated at intake (0)

None — every already-in-pool card is fully faithful.

## C. New, expressible today (18)

Authorable in the pure-authoring pass, no engine change needed. Cards marked (†#N) have a
pending correctness increment that adds regression coverage around them — the DSL expresses
the card today; the increment fixes an engine-global gap their presence makes observable.

- [x] Armillary Sphere
- [x] Boros Garrison
- [x] Darksteel Ingot
- [x] Dreamstone Hedron
- [x] Evolving Wilds
- [x] False Prophet
- [x] Izzet Boilerworks
- [x] Izzet Chronarch (†#213 — can retrieve a countered copy that should have ceased to exist)
- [x] Journey to Nowhere (†#228 — "return under its owner's control" must stay owner-routed
  once donated/exchanged permanents exist)
- [x] Perilous Research
- [x] Plumeveil
- [x] Propaganda (checked against `combat.rs:420` — the generic-{2} attack-tax auto-pay claim
  holds; Ghostly Prison's exact script)
- [x] Prophetic Prism
- [x] Repulse
- [x] Vision Skeins
- [x] Wall of Denial
- [x] Wall of Omens
- [x] Windborn Muse

Notes: Dominus of Fealty's steal sequence and Flusterstorm's storm are landed surface
(Besmirch / Reaping the Graves precedents) but both cards were demoted to D by the re-audit
— see below. Searches reveal nothing in this model (tracked Land Tax idiom), matching
Armillary Sphere's printed reveal.

## D. New, needs engine work (42)

- [x] Arbiter of Knollridge → #222
- [x] Austere Command → #205
- [x] Brainstorm → #216
- [x] Breath of Darigaaz → #217
- [x] Brion Stoutarm → #212 (demoted from C)
- [x] Champion's Helm → #204
- [x] Chromeshell Crab → #228 (morph itself is landed, #163)
- [x] Crescendo of War → #206
- [x] Death by Dragons → #211
- [x] Dominus of Fealty → #228 (demoted from C)
- [x] Flusterstorm → #213 (demoted from C)
- [x] Fog Bank → #220
- [x] Goblin Cadets → #221 (blocks trigger), #228 (opponent-gains-control payoff)
- [x] Gomazoa → #221
- [x] Guard Gomazoa → #220
- [x] Howling Mine → #219
- [x] Insurrection → #229
- [x] Jötun Grunt → #225
- [x] Lash Out → #230
- [x] Martyr's Bond → #226
- [x] Murmurs from Beyond → #210
- [x] Nin, the Pain Artist → #223
- [x] Numot, the Devastator → #218
- [x] Oblation → #215
- [x] Pollen Lullaby → #230
- [x] Punishing Fire → #208
- [x] Rapacious One → #209
- [x] Reins of Power → #228
- [x] Ruhan of the Fomori → #207
- [x] Scattering Stroke → #230
- [x] Skyscribing → #224
- [x] Soul Snare → #205 (residual: "or a planeswalker you control" unmodeled — the standing
  pool-wide no-planeswalker-permanent limitation, flagged in its `approximates`)
- [x] Spell Crumple → #214
- [x] Spurnmage Advocate → #218
- [x] Trade Secrets → #231
- [x] Vedalken Plotter → #228
- [x] Vow of Duty → #203 (residual: "or planeswalkers you control" unmodeled — same standing
  pool-wide limitation, flagged in its `approximates`)
- [x] Vow of Flight → #203 (residual: "or planeswalkers you control" unmodeled — same standing
  pool-wide limitation, flagged in its `approximates`)
- [x] Vow of Lightning → #203 (residual: "or planeswalkers you control" unmodeled — same standing
  pool-wide limitation, flagged in its `approximates`)
- [x] Whirlpool Whelm → #230
- [x] Wild Ricochet → #227
- [x] Zedruu the Greathearted → #228

## C→D demotions (3)

The classification marked these C; the re-audit falsified a shortcut each one trips, so each
gets an increment:

- **Brion Stoutarm** — the DSL expresses the card, but `resolution/damage.rs`'s
  `DealDamage → Target::Player` arm never calls `gain_lifelink`, so Brion's lifelinked fling
  gains no life. A real, previously unflagged engine bug (no `ponytail:` claims it) —
  regression test mandated → #212.
- **Flusterstorm** — `effects.rs:596`'s "no pool card counters a copy" dies with this card:
  it both mints storm copies and counters spells, so a countered copy landing in a graveyard
  (retrievable by Izzet Chronarch) is reachable, violating CR 707.10a → #213.
- **Dominus of Fealty** — the Besmirch steal sequence is landed, but `cast.rs:1686`/`1298`
  gate ability activation and tap-for-mana on the *owner*; Dominus steals *any permanent*
  (a mana rock, a Howling Mine) precisely to use it, which is broken today. Rides the #228
  slice-1 controller gates; its until-EOT steal over a donated permanent also needs the
  CR 800.4a timestamps (#228 slice 1).

Checked and deliberately kept C: Propaganda/Windborn Muse (`combat.rs:420` generic-tax claim
holds), Vision Skeins (mandatory symmetric draw — G1's declinable-draw work belongs to Trade
Secrets, #231), Izzet Chronarch and Journey to Nowhere (observers of gaps fixed in
#213/#228, marked † above).

## Observability re-audit (falsified pool-absence claims)

The re-audit checked 364 `approximates`/`ponytail:` claims against this deck and flagged 14
falsified or newly-doubtful, plus three minor fired-upgrade-trigger notes and one unflagged
bug. Each is folded into the increment that clears it:

1. `apply.rs:1527` — owner/controller conflation ("add a real control field if a card ever
   cares") → #228 slices 1–2. Nuance found reading the code: a persistent control-override
   layer already exists (`permanent_control_overrides`, Entrancing Melody), so ownership
   stays true under control changes today; donation must land in that layer, *not* as an
   owner rewrite like reanimation's — then death/zone routing stays owner-correct for free.
2. `core.rs:362` + `state.rs:71` — no per-entry control timestamps (CR 800.4a) → #228
   slice 1.
3. `combat.rs:30` — blocker legality gated on owner, not CR 509.1a's controller → #228
   slice 1 (Reins of Power's defensive steal must block).
4. `cast.rs:1686` (+ `cast.rs:1298`) — activation/tap gated on owner, not CR 602.2's
   controller → #228 slice 1 (donated permanents must work for their recipient).
5. `types/effect.rs:9` — "only spells choose an x": **stale prose, not a gap** — the {X}
   activation core is landed (see #223 and "Resolved conflicts" below). Comment cleanup
   in #223.
6. `types/mana.rs:15` — "{X} in activated-ability costs aren't modeled": **stale prose**,
   same evidence. Comment cleanup in #223.
7. `lib.rs:416` + `schema/src/intent.rs:712` — the one-click path and the wire intent carry
   no chosen X: **real** — Nin is unplayable from a client without it → #223.
8. `effects.rs:1763` + `types/effect.rs:2611` + `unbound_flourishing.toml:7` — a copied
   activated ability keeps its targets; `mana.rs:347` — Palette's HasX credit can't fund an
   ability {X}: **real**, Nin makes both observable → #223.
9. `effects.rs:596` — "no pool card counters a copy" → #213.
10. `types/effect.rs:4089` — CR 603.4 second intervening-if check skipped (Howling Mine can
    be tapped in response) + `triggers.rs:1148`/`1167` missing active-player trigger
    context → #219.
11. `types/effect.rs:2726` — delayed triggers fire only at Upkeep/End; Scattering Stroke
    schedules to a main phase → #230 (also unblocks `advanced_reconstruction.toml:25`).
12. `types/effect.rs:3213` + `types/filter.rs:24` — one target per ability → #218 (consumed
    again by #228's donation/exchange targets).
13. `query.rs:631` — ability targeting ignores protection colors (Nin is a UR source,
    CR 702.16b) → #223.
14. `characteristics.rs:393` — "no soc-pool commander is 3+ colors" upgrade condition has
    fired (Zedruu/Ruhan/Numot are WUR, Command Tower in deck) → #207.
15. `query.rs:1299` — "modified" counts any attached Aura; an opponent's Vow on your
    creature must not count (CR 701.60a) → #203.
16. `arcane_denial.toml:6` — "drawing is strictly beneficial everywhere in this pool" loses
    its footing once Trade Secrets' declinable draws exist → #231 (restore the printed "may
    draw up to two").

Minor fired-upgrade notes: `types/effect.rs:2128` (Chaos Warp tuck split — Oblation is the
awaited second card) → #215; `priority.rs:100` (a departed player's donated permanents must
leave with them) → #228 slice 4; `types/card.rs:1602` ("no morph card is in the pool" is
stale text) → deleted when Chromeshell Crab lands, #228 slice 3. Unflagged bug: the
noncombat-lifelink-to-players gap → #212.

## Resolved conflicts between classification and re-audit

**Nin's {X} activation (classification right on the core; re-audit right on the edges).**
Read at the source: `Intent::ActivateAbility` carries `x: u32` (`types/stack.rs:154`),
`activate_ability` pays `cost.mana.with_x(x)` (`cast.rs:1914`) and places the ability via
`push_ability_group_with_x`, "threading the chosen `{X}` so `Amount::X` resolves against it"
(`cast.rs:2119`); Illusionary Mask's `activation_cost = { x = true }` is the live consumer.
So the re-audit's B1/B2 (claims 5–6 above) point at stale ponytail prose, not missing
machinery. But its B3/B4 (claims 7–8) are genuine: no wire/one-click X surface, no
copy-retarget for a targeted {X} ability, no Palette credit at an ability payment. All of it
is #223.

**Donation's shape (re-audit overstated the rework).** A1 calls for "a real controller field
/ persistent control-override layer" as if none existed — `permanent_control_overrides`
already is one. What is actually missing: arbitrary-recipient control events, CR 800.4a
timestamps across the three override registries, controller-not-owner gates for
block/activate/tap, and the elimination sweep. #228 is scoped to exactly that, still
honestly XL.
