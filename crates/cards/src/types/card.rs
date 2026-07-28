use std::sync::Arc;

use super::*;
use crate::CardId;
#[cfg(feature = "card-dsl")]
use crate::de;

/// Shared owned slice storage for `CardDef` and nested payloads.
pub fn arc_slice<T, const N: usize>(items: [T; N]) -> Arc<[T]> {
    Arc::from(items)
}

/// Empty shared slice helper for tests and handwritten `CardDef` stubs.
pub fn empty_slice<T>() -> Arc<[T]> {
    Arc::default()
}

/// A seat at the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(pub u8);

/// A game object (a card in some zone), identified for its lifetime in the game.
pub type ObjectId = u32;

/// What a creature is attacking (CR 508.1): a player, or a planeswalker an opponent controls.
/// The *defending player* — who declares blocks, pays pillow-fort taxes, and is read by every
/// "attacks you" trigger (CR 509.1a) — is the player for `Player`, or the planeswalker's
/// controller for `Planeswalker`. That mapping lives in [`Game::defender_of`], so the whole
/// blocking/tax/trigger/goad machinery is unchanged by planeswalker attacks — only declaration
/// legality and combat-damage delivery read the distinction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackTarget {
    Player(PlayerId),
    Planeswalker(ObjectId),
}

impl From<PlayerId> for AttackTarget {
    fn from(player: PlayerId) -> Self {
        AttackTarget::Player(player)
    }
}

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
///
/// Variants are declared in turn order and `Ord` follows that order, so `step < Step::X` reads as
/// "earlier in this turn than X" (Master Warcraft's "only before attackers are declared"). There's
/// no extra-combat machinery, so a turn walks each variant at most once and the order is total.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum Step {
    Untap,
    /// The default [`Effect::Misc(MiscEffect::ScheduleAtNextUpkeep)`] `fire_at` — CR 603.7's "next upkeep".
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
    pub fn next(self) -> Step {
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
    pub fn has_priority_window(self) -> bool {
        !matches!(self, Step::Untap | Step::Cleanup)
    }

    /// Whether this step is one of combat's five (CR 500.4/601.3e — begin combat through end of
    /// combat inclusive), the "cast this spell only during combat" timing window (Cauldron
    /// Dance's [`CardDef::cast_only_during_combat`]).
    pub fn is_combat(self) -> bool {
        matches!(
            self,
            Step::BeginCombat
                | Step::DeclareAttackers
                | Step::DeclareBlockers
                | Step::FirstStrikeCombatDamage
                | Step::CombatDamage
                | Step::EndCombat
        )
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
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
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

    /// This color's mana-symbol letter (CR 107.4 — `{W}{U}{B}{R}{G}`), for rendering a
    /// [`Cost`](super::Cost) back as pip text.
    pub fn letter(self) -> char {
        match self {
            Color::White => 'W',
            Color::Blue => 'U',
            Color::Black => 'B',
            Color::Red => 'R',
            Color::Green => 'G',
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
/// `Copy` so [`Keyword`] stays a small value enum. In TOML, `{ protection = "<value>" }` where
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
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
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
    /// Can be blocked only by artifact creatures and/or black creatures (CR 702.36b). See
    /// [`Game::can_block`].
    Fear,
    /// Can be blocked only by artifact creatures and/or creatures that share a color with it
    /// (CR 702.13b) — the color-sharing sibling of [`Fear`](Self::Fear). See [`Game::can_block`].
    Intimidate,
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
    /// Infect (CR 702.90): this source's damage to a creature is dealt as that many -1/-1 counters
    /// (CR 702.90b) and its damage to a player as that many poison counters (CR 702.90c). The
    /// damage is still dealt (CR 120.3) at its original size — lifelink, deathtouch, the
    /// commander-damage tally and every "deals damage" watch still see it. Read at the two damage
    /// chokes [`Game::creature_damage_events`](crate::Game::creature_damage_events) and
    /// [`Game::player_damage_events`](crate::Game::player_damage_events).
    Infect,
    /// Toxic N (CR 702.164): "Players dealt combat damage by this creature also get N poison
    /// counters." Unlike Infect it does *not* reshape the damage (CR 702.164a) — the life loss
    /// still happens and the poison counters are placed on top of it. Multiple instances add
    /// (CR 702.164b), so this is read as a sum by
    /// [`Game::toxic_amount`](crate::Game::toxic_amount), not a first-match lookup like Ward's.
    /// Combat damage only: applied at [`Game::damage_player`](crate::Game::damage_player), after
    /// that choke's prevention guards, so fully prevented combat damage places no counters.
    Toxic(u8),
}

/// A small set of the permanent card types a card carries, as a bitset (creature, artifact,
/// enchantment, planeswalker, land). Used two ways: a permanent's *own* types (its [`CardKind`]
/// plus a creature's additional types — see [`CardKind::Creature`]'s `also`), and a
/// [`PermanentFilter`]'s required-type set. Kept `Copy` because it is a tiny value bitset.
/// ponytail: no subtypes (Goblin, Aura) — those are #15/#18; this is card *types* only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TypeSet(u8);

impl TypeSet {
    pub const CREATURE: TypeSet = TypeSet(1);
    pub const ARTIFACT: TypeSet = TypeSet(2);
    pub const ENCHANTMENT: TypeSet = TypeSet(4);
    pub const PLANESWALKER: TypeSet = TypeSet(8);
    pub const LAND: TypeSet = TypeSet(16);
    pub const BATTLE: TypeSet = TypeSet(32);
    /// The five nonland permanent types — "any nonland permanent."
    pub const NONLAND: TypeSet = TypeSet(1 | 2 | 4 | 8 | 32);
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
    /// A battle: a permanent that enters with `defense` starting defense counters (CR 310.1 /
    /// 310.2). Stored in [`Permanent::loyalty`] (same counter slot planeswalkers use for loyalty).
    /// Siege protectors, attack-for-defense, and transform-on-defeat are not modeled yet — the
    /// pool only needs battles as destroyable permanents for Final Act's mass mode.
    Battle { defense: i32 },
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
    pub fn types(self) -> TypeSet {
        match self {
            CardKind::Creature { also, .. } => TypeSet::CREATURE.union(also),
            CardKind::Enchantment | CardKind::Aura => TypeSet::ENCHANTMENT,
            CardKind::Artifact => TypeSet::ARTIFACT,
            CardKind::Planeswalker { .. } => TypeSet::PLANESWALKER,
            CardKind::Battle { .. } => TypeSet::BATTLE,
            CardKind::Land { .. } => TypeSet::LAND,
            CardKind::Spell { .. } => TypeSet::NONE,
        }
    }

    /// Whether casting this card is restricted to sorcery speed. Permanents are;
    /// instants are not. (Lands are played, not cast.)
    pub fn is_sorcery_speed(self) -> bool {
        match self {
            CardKind::Creature { .. }
            | CardKind::Enchantment
            | CardKind::Aura
            | CardKind::Artifact
            | CardKind::Planeswalker { .. }
            | CardKind::Battle { .. }
            | CardKind::Land { .. } => true,
            CardKind::Spell { speed } => speed == SpellSpeed::Sorcery,
        }
    }
}

/// When an ability happens.
// The `Activated(ActivationCost)` variant embeds `Effect` and dwarfs the others, but boxing
// would add indirection to a hot authored enum. Same tolerated posture as `Effect`/`StackItem`.
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ability {
    pub timing: Timing,
    pub effect: Effect,
    /// The minimum Class level this ability requires to function (CR 717.5 — a Class's
    /// level-gated triggered/static/activated abilities). An ability functions only while its
    /// source permanent's [`Permanent::level`] is at least `min_level`; `0` (every ordinary
    /// ability, and every permanent's trivial "level 1") is unconditional. Checked at each scan
    /// that reads a permanent's abilities — trigger placement ([`Game::queue_trigger_group`]),
    /// the static anthem/cost-reduction recomputes, and the activation gate. A "Level N"
    /// activated ability (an [`Effect::Counters(CountersEffect::LevelUp)`]) keeps `min_level` 0; its own exact-predecessor
    /// gate supersedes this. `min_level = N` in TOML (`#[serde(default)]` 0).
    pub min_level: u8,
    /// Whether this triggered ability is optional ("you may …"): raises a yes/no (or, with a
    /// non-free `cost`, a pay-or-decline) choice before it goes on the stack. An accepted
    /// optional trigger that targets then pauses to choose its target (Sun Titan).
    /// ponytail: only single optional triggers are wired; an optional trigger that is *also* one
    /// of a several-ability simultaneous group grows from a real card (see wire-protocol-and-visibility spec). (CR 603, CR 601.2c, CR 405)
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

/// A printed alternative cost that pays something other than mana (CR 601.2f — Invigorate: "If
/// you control a Forest, rather than pay this spell's mana cost, you may have an opponent gain 3
/// life"). Distinct from [`CardDef::free_cast_if`] (an unconditional-once-the-gate-holds `{0}`)
/// and from flashback/escape/evoke (alternative costs still denominated in mana): here the
/// replacement is a non-mana `rider` the caster pays instead, and taking it is the caster's own
/// choice, not automatic. `condition = None` — the always-castable degenerate case, unused by any
/// pool card today — makes the alternative available unconditionally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(deny_unknown_fields, rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub struct AlternativeCost {
    /// The board-state gate gating the alternative (Invigorate's "if you control a Forest").
    /// `None` if the alternative is always offered.
    #[cfg_attr(feature = "card-dsl", serde(default))]
    pub condition: Option<Condition>,
    /// The non-mana cost paid instead of the printed mana cost, fired at cast time (CR 601.2f —
    /// before the spell is put on the stack), not a resolution effect. Leaked to `'static` like
    /// every other nested [`Effect`] a `Copy` struct holds ([`GrantedAbility::effects`],
    /// [`Effect::Misc(MiscEffect::ScheduleAtNextUpkeep)`]'s `then`) so `AlternativeCost` stays a
    /// compact scalar value.
    #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_effect"))]
    pub rider: &'static Effect,
}

/// A card definition (identity + behavior). Deserializable (under the `card-dsl` feature)
/// straight from a card's TOML file — the `cards` crate loads the pool this way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardDef {
    /// Scryfall oracle id — canonical Card identity (accounts-decks-and-catalog spec). Empty for test stubs; tokens
    /// that have a Scryfall token card stamp theirs so battlefield art resolves.
    pub id: &'static str,
    /// Scryfall card UUID for the Card's default Printing (art). Empty for test stubs; tokens
    /// stamp a Scryfall token printing when one exists.
    pub default_print: &'static str,
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
    /// for it instead of the ordinary `CardKind::Aura` battlefield-permanent spec, checked ahead
    /// of it. `Kind` stays `Aura` (CR 704.5m's Aura-orphan state-based action applies to it like
    /// any other Aura), but [`Game::resolve_spell`] routes it through the same generic
    /// permanent-enter path a Creature/Enchantment uses instead of the ordinary `CardKind::Aura`
    /// immediate-attach arm, since there's no battlefield host yet to attach to at cast — its own
    /// ETB ability's `reanimate_to_battlefield` + `attach_self_to_reanimated` effects do the
    /// reanimate-then-attach instead. That leaves it unattached for the brief window between
    /// entering and its own ETB ability resolving; [`Game::check_state_based_actions`]'s Aura-
    /// orphan sweep (which runs *before* that ETB ability is even placed on the stack, per
    /// `pipeline.rs`'s phase order) exempts it for exactly that window — see its own doc. `false`
    /// for every other card.
    /// ponytail: a bare bool, not a filter — the pool has exactly one such card and its enchant
    /// subject is unrestricted ("a graveyard", not "your graveyard"); promote to a filter/scope
    /// type mirroring `enchant` if a second graveyard-enchanting Aura needs a narrower one.
    pub enchant_graveyard: bool,
    /// Whether the card is legendary — the only cards that may be a deck's commander.
    /// ponytail: a bare bool, not a full CR 205.4a supertype set; snow is the only other
    /// supertype the pool tracks today ([`Self::snow`]).
    pub legendary: bool,
    /// Whether the card is snow (CR 205.4g — Snow-Covered Forest, Ohran Frostfang). Read by
    /// snow-matters filters ([`crate::CardFilter::SnowLand`], [`crate::PermanentFilter::snow`]).
    /// `false` (default) for every ordinary card. `snow = true` in TOML.
    pub snow: bool,
    /// "This spell can't be countered" (CR 701.5g, e.g. Altered Ego). Checked in
    /// [`Game::counter_spell`], the shared choke for both the unconditional
    /// [`Effect::Misc(MiscEffect::CounterTargetSpell)`] arm and a declined `PayOrCounter` — the counter fizzles,
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
    pub keywords: Arc<[Keyword]>,
    /// Keywords granted only while a `Condition` holds (CR 702 conditional statics —
    /// Primordial Hydra's "has trample as long as it has ten or more +1/+1 counters"), read by
    /// the characteristics recompute alongside `keywords`. Empty for every ordinary card.
    pub conditional_keywords: Arc<[(Condition, Keyword)]>,
    /// The card's abilities.
    pub abilities: Arc<[Ability]>,
    /// Extra colors a card's real rules text carries for color identity (CR 903.4) that the
    /// simplified gameplay model (cost pips, a land's single modeled producer, `AddMana`
    /// effects, activated-ability costs) doesn't otherwise capture — e.g. the dropped half of
    /// a flattened dual/pain/filter land, or a colored activated ability cut entirely. Empty
    /// for ordinary cards. `identity = [...]` in TOML; consumed by `schema::color_identity`.
    pub identity_pips: Arc<[Color]>,
    /// Explicit colors (CR 105.2a: a color indicator, or CR 111.4's "colors are determined by
    /// their text" for a token) overriding the cost-pip derivation in [`color_identity`] — a
    /// token has no mana cost, so its color must be stated outright. Empty (every ordinary card)
    /// falls back to deriving color from cost pips as usual.
    /// `colors = ["green"]` / `["white", "black"]` in TOML.
    pub colors: Arc<[Color]>,
    /// Devoid (CR 702.114a): the card is colorless despite any colored mana-cost pips —
    /// overrides the cost-pip derivation in [`color_identity`] to all-false, taking priority
    /// over `colors` (a devoid card is never also given an explicit color list). `devoid = true`
    /// in TOML; `false` (every ordinary card) leaves color derivation to pips/`colors` as usual.
    pub devoid: bool,
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
    /// A CR 614.12 as-enters replacement *choice*, not a `Condition` — Overgrown Tomb's "As this
    /// land enters, you may pay 2 life. If you don't, it enters tapped.": `Some(life)` raises a
    /// [`PendingChoice::PayLifeOrEntersTapped`] before the land enters, offered only when the
    /// controller's life total is greater than or equal to `life` (CR 119.4 — a player may pay
    /// life down to and including 0; below the cost the land simply enters tapped, no prompt).
    /// `None` (the common case) leaves [`Self::enters_tapped_unless`] as the only conditional
    /// gate. `enters_tapped_unless_you_pay_life = 2` in TOML.
    pub enters_tapped_unless_you_pay_life: Option<u8>,
    /// A printed "you may cast this spell without paying its mana cost" permission, gated on a
    /// board-state [`Condition`] checked fresh at cast time (CR 118.5 — Massacre: "If an
    /// opponent controls a Plains and you control a Swamp, you may cast this spell without
    /// paying its mana cost"). `None` (the common case) leaves the printed cost untouched. When
    /// the condition holds, [`Game::cast_cost`] returns [`Cost::FREE`] outright rather than
    /// pausing for a decline — revealing is free and strictly better, so nothing in this pool
    /// wants to voluntarily pay a cost it could skip.
    pub free_cast_if: Option<Condition>,
    /// A printed alternative cost that isn't a mana cost at all (CR 601.2f — Invigorate: "If you
    /// control a Forest, rather than pay this spell's mana cost, you may have an opponent gain 3
    /// life"). `None` (the common case) leaves the printed cost the only option. `Some(alt)` is a
    /// caster *choice* (unlike [`Self::free_cast_if`]'s always-take-it permission) — see
    /// [`AlternativeCost`]. `alternative_cost = { .. }` in TOML.
    pub alternative_cost: Option<AlternativeCost>,
    /// "Cast this spell only during combat" (CR 601.3e's named-window restriction — Cauldron
    /// Dance): legal only from begin-combat through end-of-combat inclusive ([`Step::is_combat`]),
    /// on top of (not instead of) the ordinary instant/sorcery-speed gate — an instant with this
    /// flag is still open only during those steps, not any time it holds priority. Checked in
    /// [`Game::cast_timing_ok`]. `cast_only_during_combat = true` in TOML; `false` (every ordinary
    /// card) leaves timing to `kind`'s instant/sorcery speed alone.
    pub cast_only_during_combat: bool,
    /// "Cast this spell only before attackers are declared" (CR 601.3e's named-window restriction
    /// — Master Warcraft): legal from untap through the declare-attackers step, and inside that
    /// step only until the declaration is made. Like [`Self::cast_only_during_combat`] it layers
    /// on top of the ordinary instant-speed gate and is checked in [`Game::cast_timing_ok`].
    /// `cast_only_before_attackers = true` in TOML; `false` for every ordinary card.
    pub cast_only_before_attackers: bool,
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
    /// Scryfall set codes for every known printing of this card. Pure catalog + coverage
    /// metadata — the engine never consults it for rules. `sets = ["soc", …]` in TOML; empty
    /// for a card whose printings are not recorded yet.
    pub sets: Arc<[&'static str]>,
    /// The card's printed subtypes (the segment after the "—": creature types like "Goblin",
    /// "Wizard"; also artifact/enchantment subtypes). Gameplay-relevant, not just catalog
    /// metadata: [`PermanentFilter::subtypes`] and [`Effect::Static(StaticEffect::Anthem)`]'s `subtypes` axis
    /// both match against this (Goldspan Dragon's "Treasures you control", a tribal anthem). A
    /// *land's* types stay on [`CardKind::Land::subtypes`] (rules use those); `schema::catalog_card`
    /// unions the two for the wire. `subtypes = […]` in TOML; empty when unrecorded or genuinely
    /// none — including most token profiles today (grown card by card as tribal payoffs need them).
    pub subtypes: Arc<[&'static str]>,
    /// Scryfall Tagger oracle-tag slugs (catalog metadata for deck-builder search). Pure catalog
    /// metadata — the engine never reads this at runtime. `otags = […]` in TOML; empty when
    /// unrecorded. Backfilled from Scryfall by `tooling/backfill-otags.mjs`.
    pub otags: Arc<[&'static str]>,
    /// Cycling {N} (CR 702.29a): "{N}, Discard this card: Draw a card," activatable from the
    /// hand. `None` for a card with no cycling. `cycling = { generic = N }` in TOML (the same
    /// `[cost]`-table shape as a spell's cost).
    pub cycling: Option<Cost>,
    /// A sacrifice folded into the cycling cost (CR 702.29b — Edge of Autumn's "Cycling—Sacrifice
    /// a land"), on top of `cycling`'s mana. Cycling is an activated ability (CR 702.29), so the
    /// named sacrifice is validated and paid through the same choke an ordinary activation's
    /// [`ActivationCost::sacrifice`] uses (CR 602.2b — an uncompletable/unnamed cost makes the
    /// activation illegal). `SacrificeCost::None` (the default) for ordinary cycling.
    /// `cycling_sacrifice = { permanent = { types = "land" } }` in TOML, the same [`SacrificeCost`]
    /// table/shorthand shape as an activation's `sacrifice`.
    pub cycling_sacrifice: SacrificeCost,
    /// A hand-activated, discard-this-card ability (CR 113.6/602.5e — an activated ability that
    /// functions only from the hand, whose cost is "Discard this card" plus a mana cost; Magma
    /// Opus's "{U/R}{U/R}, Discard this card: Create a Treasure token."). The general sibling of
    /// [`Self::cycling`] for a card whose from-hand ability has an authored payload rather than
    /// cycling's fixed draw-1 — do not overload `cycling` for this. A slice (not `Option`) because
    /// typecycling grants one ability *per named type* (CR 702.29d — Valley Rannet's
    /// mountaincycling and forestcycling are two separate activated abilities, each with its own
    /// search filter); empty for a card without one. [`Game::activate_hand_ability`] takes an
    /// index selecting which entry to activate. `[[hand_ability]]` array-of-tables in TOML: each
    /// entry an `[[hand_ability]]` table with its own `[hand_ability.cost]` (same `[cost]`-table
    /// shape as a spell's cost) plus `[[hand_ability.effects]]` (the standard effects-array shape).
    pub hand_ability: Arc<[HandActivatedAbility]>,
    /// Forecast (CR 702.57 — Skyscribing's "Forecast — {2}{U}, Reveal this card from your hand:
    /// Each player draws a card."): a hand-activated ability that, unlike [`Self::hand_ability`],
    /// *reveals* rather than discards its card — the card stays in hand — and is activatable only
    /// during its owner's own upkeep, once each turn (CR 702.57a). Shares [`HandActivatedAbility`]'s
    /// cost+effects shape (no card needs both `hand_ability` and `forecast`). `None` for a card
    /// without it. `[forecast]` in TOML: `[forecast.cost]` (same `[cost]`-table shape as
    /// `[hand_ability.cost]`) plus `[[forecast.effects]]` (the standard effects-array shape).
    /// [`Game::activate_hand_ability`] is the shared entry point for both; it reveals-and-keeps
    /// and gates on upkeep/once-each-turn when this field (rather than `hand_ability`) is the one
    /// set.
    pub forecast: Option<HandActivatedAbility>,
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
    /// Cumulative upkeep (CR 702.24 — Jotun Grunt): "At the beginning of your upkeep, put an age
    /// counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age
    /// counter on it." `None` for a card without cumulative upkeep. `Some(cost)` queues, at the
    /// controller's every upkeep (unlike [`Self::echo`], with no "since your last upkeep" gate —
    /// this fires every time), an age counter ([`CounterKind::Age`]) followed by a
    /// [`PendingChoice::PayCumulativeUpkeepOrSacrifice`] scaled by the permanent's new total age
    /// counter count. `[cumulative_upkeep]` in TOML, the same table shape as
    /// [`CumulativeUpkeepCost`].
    pub cumulative_upkeep: Option<CumulativeUpkeepCost>,
    /// Recover (CR 702.59 — Grim Harvest): "When a creature is put into your graveyard from the
    /// battlefield, you may pay {cost}. If you do, return this card from your graveyard to your
    /// hand. Otherwise, exile this card." `None` for a card without recover. `Some(cost)` queues a
    /// pay-or-exile choice ([`PendingChoice::PayRecoverOrExile`]) once per creature death, from the
    /// battlefield, for every recover card already sitting in that creature's owner's graveyard —
    /// including the dying creature itself if it has recover (CR 702.59b). `[recover]` in TOML,
    /// the same `[cost]`-table shape as [`Self::echo`].
    pub recover: Option<Cost>,
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
    /// [`Effect::Dig(DigEffect::Cascade)`](crate::Effect::Dig(DigEffect::Cascade)) triggered ability on the stack above the
    /// cascading spell when it's cast (CR 702.85e), wired at the cast choke like `retrace`/`echo`.
    /// `cascade = true` in TOML.
    pub cascade: bool,
    /// Demonstrate (CR 702.147): "When you cast this spell, you may copy it. If you do, choose an
    /// opponent to also copy it. Players may choose new targets for their copies." `false` for a
    /// card without demonstrate. A rules-keyword (not a `[[abilities]]`): a `true` flag fabricates
    /// an [`Effect::Copy(CopyEffect::Demonstrate)`](crate::Effect::Copy(CopyEffect::Demonstrate)) triggered ability on the stack above
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
    /// which unprepares it. `None` for every ordinary card. Stored as the nested face's interned
    /// [`CardId`] so lookups stay pure once the front face is loaded. `[back]` (an inline
    /// `CardDef` table) in TOML.
    pub back: Option<CardId>,
    /// An adventure card's adventure half (CR 715 — soc/sos): the front face is the creature
    /// (this `CardDef`), and its `adventure` holds the instant/sorcery spell you may cast from
    /// hand instead (its own `cost`, `kind`, and `abilities`). On resolution the card is exiled
    /// "on an adventure" (CR 715.3d) and its owner may cast the creature half from exile later at
    /// normal cost (see [`Game::cast_adventure`]). `None` for every ordinary card. Stored as the
    /// nested face's interned [`CardId`] for the same reason as [`Self::back`]. `[adventure]` (an
    /// inline `CardDef` table) in TOML.
    pub adventure: Option<CardId>,
    /// A split card's two castable halves (CR 709 — Fire // Ice): this `CardDef` is the *fused*
    /// card (the combined characteristics every zone but the stack sees, CR 709.4 — combined name,
    /// mana cost, and colors), and `halves` holds the interned face ids you may actually cast.
    /// Only one half is ever cast (CR 709.4a), so casting goes through [`Game::cast_split_half`]
    /// and the fused def itself is not castable. Empty for every non-split card. `[[half]]`
    /// tables in TOML.
    pub halves: Arc<[CardId]>,
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
    /// it enter as a copy of any object of the [`EnterAsCopy::of`] type on the battlefield, with
    /// the riders in [`EnterAsCopy`] (Altered Ego's X extra +1/+1 counters; Cursed Mirror's
    /// until-end-of-turn duration + haste; Copy Enchantment's `of = "enchantment"`, which may copy
    /// an Aura and then pause to choose a host). The pause fires at the enter event, before ETB
    /// triggers (see [`crate::pending::ChoiceRequest::EnterAsCopy`]). `None` for a card without the replacement.
    /// `enter_as_copy = { .. }` in TOML.
    pub enter_as_copy: Option<EnterAsCopy>,
    /// Encore [cost] (CR 702.140 — Angel of Indemnity): "[cost], Exile this card from your
    /// graveyard: For each opponent, create a token copy of this card that attacks that opponent
    /// this turn if able. They gain haste. Sacrifice them at the beginning of the next end step.
    /// Activate only as a sorcery." `None` for a card without encore. A rules-keyword (not a
    /// `[[abilities]]`): a `Some` holds the encore **mana** cost; the "exile this card from your
    /// graveyard" half of the cost is intrinsic to the activation (paid by [`Game::encore`], not
    /// stored as a pip). A `&'static Cost` (leaked at load, like [`Self::suspend`]'s cost) keeps
    /// the nested rider small and shared. `[encore]` in TOML, the same `[cost]`-table shape as a
    /// spell's cost.
    pub encore: Option<&'static Cost>,
    /// "You may choose not to untap this during your untap step" (CR 502.2 — Rubinia Soulsinger):
    /// the untap turn-based action pauses this permanent's controller on a yes/no for each such
    /// permanent they control, letting them leave it tapped ([`PendingChoice::DeclineUntap`]).
    /// `false` for every ordinary permanent. `may_choose_not_to_untap = true` in TOML.
    pub may_choose_not_to_untap: bool,
    /// Dredge N (CR 702.52): a keyword ability that works from this card's graveyard. "If you would
    /// draw a card, you may instead mill exactly N and return this card from your graveyard to your
    /// hand" — a replacement for a draw, not a triggered ability (no stack item). `Some(n)` for a
    /// dredger; `None` for every other card. `dredge = N` in TOML. Read by the single-draw choke,
    /// which offers [`PendingChoice::ChooseDredge`] when the library holds at least N (CR 702.52a).
    pub dredge: Option<u8>,
    /// Vanishing N (CR 702.63 — Deadwood Treefolk): "This permanent enters with N time counters
    /// on it. At the beginning of your upkeep, remove a time counter from it. When the last is
    /// removed, sacrifice it." `None` for a card without vanishing. A rules-keyword (not
    /// `[[abilities]]`), mirroring [`Self::suspend`]'s posture: `Some(n)` places `n` time counters
    /// ([`CounterKind::Time`], tracked on [`Permanent::kind_counters`] — the battlefield sibling
    /// of suspend's exile-zone time-counter store) as the permanent enters (CR 702.63a — answered
    /// by `stack::enters_with_counters`, so it rides the ordinary enters-with-counters sites),
    /// removes one at each of the controller's upkeeps (CR 702.63b, see `Game::advance_step`'s
    /// `Step::Upkeep` arm), and — when the last is removed — synthesizes a real triggered "its
    /// controller sacrifices it" ability (CR 702.63c) so responses have a window, via the
    /// fabricated-single-ability `Game::queue_self_sacrifice_trigger` evoke also uses.
    /// `vanishing = N` in TOML.
    pub vanishing: Option<u8>,
    /// A cast-time upper bound on the announced value of {X}, beyond the mana the caster can pay
    /// (CR 601.2b — Open the Way's "X can't be greater than the number of players in the game").
    /// `None` (every ordinary {X} spell) leaves X bounded only by affordability. `Some(cap)`
    /// rejects a cast whose announced X exceeds the cap in [`Game::validate_cast`] and clamps the
    /// count-picker's offered ceiling in the snapshot. `cast_x_max = "player_count"` in TOML;
    /// ignored on a card with no {X}.
    pub cast_x_max: Option<CastXMax>,
}

/// A cast-time ceiling on a spell's announced {X} that isn't derived from mana (CR 601.2b).
/// See [`CardDef::cast_x_max`].
/// ponytail: the pool has exactly one such cap (Open the Way's player-count bound), so this is a
/// single-variant enum rather than a general `Amount`. Grow a variant (or fold in an `Amount`)
/// when a second differently-bounded {X} spell lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum CastXMax {
    /// "X can't be greater than the number of players in the game" — the count of living seats
    /// (CR 800.4a losers drop out), read from [`Game::living_player_count`].
    PlayerCount,
}

/// The riders on an [`CardDef::enter_as_copy`] replacement (CR 706/707.2). `Copy` — all scalars,
/// no `Vec` — so the nested replacement stays compact. `until_eot` reverts the copy at cleanup (Cursed Mirror,
/// [`Permanent::reverts_to_def_eot`]); `extra_counters` are additional +1/+1 counters the copy
/// enters with (Altered Ego's X); `gains_haste` grants the copy haste (Cursed Mirror's "except it
/// has haste"); `of` is the copyable type axis (Copy Enchantment's "any enchantment", CR 707.2,
/// vs. the default "any creature").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(deny_unknown_fields)
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub struct EnterAsCopy {
    #[cfg_attr(feature = "card-dsl", serde(default))]
    pub until_eot: bool,
    #[cfg_attr(feature = "card-dsl", serde(default = "de::zero_amount"))]
    pub extra_counters: Amount,
    #[cfg_attr(feature = "card-dsl", serde(default))]
    pub gains_haste: bool,
    #[cfg_attr(feature = "card-dsl", serde(default))]
    pub of: CopyTargetKind,
}

/// The candidate-object type [`CardDef::enter_as_copy`] may copy (CR 706/707.2): `Creature` (the
/// default — Altered Ego, Cursed Mirror) or `Enchantment` (Copy Enchantment, which includes Auras
/// — CR 303.2). `enter_as_copy = { of = "enchantment" }` in TOML; absent means `Creature`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum CopyTargetKind {
    #[default]
    Creature,
    Enchantment,
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
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
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
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(deny_unknown_fields)
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub struct HandActivatedAbility {
    pub cost: Cost,
    #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::arc_slice"))]
    pub effects: Arc<[Effect]>,
}

impl CardDef {
    /// This card's mana value (CR 202.3): the total pips in its mana cost — generic plus every
    /// colored, colorless `{C}`, and hybrid `{A/B}` pip. A `{X}` counts as 0 outside the stack
    /// (CR 202.3b), which is exactly how [`Cost`] stores it (the `x` marker adds nothing to the
    /// printed pips), so a graveyard/battlefield mana-value gate reads the printed value
    /// correctly. Each color/color hybrid pip counts 1 (CR 202.3f — both halves are one mana;
    /// Balefire Liege's {2}{R/W}{R/W}{R/W} is mana value 5). A Phyrexian pip counts 1 too, however
    /// it's paid (Vraska, Betrayal's Sting's {4}{B}{B/P} is mana value 6).
    pub fn mana_value(&self) -> u32 {
        let cost = self.cost;
        cost.generic as u32
            + cost.colorless as u32
            + cost.colored.iter().map(|&pips| pips as u32).sum::<u32>()
            + cost.hybrid.len() as u32
            + cost.phyrexian.len() as u32
    }

    /// Whether this card may be cast any time its owner has priority — an instant, or a
    /// spell with flash (CR 702.8a). The single timing predicate shared by the cast gate
    /// ([`Game::cast`]) and auto-pass ([`Game::meaningful_actions`]), so a future
    /// "as though it had flash" effect can't teach one site and not the other.
    pub fn is_instant_speed(&self) -> bool {
        !self.kind.is_sorcery_speed() || self.keywords.contains(&Keyword::Flash)
    }

    /// The facts about this card, as a spell being cast, that a [`SpendRestriction`] checks —
    /// derived fresh at each [`ManaPool::spend_plan`] call site rather than stored. `mana_value`
    /// reads the printed cost (CR 202.3b treats `{X}` as 0 off the stack), which is safe here
    /// because [`SpendRestriction::ManaValueAtLeastOrHasX`] always also accepts `has_x`
    /// regardless of the value actually chosen for `{X}`.
    pub fn spell_characteristics(&self) -> SpellCharacteristics {
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
pub fn color_identity(def: &CardDef) -> [bool; Color::COUNT] {
    if def.devoid {
        return [false; Color::COUNT];
    }
    if !def.colors.is_empty() {
        let mut identity = [false; Color::COUNT];
        for &color in def.colors.iter() {
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
    // A Phyrexian pip (CR 107.4f, {A/P}) contributes its color regardless of how it's paid (CR
    // 105.2b/903.4) — Vraska, Betrayal's Sting's {B/P} makes her black.
    for &color in def.cost.phyrexian {
        identity[color.index()] = true;
    }
    identity
}

/// Whether `card` is legal in a deck led by `commander` — its color identity must be a
/// subset of the commander's.
pub fn within_identity(card: &CardDef, commander: &CardDef) -> bool {
    let allowed = color_identity(commander);
    let needed = color_identity(card);
    (0..Color::COUNT).all(|i| !needed[i] || allowed[i])
}

/// Whether `def` is a basic land: has the "Basic" supertype (CR 205.4a). Reads
/// [`CardKind::Land`]'s `basic` flag rather than the card's name (or its `subtypes`, which a
/// nonbasic land can share without being basic — see the field's doc).
pub fn is_basic_land(def: &CardDef) -> bool {
    matches!(def.kind, CardKind::Land { basic: true, .. })
}
/// Scryfall oracle id for the canonical Treasure token (`cards/data/tokens/treasure.toml`).
pub const TREASURE_ORACLE_ID: &str = "3c549374-6c37-42e0-8d88-a8555d46732d";

/// The canonical Treasure token (CR: Treasure): a colorless artifact token with
/// "{T}, Sacrifice this artifact: Add one mana of any color." Every "create a Treasure" path
/// mints from this one definition. Prefers the profile installed from
/// `data/tokens/treasure.toml` (via [`crate::token_def`]) when the cards crate has loaded;
/// otherwise falls back to a builtin matching that file so engine-only tests still work.
/// The `any` mana it adds is a wildcard that pays any single colored pip or generic (see
/// [`Mana::Any`]). Carries the "Treasure" subtype so a [`PermanentFilter`] can pick Treasures
/// out from any other artifact (Goldspan Dragon's "Treasures you control" grant, #57).
pub fn treasure_token() -> CardDef {
    #[cfg(feature = "card-dsl")]
    if let Some(def) = crate::token_def(TREASURE_ORACLE_ID) {
        return def;
    }
    treasure_token_builtin()
}

/// A permanent that "becomes a Treasure artifact … and loses all other card types and abilities"
/// (CR 613.1d/613.1f — Vraska, Betrayal's Sting's −2). Unlike a copy effect (CR 707) this is a
/// type- and ability-SETTING effect: name, mana cost and identity are unchanged, only the card
/// types, subtypes and abilities are replaced by the Treasure profile. Color (CR 613 layer 5) is
/// untouched too, so `cost` carries the pip-derived colors and the explicit `colors`/`devoid`
/// overrides ride along for a target that states its color outright (a token, CR 111.4).
/// `legendary` stays because a supertype is not a card type (CR 205.4).
pub fn becomes_treasure(printed: CardDef) -> CardDef {
    CardDef {
        name: printed.name,
        id: printed.id,
        default_print: printed.default_print,
        cost: printed.cost,
        legendary: printed.legendary,
        colors: printed.colors,
        devoid: printed.devoid,
        ..treasure_token()
    }
}

/// Builtin Treasure profile — keep in lockstep with `cards/data/tokens/treasure.toml`.
fn treasure_token_builtin() -> CardDef {
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
            remove_counters_x: false,
            return_self: false,
            mill_self: 0,
            discard_cost: 0,
            exile_self: false,
            graveyard_exile_target_count: 0,
        }),
        effect: Effect::Mana(ManaEffect::Add {
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
            recipient: None,
        }),
        optional: false,
        min_level: 0,
        cost: Cost::FREE,
        condition: None,
        once_each_turn: false,
    }];
    CardDef {
        name: "Treasure",
        id: TREASURE_ORACLE_ID,
        default_print: "b4f61b5e-9c53-40b1-b93e-3ffa351ff052",
        cost: Cost::FREE,
        kind: CardKind::Artifact,
        legendary: false,
        snow: false,
        uncounterable: false,
        modal: false,
        modal_choose: 1,
        modal_choose_max: None,
        modal_choose_max_if_commander: false,
        keywords: empty_slice(),
        conditional_keywords: empty_slice(),
        abilities: ABILITIES.into(),
        identity_pips: empty_slice(),
        colors: empty_slice(),
        devoid: false,
        enters_tapped: false,
        enters_tapped_unless: None,
        enters_tapped_unless_you_pay_life: None,
        free_cast_if: None,
        alternative_cost: None,
        cast_only_during_combat: false,
        cast_only_before_attackers: false,
        approximates: None,
        oracle: None,
        sets: empty_slice(),
        subtypes: arc_slice(["Treasure"]),
        otags: empty_slice(),
        cycling: None,
        cycling_sacrifice: SacrificeCost::None,
        flashback: None,
        echo: None,
        cumulative_upkeep: None,
        recover: None,
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
        halves: empty_slice(),
        suspend: None,
        vanishing: None,
        cast_x_max: None,
        devour: None,
        demonstrate: false,
        enter_as_copy: None,
        encore: None,
        hand_ability: empty_slice(),
        forecast: None,
        may_choose_not_to_untap: false,
        dredge: None,
    }
}

/// Currency Converter's cash-out payoff for a nonland card: a 2/2 black Rogue creature token
/// (CR 400.10a).
pub fn rogue_token_stub() -> CardDef {
    CardDef {
        name: "Rogue",
        id: "9acbc363-827c-4146-a004-81be179a8c28",
        default_print: "80244f4b-3361-4776-a72b-1b0d70c7e855",
        cost: Cost::FREE,
        kind: CardKind::Creature {
            power: 2,
            toughness: 2,
            also: TypeSet::NONE,
        },
        legendary: false,
        snow: false,
        uncounterable: false,
        modal: false,
        modal_choose: 1,
        modal_choose_max: None,
        modal_choose_max_if_commander: false,
        keywords: empty_slice(),
        conditional_keywords: empty_slice(),
        abilities: empty_slice(),
        identity_pips: empty_slice(),
        colors: arc_slice([Color::Black]),
        devoid: false,
        enters_tapped: false,
        enters_tapped_unless: None,
        enters_tapped_unless_you_pay_life: None,
        free_cast_if: None,
        alternative_cost: None,
        cast_only_during_combat: false,
        cast_only_before_attackers: false,
        approximates: None,
        oracle: None,
        sets: empty_slice(),
        subtypes: arc_slice(["Rogue"]),
        otags: empty_slice(),
        cycling: None,
        cycling_sacrifice: SacrificeCost::None,
        flashback: None,
        echo: None,
        cumulative_upkeep: None,
        recover: None,
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
        halves: empty_slice(),
        suspend: None,
        vanishing: None,
        cast_x_max: None,
        devour: None,
        demonstrate: false,
        enter_as_copy: None,
        encore: None,
        hand_ability: empty_slice(),
        forecast: None,
        may_choose_not_to_untap: false,
        dredge: None,
    }
}

/// Skyclave Apparition's leaves-battlefield payoff: a blue Illusion creature token, base power
/// and toughness 0/0. The caller (`Game::check_leaves_battlefield_illusions`) bakes in the
/// exiled card's mana value as base P/T before minting, the same way `Effect::Token(TokenEffect::Create)`'s
/// `set_base_pt` does.
pub fn illusion_token() -> CardDef {
    CardDef {
        name: "Illusion",
        id: "ec406831-a1d0-4e41-bd09-f76d0ba206ae",
        default_print: "f6469938-5af9-4f0a-9f2e-603c833e48ba",
        cost: Cost::FREE,
        kind: CardKind::Creature {
            power: 0,
            toughness: 0,
            also: TypeSet::NONE,
        },
        legendary: false,
        snow: false,
        uncounterable: false,
        modal: false,
        modal_choose: 1,
        modal_choose_max: None,
        modal_choose_max_if_commander: false,
        keywords: empty_slice(),
        conditional_keywords: empty_slice(),
        abilities: empty_slice(),
        identity_pips: empty_slice(),
        colors: arc_slice([Color::Blue]),
        devoid: false,
        enters_tapped: false,
        enters_tapped_unless: None,
        enters_tapped_unless_you_pay_life: None,
        free_cast_if: None,
        alternative_cost: None,
        cast_only_during_combat: false,
        cast_only_before_attackers: false,
        approximates: None,
        oracle: None,
        sets: empty_slice(),
        subtypes: arc_slice(["Illusion"]),
        otags: empty_slice(),
        cycling: None,
        cycling_sacrifice: SacrificeCost::None,
        flashback: None,
        echo: None,
        cumulative_upkeep: None,
        recover: None,
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
        halves: empty_slice(),
        suspend: None,
        vanishing: None,
        cast_x_max: None,
        devour: None,
        demonstrate: false,
        enter_as_copy: None,
        encore: None,
        hand_ability: empty_slice(),
        forecast: None,
        may_choose_not_to_untap: false,
        dredge: None,
    }
}
