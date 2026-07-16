use super::*;
#[cfg(feature = "card-dsl")]
use crate::de;

/// A seat at the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(pub u8);

/// A game object (a card in some zone), identified for its lifetime in the game.
pub type ObjectId = u32;

/// The zones a card can occupy. Phase 0 only exercises hand → stack → battlefield → graveyard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Library,
    Hand,
    Battlefield,
    Graveyard,
    Exile,
    Command,
    Stack,
}

/// A step within a turn. Combat's five steps are modelled explicitly so triggers and
/// combat actions have precise timing slots. Untap and Cleanup have no priority window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum Step {
    Untap,
    /// The default [`Effect::ScheduleAtNextUpkeep`] `fire_at` — CR 603.7's "next upkeep".
    #[default]
    Upkeep,
    Draw,
    Main1,
    BeginCombat,
    DeclareAttackers,
    DeclareBlockers,
    /// The extra combat damage step for first/double strikers (CR 510.5); created only when
    /// one is in combat, otherwise skipped so there's exactly one combat damage step.
    FirstStrikeCombatDamage,
    CombatDamage,
    EndCombat,
    Main2,
    End,
    Cleanup,
}

impl Step {
    /// The next step in a turn; after Cleanup the turn passes to the next player's Untap.
    pub(crate) fn next(self) -> Step {
        match self {
            Step::Untap => Step::Upkeep,
            Step::Upkeep => Step::Draw,
            Step::Draw => Step::Main1,
            Step::Main1 => Step::BeginCombat,
            Step::BeginCombat => Step::DeclareAttackers,
            Step::DeclareAttackers => Step::DeclareBlockers,
            Step::DeclareBlockers => Step::FirstStrikeCombatDamage,
            Step::FirstStrikeCombatDamage => Step::CombatDamage,
            Step::CombatDamage => Step::EndCombat,
            Step::EndCombat => Step::Main2,
            Step::Main2 => Step::End,
            Step::End => Step::Cleanup,
            Step::Cleanup => Step::Untap,
        }
    }

    /// Whether players receive priority during this step (all but Untap and Cleanup).
    pub(crate) fn has_priority_window(self) -> bool {
        !matches!(self, Step::Untap | Step::Cleanup)
    }
}

/// The five colors of mana (WUBRG order). Colorless `{C}` is *not* a color (it never
/// enters color identity) — it and "any color" are modelled separately as [`Mana`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum Color {
    White,
    Blue,
    Black,
    Red,
    Green,
}

impl Color {
    /// The number of colors — the width of a mana pool / colored-pip array.
    pub const COUNT: usize = 5;

    /// The five colors in WUBRG order (index `i` has `.index() == i`).
    pub const ALL: [Color; Color::COUNT] = [
        Color::White,
        Color::Blue,
        Color::Black,
        Color::Red,
        Color::Green,
    ];

    /// This color's index into a `[_; Color::COUNT]` pool/cost array (WUBRG).
    pub fn index(self) -> usize {
        match self {
            Color::White => 0,
            Color::Blue => 1,
            Color::Black => 2,
            Color::Red => 3,
            Color::Green => 4,
        }
    }
}

/// When a spell may be cast. Instants cast anytime; sorcery-speed spells only during
/// their controller's main phase, with an empty stack, while they are the active player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellSpeed {
    Instant,
    Sorcery,
}

/// The quality a [`Keyword::ProtectionFrom`] protects against (CR 702.16): a single fixed
/// color (the common case — White Knight, "protection from black"), or a non-color quality —
/// "protection from creatures" (Spirit Mantle, CR 702.16 grants protection from a card type)
/// or "protection from multicolored" (Stonecoil Serpent, CR 105.4's ≥2-colors quality). Kept
/// `Copy` so [`Keyword`]/[`CardDef`] stay `Copy`. In TOML, `{ protection = "<value>" }` where
/// `<value>` is a color name or `"creatures"`/`"multicolored"` — see the `de` module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionScope {
    Color(Color),
    Creatures,
    Multicolored,
}

/// The evergreen keywords that change combat/timing math in the Phase 1 pool.
///
/// In TOML a keyword is a bare string (`"flying"`) or, for the parametrized ones, a
/// single-key table — `{ ward = 2 }` / `{ protection = "red" }` (serde's externally
/// tagged forms).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum Keyword {
    Flying,
    FirstStrike,
    Vigilance,
    Haste,
    Trample,
    Deathtouch,
    /// Can block creatures with flying (CR 702.9).
    Reach,
    /// Can't be blocked except by two or more creatures (CR 702.111).
    Menace,
    /// Deals combat damage in both the first-strike and the normal batch (CR 702.4).
    DoubleStrike,
    /// Damage this deals also causes its controller to gain that much life (CR 702.15).
    Lifelink,
    /// Can't attack (CR 702.3).
    Defender,
    /// Can't be blocked this turn/permanently (a fixed subset of CR 702.10's "unblockable" —
    /// no "except by …" carve-out). Read by [`Game::can_block`].
    Unblockable,
    /// "Destroy" and lethal damage don't destroy this (CR 702.12). A 0-or-less-toughness
    /// SBA still applies — indestructible doesn't save a 0-toughness creature.
    Indestructible,
    /// May be cast any time you could cast an instant (CR 702.8).
    Flash,
    /// Ward {N} (CR 702.21): when an opponent targets this, counter that spell/ability unless
    /// they pay {N} generic. Modeled as a cast-time tax (see [`Game::cast`]).
    Ward(u8),
    /// Protection from a color, card type, or color-count quality (CR 702.16): can't be
    /// blocked/targeted/damaged by a source of that quality. See [`Game::protection_scopes`].
    #[cfg_attr(feature = "card-dsl", serde(rename = "protection"))]
    ProtectionFrom(ProtectionScope),
    /// Can't be the target of spells or abilities *opponents* control (CR 702.11). Its own
    /// controller can still target it. See the target-legality retain in
    /// [`Game::legal_targets_for`].
    Hexproof,
    /// Can't be the target of any spell or ability, even its own controller's (CR 702.18).
    /// See the target-legality retain in [`Game::legal_targets_for`].
    Shroud,
    /// Whenever this creature's controller casts a noncreature spell, it gets +1/+1 until end
    /// of turn (CR 702.108). The whole ability *is* the keyword (CR 702.108a) — see
    /// [`Game::queue_prowess_triggers`] for where it's synthesized rather than authored as a
    /// TOML `[[abilities]]`.
    Prowess,
    /// Can't be blocked by creatures with greater power (CR 702.72a). See [`Game::can_block`].
    Skulk,
    /// Can only block or be blocked by other Shadow creatures (CR 702.28b/c). A *paired*
    /// restriction — it also stops a Shadow creature from blocking a non-Shadow attacker. See
    /// [`Game::can_block`].
    Shadow,
    /// Elusive Otter's printed evasion static ("Creatures with power less than this creature's
    /// power can't block it") — MTG names no keyword for it.
    /// ponytail: modeled as a card-specific keyword-bag arm on the shared block-legality check
    /// rather than new DSL surface for one card.
    LesserPowerCantBlock,
    /// "This creature can't block" (CR 509.1a — Bloodghast is never a legal blocker). Read by
    /// [`Game::can_block`].
    CantBlock,
    /// Brazen Borrower's printed "can block only creatures with flying" static — MTG names no
    /// keyword for it.
    /// ponytail: modeled as a card-specific keyword-bag arm on the shared block-legality check
    /// rather than new DSL surface for one card.
    CanBlockOnlyFlyers,
    /// Decayed (CR 702.148): can't block ([`Game::can_block`]), and "when it attacks, sacrifice
    /// it at the beginning of the end of combat step" (CR 702.148c) — a rules-defined delayed
    /// trigger, scheduled at declare-attackers rather than authored as a token ability. See
    /// [`Game::declare_attackers`].
    Decayed,
    /// Myriad (CR 702.114): "Whenever this creature attacks, for each opponent other than the
    /// defending player, you may create a token copy that's tapped and attacking that player or
    /// a planeswalker they control. Exile the tokens at the end of combat." The whole ability
    /// *is* the keyword (CR 702.114a) — like Prowess, synthesized from the keyword at attack
    /// time rather than authored as a TOML `[[abilities]]`. See
    /// [`Game::queue_myriad_triggers`](crate::Game::queue_myriad_triggers). No pool card prints
    /// this keyword; Muddle, the Ever-Changing grants it to itself temporarily via its magecraft
    /// ability.
    Myriad,
}

/// A small set of the permanent card types a card carries, as a bitset (creature, artifact,
/// enchantment, planeswalker, land). Used two ways: a permanent's *own* types (its [`CardKind`]
/// plus a creature's additional types — see [`CardKind::Creature`]'s `also`), and a
/// [`PermanentFilter`]'s required-type set. Kept `Copy` so [`CardDef`] stays `Copy`.
/// ponytail: no subtypes (Goblin, Aura) — those are #15/#18; this is card *types* only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TypeSet(u8);

impl TypeSet {
    pub const CREATURE: TypeSet = TypeSet(1);
    pub const ARTIFACT: TypeSet = TypeSet(2);
    pub const ENCHANTMENT: TypeSet = TypeSet(4);
    pub const PLANESWALKER: TypeSet = TypeSet(8);
    pub const LAND: TypeSet = TypeSet(16);
    /// The four nonland permanent types — "any nonland permanent."
    pub const NONLAND: TypeSet = TypeSet(1 | 2 | 4 | 8);
    /// No types. As a filter's `types` it means "no restriction"; as a creature's `also` it
    /// means "no additional types." Same bits, read by context.
    pub const NONE: TypeSet = TypeSet(0);

    /// The union of two type sets.
    pub const fn union(self, other: TypeSet) -> TypeSet {
        TypeSet(self.0 | other.0)
    }

    /// Whether the two sets share any type.
    pub fn intersects(self, other: TypeSet) -> bool {
        self.0 & other.0 != 0
    }

    /// Whether this set is empty (a filter with no type restriction).
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// What a card fundamentally *is*. Its behavior lives in [`CardDef::abilities`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardKind {
    /// A vanilla-bodied creature with base power/toughness. `also` carries any *additional*
    /// card types (Artifact Creature, Enchantment Creature) so "is this an artifact?" queries
    /// and artifact-type filters count them (CR 305.4 / #19). Empty for a plain creature.
    Creature {
        power: i32,
        toughness: i32,
        also: TypeSet,
    },
    /// An instant or sorcery — a one-shot spell whose effect resolves off the stack.
    Spell { speed: SpellSpeed },
    /// A noncreature permanent (e.g. an anthem) that stays on the battlefield.
    Enchantment,
    /// An Aura: an enchantment cast targeting a creature that enters *attached* to it
    /// (CR 303.4) and grants it a continuous effect while attached.
    Aura,
    /// A noncreature artifact permanent (mana rocks, equipment bodies, etc.).
    Artifact,
    /// A planeswalker: a permanent that enters with `loyalty` starting loyalty (CR 606.5b) and
    /// whose loyalty abilities are activated at sorcery speed, once per turn (see [`ActivationCost`]).
    Planeswalker { loyalty: i32 },
    /// A land. `produces` is optional sugar for the common "{T}: Add one mana" tap: `Some(m)`
    /// gives the land a free base tap-for-one ([`Game::tap_for_mana`]), while `None` marks a
    /// land with *no* intrinsic mana ability — either a fetch-only land (Prismatic Vista,
    /// Terramorphic Expanse — played only to be sacrificed) or a land whose mana comes entirely
    /// from ordinary `Timing::Activated` `add_mana` abilities (painlands, filter lands: their
    /// modes carry costs — self-damage, an extra mana — a bare `produces` can't express).
    /// `subtypes` carries its printed land types (CR 305 — "Forest", "Island", …; empty for a
    /// land with none, like a check land or an untyped scry land) — the basis for type-specific
    /// search ([`CardFilter::LandWithSubtype`]) and type-gated conditions
    /// ([`Condition::ControlsLandsWithSubtype`]). `basic` is the separate "Basic" supertype (CR
    /// 205.4a) [`is_basic_land`] actually tests: a nonbasic land routinely carries the very same
    /// type strings as a basic (Tangled Islet's "Land — Forest Island") without *being* one, so
    /// basic-ness can't be derived from `subtypes` alone.
    Land {
        produces: Option<LandProduces>,
        subtypes: &'static [&'static str],
        basic: bool,
    },
}

impl CardKind {
    /// The set of card types this permanent has: its intrinsic type plus, for a creature, any
    /// additional types (Artifact/Enchantment Creature). Auras count as enchantments (CR 303).
    /// A [`Spell`](Self::Spell) has no *permanent* type, so it returns the empty set.
    pub(crate) fn types(self) -> TypeSet {
        match self {
            CardKind::Creature { also, .. } => TypeSet::CREATURE.union(also),
            CardKind::Enchantment | CardKind::Aura => TypeSet::ENCHANTMENT,
            CardKind::Artifact => TypeSet::ARTIFACT,
            CardKind::Planeswalker { .. } => TypeSet::PLANESWALKER,
            CardKind::Land { .. } => TypeSet::LAND,
            CardKind::Spell { .. } => TypeSet::NONE,
        }
    }

    /// Whether casting this card is restricted to sorcery speed. Permanents are;
    /// instants are not. (Lands are played, not cast.)
    pub(crate) fn is_sorcery_speed(self) -> bool {
        match self {
            CardKind::Creature { .. }
            | CardKind::Enchantment
            | CardKind::Aura
            | CardKind::Artifact
            | CardKind::Planeswalker { .. }
            | CardKind::Land { .. } => true,
            CardKind::Spell { speed } => speed == SpellSpeed::Sorcery,
        }
    }
}

/// When an ability happens.
// The `Activated(ActivationCost)` variant embeds `Effect` and dwarfs the others, but boxing
// would break `CardDef: Copy`; same tolerated posture as `Effect`/`StackItem`/`StackEntry`.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timing {
    /// The one-shot effect of an instant/sorcery, resolved from the stack.
    Spell,
    /// Triggers on a game event (see [`Trigger`]); goes on the stack when a player
    /// would next receive priority.
    Triggered(Trigger),
    /// Activated by paying a cost (tap and/or mana).
    Activated(ActivationCost),
    /// A continuous static ability.
    Static,
}

/// A card's behavior: an effect gated by a timing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ability {
    pub timing: Timing,
    pub effect: Effect,
    /// The minimum Class level this ability requires to function (CR 717.5 — a Class's
    /// level-gated triggered/static/activated abilities). An ability functions only while its
    /// source permanent's [`Permanent::level`] is at least `min_level`; `0` (every ordinary
    /// ability, and every permanent's trivial "level 1") is unconditional. Checked at each scan
    /// that reads a permanent's abilities — trigger placement ([`Game::queue_trigger_group`]),
    /// the static anthem/cost-reduction recomputes, and the activation gate. A "Level N"
    /// activated ability (an [`Effect::LevelUp`]) keeps `min_level` 0; its own exact-predecessor
    /// gate supersedes this. `min_level = N` in TOML (`#[serde(default)]` 0).
    pub min_level: u8,
    /// Whether this triggered ability is optional ("you may …"): raises a yes/no (or, with a
    /// non-free `cost`, a pay-or-decline) choice before it goes on the stack. An accepted
    /// optional trigger that targets then pauses to choose its target (Sun Titan).
    /// ponytail: only single optional triggers are wired; an optional trigger that is *also* one
    /// of a several-ability simultaneous group grows from a real card (see ADR 0006). (CR 603, CR 601.2c, CR 405)
    pub optional: bool,
    /// The cost to accept an `optional` ability (`Cost::FREE` = a plain "may").
    pub cost: Cost,
    /// An intervening-if condition (CR 603.4): the trigger only goes on the stack when this
    /// holds when it would trigger. `None` for an unconditional trigger.
    pub condition: Option<Condition>,
    /// "This ability triggers only once each turn" (Morbid Opportunist, Tocasia's Welcome, Dina
    /// Essence Brewer's draw ability): caps a *triggered* ability at its first placement per
    /// turn, regardless of how many times its watched event happens. Distinct from
    /// [`ActivationCost::once_each_turn`], which caps an *activated* ability's activations
    /// instead — this field is read only when `timing` is [`Timing::Triggered`]. Checked and
    /// recorded in `Game::place_pending_triggers`; cleared at every untap alongside
    /// `Game::once_each_turn_activated`.
    pub once_each_turn: bool,
}

/// A card definition (identity + behavior). Deserializable (under the `card-dsl` feature)
/// straight from a card's TOML file — the `cards` crate loads the pool this way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardDef {
    pub name: &'static str,
    pub cost: Cost,
    pub kind: CardKind,
    /// An Aura's enchant subject restriction (CR 303.4a — "Enchant creature you control"):
    /// the [`PermanentFilter`] a cast-target/attach candidate must match. `None` (every card
    /// but a restricted Aura) falls back to "any creature" — [`Game::required_target`] and the
    /// [`CardKind::Aura`] resolution re-check both consult this, defaulting to
    /// `PermanentFilter::of(TypeSet::CREATURE)` when unset. Ignored for every non-Aura kind.
    /// `enchant = { … }` in TOML, the same table/shorthand shape as any other `PermanentFilter`.
    /// ponytail: re-attach legality (an Aura moved by another effect) doesn't consult this yet —
    /// no pool Aura re-attaches; wire it through the same filter when one does.
    pub enchant: Option<PermanentFilter>,
    /// Animate Dead's own cast-time enchant target (CR 303.4a's "enchant creature card in a
    /// graveyard"): unlike every other Aura, whose enchant subject is a battlefield permanent
    /// ([`Self::enchant`] above), this card's is a creature *card in a graveyard*, chosen when
    /// it's cast — [`Game::required_target`] reports [`TargetSpec::CreatureCardInAnyGraveyard`]
    /// for it instead of the `CardKind::Aura` battlefield-permanent spec. `false` for every
    /// other card.
    /// ponytail: a bare bool, not a filter — the pool has exactly one such card and its enchant
    /// subject is unrestricted ("a graveyard", not "your graveyard"); promote to a filter/scope
    /// type mirroring `enchant` if a second graveyard-enchanting Aura needs a narrower one. Kind
    /// stays `Enchantment`, not `Aura`: the `CardKind::Aura` resolution path expects a
    /// battlefield host to already exist to attach to, which a graveyard card isn't yet.
    pub enchant_graveyard: bool,
    /// Whether the card is legendary — the only cards that may be a deck's commander.
    /// ponytail: a bare bool, not a supertype set; the pool has no other supertypes yet.
    pub legendary: bool,
    /// "This spell can't be countered" (CR 701.5g, e.g. Altered Ego). Checked in
    /// [`Game::counter_spell`], the shared choke for both the unconditional
    /// [`Effect::CounterTargetSpell`] arm and a declined `PayOrCounter` — the counter fizzles,
    /// the spell stays on the stack.
    pub uncounterable: bool,
    /// Whether this is a modal spell (CR 700.2). When set, the card's `Timing::Spell` abilities
    /// are its *modes* (each ability = one mode) and the caster picks `modal_choose` distinct
    /// modes at cast — only those modes' effects (each with its own target) resolve, in printed
    /// order. A non-modal card runs all its `Timing::Spell` abilities as usual.
    pub modal: bool,
    /// How many distinct modes a modal spell's caster chooses at cast (CR 700.2) — 1 for a
    /// "choose one" Charm, 2 for a "choose two" Command, or the *minimum* of an open "choose one
    /// or more" range when [`modal_choose_max`](Self::modal_choose_max) is set. `choose = N` in
    /// TOML; ignored when `modal` is false.
    pub modal_choose: u8,
    /// The maximum distinct modes a "choose one or more" spell's caster may choose (CR 700.2d) —
    /// `None` means the count is fixed at exactly `modal_choose` (every "choose one"/"choose two"
    /// card). `choose_max = N` in TOML; ignored when `modal` is false.
    /// ponytail: models "one or more" as a min/max *range* only — no entwine/escalate/"choose one,
    /// two, or three" with per-pick riders. Grow those from a card that needs them.
    pub modal_choose_max: Option<u8>,
    /// Gates [`modal_choose_max`](Self::modal_choose_max) on the caster controlling a commander at
    /// cast time (CR 700.2, Nexus Mentality: "if you control a commander as you cast this spell,
    /// you may choose both instead"). When `true`, the `modal_choose_max` range is legal only if
    /// [`Game::controls_a_commander`] holds for the caster; otherwise the count collapses to the
    /// unconditional `modal_choose`. `false` (ignored) for every ordinary "choose one"/"choose one
    /// or more" card. `choose_max_if_commander = true` in TOML.
    /// ponytail: a bare bool, not a general `modal_choose_max_condition: Option<Condition>` — one
    /// pool card needs exactly this gate. Grow a `Condition`-gated max if a second, differently
    /// gated modal card ever lands.
    pub modal_choose_max_if_commander: bool,
    /// The card's intrinsic keywords.
    pub keywords: &'static [Keyword],
    /// Keywords granted only while a `Condition` holds (CR 702 conditional statics —
    /// Primordial Hydra's "has trample as long as it has ten or more +1/+1 counters"), read by
    /// the characteristics recompute alongside `keywords`. Empty for every ordinary card.
    pub conditional_keywords: &'static [(Condition, Keyword)],
    /// The card's abilities. `&'static` keeps `CardDef` `Copy` — loaded card data is
    /// interned to `'static` at deserialization time (see the `de` module).
    pub abilities: &'static [Ability],
    /// Extra colors a card's real rules text carries for color identity (CR 903.4) that the
    /// simplified gameplay model (cost pips, a land's single modeled producer, `AddMana`
    /// effects, activated-ability costs) doesn't otherwise capture — e.g. the dropped half of
    /// a flattened dual/pain/filter land, or a colored activated ability cut entirely. Empty
    /// for ordinary cards. `identity = [...]` in TOML; consumed by `schema::color_identity`.
    pub identity_pips: &'static [Color],
    /// Explicit colors (CR 105.2a: a color indicator, or CR 111.4's "colors are determined by
    /// their text" for a token) overriding the cost-pip derivation in [`color_identity`] — a
    /// token has no mana cost, so its color must be stated outright. `&'static` (keeps `CardDef`
    /// `Copy`); empty (every ordinary card) falls back to deriving color from cost pips as usual.
    /// `colors = ["green"]` / `["white", "black"]` in TOML.
    pub colors: &'static [Color],
    /// Whether this permanent enters the battlefield tapped, *unconditionally* (CR 614.13 — a
    /// replacement effect: it never enters untapped). `enters_tapped = true` in TOML; almost
    /// always a land ("This land enters tapped"). Honored by [`fresh_permanent`] so every entry
    /// path gets it.
    pub enters_tapped: bool,
    /// A conditional enters-tapped gate (check lands, slowlands, reveal lands): this permanent
    /// enters tapped *unless* `Condition` holds, checked once at the same ETB site
    /// `enters_tapped` is (see [`Game::enters_tapped`]). `None` — the common case — falls back to
    /// the unconditional `enters_tapped` flag. Mutually meaningful: a card that needs both
    /// (none currently do) would need a third state; not worth it until one does.
    pub enters_tapped_unless: Option<Condition>,
    /// A one-line plain-English note on how this card's modeled behavior diverges from its
    /// printed rules text (a dropped clause, a coarsened trigger, a folded-together mechanic) —
    /// the same fact a `# ponytail:` TOML comment records, but as a datum the catalog/deck
    /// builder/audits can read instead of hand-kept-in-sync prose. `None` for a faithful card.
    /// `approximates = "…"` in TOML; surfaced verbatim by `schema::catalog_card`.
    pub approximates: Option<&'static str>,
    /// The card's printed (oracle) rules text, verbatim from the printed card — for the deck
    /// builder's read-the-text hover and any human-facing display. Pure catalog metadata; the
    /// engine never parses it (behavior comes from `abilities`/`keywords`). A DFC joins its faces'
    /// text. `oracle = "…"` in TOML; `None` for a card whose text isn't recorded (or a vanilla).
    pub oracle: Option<&'static str>,
    /// The card's set/edition code (Scryfall's lowercase code, e.g. `"soc"`). Pure catalog
    /// metadata — the engine never consults it; it exists so the deck-builder search can match
    /// on set. `set = "…"` in TOML; empty for a card whose set isn't recorded yet.
    pub set: &'static str,
    /// The card's printed subtypes (the segment after the "—": creature types like "Goblin",
    /// "Wizard"; also artifact/enchantment subtypes). Gameplay-relevant, not just catalog
    /// metadata: [`PermanentFilter::subtypes`] and [`Effect::AnthemStatic`]'s `subtypes` axis
    /// both match against this (Goldspan Dragon's "Treasures you control", a tribal anthem). A
    /// *land's* types stay on [`CardKind::Land::subtypes`] (rules use those); `schema::catalog_card`
    /// unions the two for the wire. `subtypes = […]` in TOML; empty when unrecorded or genuinely
    /// none — including most token profiles today (grown card by card as tribal payoffs need them).
    pub subtypes: &'static [&'static str],
    /// Scryfall Tagger oracle-tag slugs (catalog metadata for deck-builder search). Pure catalog
    /// metadata — the engine never reads this at runtime. `otags = […]` in TOML; empty when
    /// unrecorded. Backfilled from Scryfall by `tooling/backfill-otags.mjs`.
    pub otags: &'static [&'static str],
    /// Cycling {N} (CR 702.29a): "{N}, Discard this card: Draw a card," activatable from the
    /// hand. `None` for a card with no cycling. `cycling = { generic = N }` in TOML (the same
    /// `[cost]`-table shape as a spell's cost).
    pub cycling: Option<Cost>,
    /// A hand-activated, discard-this-card ability (CR 113.6/602.5e — an activated ability that
    /// functions only from the hand, whose cost is "Discard this card" plus a mana cost; Magma
    /// Opus's "{U/R}{U/R}, Discard this card: Create a Treasure token."). The general sibling of
    /// [`Self::cycling`] for a card whose from-hand ability has an authored payload rather than
    /// cycling's fixed draw-1 — do not overload `cycling` for this. `None` for a card without one.
    /// `[hand_ability]` in TOML: `[hand_ability.cost]` (same `[cost]`-table shape as a spell's
    /// cost) plus `[[hand_ability.effects]]` (the standard effects-array shape).
    pub hand_ability: Option<HandActivatedAbility>,
    /// Flashback (CR 702.34): "You may cast this card from your graveyard for its flashback cost.
    /// Then exile it." `None` for a card without flashback. `Some(cost)` makes the card castable
    /// from its owner's graveyard for `cost` (an alternative cost, CR 118.9) via [`Game::cast`];
    /// the resolved spell is exiled instead of going to the graveyard (CR 702.34e). The cost may
    /// carry its own `additional` rider (Deep Analysis's `pay_life = 3`). `[flashback]` in TOML,
    /// the same `[cost]`-table shape as a spell's cost.
    pub flashback: Option<Cost>,
    /// Echo (CR 702.31): "At the beginning of your upkeep, if this came under your control since
    /// the beginning of your last upkeep, sacrifice it unless you pay its echo cost." `None` for
    /// a card without echo. `Some(cost)` queues a pay-or-sacrifice choice
    /// ([`PendingChoice::PayEchoOrSacrifice`]) at the controller's first upkeep after the
    /// permanent enters, gated by [`Permanent::echo_unpaid`]. `[echo]` in TOML, the same
    /// `[cost]`-table shape as a spell's cost.
    pub echo: Option<Cost>,
    /// Bestow (CR 702.103 — Eidolon of Countless Battles): a permanent (enchantment) creature card
    /// with an alternative cast mode. `Some(cost)` lets its owner cast it as an *Aura spell with
    /// enchant creature* for `cost` (via [`Game::cast_bestow`]) instead of as a creature spell;
    /// while attached it's an Aura, not a creature (CR 702.103e), and becomes a creature again when
    /// it stops being attached (CR 702.103i — a state-based action). The bestowed status is runtime
    /// state on the resulting [`Spell::bestowed`]/[`Permanent::bestowed`], not the static `def`.
    /// `None` for a card without bestow. `[bestow]` in TOML, the same `[cost]`-table shape as
    /// [`Self::echo`].
    pub bestow: Option<Cost>,
    /// Morph (CR 702.37 — Willbender): "You may cast this card face down as a 2/2 creature for
    /// {3}. Turn it face up any time for its morph cost." `None` for a card without morph.
    /// `Some(cost)` is the card's *morph cost*: casting the card face down instead pays a flat
    /// generic {3} (CR 702.37b — [`Intent::CastFaceDown`]), and this cost is what turns the
    /// resulting face-down permanent face up ([`Game::turn_face_up`], CR 702.37c) rather than the
    /// printed cost a manifest pays. `[morph]` in TOML, the same `[cost]`-table shape as
    /// [`Self::bestow`].
    pub morph: Option<Cost>,
    /// Evoke (CR 702.74 — Mulldrifter): "You may cast this spell for its evoke cost. If you do,
    /// it's sacrificed when it enters." `None` for a card without evoke. `Some(cost)` is the
    /// card's alternative evoke cost, charged instead of the printed `[cost]` when the caster
    /// declares it (CR 702.74a — [`Spell::evoked`]); the resulting permanent is sacrificed the
    /// instant it enters, via a self-sacrifice trigger queued alongside its own ETB triggers so
    /// an ETB payoff (Mulldrifter's draw two) still resolves first (CR 702.74a, CR 603.3b — see
    /// [`Permanent::evoked`]). `[evoke]` in TOML, the same `[cost]`-table shape as [`Self::echo`].
    pub evoke: Option<Cost>,
    /// Delve (CR 702.66): "Each card you exile from your graveyard while casting this spell pays
    /// for {1}." `true` makes the card's cast accept a player-chosen number of graveyard cards to
    /// exile as part of casting (from hand, unlike flashback/escape), each reducing the cast's
    /// generic cost by {1} (floored at 0, CR 601.2f). `delve = true` in TOML; `false` for every
    /// ordinary card.
    pub delve: bool,
    /// Escape (CR 702.19): "You may cast this card from your graveyard for its escape cost. Then
    /// exile [N] other cards from your graveyard." `None` for a card without escape. `Some` makes
    /// the card castable from its owner's graveyard for [`EscapeCost::cost`] (an alternative cost,
    /// CR 118.9) plus exiling [`EscapeCost::exile`] other graveyard cards as an additional cost
    /// (CR 601.2f); the resolved spell is exiled like flashback's (CR 702.19d — only relevant to a
    /// noncreature/nonland escape spell, since a permanent enters the battlefield instead of
    /// leaving the stack for the graveyard). `[escape]` in TOML.
    pub escape: Option<EscapeCost>,
    /// Retrace (CR 702.83): "You may cast this card from your graveyard by discarding a land
    /// card in addition to paying its other costs." `false` for a card without retrace. Unlike
    /// flashback/escape, retrace pays the card's **normal** [`Self::cost`] (not an alternative
    /// cost) plus the discard-a-land additional cost ([`AdditionalCost::discard_land`]), and the
    /// resolved spell is put into the graveyard as usual — no exile rider (CR 702.83a), so it's
    /// repeatable as long as the caster keeps finding lands to discard. `retrace = true` in TOML.
    pub retrace: bool,
    /// Cast-from-graveyard alternative cost for a permanent (CR 118.9, Raffine's Guidance):
    /// "You may cast this card from your graveyard by paying [cost] rather than paying its mana
    /// cost." `None` for a card without this permission. Unlike flashback/escape, the card is a
    /// permanent — it resolves normally onto the battlefield, no exile rider (a permanent never
    /// reaches the graveyard-or-exile fork those alternative costs gate). Distinct from retrace:
    /// this *replaces* the printed cost rather than adding an additional cost on top of it.
    /// `[graveyard_cast_cost]` in TOML, the same `[cost]`-table shape as a spell's cost.
    pub graveyard_cast_cost: Option<Cost>,
    /// Cascade (CR 702.85): "When you cast this spell, exile cards from the top of your library
    /// until you exile a nonland card that costs less. You may cast it without paying its mana
    /// cost. Put the exiled cards on the bottom of your library in a random order." `false` for a
    /// card without cascade. A rules-keyword (not a `[[abilities]]`): a `true` flag places an
    /// [`Effect::Cascade`](crate::Effect::Cascade) triggered ability on the stack above the
    /// cascading spell when it's cast (CR 702.85e), wired at the cast choke like `retrace`/`echo`.
    /// `cascade = true` in TOML.
    pub cascade: bool,
    /// Demonstrate (CR 702.147): "When you cast this spell, you may copy it. If you do, choose an
    /// opponent to also copy it. Players may choose new targets for their copies." `false` for a
    /// card without demonstrate. A rules-keyword (not a `[[abilities]]`): a `true` flag fabricates
    /// an [`Effect::Demonstrate`](crate::Effect::Demonstrate) triggered ability on the stack above
    /// the cast spell (CR 702.147a), wired at the cast choke like `cascade`. `demonstrate = true`
    /// in TOML.
    pub demonstrate: bool,
    /// Devour N (CR 702.82): "As this creature enters, you may sacrifice any number of creatures.
    /// It enters with N +1/+1 counters on it for each creature sacrificed this way." `Some(N)`
    /// carries the multiplier (Mycoloth's 2, Ribtruss Roaster's 1); `None` for a card without
    /// devour. A rules-keyword (not a `[[abilities]]`): as the creature enters it pauses on a
    /// [`PendingChoice::Devour`](crate::PendingChoice::Devour) so its controller may sacrifice a
    /// subset of the creatures they control, then it gains `N × count` +1/+1 counters routed
    /// through [`Game::counters_after_replacements`] so CR 614 doublers apply. `devour = N` in TOML.
    /// ponytail: modeled as an as-enters *step* (counters placed after the entry) rather than a
    /// true CR 614.13 replacement (counters present the instant it enters, before any ETB trigger
    /// could read them). Not observable for the pool — both devour cards read their counters at a
    /// later upkeep/end step. Upgrade to a real replacement hook when a devour card fields an ETB
    /// that reads its own devour counters.
    pub devour: Option<u32>,
    /// Whether this card's *triggered* abilities function while it sits in its owner's graveyard,
    /// rather than from the battlefield (CR 603.6e — Squee's upkeep self-return, Nether Traitor's
    /// death-watch self-reanimation). `functions_in_graveyard = true` in TOML; `false` for every
    /// ordinary card (triggers fire only from play).
    /// ponytail: whole-card flag — assumes *every* triggered ability on the card is graveyard-only
    /// (true for Squee/Nether Traitor; Anger's *static* haste anthem is out of scope, a separate
    /// #53 static slice). A card mixing battlefield and graveyard abilities would need per-ability
    /// zone tags — defer until one exists. (CR 603, CR 108.4, CR 403.5)
    pub functions_in_graveyard: bool,
    /// A "prepare" double-faced card's back face (soc/sos — CR-style): the front creature has an
    /// ability that makes it "become prepared" (a [`Permanent::prepared`] status), and while
    /// prepared its controller may cast a copy of this back-face spell (see [`Game::cast_prepared`]),
    /// which unprepares it. `None` for every ordinary card. A `&'static CardDef` (not a nested
    /// `CardDef` by value) so [`CardDef`] stays `Copy` and finitely sized — the back def is leaked
    /// to `'static` at load, like the rest of the interned card data (see the `de` module).
    /// `[back]` (an inline `CardDef` table) in TOML.
    pub back: Option<&'static CardDef>,
    /// An adventure card's adventure half (CR 715 — soc/sos): the front face is the creature
    /// (this `CardDef`), and its `adventure` holds the instant/sorcery spell you may cast from
    /// hand instead (its own `cost`, `kind`, and `abilities`). On resolution the card is exiled
    /// "on an adventure" (CR 715.3d) and its owner may cast the creature half from exile later at
    /// normal cost (see [`Game::cast_adventure`]). `None` for every ordinary card. A
    /// `&'static CardDef` (not a nested `CardDef` by value), like [`Self::back`], so [`CardDef`]
    /// stays `Copy`. `[adventure]` (an inline `CardDef` table) in TOML.
    pub adventure: Option<&'static CardDef>,
    /// Suspend N—[cost] (CR 702.62 — Rousing Refrain): "Rather than cast this card from your
    /// hand, you may pay [cost] and exile it with N time counters on it." `None` for a card
    /// without suspend. A rules-keyword (not a `[[abilities]]`): a `Some` lets its owner pay
    /// [`Suspend::cost`] to exile the card from hand with [`Suspend::counters`] time counters (see
    /// [`Game::suspend`]); a time counter is removed at each of the owner's upkeeps (CR 702.62d),
    /// and when the last is removed the owner may cast it from exile without paying its mana cost
    /// (CR 702.62e). `[suspend]` in TOML.
    pub suspend: Option<Suspend>,
    /// Enter-as-a-copy replacement (CR 706/707.2), carried as a rules-keyword marker rather than a
    /// `[[abilities]]` (like [`Self::devour`]): as this permanent enters, its controller may have
    /// it enter as a copy of any creature on the battlefield, with the riders in [`EnterAsCopy`]
    /// (Altered Ego's X extra +1/+1 counters; Cursed Mirror's until-end-of-turn duration + haste).
    /// The pause fires at the enter event, before ETB triggers (see `Game::begin_enter_as_copy`).
    /// `None` for a card without the replacement. `enter_as_copy = { .. }` in TOML.
    pub enter_as_copy: Option<EnterAsCopy>,
    /// Encore [cost] (CR 702.140 — Angel of Indemnity): "[cost], Exile this card from your
    /// graveyard: For each opponent, create a token copy of this card that attacks that opponent
    /// this turn if able. They gain haste. Sacrifice them at the beginning of the next end step.
    /// Activate only as a sorcery." `None` for a card without encore. A rules-keyword (not a
    /// `[[abilities]]`): a `Some` holds the encore **mana** cost; the "exile this card from your
    /// graveyard" half of the cost is intrinsic to the activation (paid by [`Game::encore`], not
    /// stored as a pip). A `&'static Cost` (leaked at load, like [`Self::suspend`]'s cost) so
    /// [`CardDef`] stays `Copy`. `[encore]` in TOML, the same `[cost]`-table shape as a spell's cost.
    pub encore: Option<&'static Cost>,
    /// "You may choose not to untap this during your untap step" (CR 502.2 — Rubinia Soulsinger):
    /// the untap turn-based action pauses this permanent's controller on a yes/no for each such
    /// permanent they control, letting them leave it tapped ([`PendingChoice::DeclineUntap`]).
    /// `false` for every ordinary permanent. `may_choose_not_to_untap = true` in TOML.
    pub may_choose_not_to_untap: bool,
}

/// The riders on an [`CardDef::enter_as_copy`] replacement (CR 706/707.2). `Copy` — all scalars,
/// no `Vec` — so [`CardDef`] stays `Copy`. `until_eot` reverts the copy at cleanup (Cursed Mirror,
/// [`Permanent::reverts_to_def_eot`]); `extra_counters` are additional +1/+1 counters the copy
/// enters with (Altered Ego's X); `gains_haste` grants the copy haste (Cursed Mirror's "except it
/// has haste").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(deny_unknown_fields)
)]
pub struct EnterAsCopy {
    #[cfg_attr(feature = "card-dsl", serde(default))]
    pub until_eot: bool,
    #[cfg_attr(feature = "card-dsl", serde(default = "de::zero_amount"))]
    pub extra_counters: Amount,
    #[cfg_attr(feature = "card-dsl", serde(default))]
    pub gains_haste: bool,
}

/// Suspend N—[cost] (CR 702.62), carried by [`CardDef::suspend`]. `counters` is the N time
/// counters the card is exiled with; `cost` is the alternative cost paid to suspend it. `cost` is
/// a `&'static Cost` (leaked at load, like the rest of the interned card data) so [`CardDef`] stays
/// `Copy` and finitely sized — [`Cost`] embeds an [`AdditionalCost`] rider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(deny_unknown_fields)
)]
pub struct Suspend {
    pub counters: u32,
    #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::leaked_cost"))]
    pub cost: &'static Cost,
}

/// A hand-activated, discard-this-card ability (CR 113.6/602.5e), carried by
/// [`CardDef::hand_ability`] — the general sibling of [`CardDef::cycling`] for a from-hand
/// ability whose payload is authored rather than a fixed draw-1. `cost` is the mana paid
/// alongside "Discard this card" (the rest of the cost, like cycling's); `effects` runs in order
/// when the ability resolves. `[hand_ability]` in TOML.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(deny_unknown_fields)
)]
pub struct HandActivatedAbility {
    pub cost: Cost,
    #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_slice"))]
    pub effects: &'static [Effect],
}

impl CardDef {
    /// This card's mana value (CR 202.3): the total pips in its mana cost — generic plus every
    /// colored, colorless `{C}`, and hybrid `{A/B}` pip. A `{X}` counts as 0 outside the stack
    /// (CR 202.3b), which is exactly how [`Cost`] stores it (the `x` marker adds nothing to the
    /// printed pips), so a graveyard/battlefield mana-value gate reads the printed value
    /// correctly. Each color/color hybrid pip counts 1 (CR 202.3f — both halves are one mana;
    /// Balefire Liege's {2}{R/W}{R/W}{R/W} is mana value 5).
    pub fn mana_value(self) -> u32 {
        let cost = self.cost;
        cost.generic as u32
            + cost.colorless as u32
            + cost.colored.iter().map(|&pips| pips as u32).sum::<u32>()
            + cost.hybrid.len() as u32
    }

    /// Whether this card may be cast any time its owner has priority — an instant, or a
    /// spell with flash (CR 702.8a). The single timing predicate shared by the cast gate
    /// ([`Game::cast`]) and auto-pass ([`Game::meaningful_actions`]), so a future
    /// "as though it had flash" effect can't teach one site and not the other.
    pub(crate) fn is_instant_speed(&self) -> bool {
        !self.kind.is_sorcery_speed() || self.keywords.contains(&Keyword::Flash)
    }

    /// The facts about this card, as a spell being cast, that a [`SpendRestriction`] checks —
    /// derived fresh at each [`ManaPool::spend_plan`] call site rather than stored. `mana_value`
    /// reads the printed cost (CR 202.3b treats `{X}` as 0 off the stack), which is safe here
    /// because [`SpendRestriction::ManaValueAtLeastOrHasX`] always also accepts `has_x`
    /// regardless of the value actually chosen for `{X}`.
    pub(crate) fn spell_characteristics(self) -> SpellCharacteristics {
        SpellCharacteristics {
            mana_value: self.mana_value(),
            has_x: self.cost.x > 0,
            is_instant_or_sorcery: matches!(self.kind, CardKind::Spell { .. }),
        }
    }
}

/// A card's *colors* (CR 105.2a: mana-cost pips, or `def.colors` for a card whose color a cost
/// can't express — a color indicator, or a token's stated color) — used internally for
/// protection and color-based target filtering (`colors_of`, `legal_targets_for`).
/// ponytail: cost-only (plus the explicit override) is correct forever for that use, not a
/// placeholder for full CR 903.4 color identity — Commander deck-identity validation lives in
/// `schema::color_identity` (crates/schema/src/lib.rs), which `server::legality::validate`
/// checks against the pool; `def.colors` never affects deck legality (no pool token is
/// deck-legal, and no real card sets it).
pub fn color_identity(def: CardDef) -> [bool; Color::COUNT] {
    if !def.colors.is_empty() {
        let mut identity = [false; Color::COUNT];
        for &color in def.colors {
            identity[color.index()] = true;
        }
        return identity;
    }
    let mut identity = [false; Color::COUNT];
    for (slot, &pips) in identity.iter_mut().zip(def.cost.colored.iter()) {
        *slot = pips > 0;
    }
    // A hybrid pip (CR 107.4e, {A/B}) contributes to both of its colors (CR 105.2b) — Balefire
    // Liege's {R/W} pips make it both red and white.
    for &(a, b) in def.cost.hybrid {
        identity[a.index()] = true;
        identity[b.index()] = true;
    }
    identity
}

/// Whether `card` is legal in a deck led by `commander` — its color identity must be a
/// subset of the commander's.
pub fn within_identity(card: CardDef, commander: CardDef) -> bool {
    let allowed = color_identity(commander);
    let needed = color_identity(card);
    (0..Color::COUNT).all(|i| !needed[i] || allowed[i])
}

/// Whether `def` is a basic land: has the "Basic" supertype (CR 205.4a). Reads
/// [`CardKind::Land`]'s `basic` flag rather than the card's name (or its `subtypes`, which a
/// nonbasic land can share without being basic — see the field's doc).
pub(crate) fn is_basic_land(def: CardDef) -> bool {
    matches!(def.kind, CardKind::Land { basic: true, .. })
}

/// A permanent entering the battlefield: all per-object state at its defaults.
pub(crate) fn fresh_permanent(
    def: CardDef,
    owner: PlayerId,
    summoning_sick: bool,
    commander: bool,
) -> Permanent {
    Permanent {
        def,
        owner,
        level: 1,
        tapped: def.enters_tapped,
        summoning_sick,
        entered_this_turn: true,
        plus_counters: 0,
        kind_counters: [0; CounterKind::COUNT],
        temp_power: 0,
        temp_toughness: 0,
        base_pt_set_eot: None,
        added_types_eot: TypeSet::NONE,
        added_subtypes_eot: &[],
        added_colors_eot: &[],
        temp_keywords: &[],
        temp_lost_keywords: &[],
        set_base_pt: None,
        added_types: TypeSet::NONE,
        added_subtypes: &[],
        granted_keywords: &[],
        marked_damage: 0,
        deathtouched: false,
        commander,
        token: false,
        attached_to: None,
        loyalty: starting_loyalty(def),
        loyalty_activated: false,
        finality_counter: false,
        regeneration_shields: 0,
        prepared: false,
        echo_unpaid: def.echo.is_some(),
        chosen_subtype: None,
        chosen_color: None,
        entered_with_x: 0,
        cast_time_enchant_target: None,
        vow_protected: None,
        phased_out: false,
        serra_recursion: false,
        bestowed: false,
        face_down: false,
        evoked: false,
        reverts_to_def_eot: None,
        spent_colors: [false; Color::COUNT],
    }
}

/// A planeswalker's printed starting loyalty (CR 606.5b — it enters with that many loyalty
/// counters); 0 for any other card kind.
pub(crate) fn starting_loyalty(def: CardDef) -> i32 {
    match def.kind {
        CardKind::Planeswalker { loyalty } => loyalty,
        _ => 0,
    }
}

/// A token entering the battlefield: like [`fresh_permanent`], but flagged as a token
/// (ceases to exist when it leaves the battlefield) and summoning-sick.
pub(crate) fn fresh_token(def: CardDef, controller: PlayerId) -> Permanent {
    Permanent {
        token: true,
        ..fresh_permanent(def, controller, true, false)
    }
}

/// The canonical Treasure token (CR: Treasure): a colorless artifact token with
/// "{T}, Sacrifice this artifact: Add one mana of any color." Every "create a Treasure" path
/// mints from this one definition. The `any` mana it adds is a wildcard that pays any single
/// colored pip or generic (see [`Mana::Any`]). Carries the "Treasure" subtype so a
/// [`PermanentFilter`] can pick Treasures out from any other artifact (Goldspan Dragon's
/// "Treasures you control" grant, #57).
pub fn treasure_token() -> CardDef {
    const ABILITIES: &[Ability] = &[Ability {
        timing: Timing::Activated(ActivationCost {
            taps_self: true,
            mana: Cost::FREE,
            sacrifice: SacrificeCost::This,
            pay_life: Amount::Fixed(0),
            self_damage: 0,
            loyalty: None,
            once_each_turn: false,
            sorcery_speed: false,
            remove_counters: 0,
            remove_counters_kind: None,
            return_self: false,
            mill_self: 0,
            exile_self: false,
        }),
        effect: Effect::AddMana {
            mana: ManaPool {
                colored: [0; Color::COUNT],
                colorless: 0,
                any: 1,
                either: [0; COLOR_PAIRS.len()],
                of_colors: [0; 1 << Color::COUNT],
                restricted: [RestrictedSlot {
                    key: None,
                    amount: 0,
                }; RESTRICTED_SLOTS],
            },
            identity: 0,
            opponent_colors: 0,
            repeat: Amount::Fixed(1),
            restriction: None,
            single_color: false,
            track_provenance: false,
            target: TargetSpec::None,
            persist_until_end_of_turn: false,
        },
        optional: false,
        min_level: 0,
        cost: Cost::FREE,
        condition: None,
        once_each_turn: false,
    }];
    CardDef {
        name: "Treasure",
        cost: Cost::FREE,
        kind: CardKind::Artifact,
        legendary: false,
        uncounterable: false,
        modal: false,
        modal_choose: 1,
        modal_choose_max: None,
        modal_choose_max_if_commander: false,
        keywords: &[],
        conditional_keywords: &[],
        abilities: ABILITIES,
        identity_pips: &[],
        colors: &[],
        enters_tapped: false,
        enters_tapped_unless: None,
        approximates: None,
        oracle: None,
        set: "",
        subtypes: &["Treasure"],
        otags: &[],
        cycling: None,
        flashback: None,
        echo: None,
        bestow: None,
        morph: None,
        evoke: None,
        delve: false,
        escape: None,
        retrace: false,
        graveyard_cast_cost: None,
        cascade: false,
        functions_in_graveyard: false,
        enchant: None,
        enchant_graveyard: false,
        back: None,
        adventure: None,
        suspend: None,
        devour: None,
        demonstrate: false,
        enter_as_copy: None,
        encore: None,
        hand_ability: None,
        may_choose_not_to_untap: false,
    }
}

/// Currency Converter's cash-out payoff for a nonland card: a 2/2 black Rogue creature token
/// (CR 400.10a).
pub(crate) fn rogue_token_stub() -> CardDef {
    CardDef {
        name: "Rogue",
        cost: Cost::FREE,
        kind: CardKind::Creature {
            power: 2,
            toughness: 2,
            also: TypeSet::NONE,
        },
        legendary: false,
        uncounterable: false,
        modal: false,
        modal_choose: 1,
        modal_choose_max: None,
        modal_choose_max_if_commander: false,
        keywords: &[],
        conditional_keywords: &[],
        abilities: &[],
        identity_pips: &[],
        colors: &[Color::Black],
        enters_tapped: false,
        enters_tapped_unless: None,
        approximates: None,
        oracle: None,
        set: "",
        subtypes: &["Rogue"],
        otags: &[],
        cycling: None,
        flashback: None,
        echo: None,
        bestow: None,
        morph: None,
        evoke: None,
        delve: false,
        escape: None,
        retrace: false,
        graveyard_cast_cost: None,
        cascade: false,
        functions_in_graveyard: false,
        enchant: None,
        enchant_graveyard: false,
        back: None,
        adventure: None,
        suspend: None,
        devour: None,
        demonstrate: false,
        enter_as_copy: None,
        encore: None,
        hand_ability: None,
        may_choose_not_to_untap: false,
    }
}

/// Skyclave Apparition's leaves-battlefield payoff: a blue Illusion creature token, base power
/// and toughness 0/0. The caller (`Game::check_leaves_battlefield_illusions`) bakes in the
/// exiled card's mana value as base P/T before minting, the same way `Effect::CreateToken`'s
/// `set_base_pt` does.
pub(crate) fn illusion_token() -> CardDef {
    CardDef {
        name: "Illusion",
        cost: Cost::FREE,
        kind: CardKind::Creature {
            power: 0,
            toughness: 0,
            also: TypeSet::NONE,
        },
        legendary: false,
        uncounterable: false,
        modal: false,
        modal_choose: 1,
        modal_choose_max: None,
        modal_choose_max_if_commander: false,
        keywords: &[],
        conditional_keywords: &[],
        abilities: &[],
        identity_pips: &[],
        colors: &[Color::Blue],
        enters_tapped: false,
        enters_tapped_unless: None,
        approximates: None,
        oracle: None,
        set: "",
        subtypes: &["Illusion"],
        otags: &[],
        cycling: None,
        flashback: None,
        echo: None,
        bestow: None,
        morph: None,
        evoke: None,
        delve: false,
        escape: None,
        retrace: false,
        graveyard_cast_cost: None,
        cascade: false,
        functions_in_graveyard: false,
        enchant: None,
        enchant_graveyard: false,
        back: None,
        adventure: None,
        suspend: None,
        devour: None,
        demonstrate: false,
        enter_as_copy: None,
        encore: None,
        hand_ability: None,
        may_choose_not_to_untap: false,
    }
}

/// A card at rest in a hidden/graveyard/command zone: identity only, no battlefield state.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Card {
    pub(crate) def: CardDef,
    pub(crate) owner: PlayerId,
    /// One of Library / Hand / Graveyard / Exile / Command.
    pub(crate) zone: Zone,
    /// Whether this is (a form of) its owner's commander — carried across zone changes.
    pub(crate) commander: bool,
    /// Whether this exiled card is face down and hidden from every viewer but `owner` (CR
    /// 701.9 — Abstract Performance's first pile: exiled face-down while its opponent-chooser
    /// pauses on which pile to take). `false` for every ordinarily-visible card in a
    /// hidden/graveyard/command/exile zone. A fresh object is minted on every zone change (see
    /// [`Object::Moved`]'s doc), so this clears for free the moment the card leaves the pile —
    /// there is no separate "reveal" event.
    pub(crate) face_down: bool,
}

/// A spell on the stack (a cast card waiting to resolve).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Spell {
    pub(crate) def: CardDef,
    pub(crate) controller: PlayerId,
    /// The chosen targets (CR 601.2c). A single-target spell fills one slot; Aether Gale fills up
    /// to six. Empty until a multi-target spell's targets are chosen (see [`Event::SpellTargetsChosen`]).
    /// This is the spell's *first* independent target clause (clause 0); [`Self::targets_second`]
    /// holds a second one.
    pub(crate) targets: TargetList,
    /// A *second* independent target clause's chosen targets (CR 601.2c — Magma Opus's "Tap two
    /// target permanents" alongside its divided-damage clause). Filled by a second
    /// [`Event::SpellTargetsChosen`] with `clause == 1`; empty for the single-clause majority.
    /// ponytail: exactly two independent target clauses per spell — clause 0 in `targets`, clause 1
    /// here. No pool spell prints three; add a `[TargetList; N]` array (with `MAX_TARGET_CLAUSES`)
    /// if one ever does.
    pub(crate) targets_second: TargetList,
    pub(crate) commander: bool,
    /// The chosen `{X}` value, read by X-scaled effects at resolution (0 if the spell has no `{X}`).
    pub(crate) x: u32,
    /// A modal spell's chosen modes (CR 700.2), each with its own target. An empty selection for
    /// a non-modal spell (which uses `target` and runs every effect).
    pub(crate) modes: Modes,
    /// Whether this spell is a *copy* (CR 707.10) rather than a cast card: it was put on the
    /// stack by a copy effect (Twincast), pays no cost, and ceases to exist when it resolves
    /// instead of going to a graveyard.
    pub(crate) copy: bool,
    /// Whether this spell was cast with flashback (CR 702.34): from the graveyard for its
    /// flashback cost. When set, the resolved spell is exiled instead of moved to the graveyard
    /// (CR 702.34e). A copy of a flashback spell inherits the flag but ceases to exist first, so
    /// it never reaches the exile branch.
    pub(crate) flashback: bool,
    /// Whether this spell was cast via escape (CR 702.19): from the graveyard for its escape
    /// cost, exiling other graveyard cards as an additional cost. Mirrors [`Self::flashback`]'s
    /// exile-on-resolve treatment for a noncreature/nonland escape spell (CR 702.19d); a creature
    /// or Aura escape spell instead becomes a permanent and never reaches that branch.
    pub(crate) escape: bool,
    /// Whether this spell was cast from its controller's hand (CR 601's default cast zone) —
    /// `false` for a flashback/escape/retrace cast from a graveyard, a commander cast from the
    /// command zone, or an impulse-play permission cast from exile. Feeds
    /// [`Trigger::CastSpell`]'s `from_hand` gate (Dirgur Focusmage's "you cast … from your
    /// hand"); read at [`Event::SpellCast`] apply time off the source card's zone before it
    /// moves to the stack (see `apply.rs`).
    pub(crate) cast_from_hand: bool,
    /// CR 601.2d's damage division for a `divided: true` `Effect::DealDamage` on this spell
    /// (Magma Opus's "4 damage divided as you choose"): `(target, assigned amount)` pairs,
    /// settled right after `targets` above by [`Game::maybe_begin_damage_division`]. Empty for a
    /// spell with no divided-damage effect. Reuses [`DamageAssignment`], the same `Copy`
    /// division shape combat's [`Event::CombatDamageDivided`] uses (CR 510.1c) — a divided
    /// spell's targets are always permanents (see [`Effect::DealDamage`]'s doc), so the same
    /// `ObjectId`-keyed shape fits without a parallel type.
    pub(crate) damage_division: DamageAssignment,
    /// CR 601.2d's *player* shares of a `divided: true` `Effect::DealDamage`'s division (Magma
    /// Opus's "any number of targets" includes players): `(player, assigned amount)` pairs,
    /// settled alongside [`Self::damage_division`] by [`Game::maybe_begin_damage_division`]. A
    /// separate fixed `Copy` array (not `DamageAssignment`, which is `ObjectId`-keyed and shared
    /// with combat — a player isn't an object) so `Spell` stays `Copy`; `[None; MAX_TARGETS]` for a
    /// spell with no player among its divided targets.
    pub(crate) damage_division_players: [Option<(PlayerId, i32)>; MAX_TARGETS],
    /// CR 601.2d's counter division for a `divided: true` `Effect::PutCounters` on this spell
    /// (Grove's Bounty's "Distribute X +1/+1 counters among any number of target creatures you
    /// control"): `(target, assigned count)` pairs, settled right after `targets` above by
    /// [`Game::maybe_begin_counter_division`]. Empty for a spell with no divided-counters effect.
    /// Reuses [`DamageAssignment`], the same `Copy` division shape [`Self::damage_division`] uses
    /// — a divided spell's targets are always permanents, so the same `ObjectId`-keyed shape fits.
    pub(crate) counter_division: DamageAssignment,
    /// How many permanents were sacrificed to pay [`AdditionalCost::sacrifice`] (CR 601.2f —
    /// Plumb the Forbidden's "you may sacrifice one or more creatures"), 0 if the spell has no
    /// such cost or the caster declined. Read by a copy-per-sacrifice rider once one exists (no
    /// pool card reads it yet); recorded here the way `x` is, for the same reason.
    pub(crate) sacrifice_count: u8,
    /// Whether the caster paid this spell's kicker cost (CR 702.33d — [`AdditionalCost::kicker`]),
    /// `false` for a spell with no kicker or a decline. Read by [`Amount::IfSpellKicked`] (Rite
    /// of Replication's "If this spell was kicked, create five of those tokens instead") via
    /// [`Game::spell_was_kicked`], the kicked-flag sibling of [`Self::sacrifice_count`]'s read.
    pub(crate) kicked: bool,
    /// Whether the caster paid this spell's buyback cost (CR 702.27c — [`AdditionalCost::buyback`]),
    /// `false` for a spell with no buyback or a decline. Read by
    /// [`Game::finish_instant_sorcery_resolution`] (Capsize's "put this card into your hand as it
    /// resolves" instead of the graveyard), the buyback-flag sibling of [`Self::kicked`]'s read.
    pub(crate) bought_back: bool,
    /// The caster's declared Strive target count (CR 702.42 — [`AdditionalCost::strive`]), 0 if
    /// the spell has no Strive cost. Settled before the spell hits the stack (CR 601.2c precedes
    /// 601.2f) and recorded here the way `sacrifice_count`/`kicked` are; read back by
    /// [`TargetCount::strive_scaled`]'s cast-time target-count substitution in
    /// [`Game::choose_spell_targets`](crate::Game::choose_spell_targets).
    pub(crate) strive_count: u8,
    /// How many times the caster paid this spell's Replicate cost (CR 702.108 —
    /// [`AdditionalCost::replicate`]), 0 if the spell has no Replicate cost or the caster paid it
    /// zero times. Settled before the spell hits the stack (CR 601.2b) and recorded here the way
    /// `strive_count` is; read at the [`Event::SpellCast`] choke to mint that many copies via
    /// [`Game::mint_spell_copies`] (CR 702.108b).
    pub(crate) replicate_count: u8,
    /// Whether this spell was cast from a graveyard under Serra Paragon's permission (CR 118.9 —
    /// [`Effect::PlayFromGraveyardOncePerTurn`]). Copied onto the resulting
    /// [`Permanent::serra_recursion`] when the spell resolves ([`Event::PermanentEntered`]), so the
    /// recurred permanent carries the granted "exile-and-gain-2-life" rider. `false` for any other
    /// cast (from hand, flashback, escape, …).
    pub(crate) serra_recursion: bool,
    /// Whether this spell was cast via bestow (CR 702.103 — Eidolon of Countless Battles): for its
    /// [`CardDef::bestow`] cost, as an Aura spell with enchant creature. When set, the spell
    /// resolves through the Aura attach path ([`Game::resolve_spell`]) rather than entering as a
    /// creature, and the resulting permanent carries [`Permanent::bestowed`]. `false` for an
    /// ordinary creature cast.
    pub(crate) bestowed: bool,
    /// Whether this spell was cast face down (CR 702.37b — a morph cast, [`Intent::CastFaceDown`]):
    /// a 2/2 colorless creature spell whose real characteristics are hidden. Copied onto the
    /// resulting [`Permanent::face_down`] when the spell resolves ([`Event::PermanentEntered`]),
    /// so the permanent enters face down (CR 708). `false` for an ordinary face-up cast.
    pub(crate) face_down: bool,
    /// Whether this spell was cast for its evoke cost (CR 702.74a — [`CardDef::evoke`]). Copied
    /// onto the resulting [`Permanent::evoked`] when the spell resolves ([`Event::PermanentEntered`]),
    /// so the permanent is sacrificed the instant it enters. `false` for an ordinary cast.
    pub(crate) evoked: bool,
    /// The colors of mana actually spent to cast this spell (CR 106.9 — Court Hussar's "unless
    /// {W} was spent to cast it"), snapshotted from [`ManaPool::colors_spent`] against the
    /// [`Event::ManaSpent`] [`Game::settle_payment`](crate::Game::settle_payment) appends right
    /// before this spell hits the stack. Copied onto the resulting [`Permanent::spent_colors`]
    /// when the spell resolves, the same "read the spell's own info before it's gone" idiom as
    /// `entered_with_x`. `[false; Color::COUNT]` for a spell that paid no mana (a copy, a free
    /// cast) or a cast form (adventure, prepared copy) this snapshot isn't wired through yet — no
    /// pool card checks color-spent off those forms.
    pub(crate) spent_colors: [bool; Color::COUNT],
}

/// A permanent on the battlefield, with its mutable per-object state.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Permanent {
    pub(crate) def: CardDef,
    pub(crate) owner: PlayerId,
    /// This permanent's Class level (CR 717.4 — a Class enchantment's level counter). Raised one
    /// step at a time by [`Effect::LevelUp`] (via [`Event::LeveledUp`]); read by every
    /// level-gated ability's [`Ability::min_level`] check. Runtime state, not TOML-authored —
    /// **defaults to 1** at every construction site (a Class enters at level 1; every ordinary
    /// permanent is trivially level 1, so a `min_level = 0/1` ability always functions). Not
    /// wire-mirrored (like `finality_counter`/`regeneration_shields`).
    pub(crate) level: u8,
    /// Whether the permanent is tapped.
    pub(crate) tapped: bool,
    /// Whether it entered this turn (can't attack / use tap abilities without haste);
    /// cleared at its controller's untap step.
    pub(crate) summoning_sick: bool,
    /// Whether this permanent entered the battlefield this turn (CR "entered the battlefield
    /// this turn" — Oran-Rief, the Vastwood's "each green creature that entered this turn").
    /// Distinct from `summoning_sick`, which is scoped to the permanent's own *controller's*
    /// next untap (CR 302.6, [`Event::LostSummoningSickness`]): this instead clears for every
    /// battlefield permanent at *every* Untap step (whichever player's turn is beginning — see
    /// [`Event::StepBegan`]'s turn-boundary reset block, alongside the `*_this_turn` tallies),
    /// since a new turn — anyone's — ends "this turn" for CR purposes. `true` for every
    /// permanent minted by [`fresh_permanent`]/[`fresh_token`] (an ETB, by definition, is "this
    /// turn"); the `spawn_on_battlefield`/`spawn_token_on_battlefield` test helpers override it
    /// back to `false` to keep their "as if it had been there since before the turn" contract,
    /// the same way they override `summoning_sick`.
    pub(crate) entered_this_turn: bool,
    /// Net +1/+1 counters (each adds +1 power and +1 toughness).
    pub(crate) plus_counters: i32,
    /// Named non-P/T counters (CR 122.1 — charge, story, …), indexed by [`CounterKind`] as
    /// `usize`; `0` = none of that kind. Kept separate from `plus_counters` above — no
    /// replacement effect (Hardened Scales, a doubler) reads or grows this map.
    pub(crate) kind_counters: [u8; CounterKind::COUNT],
    /// Until-end-of-turn power/toughness boosts (pumps), cleared at cleanup.
    pub(crate) temp_power: i32,
    pub(crate) temp_toughness: i32,
    /// An until-end-of-turn base-P/T SET (CR 613.3(7b) — Biomass Mutation, Quandrix Charm's
    /// "has base power and toughness X/X until end of turn"): runtime bookkeeping, `Some((p, t))`
    /// while active, emitted as a `BasePtSet` layer by [`Game::pt_layers`] (applied before the 7c
    /// counters/pumps/anthems), and cleared alongside `temp_power`/`temp_toughness` at cleanup
    /// (see [`Event::TempBoostsEnded`]'s handler). Not a `CardDef`/TOML surface — P/T is derived.
    pub(crate) base_pt_set_eot: Option<(i32, i32)>,
    /// Card types added until end of turn by a self-animation (CR 613.4 — Restless Spire's "this
    /// land becomes a … creature … It's still a land"): `TypeSet::CREATURE` while a manland is
    /// animated, unioned onto its printed types by [`Game::effective_types`], and cleared alongside
    /// `base_pt_set_eot` at cleanup (see [`Event::TempBoostsEnded`]'s handler). The twin of
    /// `base_pt_set_eot` for the type layer — runtime bookkeeping, never a `CardDef`/TOML surface.
    pub(crate) added_types_eot: TypeSet,
    /// Creature subtypes added until end of turn by the same self-animation (Restless Spire →
    /// "Elemental"): unioned onto the printed subtypes by [`Game::effective_subtypes`], cleared with
    /// `added_types_eot`. `&'static` because it's copied straight from the granting ability's
    /// already-leaked `CardDef` data — no runtime leak.
    pub(crate) added_subtypes_eot: &'static [&'static str],
    /// Colors added until end of turn by the same self-animation (Restless Spire → blue, red):
    /// unioned onto [`color_identity`] by [`Game::colors_of`], cleared alongside
    /// `added_types_eot`/`added_subtypes_eot` at cleanup (see [`Event::TempBoostsEnded`]'s
    /// handler). Not a `CardDef`/TOML surface — runtime bookkeeping like its type-layer siblings.
    pub(crate) added_colors_eot: &'static [Color],
    /// Keywords granted until end of turn (a [`Effect::PumpUntilEndOfTurn`]/
    /// [`Effect::PumpCreaturesYouControlUntilEndOfTurn`] grant), cleared at cleanup alongside
    /// the temp P/T. `&'static` because it's usually copied straight from the granting
    /// ability's already-leaked `CardDef` data (see the `de` module) — no runtime leak. When a
    /// second non-empty grant lands on the same permanent the same turn (e.g. Selfless Spirit +
    /// Moonshaker Cavalry), [`Event::TempBoost`]'s handler unions the two into a freshly leaked
    /// slice instead of clobbering the first.
    pub(crate) temp_keywords: &'static [Keyword],
    /// Keywords this permanent has lost until end of turn AND can't regain this turn (CR
    /// 702.11e/702.18d-style "lose ... and can't have" — arcane_lighthouse's "creatures your
    /// opponents control lose hexproof and shroud and can't have hexproof or shroud"). Removed
    /// from the final unioned set at the end of [`Game::compute_effective_keywords_uncached`]
    /// rather than blocked at each granting source, so a keyword granted *after* this lands
    /// (Tyvar's Stand, an Equipment) is filtered right back out the same turn — "can't have" for
    /// free from the same mechanism as "lose." Cleared at cleanup alongside `temp_keywords`
    /// above (see [`Event::TempBoostsEnded`]'s handler).
    pub(crate) temp_lost_keywords: &'static [Keyword],
    /// An *indefinite* base-P/T SET (CR 611.2c — Excava, the Risen Past's "It's a 1/1 Spirit
    /// creature with flying"): the indefinite twin of `base_pt_set_eot`, `Some((p, t))` while
    /// active, emitted as the same 7b `BasePtSet` layer by [`Game::pt_layers`] (before the 7c
    /// counters/pumps). Written once as the reanimated permanent enters (see
    /// [`Event::ReanimatedCreatureBecame`]) and **never cleared at cleanup** — it naturally resets
    /// because a permanent that leaves the battlefield becomes a new object (CR 400.7). Runtime
    /// bookkeeping, never a `CardDef`/TOML surface.
    pub(crate) set_base_pt: Option<(i32, i32)>,
    /// Card types added indefinitely (CR 611.2c — Excava's "It's a … creature … in addition to its
    /// other types", turning a reanimated noncreature into a creature): the indefinite twin of
    /// `added_types_eot`, unioned onto the printed types by [`Game::effective_types`], never
    /// cleared at cleanup (resets with the object per CR 400.7).
    pub(crate) added_types: TypeSet,
    /// Creature subtypes added indefinitely by the same set (Excava → "Spirit"): the indefinite
    /// twin of `added_subtypes_eot`, unioned onto the printed subtypes by
    /// [`Game::effective_subtypes`]. `&'static` — copied straight from the granting ability's
    /// already-leaked `CardDef` data, no runtime leak.
    pub(crate) added_subtypes: &'static [&'static str],
    /// Keywords granted indefinitely by the same set (Excava → flying): the indefinite twin of
    /// `temp_keywords`, unioned onto the effective keywords by
    /// [`Game::compute_effective_keywords_uncached`], never cleared at cleanup. `&'static`.
    pub(crate) granted_keywords: &'static [Keyword],
    /// Damage marked this turn (compared against toughness by a state-based action).
    pub(crate) marked_damage: i32,
    /// Set when dealt damage by a deathtouch source — lethal regardless of amount.
    pub(crate) deathtouched: bool,
    pub(crate) commander: bool,
    /// A token (CR 111): created directly on the battlefield, not from a card. When it
    /// leaves the battlefield it ceases to exist (a state-based action).
    pub(crate) token: bool,
    /// The permanent this is attached to, for an Aura/Equipment (CR 301.5/303.4). `None`
    /// when unattached. Its grant (see [`Effect::GrantToAttached`]) applies to that host.
    pub(crate) attached_to: Option<ObjectId>,
    /// A planeswalker's current loyalty (its loyalty counters, CR 606.5b). 0 for a non-planeswalker.
    pub(crate) loyalty: i32,
    /// Whether a loyalty ability was activated on this planeswalker this turn (CR 606.3 — at most
    /// one per turn). Cleared at its controller's untap.
    pub(crate) loyalty_activated: bool,
    /// Whether this permanent has a finality counter (CR 122.3g/614.12): if it would be put into
    /// a graveyard from the battlefield, it's exiled instead (see `Game::graveyard_or_command`).
    /// A permanent either has one or it doesn't — no pool card stacks or removes them, so this is
    /// a flag rather than a count (unlike `plus_counters`). Set only by a reanimation with
    /// `finality = true` (Excava, the Risen Past); default `false`.
    pub(crate) finality_counter: bool,
    /// How many regeneration shields this permanent currently has (CR 701.15b): each is a
    /// replacement effect that replaces the next "destroy" this turn with a regeneration (tap,
    /// remove from combat, heal all damage). Consumed one at a time by the destroy path unless
    /// the destruction carries [`Effect::DestroyTarget::cant_be_regenerated`] (CR 701.15d); all
    /// reset to 0 at cleanup (CR 701.15b's "this turn"). Runtime state, not TOML-authored,
    /// defaulted 0 like `finality_counter`. Granted by [`Effect::RegenerateShield`].
    pub(crate) regeneration_shields: u8,
    /// Whether this permanent is "prepared" (soc/sos prepare DFCs — CR-style status): a front-face
    /// ability ([`Effect::BecomePrepared`]) set it, and while set its controller may cast a copy of
    /// its back-face spell ([`CardDef::back`], via [`Game::cast_prepared`]), which clears the flag.
    /// The status persists across turns until the copy is cast (it is *not* reset at turn
    /// boundaries). `false` for every ordinary permanent.
    pub(crate) prepared: bool,
    /// Echo (CR 702.31e) unpaid: set when a permanent with [`CardDef::echo`] enters, cleared at
    /// its controller's first upkeep after entering (whether echo was paid or the permanent was
    /// sacrificed) — the honest "came under your control since your last upkeep" flag, distinct
    /// from [`Self::summoning_sick`] (which clears at the *untap* step, one step earlier).
    /// `false` for every permanent without echo.
    pub(crate) echo_unpaid: bool,
    /// The creature type named by an as-enters choice (CR 614.12/700.9-style "as ~ enters,
    /// choose a creature type" — Patchwork Banner), read back by a chosen-type-gated anthem
    /// ([`Effect::AnthemStatic`]'s `chosen_subtype`). `None` until the choice is answered (see
    /// [`Effect::ChooseCreatureType`]), and for every permanent without such a choice.
    pub(crate) chosen_subtype: Option<&'static str>,
    /// The color named by an as-enters choice (CR 614.12/700.9-style "as this Aura enters, choose
    /// a color" — Flickering Ward), read back by a `protection_from_chosen_color`
    /// [`Effect::GrantToAttached`] to confer [`Keyword::ProtectionFrom`] of that color on the
    /// enchanted creature. `None` until the choice is answered (see [`Effect::ChooseColor`]), and
    /// for every permanent without such a choice.
    pub(crate) chosen_color: Option<Color>,
    /// The {X} chosen for the spell that became this permanent (CR 601.2b), fixed for the rest
    /// of this permanent's existence — read by [`Game::ability_source_x`] so a later-resolving
    /// ability (an ETB trigger, an `mv_max_x` filter) can still reference "X" once the casting
    /// spell has left the stack and X would otherwise revert to 0 (CR 107.3i). For a hydra-style
    /// card this duplicates `plus_counters` (both are set to the same cast X); Fractal Harness's
    /// "put X +1/+1 counters on [a separate token]" ETB is the case that actually needs it, since
    /// nothing places counters on Fractal Harness itself. 0 for a token or a permanent with no
    /// {X} in its cost.
    pub(crate) entered_with_x: u32,
    /// The graveyard-card object id this Aura targeted when cast (CR 303.4a's "enchant creature
    /// card in a graveyard" — [`CardDef::enchant_graveyard`]), locked in as it enters, the same
    /// "read the spell's own info before it's gone" idiom as `entered_with_x` above (the spell
    /// object is destroyed by the time this permanent's own ETB ability resolves). Read back by
    /// [`TargetSpec::ThisAurasGraveyardTarget`] as a fixed reference, not a fresh choice — empty
    /// once it's left the graveyard (CR 603.3c: the ETB ability then has no legal target and is
    /// dropped, rather than reanimating whatever moved in). `None` for every permanent whose
    /// spell had no chosen target, or wasn't cast with `enchant_graveyard` set.
    pub(crate) cast_time_enchant_target: Option<ObjectId>,
    /// The player this creature "can't attack … for as long as it has a vow counter on it" (CR
    /// 122.1 — Promise of Loyalty): set alongside a [`CounterKind::Vow`] counter by
    /// [`Event::VowCountersPlaced`], read in [`Game::declare_attackers`]. `None` for any creature
    /// with no vow counter. Engine-internal, not wire-mirrored (like `entered_with_x`/`echo_unpaid`);
    /// the restriction is read live off `kind_counters[Vow]` + this, so removing the counter lifts it.
    pub(crate) vow_protected: Option<PlayerId>,
    /// Whether this permanent is *phased out* (CR 702.26): treated as though it doesn't exist —
    /// excluded from every battlefield scan (statics, combat, SBAs, targeting, board counts) until
    /// it phases in at the start of its controller's next turn (CR 702.26f, before untapping).
    /// Set by [`Effect::PhaseOut`] (Guardian of Faith's ETB) and on anything attached to a
    /// phased-out permanent (CR 702.26g — indirect phasing); cleared at that untap step. `false`
    /// for every permanent that hasn't phased out.
    /// ponytail: a plain "did/didn't phase out" flag — no "phased in tapped" bit (CR 702.26e: a
    /// permanent phases in tapped if it phased out tapped). Guardian phases out untapped creatures
    /// and the flag doesn't touch `tapped`, so tapped state is preserved for free; add a companion
    /// bit if a card ever phases out a tapped permanent whose re-tap must be observable.
    pub(crate) phased_out: bool,
    /// Whether this permanent was played/cast from a graveyard under Serra Paragon's permission
    /// (CR 118.9) and so carries the granted rider "when this permanent is put into a graveyard
    /// from the battlefield, exile it and you gain 2 life." Set as it enters (from the casting
    /// [`Spell::serra_recursion`], or directly for a land-play); read at the death choke
    /// ([`Game::graveyard_or_command`]) to exile it instead and by
    /// [`Event::MovedToExile`]'s handler to queue the +2 life. Runtime state, not TOML-authored,
    /// defaulted `false` like `finality_counter`.
    pub(crate) serra_recursion: bool,
    /// Whether this permanent was cast via bestow (CR 702.103 — Eidolon of Countless Battles) and
    /// so is a dual-nature Aura/creature. While set *and* attached ([`Permanent::attached_to`] is
    /// `Some`), it's an Aura enchantment and **not** a creature (CR 702.103e) — the "attached?"
    /// gate, not this flag alone, decides which nature is live. When it stops being attached it
    /// becomes a creature again (CR 702.103i). Set as it enters from the casting [`Spell::bestowed`];
    /// runtime state, not TOML-authored, defaulted `false` like `serra_recursion`.
    pub(crate) bestowed: bool,
    /// Whether this permanent is *face down* (CR 708 — a manifested card, CR 701.34): while set,
    /// its real `def` is hidden and it is a 2/2 colorless creature with no name, no card types
    /// other than creature, no subtypes, no mana cost, and no abilities (CR 708.2 — the
    /// characteristics overrides in [`Game::effective_types`]/`pt_base`/`functional_abilities`/
    /// `effective_subtypes`/`compute_effective_keywords_uncached` all short-circuit on it). The
    /// real card stays in `def` so it can be revealed by the turn-face-up special action
    /// ([`Intent::TurnFaceUp`]), which clears this flag; the wire redaction layer anonymizes it.
    /// ponytail: the face-down 2/2 status is the shared substrate for the morph family (CR 702.37
    /// morph / megamorph / disguise) — no morph card is in the pool, so only plain manifest is
    /// built; a morph card would add its face-down cost + the morph keyword on top of this status.
    pub(crate) face_down: bool,
    /// Whether this permanent was cast for its evoke cost (CR 702.74a — [`CardDef::evoke`]): it is
    /// sacrificed the instant it enters, via a self-sacrifice trigger queued alongside its own ETB
    /// triggers so an ETB payoff (Mulldrifter's draw two) resolves first. Set as it enters from the
    /// casting [`Spell::evoked`]; runtime state, not TOML-authored, defaulted `false` like
    /// `bestowed`.
    pub(crate) evoked: bool,
    /// The `def` to restore at cleanup for an *until-end-of-turn* enter-as-copy (Cursed Mirror,
    /// CR 706/613 — "become a copy … until end of turn"): when the copy is established, the
    /// permanent's original printed `def` is stashed here and `def` is overwritten with the copied
    /// creature's; at cleanup ([`Event::TempBoostsEnded`]) `def` is restored from this and it is
    /// cleared back to `None` (CR 514.2). `None` for an ordinary permanent or a *permanent* copy
    /// (Altered Ego leaves the overwritten `def` in place). Runtime state, not TOML-authored — a
    /// `&'static CardDef` (leaked at the copy step, like [`CardDef::back`]) so [`Permanent`] stays
    /// `Copy` and small (a pointer, not a second inlined [`CardDef`]).
    pub(crate) reverts_to_def_eot: Option<&'static CardDef>,
    /// The colors of mana spent to cast the spell that became this permanent (CR 106.9), fixed
    /// for the rest of this permanent's existence — copied from [`Spell::spent_colors`] as it
    /// enters, the same "read the spell's own info before it's gone" idiom as `entered_with_x`.
    /// Read by [`Condition::ColorWasSpentToCastThis`] (Court Hussar's "unless {W} was spent to
    /// cast it"). `[false; Color::COUNT]` for a token, a reanimated/reconstructed permanent, or
    /// any permanent whose casting spell paid no mana or isn't wired through yet (see
    /// [`Spell::spent_colors`]'s doc).
    pub(crate) spent_colors: [bool; Color::COUNT],
}

/// One slot in the object arena. A card's slot becomes [`Object::Moved`] when it changes
/// zones (a fresh slot/id is minted for its new form); `to` points at that new id so an
/// old id's lineage can still be followed (see [`Game::zone_of`]).
// The `Spell`/`Permanent` variants inline a whole `CardDef` and are near-equal in size (~2.3 KB);
// the id-indexed object arena needs `Object: Copy`, so boxing a variant isn't an option — the same
// carve-out the sibling `Copy` enums in this crate take.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum Object {
    Card(Card),
    Spell(Spell),
    Permanent(Permanent),
    Moved {
        to: ObjectId,
    },
    /// The object left the game (its owner was eliminated) — no longer live (CR 800.4a).
    Removed,
}

/// The default number of seats when a constructor doesn't specify one (the 1v1 games
/// most tests build). Real tables set their own count via [`Game::with_players`].
pub(crate) const NUM_PLAYERS: u8 = 2;

/// Default starting life for a plain game; Commander games use [`COMMANDER_LIFE`].
pub(crate) const STARTING_LIFE: i32 = 20;

/// Starting life in the Commander format.
pub(crate) const COMMANDER_LIFE: i32 = 40;

/// Combat damage from a single commander that loses the game.
pub(crate) const LETHAL_COMMANDER_DAMAGE: i32 = 21;

/// The maximum hand size enforced by the cleanup step.
pub(crate) const HAND_SIZE: usize = 7;

/// Per-player game state that isn't tied to a single object.
#[derive(Debug, Clone, Default)]
pub(crate) struct Player {
    pub(crate) life: i32,
    /// Available mana this step (colored, colorless, and "any"). Empties between steps.
    pub(crate) mana_pool: ManaPool,
    /// The player's library, top of library first (index 0 is drawn next).
    /// ponytail: `Vec::remove(0)` to draw is O(n); trivial for a ~100-card deck.
    pub(crate) library: Vec<ObjectId>,
    /// Set when the player tried to draw from an empty library (loses via SBA).
    pub(crate) attempted_empty_draw: bool,
    /// Lands played this turn (reset at untap; limited to one).
    pub(crate) lands_played: u8,
    /// Life this player has gained this turn (turn-scoped; reset each turn at untap). Feeds
    /// [`Amount::LifeGainedThisTurn`] and "if you gained life this turn" conditions.
    pub(crate) life_gained_this_turn: u32,
    /// Spells this player has cast this turn (turn-scoped; reset each turn at untap). Feeds
    /// [`Amount::SpellsCastThisTurn`].
    pub(crate) spells_cast_this_turn: u32,
    /// Spells with {X} in their mana cost this player has cast this turn (turn-scoped; reset
    /// at untap) — the filter-scoped sibling of `spells_cast_this_turn`, for the "first {X}-
    /// spell each turn" gate (Nev, Zimone Infinite Analyst). CR 107.3.
    pub(crate) x_spells_cast_this_turn: u32,
    /// Cards this player has drawn this turn (turn-scoped; reset each turn at untap) — the
    /// draw-side sibling of `spells_cast_this_turn`. Feeds [`Trigger::PlayerDraws`] (Faerie
    /// Mastermind's "an opponent draws their second card each turn"). A [`Event::CardDrawn`]
    /// bumps this; drawing from an empty library ([`Event::DrewFromEmptyLibrary`]) does not —
    /// CR 120.3, you don't draw if the library is empty.
    pub(crate) draws_this_turn: u32,
    /// How many times this player has lost life this turn (turn-scoped; reset each turn at
    /// untap) — the life-loss sibling of `draws_this_turn`. A [`Event::LifeChanged`] with a
    /// *negative* amount bumps this (CR 118.9/119.3 — only a decrease is a life loss; gaining
    /// life doesn't). Feeds [`Trigger::YouLoseLifeFirstTimeEachTurn`] (Intermediate
    /// Chirography's level-2 "whenever you lose life for the first time each turn"): the trigger
    /// fires only when the losing event's ordinal this turn is 1. A count (not a bool) so the
    /// first-loss ordinal can be recovered within a batch that carries several losses, exactly
    /// as `draws_this_turn` recovers a draw's ordinal.
    pub(crate) life_losses_this_turn: u32,
    /// Creatures that died under this player's control this turn (turn-scoped; reset each turn
    /// at untap) — the death-side sibling of `spells_cast_this_turn`. Feeds
    /// [`Amount::CreaturesDiedThisTurn`] (Gorma, the Gullet).
    pub(crate) creatures_died_this_turn: u32,
    /// Whether a *modified* creature (CR 701.29 — has a counter, is enchanted by an Aura, or is
    /// equipped) died under this player's control this turn (turn-scoped; reset each turn at
    /// untap) — the modified-scoped sibling of `creatures_died_this_turn`. Feeds
    /// [`Condition::ModifiedCreatureDiedThisTurn`] (Intermediate Chirography's Level 3
    /// morbid-of-modified end step). Set at the death choke ([`Event::MovedToGraveyard`]/
    /// [`Event::TokenCeasedToExist`] in `apply.rs`) by last-known information — `is_modified` is
    /// read *before* the dying object's attachments/counters are torn down by the zone change
    /// (CR 700.4).
    pub(crate) modified_creature_died_this_turn: bool,
    /// Nontoken creatures that entered the battlefield under this player's control this turn
    /// (turn-scoped; reset each turn at untap) — the entering-side sibling of
    /// `creatures_died_this_turn`, excluding tokens. Feeds
    /// [`Amount::NontokenCreaturesEnteredThisTurn`] (Gyome, Master Chef).
    pub(crate) nontoken_creatures_entered_this_turn: u32,
    /// Whether a land entered the battlefield under this player's control this turn (turn-scoped;
    /// reset each turn at untap) — CR landfall's own "enters" (cast, fetched, or a token land all
    /// count), not "played." Feeds [`Condition::LandEnteredUnderYourControlThisTurn`] (Zimone,
    /// All-Questioning's end step). Set at the same permanent-enters choke as
    /// `nontoken_creatures_entered_this_turn`.
    pub(crate) land_entered_under_your_control_this_turn: bool,
    /// Whether a card has left this player's graveyard this turn (turn-scoped; reset each turn at
    /// untap). Set at the object-move choke point ([`Game::create_object`]); feeds
    /// [`Condition::CardLeftYourGraveyardThisTurn`].
    pub(crate) card_left_graveyard_this_turn: bool,
    /// Whether this player has cast an instant or sorcery spell this turn (turn-scoped; reset
    /// each turn at untap). Feeds [`Condition::CastInstantOrSorceryThisTurn`] (Hall of Oracles's
    /// counter ability's activation restriction).
    pub(crate) instant_or_sorcery_cast_this_turn: bool,
    /// The greatest mana value among instant and sorcery spells this player has cast this turn
    /// (turn-scoped; reset each turn at untap, 0 if none) — Rootha, Mastering the Moment's "X is
    /// the greatest mana value among instant and sorcery spells you've cast this turn." Feeds
    /// [`Amount::GreatestInstantOrSorceryManaValueCastThisTurn`].
    pub(crate) greatest_instant_or_sorcery_mana_value_cast_this_turn: u32,
    /// How many instant and sorcery spells this player has cast this turn (turn-scoped; reset
    /// each turn at untap, 0 if none) — Rionya, Fire Dancer's "X is one plus the number of
    /// instant and sorcery spells you've cast this turn." Feeds
    /// [`Amount::OnePlusInstantsAndSorceriesCastThisTurn`]. A copied spell doesn't bump this —
    /// same "cast" boundary as `instant_or_sorcery_cast_this_turn` above.
    pub(crate) instants_and_sorceries_cast_this_turn: u32,
    /// Whether this player may cast spells this turn as though they had flash (turn-scoped;
    /// reset each turn at untap) — CR 601.3a, granted by [`Effect::GrantFlashThisTurn`]
    /// (Alchemist's Refuge). Unfiltered: every spell, not a subset. Read by
    /// [`CardDef::is_instant_speed`]'s cast-timing gate.
    pub(crate) flash_permission_this_turn: bool,
    /// Whether this player may, at mana-ability timing, pay 1 life to add {C} (turn-scoped;
    /// reset each turn at untap) — Yavimaya Bloomsage's Channel back face, granted by
    /// [`Effect::GrantChannelColorlessManaThisTurn`]. Read by
    /// [`Game::channel_colorless_mana`](crate::Game::channel_colorless_mana).
    pub(crate) channel_colorless_mana_this_turn: bool,
    /// Whether this player has already used Serra Paragon's graveyard-play permission this turn
    /// (turn-scoped; reset each turn at untap) — CR 118.9's "once during each of your turns."
    /// Set when a land / permanent spell is played or cast from the graveyard under
    /// [`Effect::PlayFromGraveyardOncePerTurn`], read by [`Game::playable_zone`] to reject a
    /// second such play the same turn. `false` until the permission is used.
    pub(crate) graveyard_play_used_this_turn: bool,
    /// Times this player has cast their commander from the command zone (tax = 2× this).
    pub(crate) command_casts: u8,
    /// Commander combat damage taken, keyed by the source commander's owner (each player
    /// has one commander); 21 from one source loses the game.
    pub(crate) commander_damage: Vec<(PlayerId, i32)>,
    /// Set once the player has lost the game (a state-based action).
    pub(crate) lost: bool,
    /// Whether this player has the city's blessing (CR 702.131 ascend). Sticky: set once by a
    /// state-based action when the player controls ten or more permanents, and never cleared —
    /// CR 702.130's "for the rest of the game." Feeds [`Condition::YouHaveCitysBlessing`].
    pub(crate) has_citys_blessing: bool,
    /// Mana-provenance side-channel (CR 106.9-adjacent "spend this mana to …" tracking, Study
    /// Hall / Path of Ancestry / Opal Palace): one `(producing source, mana kind)` entry per unit
    /// of provenance-tagged mana this player currently holds, kept beside the summed
    /// [`mana_pool`](Self::mana_pool) which can't tag individual credits. Pushed when an
    /// [`Effect::AddMana`] flagged `track_provenance` resolves (see `Game::activate_ability`),
    /// read at a spell-cast payment to fire the source's `Trigger::SpendManaToCast`
    /// (see `Game::queue_spend_to_cast_triggers`), and cleared wholesale with the pool at
    /// [`Event::ManaEmptied`].
    /// ponytail: a summed pool can't prove *which* physical credit paid a cast, so provenance is
    /// only cleared at pool-empty and at a matched fire — a tagged credit spent on a non-cast /
    /// non-matching payment lingers as an entry with no backing mana, an over-fire only if a
    /// same-kind credit is later spent on a qualifying cast in the same step (unobserved in the
    /// pool). The upgrade path is per-credit tagging (tag each unit in `mana_pool` itself and
    /// consume the exact tagged credit on every `ManaSpent`); no pool card observes the gap today.
    pub(crate) mana_provenance: Vec<(ObjectId, Mana)>,
    /// "Until end of turn, you don't lose this mana as steps and phases end" side-channel (CR
    /// 500.4 exception; Rousing Refrain) — a mirror pool in lockstep with the "persist" credits
    /// still floating in [`mana_pool`](Self::mana_pool), same shape as [`mana_provenance`](Self::mana_provenance)'s
    /// own side-channel. Populated by an [`Effect::AddMana`] flagged `persist_until_end_of_turn`
    /// (see `Game::effects.rs`'s mint arm and [`Event::ManaAdded`]'s `persist` flag). Read at
    /// [`Event::ManaEmptied`]: a mid-turn boundary keeps only the credits still present in both
    /// pools (some may have been spent since); the turn-ending boundary (CR 514.2 cleanup) clears
    /// both wholesale like everything else.
    pub(crate) persistent_mana: ManaPool,
}
