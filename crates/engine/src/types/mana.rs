use super::*;

/// A mana cost: generic mana, colored pips indexed by [`Color::index`], and colorless `{C}` pips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cost {
    pub generic: u8,
    pub colored: [u8; Color::COUNT],
    /// Colorless `{C}` pips — payable only by colorless mana (not by colored or "any").
    pub colorless: u8,
    /// The number of `{X}` symbols in the cost (0 = none) — a spell's cast cost or an activated
    /// ability's activation cost alike (Nin, the Pain Artist's `{X}{U}{R}, {T}`). CR 107.3: every
    /// `{X}` in a cost is the same chosen value, paid once per symbol — `{X}{X}{X}` (Astral
    /// Cornucopia) pays the chosen X three times. The count lives here; the chosen value is
    /// multiplied in and added to [`Cost::generic`] via [`Cost::with_x`] before payment, so mana
    /// planning never has to know about `{X}`.
    /// ponytail: `{X}` on a permanent's own characteristics (a CDA, CR 107.3) isn't modeled; grow
    /// that from a real card that needs it.
    pub x: u8,
    /// Hybrid mana pips (CR 107.4e — `{a/b}`), one entry per symbol: each is payable by mana of
    /// color `a` *or* `b`, a dual credit touching either color, or an "any" wildcard — strictly
    /// more flexible than a mono colored pip, so [`ManaPool::spend_plan`] pays these after the
    /// fixed `colored` pips. A cost with `{W/B}{W/B}` carries two entries. Empty for a cost with
    /// no hybrid symbols (the overwhelming majority).
    pub hybrid: &'static [(Color, Color)],
    /// An additional cost paid alongside mana, before the spell hits the stack (CR 601.2b/
    /// 601.2f–h) — e.g. Big Score's "As an additional cost to cast this spell, discard a card."
    pub additional: AdditionalCost,
    /// A spell's own board-derived generic reduction (CR 601.2f/118.9) — "This spell costs {1}
    /// less to cast for each ..." written on the spell itself (Blasphemous Act), as opposed to a
    /// permanent's static [`Effect::ReduceSpellCost`] discounting *other* spells. Resolved once
    /// at [`Game::cast_cost`] and subtracted from generic after any permanent-static reducers,
    /// floored at 0 (CR 601.2f — cost reduction never touches colored pips).
    pub reduce_own_generic: Option<Amount>,
}

impl Default for Cost {
    /// The empty cost — [`Cost::FREE`].
    fn default() -> Self {
        Cost::FREE
    }
}

impl Cost {
    /// The empty cost (lands, tokens).
    pub const FREE: Cost = Cost {
        generic: 0,
        colored: [0; Color::COUNT],
        colorless: 0,
        x: 0,
        hybrid: &[],
        additional: AdditionalCost {
            discard: 0,
            discard_land: false,
            pay_life_x: false,
            pay_life: 0,
            sacrifice: None,
            kicker: None,
            buyback: None,
            strive: None,
            replicate: None,
        },
        reduce_own_generic: None,
    };

    /// This cost with a chosen `x` folded into its generic component (CR 601.2b/107.3: paying
    /// `{X}` adds `x` generic mana per `{X}` symbol in the cost — `{X}{X}{X}` pays the chosen
    /// value three times). A no-`{X}` cost ignores `x`, matching "must stay 0."
    pub fn with_x(self, x: u32) -> Cost {
        // Clamp before narrowing so a huge chosen X (or a large multiplier) can never truncate
        // down to a cheap cost (a payment-bypass); a cost that saturates at 255 generic is
        // already unpayable.
        let extra = (self.x as u32).saturating_mul(x).min(u8::MAX as u32) as u8;
        Cost {
            generic: self.generic.saturating_add(extra),
            x: 0,
            ..self
        }
    }

    /// Whether this mana cost "could be paid by some amount of, or all of" a spent-mana multiset
    /// (Illusionary Mask's `{X}` test, CR 107.3). `spent` is [`ManaPool::spent_counts`]'s shape:
    /// one count per color plus a sixth bucket of credits with no one specific color. Each colored
    /// pip needs a spent unit of its color; each hybrid pip (CR 107.4e — `{a/b}`) a unit of either
    /// of its two colors; generic pips take any remaining unit. This cost's own `{X}` is
    /// necessarily chosen as 0 for a cast it must fund at 0 (CR 107.3b), so its `x` pips need
    /// nothing.
    /// ponytail: `{C}` pips are treated as unpayable — the sixth bucket can't tell true colorless
    /// from an "any"/dual credit (mana of a color is never colorless); no pool creature card
    /// prints `{C}` in its cost. Split the bucket if one lands.
    pub fn payable_from_multiset(&self, spent: &[u8; 6]) -> bool {
        if self.colorless > 0 {
            return false;
        }
        let mut remaining = *spent;
        for (remaining, &pips) in remaining.iter_mut().zip(self.colored.iter()) {
            if *remaining < pips {
                return false;
            }
            *remaining -= pips;
        }
        if !hybrid_pips_assignable(self.hybrid, &mut remaining) {
            return false;
        }
        let left: u32 = remaining.iter().map(|&n| u32::from(n)).sum();
        left >= u32::from(self.generic)
    }

    /// Render this cost's mana pips as cost text (CR 202.3) — `{X}`, the generic number, `{C}`
    /// colorless pips, WUBRG colored pips, then `{a/b}` hybrid pips. Ignores non-mana riders
    /// (`additional`); used wherever a full `Cost` needs to read back as a pip string, e.g.
    /// [`Effect`](super::Effect)'s `SacrificeSelfUnlessPay` label (Keldon Vandals' `{2}{R}`).
    pub fn mana_label(&self) -> String {
        let mut out = String::new();
        for _ in 0..self.x {
            out.push_str("{X}");
        }
        if self.generic > 0 {
            out.push_str(&format!("{{{}}}", self.generic));
        }
        for _ in 0..self.colorless {
            out.push_str("{C}");
        }
        for color in Color::ALL {
            for _ in 0..self.colored[color.index()] {
                out.push_str(&format!("{{{}}}", color.letter()));
            }
        }
        for &(a, b) in self.hybrid {
            out.push_str(&format!("{{{}/{}}}", a.letter(), b.letter()));
        }
        out
    }
}

/// Assign each hybrid pip (CR 107.4e — `{a/b}`) one remaining spent unit of either of its colors,
/// backtracking when a pick strands a later pip. On success the picks stay deducted from
/// `remaining`; which colors fund which pips never changes the total left for generic, so any
/// successful assignment is as good as any other. Exhaustive (2^pips), but a real cost carries at
/// most a handful of hybrid pips.
fn hybrid_pips_assignable(pips: &[(Color, Color)], remaining: &mut [u8; 6]) -> bool {
    let Some((&(a, b), rest)) = pips.split_first() else {
        return true;
    };
    for color in [a, b] {
        if remaining[color.index()] == 0 {
            continue;
        }
        remaining[color.index()] -= 1;
        if hybrid_pips_assignable(rest, remaining) {
            return true;
        }
        remaining[color.index()] += 1;
    }
    false
}

/// An additional cost to cast a spell (CR 601.2f), on top of its mana cost. Paid synchronously
/// during [`Game::cast`] — the client names which cards pay it (mirroring how
/// [`Game::activate_ability`] takes its sacrifice choice as an intent parameter), so casting
/// never has to pause mid-cast on a [`PendingChoice`].
/// ponytail: `discard` (Big Score, Seize the Spoils's "discard a card"), `discard_land`
/// (Throes of Chaos's retrace), `pay_life_x` (Toxic Deluge's "pay X life"), and `sacrifice`
/// (Plumb the Forbidden's "you may sacrifice one or more creatures") only. A remove-a-counter
/// additional cost isn't modeled yet — grow this struct from the real card that needs the next
/// one.
/// Deserialized by a hand-written `Deserialize` impl in `de.rs` (not derived here), since
/// `pay_life_x` spells as the TOML marker string `pay_life = "x"`, not a bool key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AdditionalCost {
    /// The number of cards to discard as part of the cost (0 for none).
    pub discard: u8,
    /// Retrace's "discard a land card" (CR 702.83a) — an additional cost of discarding exactly
    /// one card that's a land, distinct from [`Self::discard`]'s unfiltered discard(s) (no pool
    /// card combines the two). `false` for a card without it. TOML `discard_land = true`.
    pub discard_land: bool,
    /// Whether this spell's chosen `{X}` (CR 601.2b) is paid as life rather than mana (CR
    /// 601.2f) — Toxic Deluge's "As an additional cost to cast this spell, pay X life." When
    /// set, [`Game::cast_cost`] keeps `{X}` out of the mana cost and [`Game::cast`] pays it as
    /// life instead (CR 119.4 — capped by the caster's life total).
    pub pay_life_x: bool,
    /// A *fixed* life payment (CR 601.2f) — Deep Analysis's flashback "Pay 3 life" rider. Distinct
    /// from [`Self::pay_life_x`] (which spends the chosen `{X}` as life): this is a flat amount that
    /// never touches mana. 0 for none. [`Game::cast`] rejects the cast if the caster has less life
    /// (CR 119.4) and otherwise pays it alongside the mana cost. TOML `pay_life = 3`.
    pub pay_life: u8,
    /// A sacrifice rider on the cost (CR 601.2f), either [`SacrificeAdditionalCostCount::OneOrMore`]'s
    /// entirely-optional "sacrifice any number" (Plumb the Forbidden) or
    /// [`SacrificeAdditionalCostCount::Exactly`]'s mandatory fixed count (Dread Return's
    /// Flashback—Sacrifice three creatures, CR 601.2f/602.2b). `None` for no sacrifice cost on
    /// this spell. The chosen permanents are named in [`Intent::Cast`]'s `sacrifice_cost`, paid
    /// alongside `discard_cost` by [`Game::cast`]; the count actually paid is recorded on the
    /// resulting [`Spell`] (read by a copy-per-sacrifice rider for the optional shape). TOML
    /// `sacrifice = { count = "one_or_more", filter = "creature" }` or `sacrifice = { count = 3,
    /// filter = "creature" }`.
    pub sacrifice: Option<SacrificeAdditionalCost>,
    /// Kicker (CR 702.33) — "You may pay an additional [cost] as you cast this spell" (Rite of
    /// Replication's "Kicker {5}"). `None` for a spell with no kicker. Entirely optional: the
    /// caster chooses whether to pay when casting (CR 702.33d), recorded on the resulting
    /// [`Spell::kicked`] for a resolution-time effect to branch on (Rite of Replication's "If
    /// this spell was kicked, create five of those tokens instead"). A `&'static` reference
    /// (leaked, like other card-data fields) rather than a nested `Cost` — `Cost` already embeds
    /// `AdditionalCost`, so a bare `Option<Cost>` here would be an infinitely-sized recursive
    /// type. TOML `[cost.additional.kicker]` — the same shape as `[cost]`.
    /// ponytail: single-kicker only (one kicker cost, paid or not) — a multikicker
    /// (`{N}` any number of times, CR 702.34) or a two-kicker-costs card (Rite of Flame-style
    /// "Kicker {1}, Kicker {R}") isn't modeled; grow those from a real card that needs one.
    pub kicker: Option<&'static Cost>,
    /// Buyback (CR 702.27) — "You may pay an additional [cost] as you cast this spell. If you
    /// do, put this card into your hand as it resolves" (Capsize's "Buyback {3}"). `None` for a
    /// spell with no buyback. Entirely optional, mirroring [`Self::kicker`]'s own opt-in shape:
    /// the caster chooses whether to pay when casting (CR 702.27c), recorded on the resulting
    /// [`Spell::bought_back`] for [`Game::finish_instant_sorcery_resolution`] to branch on (CR
    /// 702.27d — the card returns to its owner's hand instead of the graveyard). A `&'static`
    /// reference for the same recursive-`Cost` reason as [`Self::kicker`]. TOML
    /// `[cost.additional.buyback]` — the same shape as `[cost.additional.kicker]`.
    /// ponytail: single-buyback only, mirroring kicker's own single-cost shape — no pool card
    /// prints two buyback costs; grow that if one ever does.
    pub buyback: Option<&'static Cost>,
    /// Strive (CR 702.42) — "This spell costs [cost] more to cast for each target beyond the
    /// first" (Twinflame's `{2}{R}`). `None` for a spell with no Strive. Unlike kicker (paid or
    /// not), Strive's total depends on *how many* targets the caster commits to: CR 601.2c
    /// chooses targets before CR 601.2f locks the total cost, but this engine puts a spell on
    /// the stack before pausing to choose multi-targets, so the caster instead declares the
    /// target count up front on [`crate::Intent::Cast`] (mirroring how [`Self::sacrifice`]'s
    /// paid count settles before the stack) — [`Game::cast_cost`] multiplies this cost by
    /// `declared count − 1` and adds it in, and the same declared count substitutes as the
    /// spell's target-count clamp (see [`TargetCount::strive_scaled`]). A `&'static` reference
    /// for the same recursive-`Cost` reason as [`Self::kicker`]. TOML `[cost.additional.strive]`
    /// — the same shape as `[cost.additional.kicker]`.
    /// ponytail: exactly one per-extra-target cost (Strive's own shape) — not a generalized
    /// N-times multiplier cost (escalate, multikicker); grow that from a card that needs it.
    pub strive: Option<&'static Cost>,
    /// Replicate (CR 702.108) — "You may pay [cost] any number of times as you cast this spell.
    /// When you cast this spell, copy it for each time you paid its replicate cost" (Changing
    /// Loyalty's "Replicate {2}"). `None` for a spell with no Replicate. Unlike Strive (whose
    /// extra cost is per target beyond the first), each replicate payment is a full extra
    /// instance of the cost — the caster's declared payment count settles pre-stack on
    /// [`crate::Intent::Cast`]'s `replicate_count` (mirroring [`Self::strive`]'s own pre-stack
    /// declaration), multiplies this cost by that count in [`Game::cast_cost`], and is recorded
    /// on the resulting [`Spell::replicate_count`] so the cast choke can mint that many copies
    /// (CR 702.108b — reusing [`Game::mint_spell_copies`], the same rider [`Effect::CopyThisSpell`]
    /// uses). A `&'static` reference for the same recursive-`Cost` reason as [`Self::kicker`].
    /// TOML `[cost.additional.replicate]` — the same shape as `[cost.additional.kicker]`.
    /// ponytail: single replicate cost only (Changing Loyalty is the pool's only replicate
    /// card) — grow a per-card cap or multi-replicate-cost shape from a real card that needs one.
    pub replicate: Option<&'static Cost>,
}

/// A sacrifice rider on a spell's mana cost (CR 601.2f — [`AdditionalCost::sacrifice`]): the
/// permanent filter the named sacrifices must match, plus how many are required.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SacrificeAdditionalCost {
    pub filter: PermanentFilter,
    pub count: SacrificeAdditionalCostCount,
}

/// How many permanents [`SacrificeAdditionalCost`] requires (CR 601.2f).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SacrificeAdditionalCostCount {
    /// "You may sacrifice any number of permanents..." (Plumb the Forbidden) — entirely
    /// optional, 0 up to however many the caster controls.
    OneOrMore,
    /// "Sacrifice N creatures" (Dread Return's Flashback—Sacrifice three creatures) — mandatory:
    /// the cast names exactly N distinct matching permanents (CR 601.2f/602.2b) or is rejected.
    Exactly(u8),
}

/// Escape's cost (CR 702.19): the escape mana cost plus how many *other* graveyard cards must
/// be exiled to cast it, and how many +1/+1 counters the permanent escapes with (CR 702.19c —
/// "This creature escapes with N +1/+1 counters on it"; 0 for an escape card with no such
/// clause). `[escape]` in TOML — a `[escape.cost]` sub-table (the same shape as `[cost]`/
/// `[flashback]`) plus `exile`/`plus_one_plus_one_counters`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(deny_unknown_fields)
)]
pub struct EscapeCost {
    pub cost: Cost,
    pub exile: u8,
    #[cfg_attr(feature = "card-dsl", serde(default))]
    pub plus_one_plus_one_counters: u8,
}

/// A single kind of mana that can be produced or held: one of the five colors, colorless
/// `{C}`, or "any color" (a wildcard). Distinct from [`Color`] so colorless/any never leak
/// into color identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mana {
    Color(Color),
    /// Colorless `{C}` — a mana type, not a color.
    Colorless,
    /// "Any color" — a wildcard chosen at payment time.
    Any,
    /// One mana of either of two colors — a dual land's "{T}: Add {G} or {U}". Like
    /// [`Mana::Any`], a credit that picks its color at payment time (no choice on tap),
    /// but restricted to these two colors. The pair is unordered; the card DSL normalizes
    /// it to WUBRG order, and [`color_pair_index`] accepts either order.
    Either(Color, Color),
    /// One mana of any color in a computed set of 2–4 colors — Fellwar Stone / Exotic
    /// Orchard's "any color that a land an opponent controls could produce" (CR intent behind
    /// both), generalizing [`Mana::Either`]'s fixed pair to an arbitrary WUBRG bitmask (bit `i`
    /// set = [`Color::ALL`]`[i]` is in the set; same bit order as [`Color::index`]). Computed at
    /// resolution by [`Game::opponent_producible_colors_credit`], which collapses a 0-, 1-, or
    /// 5-color result to `None`/[`Mana::Color`]/[`Mana::Any`] instead — this variant only ever
    /// carries 2–4 bits.
    OfColors(u8),
    /// A credit that may be spent only per `restriction` (CR 106.9's "restrictions" on mana,
    /// e.g. "Spend this mana only to cast an instant or sorcery spell") — Troyan, Gutsy
    /// Explorer's `{G}{U}`, Galazeth Prismari's granted artifact "any color". `base` is the
    /// credit it otherwise behaves as; [`ManaPool::spend_plan`] excludes it entirely from a
    /// payment [`SpendRestriction::allows`] rejects, rather than treating it as a differently
    /// -flexible source.
    Restricted {
        base: RestrictedManaBase,
        restriction: SpendRestriction,
    },
}

/// The unrestricted mana kind a [`Mana::Restricted`] credit is otherwise. Only the kinds real
/// restricted-mana cards in the pool produce (a colored, colorless, or "any color" credit) —
/// no card restricts a dual ([`Mana::Either`]) or restricted-set ([`Mana::OfColors`]) credit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestrictedManaBase {
    Color(Color),
    Colorless,
    Any,
}

/// A restriction on when a [`Mana::Restricted`] credit may be spent (CR 106.9), checked by
/// [`ManaPool::spend_plan`] against the spell being paid for. Flag-don't-force: only the two
/// restrictions the pool's two restricted-mana cards actually print.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum SpendRestriction {
    /// "Spend this mana only to cast an instant or sorcery spell" (Galazeth Prismari's granted
    /// artifact ability).
    InstantOrSorcery,
    /// "Spend this mana only to cast spells with mana value N or greater or spells with `{X}`
    /// in their mana cost[s]" (Troyan, Gutsy Explorer — `N` = 5).
    ManaValueAtLeastOrHasX(u32),
    /// "Spend this mana only on costs that contain `{X}`" (Elementalist's Palette) — a cast's
    /// mana cost or an activated ability's own activation cost alike (Nin, the Pain Artist's
    /// `{X}{U}{R}`, CR 106.9); `Game::activate_ability` feeds `allows` the ability's own cost's
    /// `{X}` count (`mana_value`/`is_instant_or_sorcery` are meaningless for an ability payment,
    /// so both stay at their default/`false`). (CR 602, CR 601, CR 113)
    HasX,
}

impl SpendRestriction {
    /// Whether a credit under this restriction may fund a payment toward `spell`.
    fn allows(self, spell: SpellCharacteristics) -> bool {
        match self {
            SpendRestriction::InstantOrSorcery => spell.is_instant_or_sorcery,
            SpendRestriction::ManaValueAtLeastOrHasX(at_least) => {
                spell.mana_value >= at_least || spell.has_x
            }
            SpendRestriction::HasX => spell.has_x,
        }
    }
}

/// The facts about a spell being cast that a [`SpendRestriction`] checks — see
/// [`CardDef::spell_characteristics`]. `None` at an ability-activation payment (no spell is
/// being cast, so CR 106.9's restricted credits can never fund one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCharacteristics {
    pub mana_value: u32,
    pub has_x: bool,
    pub is_instant_or_sorcery: bool,
}

/// What a land's base tap-for-one ([`CardKind::Land::produces`]) yields: a concrete [`Mana`]
/// kind, or one of two credits resolved from table state at tap time rather than baked into a
/// static `Mana` kind — "one mana of any color in your commander's color identity" (CR 903.4 —
/// Command Tower) or "any color a land an opponent controls could produce" (Fellwar Stone /
/// Exotic Orchard, CR intent). Resolved via [`Game::commander_identity_credit`] /
/// [`Game::opponent_producible_colors_credit`] — `ManaPool`/the payment planner never need to
/// know about either case, only the [`Mana`] credit they resolve to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandProduces {
    Mana(Mana),
    CommanderIdentity,
    /// "{T}: Add one mana of any color that a land an opponent controls could produce"
    /// (Exotic Orchard) — the land-sugar spelling of [`Game::opponent_producible_colors_credit`].
    OpponentColors,
}

/// The ten unordered color pairs in WUBRG-major order — the index space of the dual
/// ("either of two colors") credits in [`ManaPool::either`].
pub const COLOR_PAIRS: [(Color, Color); 10] = [
    (Color::White, Color::Blue),
    (Color::White, Color::Black),
    (Color::White, Color::Red),
    (Color::White, Color::Green),
    (Color::Blue, Color::Black),
    (Color::Blue, Color::Red),
    (Color::Blue, Color::Green),
    (Color::Black, Color::Red),
    (Color::Black, Color::Green),
    (Color::Red, Color::Green),
];

/// The [`COLOR_PAIRS`] slot holding `{a, b}`, in either order. Panics on `a == b` —
/// a "dual" of one color is a mono producer, and the card DSL rejects it at load.
pub fn color_pair_index(a: Color, b: Color) -> usize {
    COLOR_PAIRS
        .iter()
        .position(|&(x, y)| (x, y) == (a, b) || (x, y) == (b, a))
        .expect("two distinct colors always form a pair")
}

/// Distinct spend-restricted (base, restriction) combos a pool holds concurrently
/// ([`ManaPool::restricted`]) — generous headroom over the 2 the card pool exercises today (a
/// player controlling both Troyan and a Galazeth-granted artifact holds 2 at once).
pub(crate) const RESTRICTED_SLOTS: usize = 4;

/// One [`Mana::Restricted`] (base, restriction) bucket and how much of it is pooled — see
/// [`ManaPool::restricted`]. `key = None` marks an unused slot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RestrictedSlot {
    pub key: Option<(RestrictedManaBase, SpendRestriction)>,
    pub amount: u8,
}

/// A multiset of mana: a player's pool, and also the fixed batch a mana ability produces.
///
/// ponytail: "any color" is a wildcard *credit* spent at payment time rather than a color
/// chosen up front — so producing it needs no extra choice/UI, and it can pay any single
/// colored pip or generic (never `{C}`, since mana of any *color* is never colorless).
/// A dual land's "either of two colors" ([`Mana::Either`]) is the same credit restricted
/// to its pair, carried per unordered pair in `either`. [`Mana::OfColors`] generalizes that
/// pair to an arbitrary WUBRG bitmask, carried per exact mask value in `of_colors`. (CR 605, CR 113)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ManaPool {
    pub colored: [u8; Color::COUNT],
    pub colorless: u8,
    pub any: u8,
    /// Dual credits ("one mana of either `a` or `b`"), indexed by [`color_pair_index`].
    pub either: [u8; COLOR_PAIRS.len()],
    /// Restricted-color-set credits ([`Mana::OfColors`]), indexed by the mask value itself
    /// (`0..1 << Color::COUNT`) — only indices with 2–4 bits set are ever populated.
    pub of_colors: [u8; 1 << Color::COUNT],
    /// Spend-restricted credits ([`Mana::Restricted`], CR 106.9), a small fixed set of
    /// (base, restriction) buckets rather than an indexed array — a restriction carries
    /// arbitrary data (`ManaValueAtLeastOrHasX`'s `N`), unlike `of_colors`'s fixed bitmask key.
    pub restricted: [RestrictedSlot; RESTRICTED_SLOTS],
}

impl ManaPool {
    /// A pool holding `amount` of a single kind of mana.
    pub fn of(mana: Mana, amount: u8) -> ManaPool {
        let mut pool = ManaPool::default();
        pool.add(mana, amount);
        pool
    }

    /// Add `amount` of one mana kind.
    /// Every credit in the pool, whatever its kind. The colored array alone answers "how much mana
    /// is floating?" wrongly: `{C}`, "any color", and the dual/restricted credits a dual or filter
    /// land leaves behind all live outside it.
    pub fn total(&self) -> u32 {
        let sum = |xs: &[u8]| xs.iter().map(|&n| u32::from(n)).sum::<u32>();
        sum(&self.colored)
            + sum(&self.either)
            + sum(&self.of_colors)
            + u32::from(self.colorless)
            + u32::from(self.any)
            + self
                .restricted
                .iter()
                .map(|s| u32::from(s.amount))
                .sum::<u32>()
    }

    pub fn add(&mut self, mana: Mana, amount: u8) {
        match mana {
            Mana::Color(c) => self.colored[c.index()] += amount,
            Mana::Colorless => self.colorless += amount,
            Mana::Any => self.any += amount,
            Mana::Either(a, b) => self.either[color_pair_index(a, b)] += amount,
            Mana::OfColors(mask) => self.of_colors[mask as usize] += amount,
            Mana::Restricted { base, restriction } => {
                self.add_restricted(base, restriction, amount)
            }
        }
    }

    /// Remove one credit of exactly `mana`'s kind if present, returning whether it was. Used by
    /// [`Game::queue_spend_to_cast_triggers`](crate::Game) to walk a spend multiset and match it
    /// against provenance-tagged credits one at a time.
    pub(crate) fn take_one(&mut self, mana: Mana) -> bool {
        let slot: &mut u8 = match mana {
            Mana::Color(c) => &mut self.colored[c.index()],
            Mana::Colorless => &mut self.colorless,
            Mana::Any => &mut self.any,
            Mana::Either(a, b) => &mut self.either[color_pair_index(a, b)],
            Mana::OfColors(mask) => &mut self.of_colors[mask as usize],
            Mana::Restricted { base, restriction } => {
                let key = Some((base, restriction));
                match self.restricted.iter_mut().find(|s| s.key == key) {
                    Some(s) if s.amount > 0 => &mut s.amount,
                    _ => return false,
                }
            }
        };
        if *slot == 0 {
            return false;
        }
        *slot -= 1;
        true
    }

    /// Add `amount` of a spend-restricted credit, coalescing into an existing slot for the
    /// same (base, restriction) or claiming a free one.
    /// ponytail: silently drops the amount if every slot is taken instead of panicking
    /// mid-game — [`RESTRICTED_SLOTS`] is sized well past what the pool exercises today; raise
    /// it if a deck combination ever needs more concurrent distinct restrictions.
    fn add_restricted(
        &mut self,
        base: RestrictedManaBase,
        restriction: SpendRestriction,
        amount: u8,
    ) {
        if amount == 0 {
            return;
        }
        let key = Some((base, restriction));
        if let Some(slot) = self.restricted.iter_mut().find(|s| s.key == key) {
            slot.amount += amount;
            return;
        }
        if let Some(slot) = self.restricted.iter_mut().find(|s| s.key.is_none()) {
            slot.key = key;
            slot.amount = amount;
        }
    }

    /// This pool's credits, each wrapped as [`Mana::Restricted`] under `restriction` — the
    /// transform an `Effect::AddMana`/`GrantManaAbility`'s static batch takes everywhere it's
    /// read (its own resolution and the `available_mana` estimate), so a restricted batch reads
    /// restricted the same way everywhere. A no-op when `restriction` is `None`.
    ///
    /// ponytail: only color/colorless/"any" credits convert — a dual ([`Mana::Either`]) or
    /// restricted-set ([`Mana::OfColors`]) credit under a restriction passes through
    /// unrestricted; no pool card restricts one. Widen if a future card does.
    pub(crate) fn restricted_by(self, restriction: Option<SpendRestriction>) -> ManaPool {
        let Some(restriction) = restriction else {
            return self;
        };
        let mut out = ManaPool {
            either: self.either,
            of_colors: self.of_colors,
            ..ManaPool::default()
        };
        for (i, &n) in self.colored.iter().enumerate() {
            out.add(
                Mana::Restricted {
                    base: RestrictedManaBase::Color(Color::ALL[i]),
                    restriction,
                },
                n,
            );
        }
        out.add(
            Mana::Restricted {
                base: RestrictedManaBase::Colorless,
                restriction,
            },
            self.colorless,
        );
        out.add(
            Mana::Restricted {
                base: RestrictedManaBase::Any,
                restriction,
            },
            self.any,
        );
        out
    }

    /// Add another pool's contents into this one, component-wise (saturating — a pool holds at
    /// most 255 of any one kind, which the auto-pass heuristic that uses this never approaches).
    pub(crate) fn merge(&mut self, other: &ManaPool) {
        for i in 0..Color::COUNT {
            self.colored[i] = self.colored[i].saturating_add(other.colored[i]);
        }
        self.colorless = self.colorless.saturating_add(other.colorless);
        self.any = self.any.saturating_add(other.any);
        for i in 0..COLOR_PAIRS.len() {
            self.either[i] = self.either[i].saturating_add(other.either[i]);
        }
        for i in 0..self.of_colors.len() {
            self.of_colors[i] = self.of_colors[i].saturating_add(other.of_colors[i]);
        }
        for slot in other.restricted {
            if let Some((base, restriction)) = slot.key {
                self.add_restricted(base, restriction, slot.amount);
            }
        }
    }

    /// Remove a multiset (a computed payment) from this pool.
    pub(crate) fn subtract(&mut self, other: &ManaPool) {
        for i in 0..Color::COUNT {
            self.colored[i] -= other.colored[i];
        }
        self.colorless -= other.colorless;
        self.any -= other.any;
        for i in 0..COLOR_PAIRS.len() {
            self.either[i] -= other.either[i];
        }
        for i in 0..self.of_colors.len() {
            self.of_colors[i] -= other.of_colors[i];
        }
        for slot in other.restricted {
            let Some(key) = slot.key else { continue };
            let mine = self
                .restricted
                .iter_mut()
                .find(|s| s.key == Some(key))
                .expect("subtract only ever removes restricted mana a spend plan found in self");
            mine.amount -= slot.amount;
            if mine.amount == 0 {
                mine.key = None;
            }
        }
    }

    /// The colors actually spent in this multiset (CR 106.9's "spent to cast" query — Court
    /// Hussar's "unless {W} was spent to cast it"): `true` at index `i` iff any of `colored[i]`
    /// was spent. Read off the exact payment [`ManaPool::spend_plan`] returns, right after
    /// [`Game::settle_payment`](crate::Game::settle_payment) appends its [`Event::ManaSpent`].
    /// ponytail: only literally-colored `colored` credits count — a dual ([`Mana::Either`]),
    /// restricted-set ([`Mana::OfColors`]), or "any" ([`Mana::Any`]) credit spent toward a pip
    /// never resolves to one specific color in this model (see this type's own doc), so paying,
    /// say, a generic cost entirely from a Tundra never counts as white (or blue) spent here. No
    /// pool card's mana base exercises that gap yet; widen if a dual/filter-land-heavy deck needs it.
    pub(crate) fn colors_spent(&self) -> [bool; Color::COUNT] {
        self.colored.map(|n| n > 0)
    }

    /// The multiset of mana actually spent, counted per kind for Illusionary Mask's CR 107.3
    /// "the mana you spent on {X}" payability test ([`Cost::payable_from_multiset`]): one count
    /// per color, plus a sixth bucket for every credit that never resolves to one specific color
    /// in this model — colorless, "any", dual, restricted-set, and spend-restricted credits (the
    /// same modeling line as [`ManaPool::colors_spent`]'s ponytail; those pay only generic pips).
    /// Read off the exact payment [`ManaPool::spend_plan`] returns, like `colors_spent`.
    pub(crate) fn spent_counts(&self) -> [u8; 6] {
        let sum = |xs: &[u8]| xs.iter().map(|&n| u32::from(n)).sum::<u32>();
        let other = u32::from(self.colorless)
            + u32::from(self.any)
            + sum(&self.either)
            + sum(&self.of_colors)
            + self
                .restricted
                .iter()
                .map(|s| u32::from(s.amount))
                .sum::<u32>();
        let mut counts = [0u8; 6];
        counts[..Color::COUNT].copy_from_slice(&self.colored);
        counts[Color::COUNT] = other.min(u8::MAX.into()) as u8;
        counts
    }

    /// This pool capped at `cap`, per bucket (CR 500.4's "until end of turn" mana exception,
    /// [`Event::ManaEmptied`]): a credit surviving a mid-turn boundary is the smaller of what's
    /// still floating and what was ever marked persistent (some may have been spent since).
    /// ponytail: `restricted` isn't capped per-slot by matching key — it just keeps `self`'s
    /// slots as-is. No pool card produces persistent *restricted* mana (Rousing Refrain's `{R}`
    /// isn't restricted), so the gap is unobserved; widen to a per-(base, restriction) min if one
    /// ever does.
    pub(crate) fn componentwise_min(&self, cap: &ManaPool) -> ManaPool {
        let mut out = ManaPool {
            restricted: self.restricted,
            ..ManaPool::default()
        };
        for i in 0..Color::COUNT {
            out.colored[i] = self.colored[i].min(cap.colored[i]);
        }
        out.colorless = self.colorless.min(cap.colorless);
        out.any = self.any.min(cap.any);
        for i in 0..COLOR_PAIRS.len() {
            out.either[i] = self.either[i].min(cap.either[i]);
        }
        for i in 0..self.of_colors.len() {
            out.of_colors[i] = self.of_colors[i].min(cap.of_colors[i]);
        }
        out
    }

    /// Plan how to pay `cost` from this pool, given the spell it's paying for (`None` for an
    /// ability-activation payment — no spell, so no [`Mana::Restricted`] credit ever applies,
    /// CR 106.9). Folds every restricted credit `spell` makes usable into the plain algorithm
    /// below as its `base` kind — a restriction only ever gates whether a credit is *present*
    /// for a payment, never how it's spent once it is — then splits the resulting plan back into
    /// real vs. restricted usage (real mana first, an arbitrary but deterministic tie-break
    /// between otherwise-equivalent legal spends). Returns the exact multiset to spend, or
    /// `None` if the pool can't cover it. Pure — the caller applies it.
    pub(crate) fn spend_plan(
        &self,
        cost: &Cost,
        spell: Option<SpellCharacteristics>,
    ) -> Option<ManaPool> {
        let mut effective = *self;
        effective.restricted = [RestrictedSlot::default(); RESTRICTED_SLOTS];
        for slot in self.restricted {
            let Some((base, restriction)) = slot.key else {
                continue;
            };
            if slot.amount == 0 || !spell.is_some_and(|s| restriction.allows(s)) {
                continue;
            }
            match base {
                RestrictedManaBase::Color(c) => effective.colored[c.index()] += slot.amount,
                RestrictedManaBase::Colorless => effective.colorless += slot.amount,
                RestrictedManaBase::Any => effective.any += slot.amount,
            }
        }

        let plan = effective.spend_plan_unrestricted(cost)?;
        let mut spend = plan;
        for i in 0..Color::COUNT {
            let real = plan.colored[i].min(self.colored[i]);
            spend.colored[i] = real;
            take_restricted(
                self,
                &mut spend,
                RestrictedManaBase::Color(Color::ALL[i]),
                plan.colored[i] - real,
                spell,
            );
        }
        let real_colorless = plan.colorless.min(self.colorless);
        spend.colorless = real_colorless;
        take_restricted(
            self,
            &mut spend,
            RestrictedManaBase::Colorless,
            plan.colorless - real_colorless,
            spell,
        );
        let real_any = plan.any.min(self.any);
        spend.any = real_any;
        take_restricted(
            self,
            &mut spend,
            RestrictedManaBase::Any,
            plan.any - real_any,
            spell,
        );

        Some(spend)
    }

    /// The core payment planner, blind to spend restrictions — see [`ManaPool::spend_plan`],
    /// which folds usable restricted credits in as their plain `base` kind before calling this.
    /// Each colored pip from its own color, then dual ("either") credits, then "any"; each
    /// hybrid pip from its own two colors' leftover mono mana (strictly more flexible than a
    /// mono pip, so it goes after them), then a dual credit touching either color, then "any";
    /// each `{C}` pip from colorless mana; generic from whatever's left.
    fn spend_plan_unrestricted(&self, cost: &Cost) -> Option<ManaPool> {
        let mut spend = ManaPool::default();

        // Colored pips, most-restricted mana first: the pip's own color, then dual
        // credits, then "any" wildcards. Each step is strictly more flexible than the one
        // before, so spending the restricted mana first never costs a payment.
        let mut shortfall = [0u8; Color::COUNT];
        let mut leftover_colored = [0u8; Color::COUNT];
        for i in 0..Color::COUNT {
            spend.colored[i] = self.colored[i].min(cost.colored[i]);
            shortfall[i] = cost.colored[i] - spend.colored[i];
            leftover_colored[i] = self.colored[i] - spend.colored[i];
        }
        // Hybrid pips (CR 107.4e — `{a/b}`), tallied per unordered color pair like `either`
        // credits, so `pips_coverable`'s Hall's-marriage check can fold them in alongside the
        // mono shortfall.
        let mut hybrid_left = [0u8; COLOR_PAIRS.len()];
        for &(a, b) in cost.hybrid {
            hybrid_left[color_pair_index(a, b)] += 1;
        }
        let mut either_left = self.either;
        let mut of_colors_left = self.of_colors;
        let mut any_left = self.any;
        if !pips_coverable(
            shortfall,
            hybrid_left,
            leftover_colored,
            either_left,
            of_colors_left,
            any_left,
        ) {
            return None;
        }
        // Feasible — commit a credit per pip, most-restricted source first: the pip's own
        // color (above), a dual ([`Mana::Either`]) credit touching it, a restricted
        // ([`Mana::OfColors`]) credit whose set includes it, then an "any" wildcard. A dual/
        // restricted credit is only taken when the *rest* stays coverable without it (with
        // overlapping sets, a blind pick can strand a later pip); when none is safe, this pip
        // is wildcard-funded in every payment.
        for color in 0..Color::COUNT {
            for _ in 0..shortfall[color] {
                shortfall[color] -= 1;
                let safe_dual = (0..COLOR_PAIRS.len()).find(|&pair| {
                    let (a, b) = COLOR_PAIRS[pair];
                    if either_left[pair] == 0 || (a.index() != color && b.index() != color) {
                        return false;
                    }
                    let mut remaining = either_left;
                    remaining[pair] -= 1;
                    pips_coverable(
                        shortfall,
                        hybrid_left,
                        leftover_colored,
                        remaining,
                        of_colors_left,
                        any_left,
                    )
                });
                if let Some(pair) = safe_dual {
                    either_left[pair] -= 1;
                    spend.either[pair] += 1;
                    continue;
                }
                let safe_of_colors = (0..of_colors_left.len()).find(|&mask| {
                    if of_colors_left[mask] == 0 || mask & (1 << color) == 0 {
                        return false;
                    }
                    let mut remaining = of_colors_left;
                    remaining[mask] -= 1;
                    pips_coverable(
                        shortfall,
                        hybrid_left,
                        leftover_colored,
                        either_left,
                        remaining,
                        any_left,
                    )
                });
                let Some(mask) = safe_of_colors else {
                    // The up-front coverable check guarantees a wildcard is free here;
                    // guard anyway rather than trust a theorem with an underflow.
                    if any_left == 0 {
                        return None;
                    }
                    any_left -= 1;
                    spend.any += 1;
                    continue;
                };
                of_colors_left[mask] -= 1;
                spend.of_colors[mask] += 1;
            }
        }

        // Hybrid pips {a/b}: strictly more flexible than a mono pip (accepts either color), so
        // they're paid after mono/dual colored pips (which are more restricted). For each pip,
        // try the least-flexible matching source first — own-color mono mana (the scarcer of
        // the two colors first, to leave the more plentiful one free for whatever's left), then
        // a dual credit touching either color, then an "any" wildcard — verifying via
        // `pips_coverable` after each tentative pick so a greedy choice can never strand a
        // later hybrid pip.
        // ponytail: candidates are tried in a fixed priority order rather than an exhaustive (CR 117)
        // search over all sources for all pips at once; correct for any number of hybrid pips
        // via the per-step coverable check, but not necessarily the *only* valid assignment.
        // Every card in the pool today presents at most one hybrid pip per cost (a filter
        // land's activation cost) where ordering can't matter — revisit if a multi-hybrid cost
        // (or a hybrid cost stacked with heavy colored/dual competition) shows this greedy
        // order failing a payment a smarter search would find.
        for &(a, b) in cost.hybrid {
            let hybrid_pair = color_pair_index(a, b);
            hybrid_left[hybrid_pair] -= 1;

            let mono_order = if leftover_colored[a.index()] <= leftover_colored[b.index()] {
                [a, b]
            } else {
                [b, a]
            };
            let mono_pick = mono_order.into_iter().find(|&c| {
                if leftover_colored[c.index()] == 0 {
                    return false;
                }
                let mut remaining = leftover_colored;
                remaining[c.index()] -= 1;
                pips_coverable(
                    shortfall,
                    hybrid_left,
                    remaining,
                    either_left,
                    of_colors_left,
                    any_left,
                )
            });
            if let Some(c) = mono_pick {
                leftover_colored[c.index()] -= 1;
                spend.colored[c.index()] += 1;
                continue;
            }

            let safe_dual = (0..COLOR_PAIRS.len()).find(|&pair| {
                let (x, y) = COLOR_PAIRS[pair];
                if either_left[pair] == 0 || (x != a && x != b && y != a && y != b) {
                    return false;
                }
                let mut remaining = either_left;
                remaining[pair] -= 1;
                pips_coverable(
                    shortfall,
                    hybrid_left,
                    leftover_colored,
                    remaining,
                    of_colors_left,
                    any_left,
                )
            });
            if let Some(pair) = safe_dual {
                either_left[pair] -= 1;
                spend.either[pair] += 1;
                continue;
            }

            let safe_of_colors = (0..of_colors_left.len()).find(|&mask| {
                let touches = mask & (1 << a.index()) != 0 || mask & (1 << b.index()) != 0;
                if of_colors_left[mask] == 0 || !touches {
                    return false;
                }
                let mut remaining = of_colors_left;
                remaining[mask] -= 1;
                pips_coverable(
                    shortfall,
                    hybrid_left,
                    leftover_colored,
                    either_left,
                    remaining,
                    any_left,
                )
            });
            if let Some(mask) = safe_of_colors {
                of_colors_left[mask] -= 1;
                spend.of_colors[mask] += 1;
                continue;
            }

            // The up-front coverable check guarantees a wildcard is free here; guard anyway
            // rather than trust a theorem with an underflow.
            if any_left == 0 {
                return None;
            }
            any_left -= 1;
            spend.any += 1;
        }

        // Colorless {C} pips: only colorless mana can pay these (mana of a color — a dual
        // or "any" credit included — is never colorless).
        if self.colorless < cost.colorless {
            return None;
        }
        spend.colorless = cost.colorless;

        // Generic: pay from any leftover mana (colored, then colorless, then dual/restricted
        // credits, then "any").
        let mut generic = cost.generic;
        for i in 0..Color::COUNT {
            let take = (self.colored[i] - spend.colored[i]).min(generic);
            spend.colored[i] += take;
            generic -= take;
        }
        let take = (self.colorless - spend.colorless).min(generic);
        spend.colorless += take;
        generic -= take;
        for (pair, left) in either_left.iter_mut().enumerate() {
            let take = (*left).min(generic);
            spend.either[pair] += take;
            *left -= take;
            generic -= take;
        }
        for (mask, left) in of_colors_left.iter_mut().enumerate() {
            let take = (*left).min(generic);
            spend.of_colors[mask] += take;
            *left -= take;
            generic -= take;
        }
        let take = any_left.min(generic);
        spend.any += take;
        generic -= take;

        (generic == 0).then_some(spend)
    }

    /// Whether this pool can cover `cost`, given the spell it's paying for (see
    /// [`ManaPool::spend_plan`]).
    pub(crate) fn can_pay(&self, cost: &Cost, spell: Option<SpellCharacteristics>) -> bool {
        self.spend_plan(cost, spell).is_some()
    }
}

/// Move `need` spend-restricted credits of `base` — usable for `spell` — from `pool`'s slots
/// into `spend`'s restricted slots: first-fit across whichever usable slots hold one, an
/// arbitrary but deterministic tie-break among otherwise-equivalent legal choices. `need` is
/// always fully satisfiable: [`ManaPool::spend_plan`] only ever calls this for the portion of a
/// plan that exceeded `pool`'s real (unrestricted) `base` mana, which its `effective` pool
/// folded in from exactly these same usable slots.
fn take_restricted(
    pool: &ManaPool,
    spend: &mut ManaPool,
    base: RestrictedManaBase,
    mut need: u8,
    spell: Option<SpellCharacteristics>,
) {
    for slot in pool.restricted {
        if need == 0 {
            break;
        }
        let Some((slot_base, restriction)) = slot.key else {
            continue;
        };
        if slot_base != base || !spell.is_some_and(|s| restriction.allows(s)) {
            continue;
        }
        let take = slot.amount.min(need);
        spend.add_restricted(slot_base, restriction, take);
        need -= take;
    }
    debug_assert_eq!(
        need, 0,
        "spend_plan's effective pool can't demand more restricted mana than it folded in from these same slots"
    );
}

/// Whether colored-pip `shortfall`s (mono, singleton demand) and `hybrid` pips (`{a/b}`,
/// two-color demand) can be covered by leftover mono `colored` mana plus dual (`either`)
/// credits plus restricted-set (`of_colors`, [`Mana::OfColors`]) credits plus `any` wildcards:
/// Hall's marriage condition on the demand→supply graph — coverable exactly when no set of
/// colors demands more pips than the sources able to pay into it. A demand unit is "inside" a
/// color set only when *all* the colors it accepts are in that set (a mono pip's singleton
/// color, or both of a hybrid pip's colors) — a supply unit counts once *any* of the colors it
/// can produce is in the set (an `of_colors` credit's mask is itself such a set of colors). All
/// 31 subsets of WUBRG, so the check is exact and allocation-free.
fn pips_coverable(
    shortfall: [u8; Color::COUNT],
    hybrid: [u8; COLOR_PAIRS.len()],
    leftover_colored: [u8; Color::COUNT],
    either: [u8; COLOR_PAIRS.len()],
    of_colors: [u8; 1 << Color::COUNT],
    any: u8,
) -> bool {
    for colors in 1u32..(1 << Color::COUNT) {
        let inside = |c: Color| colors & (1 << c.index()) != 0;
        let demanded: u32 = (0..Color::COUNT)
            .filter(|&i| colors & (1 << i) != 0)
            .map(|i| shortfall[i] as u32)
            .sum::<u32>()
            + COLOR_PAIRS
                .iter()
                .zip(hybrid)
                .filter(|&(&(a, b), _)| inside(a) && inside(b))
                .map(|(_, n)| n as u32)
                .sum::<u32>();
        let supplied: u32 = any as u32
            + (0..Color::COUNT)
                .filter(|&i| colors & (1 << i) != 0)
                .map(|i| leftover_colored[i] as u32)
                .sum::<u32>()
            + COLOR_PAIRS
                .iter()
                .zip(either)
                .filter(|&(&(a, b), _)| inside(a) || inside(b))
                .map(|(_, n)| n as u32)
                .sum::<u32>()
            + (0..of_colors.len())
                .filter(|&mask| mask as u32 & colors != 0)
                .map(|mask| of_colors[mask] as u32)
                .sum::<u32>();
        if demanded > supplied {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod mana_pool_tests {
    use super::*;

    #[test]
    fn of_populates_the_requested_credit() {
        let pool = ManaPool::of(Mana::Color(Color::Red), 2);
        assert_eq!(pool.colored[Color::Red.index()], 2);
        assert_eq!(pool.total(), 2);
    }

    #[test]
    fn total_counts_every_credit_kind() {
        let mut pool = ManaPool::default();
        pool.add(Mana::Color(Color::White), 1);
        pool.add(Mana::Colorless, 1);
        pool.add(Mana::Any, 1);
        pool.add(Mana::Either(Color::Blue, Color::Green), 1);
        pool.add(Mana::OfColors(0b0_0110), 1);
        assert_eq!(pool.total(), 5);
    }

    #[test]
    fn subtract_removes_a_payment_multiset() {
        let mut pool = ManaPool::default();
        pool.add(Mana::Color(Color::Blue), 3);
        pool.add(Mana::Colorless, 2);
        pool.add(Mana::Any, 2);
        pool.add(Mana::Either(Color::White, Color::Blue), 2);
        pool.add(Mana::OfColors(0b0_0110), 2);

        let mut spend = ManaPool::default();
        spend.add(Mana::Color(Color::Blue), 2);
        spend.add(Mana::Colorless, 1);
        spend.add(Mana::Any, 1);
        spend.add(Mana::Either(Color::White, Color::Blue), 1);
        spend.add(Mana::OfColors(0b0_0110), 1);
        pool.subtract(&spend);

        assert_eq!(pool.colored[Color::Blue.index()], 1);
        assert_eq!(pool.colorless, 1);
        assert_eq!(pool.any, 1);
        assert_eq!(pool.either[color_pair_index(Color::White, Color::Blue)], 1);
        assert_eq!(pool.of_colors[0b0_0110], 1);
    }

    #[test]
    fn componentwise_min_picks_the_smaller_count_per_bucket() {
        let mut pool = ManaPool::default();
        pool.add(Mana::Color(Color::Red), 3);
        pool.add(Mana::Colorless, 1);

        let mut cap = ManaPool::default();
        cap.add(Mana::Color(Color::Red), 2);
        cap.add(Mana::Any, 5);

        let min = pool.componentwise_min(&cap);
        assert_eq!(
            min.colored[Color::Red.index()],
            2,
            "capped down to the smaller of pool and cap"
        );
        assert_eq!(min.colorless, 0, "absent from the cap, so kept at zero");
        assert_eq!(min.any, 0, "absent from the pool, so kept at zero");
    }

    #[test]
    fn spend_plan_pays_a_colored_pip_from_matching_mana() {
        let pool = ManaPool::of(Mana::Color(Color::Green), 1);
        let cost = Cost {
            colored: {
                let mut pips = [0; Color::COUNT];
                pips[Color::Green.index()] = 1;
                pips
            },
            ..Cost::FREE
        };
        let spend = pool.spend_plan(&cost, None).expect("affordable");
        assert_eq!(spend.colored[Color::Green.index()], 1);
    }

    #[test]
    fn spend_plan_uses_either_credits_for_colored_shortfall() {
        let mut pool = ManaPool::default();
        pool.add(Mana::Either(Color::White, Color::Blue), 1);
        let cost = Cost {
            colored: {
                let mut pips = [0; Color::COUNT];
                pips[Color::White.index()] = 1;
                pips
            },
            ..Cost::FREE
        };
        let spend = pool
            .spend_plan(&cost, None)
            .expect("dual covers the white pip");
        assert_eq!(spend.either[color_pair_index(Color::White, Color::Blue)], 1);
    }

    #[test]
    fn spend_plan_pays_hybrid_pips_from_either_color() {
        let mut pool = ManaPool::default();
        pool.add(Mana::Color(Color::Blue), 1);
        let cost = Cost {
            hybrid: &[(Color::Blue, Color::Green)],
            ..Cost::FREE
        };
        let spend = pool
            .spend_plan(&cost, None)
            .expect("mono blue pays a blue/green hybrid pip");
        assert_eq!(spend.colored[Color::Blue.index()], 1);
    }

    #[test]
    fn spend_plan_pays_generic_from_leftover_colored() {
        let mut pool = ManaPool::default();
        pool.add(Mana::Color(Color::Red), 2);
        let cost = Cost {
            generic: 2,
            ..Cost::FREE
        };
        let spend = pool.spend_plan(&cost, None).expect("two red pays {2}");
        assert_eq!(spend.colored[Color::Red.index()], 2);
    }

    #[test]
    fn spend_plan_returns_none_when_underfunded() {
        let pool = ManaPool::of(Mana::Color(Color::Black), 1);
        let cost = Cost {
            generic: 2,
            ..Cost::FREE
        };
        assert!(pool.spend_plan(&cost, None).is_none());
    }

    #[test]
    fn with_x_folds_chosen_x_into_generic() {
        let cost = Cost {
            generic: 1,
            x: 1,
            ..Cost::FREE
        };
        let paid = cost.with_x(3);
        assert_eq!(paid.generic, 4);
        assert_eq!(paid.x, 0);
    }

    #[test]
    fn with_x_ignores_x_when_cost_has_no_x_symbol() {
        let cost = Cost {
            generic: 2,
            x: 0,
            ..Cost::FREE
        };
        assert_eq!(cost.with_x(99).generic, 2);
    }

    #[test]
    fn is_permutation_accepts_valid_orders() {
        assert!(is_permutation(&[1, 0, 2], 3));
    }

    #[test]
    fn is_permutation_rejects_duplicates_and_out_of_range() {
        assert!(!is_permutation(&[0, 0], 2));
        assert!(!is_permutation(&[2], 2));
        assert!(!is_permutation(&[0, 1], 3));
    }
}

// ── Objects: a card takes a different form (and a *new* [`ObjectId`]) in each zone,
//    matching MTG's rule that a card becomes a new object when it changes zones.
