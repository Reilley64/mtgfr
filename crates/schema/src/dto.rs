//! The view/DTO structs: per-viewer game state, the pending-choice union, the lobby, accounts,
//! and deck-building/catalog wire shapes.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ObjectId;
use crate::intent::{WireAttack, WireBlock, WireTarget};

// ── Snapshot view types: the redacted full view of the game a client renders from ────

/// Per-player public facts plus that player's private counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PlayerView {
    pub player: u8,
    /// Display name from the seated account (not unique across players).
    #[serde(default)]
    pub username: String,
    pub life: i32,
    /// Extra generic mana to recast this player's commander from the command zone.
    pub commander_tax: u8,
    pub lost: bool,
    /// Card count only — hand identities are private (see the object list for own hand).
    pub hand_count: u32,
    pub library_count: u32,
    /// Floating mana pool for this player (every credit kind). Shown under each life orb.
    pub mana_pool: WireManaPool,
    /// Commander damage this player has taken, one entry per commander that has connected.
    /// 21 from any single one eliminates them (CR 903.10a) — a Commander win condition the client
    /// otherwise cannot see coming.
    #[serde(default)]
    pub commander_damage: Vec<CommanderDamageView>,
}

/// Wire form of [`engine::ManaPool`]: WUBRG counts, `{C}`, any, dual either-credits, and
/// restricted of-colors credits. Zero either/of_colors rows are omitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
pub struct WireManaPool {
    /// WUBRG amounts (length 5).
    pub colored: Vec<u8>,
    pub colorless: u8,
    pub any: u8,
    #[serde(default)]
    pub either: Vec<WireEitherMana>,
    #[serde(default)]
    pub of_colors: Vec<WireOfColorsMana>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WireEitherMana {
    /// First color index (WUBRG).
    pub a: u8,
    /// Second color index (WUBRG).
    pub b: u8,
    pub amount: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WireOfColorsMana {
    /// WUBRG bitmask of the restricted credit.
    pub mask: u8,
    pub amount: u8,
}

impl WireManaPool {
    pub fn from_engine(pool: &engine::ManaPool) -> Self {
        use engine::{COLOR_PAIRS, Color};
        let mut either = Vec::new();
        for (i, &(ca, cb)) in COLOR_PAIRS.iter().enumerate() {
            let amount = pool.either[i];
            if amount == 0 {
                continue;
            }
            either.push(WireEitherMana {
                a: ca.index() as u8,
                b: cb.index() as u8,
                amount,
            });
        }
        let mut of_colors = Vec::new();
        for (mask, &amount) in pool.of_colors.iter().enumerate() {
            if amount == 0 {
                continue;
            }
            of_colors.push(WireOfColorsMana {
                mask: mask as u8,
                amount,
            });
        }
        Self {
            colored: (0..Color::COUNT).map(|i| pool.colored[i]).collect(),
            colorless: pool.colorless,
            any: pool.any,
            either,
            of_colors,
        }
    }
}

/// Commander damage dealt to one player by one commander, keyed by that commander's owner (each
/// player has exactly one commander).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CommanderDamageView {
    /// The seat whose commander dealt it.
    pub from: u8,
    pub amount: i32,
}

/// What a card fundamentally is, for the client (mirror of `engine::CardKind`). Lets the UI
/// distinguish creatures/lands/spells for targeting highlights and drag-to-play validity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WireKind {
    Creature {
        power: i32,
        toughness: i32,
    },
    Instant,
    Sorcery,
    Enchantment,
    Artifact,
    /// A planeswalker with its starting loyalty (the printed number it enters with).
    Planeswalker {
        loyalty: i32,
    },
    /// A land and the colors it can tap for (WUBRG indices; see `engine::Color::index`):
    /// one entry for a mono producer, two for a dual ("{T}: Add {G} or {U}"), all five
    /// for "any color", empty for a pure colorless producer. Informational for the client
    /// (row layout / inspect); payment planning is engine-side (ADR 0022).
    Land {
        colors: Vec<u8>,
    },
}

/// A mana cost for the client: generic plus per-color pips (WUBRG).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct WireCost {
    pub generic: u8,
    /// Colored pips indexed WUBRG (see `engine::Color::index`).
    pub colored: [u8; 5],
    /// Whether the cost includes a variable `{X}` — the client must ask the caster to choose a
    /// value before submitting the cast (CR 601.2b). The chosen value rides the cast intent's
    /// `x` field; this is only the marker.
    #[serde(default)]
    pub has_x: bool,
}

/// One object the viewer may see, with its render-relevant state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ObjectView {
    pub id: ObjectId,
    /// Zone discriminant; see `engine::Zone`.
    pub zone: u8,
    pub owner: u8,
    pub controller: u8,
    /// Card id (Scryfall oracle id). Empty when face-down or a token without one.
    #[serde(default)]
    pub card_id: String,
    pub name: String,
    /// Printing UUID for art (CDN). Empty when unknown / token without art.
    #[serde(default)]
    pub print: String,
    pub kind: WireKind,
    pub mana_cost: WireCost,
    /// Whether casting this card requires choosing a target (drives the targeting UI).
    pub needs_target: bool,
    pub tapped: bool,
    pub summoning_sick: bool,
    /// Has haste — so it may attack / tap even while summoning sick (the client combines the two
    /// exactly as the engine does when deciding what can attack).
    pub has_haste: bool,
    /// Effective keywords right now (printed ∪ granted ∪ anthem) as stable snake_case ids
    /// (`flying`, `first_strike`, `ward:2`, `protection:red`). Drives Arena-style ability badges
    /// on the battlefield canvas.
    #[serde(default)]
    pub keywords: Vec<String>,
    pub power: i32,
    pub toughness: i32,
    /// Current loyalty for a planeswalker (0 otherwise). Painted like P/T on the canvas.
    #[serde(default)]
    pub loyalty: i32,
    pub plus_counters: i32,
    pub marked_damage: i32,
    pub is_commander: bool,
    /// Whether this creature is currently goaded (CR 701.38) — one-shot or continuous-from-Aura.
    #[serde(default)]
    pub goaded: bool,
    /// Whether tapping this permanent produces mana — a `produces` land, or anything with a
    /// free-tap mana ability (Sol Ring, Arcane Signet, a mana dork). Mana abilities never reach
    /// the action list (`meaningful_actions` skips them), so this is what tells the board where to
    /// offer the tap-for-mana click instead of guessing at an ability index.
    #[serde(default)]
    pub taps_for_mana: bool,
    /// Whether this permanent is prepared (soc/sos prepare DFCs). False for non-permanents and
    /// cards without a back face. Drives card-inspect play-face default.
    #[serde(default)]
    pub prepared: bool,
    /// Whether this permanent is phased out (CR 702.26 — treated as though it doesn't exist until
    /// its controller's next turn). False for every permanent that hasn't phased out.
    /// ponytail: the phased permanent is still projected into every viewer's board (with this flag
    /// set) rather than hidden from opponents the way CR's "doesn't exist" would suggest — the
    /// client renders it as phased. Hide it from opponents' snapshot if that posture is wanted.
    #[serde(default)]
    pub phased_out: bool,
    /// Whether this permanent is face down (CR 708 — a manifest): an anonymous 2/2 creature with
    /// its real `name`/`kind`/`mana_cost`/subtypes hidden. The client renders it as a face-down
    /// card back. False for every ordinary permanent.
    /// ponytail: the real card is hidden from *every* viewer (including its own controller, who in
    /// paper MTG may look at it) — the simplest correct redaction. Reveal it to the controller if
    /// that convenience is ever wanted.
    #[serde(default)]
    pub face_down: bool,
    /// Host this Aura/Equipment is attached to, if any. Drives Arena-style attachment stacks on
    /// the host instead of a battlefield row slot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attached_to: Option<ObjectId>,
    /// Live sourced modifiers for Alt-inspect (grouped by source card def name). Empty off the
    /// battlefield and when nothing modifies the permanent beyond its printed oracle.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<ModifierSourceView>,
}

/// One source card def's contributions to a permanent (Alt-inspect ledger).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ModifierSourceView {
    /// Card def name — clicking inspects this catalog card.
    pub source_name: String,
    /// Card id when resolvable (so inspect can load oracle text / default art). Empty when the
    /// source is a synthetic label (e.g. "Goad") or the permanent has left the board.
    #[serde(default)]
    pub source_card_id: String,
    /// Display crumbs: `"+1/+1"`, `"Flying"`, `"goaded"`, `"controls"`, `"mana ability"`, …
    pub contributions: Vec<String>,
}

/// One entry on the stack, for the stack panel. Bottom-first in `VisibleState.stack`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct StackObjectView {
    /// `spell` or `ability`.
    pub kind: String,
    /// The spell's stack-object id, or the ability's source permanent.
    pub source: ObjectId,
    pub controller: u8,
    /// A human-readable label (the spell's name, or a description of the ability's effect).
    pub label: String,
    /// The chosen target, if any.
    pub target: Option<WireTarget>,
}

/// One labelled item offered by a pending choice (a legal target, or a blocker to assign
/// damage to). The `label` is the object's name, so the prompt UI needn't join against the
/// object list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ChoiceItem {
    /// The object being chosen. Ignored when `player` is set — player-target choices (Bojuka Bog,
    /// Remorseful Cleric) name a seat instead of a battlefield object.
    pub id: ObjectId,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<u8>,
}

/// One action the viewer may take right now (`Game::legal_actions`, filtered to their own seat),
/// for the client's sectioned action bar. `id` is what a `take_action` intent sends back.
/// `kind`/`object`/`ability_index` are a flat trio rather than a tagged union — `kind` alone
/// (`"play_land"` / `"cast"` / `"activate"` / `"declare_attackers"` / `"declare_blockers"`) tells
/// the client which of the other two fields (if either) is set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ActionView {
    pub id: u64,
    pub kind: String,
    /// The card (play_land/cast) or ability source (activate); absent for a combat declaration.
    pub object: Option<ObjectId>,
    /// The activated ability's index on `object`; set only for `kind == "activate"`.
    pub ability_index: Option<u32>,
    /// Section bucket: "hand" | "battlefield" | "command" | "graveyard" | "exile" | "combat".
    pub section: String,
    /// Card name (play_land/cast) or ability label (activate/combat).
    pub label: String,
    pub needs_target: bool,
    /// The targets legal for this action right now (`Game::legal_targets`), so the client
    /// highlights the real set instead of reimplementing `TargetSpec`. Empty when the action
    /// takes no target — and also, for an activated ability, when it wants one but none is
    /// legal (unlike a cast, an ability is offered without checking that), which is why
    /// `needs_target` stays a separate fact rather than `!targets.is_empty()`.
    #[serde(default)]
    pub targets: Vec<WireTarget>,
    /// Set when this is a cast of a modal spell ("choose one —"). A modal spell's targets travel
    /// per mode (CR 700.2), so `targets` above is empty and `needs_target` is false for one: the
    /// client must pick modes first, then a target for each mode that wants one.
    #[serde(default)]
    pub modal: Option<ModalView>,
    /// The creatures that may pay this action's "Sacrifice a creature" activation cost, when it has
    /// one (CR 118.9 — the cost is chosen as it's paid, and rides the intent). `None` when it has no
    /// such cost; `Some([])` when it has one and nothing can pay it, so the client shows it as
    /// unusable instead of firing an intent the engine must reject.
    #[serde(default)]
    pub sacrifice_choices: Option<Vec<ObjectId>>,
    /// Hand cards that may pay this cast's additional discard cost. `None` when the cast has no
    /// such cost; `Some([])` when it needs one and the hand can't pay (mirrors `sacrifice_choices`).
    #[serde(default)]
    pub discard_choices: Option<Vec<ObjectId>>,
    /// Exact number of hand cards to discard when `discard_choices` is set; `0` when it is not.
    #[serde(default)]
    pub discard_count: u8,
    /// Own graveyard cards that may pay delve or escape exile for this cast. `None` when neither
    /// applies; `Some([])` when escape needs exile and the graveyard can't pay.
    #[serde(default)]
    pub graveyard_exile_choices: Option<Vec<ObjectId>>,
    /// Inclusive minimum cards to exile from the graveyard (0 for delve; escape's exact N).
    /// Meaningful only when `graveyard_exile_choices` is set; otherwise `0`.
    #[serde(default)]
    pub graveyard_exile_min: u8,
    /// Inclusive maximum cards to exile from the graveyard. Meaningful only when
    /// `graveyard_exile_choices` is set; otherwise `0`.
    #[serde(default)]
    pub graveyard_exile_max: u8,
    /// Whether this cast asks for `{X}` in the mana cost being paid (printed face for `cast`,
    /// back face for `cast_prepared`). False for non-cast actions.
    #[serde(default)]
    pub has_x: bool,
    /// Battlefield object ids `Game::plan_auto_taps` would tap to pay this action's mana
    /// (empty when the pool covers it or there is no mana cost). Same planner settle uses —
    /// the client paints a tap preview from this list on hover (ADR 0022).
    #[serde(default)]
    pub auto_tap: Vec<ObjectId>,
    /// Creatures that must be declared as attackers for this `declare_attackers` action to be
    /// legal (goad "attacks if able", CR 701.38a), each with a suggested legal defender. Empty
    /// for every other action kind. The client seeds combat staging from this so it never offers
    /// an empty "No attackers" confirm when the engine would reject it.
    #[serde(default)]
    pub required_attacks: Vec<WireAttack>,
}

/// A modal spell's printed modes and how many of them the caster picks (CR 700.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ModalView {
    /// The minimum number of distinct modes to choose.
    pub choose: u8,
    /// The maximum. Equal to `choose` for a plain "choose one"/"choose two"; larger when the card
    /// offers a range (Nexus Mentality's "you may choose both instead").
    pub choose_max: u8,
    /// The printed modes, in printed order. A mode's index is its position here — that's the
    /// `index` a `WireModeChoice` sends back.
    pub modes: Vec<ModeView>,
}

/// One printed mode of a modal spell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ModeView {
    /// A human-readable description of the mode's effect.
    pub label: String,
    /// The targets legal for *this mode* right now. Empty when the mode takes no target — and also
    /// when it takes one but none is legal, which is why `needs_target` is separate.
    pub targets: Vec<WireTarget>,
    pub needs_target: bool,
}

/// The current combat state a client renders: who's attacking and the declared blocks. Lets a
/// defending client highlight attackers and both sides draw attack/block arrows.
///
/// `attackers_declared` / `blockers_declared` are load-bearing for empty declarations: a
/// zero-attacker (or zero-blocker) confirm leaves `attackers`/`blocks` empty, but the engine
/// still treats the declaration as final — the client must flip off "No attackers/blockers"
/// from these flags, not from list length alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
pub struct CombatView {
    /// Each attacker with the player it's attacking, so clients draw the right arrow and each
    /// defender sees who's coming.
    pub attackers: Vec<WireAttack>,
    pub blocks: Vec<WireBlock>,
    /// The active player has already confirmed their attack declaration this combat (including
    /// an empty one).
    pub attackers_declared: bool,
    /// Seats that have already confirmed their block declaration this combat (including empty).
    pub blockers_declared: Vec<u8>,
}

/// A summary of the decision the engine is blocked on, if any. A discriminated union so each
/// wire `kind` carries only the fields that choice actually needs — the client's `PromptModal`
/// switches on `kind` and the compiler checks the switch is exhaustive (see ADR 0001 refinement).
/// The `kind` strings are load-bearing wire contract: they must stay exactly what they were
/// when this was one flattened struct with a `kind: String` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PendingChoiceView {
    /// How many items must be permuted, and a label per item so the player can tell them apart.
    OrderTriggers {
        player: u8,
        source: ObjectId,
        count: u32,
        labels: Vec<String>,
    },
    /// Legal targets to choose among. `label` is the effect being aimed (e.g. "Deal 3 damage to
    /// any target"); `source` is the permanent or spell it comes from. `optional` ("up to one" —
    /// Killian, Decisive Mentor) lets the player submit no target instead of one of `items`.
    ChooseTarget {
        player: u8,
        source: ObjectId,
        label: String,
        items: Vec<ChoiceItem>,
        optional: bool,
    },
    /// A multi-target spell's targets to choose (CR 601.2c): between `min` and `max` distinct
    /// `items` for the `spell` on the stack. `label` is the spell's name.
    ChooseSpellTargets {
        player: u8,
        spell: ObjectId,
        label: String,
        min: u8,
        max: u8,
        items: Vec<ChoiceItem>,
    },
    /// "Any number of target players" to choose for a targeted edict (CR 601.2c/608.2b — Priest
    /// of Forgotten Gods): between `min` (0) and `max` distinct `items` (players). `label` is the
    /// edict source's name.
    ChooseTargetPlayers {
        player: u8,
        source: ObjectId,
        label: String,
        min: u8,
        max: u8,
        items: Vec<ChoiceItem>,
    },
    /// Optional "you may" trigger — `label` is the effect (`Effect::label`) so the client can
    /// name what accepting does (mirrors `PayCost`).
    MayYesNo {
        player: u8,
        source: ObjectId,
        label: String,
    },
    /// The cost to accept an optional paid trigger (Trudge Garden's "you may pay {2}"), plus the
    /// effect label so the client can say what paying does.
    PayCost {
        player: u8,
        source: ObjectId,
        cost: WireCost,
        /// Short English for the paid effect (`Effect::label`) — e.g. "Create 1 Fungus Beast token(s)".
        label: String,
    },
    /// Pay `cost` to save `spell` from being countered, or decline and let it be countered
    /// (CR 701.5c "unless its controller pays" — the mirror image of `PayCost`).
    PayOrCounter {
        player: u8,
        spell: ObjectId,
        cost: WireCost,
    },
    /// Pay `cost` (Echo) to keep `source`, or decline and sacrifice it (CR 702.31 — the
    /// permanent-scoped twin of `PayOrCounter`).
    PayEchoOrSacrifice {
        player: u8,
        source: ObjectId,
        cost: WireCost,
    },
    /// Blockers to divide the attacker's damage among.
    AssignCombatDamage {
        player: u8,
        source: ObjectId,
        items: Vec<ChoiceItem>,
    },
    /// Targets to divide a divided-damage spell's total among (CR 601.2d).
    DivideSpellDamage {
        player: u8,
        spell: ObjectId,
        items: Vec<ChoiceItem>,
        total: i32,
    },
    /// Targets to divide a divided-counters spell's total among (CR 601.2d — Grove's Bounty).
    DivideCounters {
        player: u8,
        spell: ObjectId,
        items: Vec<ChoiceItem>,
        total: i32,
    },
    /// The looked-at cards, top of library. Private to the scrying player.
    Scry { player: u8, items: Vec<ChoiceItem> },
    /// The looked-at cards, top of library. Private to the surveilling player.
    Surveil { player: u8, items: Vec<ChoiceItem> },
    /// The matching library cards found. Private to the searching player.
    SearchLibrary { player: u8, items: Vec<ChoiceItem> },
    /// The looked-at top cards; the player may select up to `up_to` into a zone. Private to them.
    SelectFromTop {
        player: u8,
        up_to: u32,
        items: Vec<ChoiceItem>,
    },
    /// The looked-at top cards; the player must route `to_hand`/`to_bottom`/`to_exile_may_play`
    /// of them into their respective slots. Private to them.
    DistributeTop {
        player: u8,
        to_hand: u32,
        to_bottom: u32,
        to_exile_may_play: u32,
        items: Vec<ChoiceItem>,
    },
    /// `owner`'s graveyard cards (`items`, public — graveyard-zone); `player` may shuffle up to
    /// `max` (`0` = unbounded) of them into `owner`'s library. `owner` differs from `player` when
    /// the ability targeted a player (Quandrix Command); they coincide for a self-scoped ability
    /// (Perpetual Timepiece).
    ShuffleFromGraveyard {
        player: u8,
        owner: u8,
        source: ObjectId,
        max: u32,
        items: Vec<ChoiceItem>,
    },
    /// Permanents this player may choose to sacrifice.
    SacrificeEdict {
        player: u8,
        source: ObjectId,
        /// When true, the player keeps one permanent and sacrifices all others.
        #[serde(default)]
        keep_one: bool,
        items: Vec<ChoiceItem>,
    },
    /// Every counter-bearing permanent on the battlefield; this player may choose any number
    /// (including none) to proliferate (CR 701.27).
    Proliferate {
        player: u8,
        source: ObjectId,
        items: Vec<ChoiceItem>,
    },
    /// The other creatures this player controls; they may choose any number (including none) to
    /// phase out (CR 702.26 — Guardian of Faith). Answered like `Proliferate` (any subset).
    PhaseOut {
        player: u8,
        source: ObjectId,
        items: Vec<ChoiceItem>,
    },
    /// A triggered ability's second independent target clause (CR 603.3d — Kinetic Ooze's X≥10
    /// "double ... any number of other target creatures"), chosen as the trigger goes on the stack:
    /// between `min` and `max` distinct `items` for the ability from `source`. `label` is the effect
    /// being aimed. Answered by `ChooseTargets`, like `ChooseSpellTargets`.
    ChooseAbilityTargets {
        player: u8,
        source: ObjectId,
        label: String,
        min: u8,
        max: u8,
        items: Vec<ChoiceItem>,
    },
    /// This player may sacrifice one of `items` (a permanent they control) to gain a rider
    /// effect, or decline (CR 601.2f-style resolution-time optional cost).
    MaySacrifice {
        player: u8,
        source: ObjectId,
        items: Vec<ChoiceItem>,
    },
    /// This player must choose exactly `count` of `items` (their own permanents) to sacrifice —
    /// a forced sacrifice they direct (CR 701.16a — Lotus Field's ETB, Smothering Abomination's
    /// upkeep). Unlike [`MaySacrifice`], declining isn't legal.
    ChooseOwnSacrifices {
        player: u8,
        source: ObjectId,
        count: u32,
        items: Vec<ChoiceItem>,
    },
    /// This player may sacrifice any subset of `items` (the other creatures they control) as a
    /// Devour N creature (`source`) enters; it gains `multiplier × count` +1/+1 counters (CR
    /// 702.82 — Mycoloth, Ribtruss Roaster). An empty selection declines (0 counters). The
    /// battlefield candidates are public.
    Devour {
        player: u8,
        source: ObjectId,
        multiplier: u32,
        items: Vec<ChoiceItem>,
    },
    /// This player must exile one of `items` (a card in their own graveyard) to a multi-player
    /// fan-out (Augusta, Order Returned). The graveyard is public, so `items` are public.
    ExileFromGraveyard {
        player: u8,
        source: ObjectId,
        items: Vec<ChoiceItem>,
    },
    /// This player (Tragic Arrogance's caster) chooses which of `target_player`'s nonland
    /// permanents (`items`) to keep — up to one of each type (artifact/creature/enchantment); the
    /// rest are sacrificed. The battlefield is public, so `items` are public. Answered like the
    /// sacrifice choices (naming the kept subset); the answering seat is `player` (the caster), not
    /// `target_player`.
    CasterKeepPermanents {
        player: u8,
        source: ObjectId,
        target_player: u8,
        items: Vec<ChoiceItem>,
    },
    /// This player (Nils' controller) puts a +1/+1 counter on up to one of `target_player`'s
    /// creatures (`items`), or declines. The battlefield is public, so `items` are public. Answered
    /// like the sacrifice choices (naming the 0-or-1 chosen creature); the answering seat is
    /// `player` (the chooser), not `target_player`.
    ChooseCounterTargetForPlayer {
        player: u8,
        source: ObjectId,
        target_player: u8,
        items: Vec<ChoiceItem>,
    },
    /// This player may return one of `items` (a card in their own graveyard) to their hand, or
    /// decline (CR 601.2f-style resolution-time optional rider — Deadly Brew).
    MayReturnFromGraveyard {
        player: u8,
        source: ObjectId,
        items: Vec<ChoiceItem>,
    },
    /// This player may discard one of `items` (a card in their own hand, private to them) to
    /// gain a rider effect, or decline (CR 608.2c-style resolution-time optional sub-action —
    /// Quintorius, History Chaser's +1).
    MayDiscard {
        player: u8,
        source: ObjectId,
        items: Vec<ChoiceItem>,
    },
    /// This player must discard `count` cards from their hand (`items`, private to them).
    Discard {
        player: u8,
        count: u32,
        items: Vec<ChoiceItem>,
    },
    /// This player may put one hand land (`items`, private to them) onto the battlefield, or
    /// decline.
    PutLandFromHand { player: u8, items: Vec<ChoiceItem> },
    /// This player may put one card exiled with `source` (`items`, public — exile-zone) into its
    /// owner's graveyard, or decline.
    ChooseExiledWithCard {
        player: u8,
        source: ObjectId,
        items: Vec<ChoiceItem>,
    },
    /// This player may choose one card exiled with `source` (`items`, public — exile-zone) to
    /// grant the free-cast permission (CR 118.5), or decline.
    ChooseExiledWithCardToCast {
        player: u8,
        source: ObjectId,
        items: Vec<ChoiceItem>,
    },
    /// This player may choose one card (`items`, public — exile-zone) among a just-exiled dig
    /// batch to grant the free-cast permission (CR 118.5), or decline. Every other card in the
    /// batch goes to the bottom of the library once answered.
    ChooseExiledDigToCastFree {
        player: u8,
        source: ObjectId,
        items: Vec<ChoiceItem>,
    },
    /// This player (the caster) may exile another top-of-library card or stop (Dance with
    /// Calamity). `items` are the cards exiled so far (public — exile-zone), `total_mv` their
    /// summed mana value, and `budget` the bust threshold. Answered by the yes/no `AnswerMay`.
    DanceExileMore {
        player: u8,
        source: ObjectId,
        total_mv: u32,
        budget: u32,
        items: Vec<ChoiceItem>,
    },
    /// This player (an **opponent** of the controller) must choose one of two exile piles; the
    /// chosen pile goes to the controller's graveyard (Abstract Performance). `pile_a`/`pile_b`
    /// are the piles' cards in order — a card exiled face down (CR 701.9, Abstract Performance's
    /// first pile) keeps its slot (so pile size/position stay visible to this player) but an
    /// empty `label`; the controller's own view gets every card's real name.
    OpponentChoosesPile {
        player: u8,
        source: ObjectId,
        pile_a: Vec<ChoiceItem>,
        pile_b: Vec<ChoiceItem>,
    },
    /// This player (an **opponent** of the controller) must choose one nonland card exiled this
    /// way (`items`, public — exile-zone); the controller then casts up to two of the others free
    /// (Plargg and Nassari).
    OpponentChoosesExiledNonland {
        player: u8,
        source: ObjectId,
        items: Vec<ChoiceItem>,
    },
    /// This player (the controller) may choose up to `count` of `items` (public — exile-zone) to
    /// grant the free-cast permission (CR 118.5); the rest go to hand or stay exiled per the card
    /// (Abstract Performance's kept pile, Plargg and Nassari's other exiled cards).
    ChooseExiledToCastFree {
        player: u8,
        source: ObjectId,
        count: u8,
        items: Vec<ChoiceItem>,
    },
    /// This player may put `item` (a just-revealed library card, public) onto the battlefield,
    /// or decline and put it into hand instead.
    RevealedCardToBattlefieldOrHand { player: u8, item: ChoiceItem },
    /// This player must choose one mode of a "choose one" triggered ability. `labels` describes
    /// each mode (public card text), one per selectable index.
    ChooseMode {
        player: u8,
        source: ObjectId,
        labels: Vec<String>,
    },
    /// This player may choose `choose` distinct modes of a modal *triggered* ability (CR 700.2 —
    /// Shadrix Silverquill's begin-combat "you may choose two"), each with its own target where
    /// the mode needs one — reuses the modal-*cast*-action's per-mode shape ([`ModeView`]).
    /// `optional`: declining (an empty selection) is legal and drops the whole ability.
    ChooseTriggerModes {
        player: u8,
        source: ObjectId,
        choose: u8,
        optional: bool,
        modes: Vec<ModeView>,
    },
    /// This player (a mana ability's controller) must name one color; `amount` mana of that
    /// color is added to their pool (CR 106.4 "add N mana of any one color" — Lotus Field, Kami
    /// of Whispered Hopes).
    ChooseManaColor {
        player: u8,
        source: ObjectId,
        amount: u8,
    },
    /// This player (an as-enters permanent's controller) must name a creature type for
    /// `source` (CR 614.12/700.9-style "as ~ enters, choose a creature type" — Patchwork
    /// Banner). `options` is the pool's known creature-type table.
    ChooseCreatureType {
        player: u8,
        source: ObjectId,
        options: Vec<String>,
    },
    /// This player (an as-enters permanent's controller) must name a color for `source` (CR
    /// 614.12/700.9-style "as ~ enters, choose a color" — Flickering Ward). The candidates are the
    /// fixed five WUBRG colors, so no `options` list is carried.
    ChooseColor { player: u8, source: ObjectId },
    /// This player (an entering permanent's controller) may have `source` enter as a copy of one
    /// of `items` (every other creature on the battlefield, public — CR 706/707.2, Altered Ego,
    /// Cursed Mirror). Answered with the chosen creature, or a decline (the "you may").
    ChooseCopyTarget {
        player: u8,
        source: ObjectId,
        items: Vec<ChoiceItem>,
    },
    /// This player (the deployed attachment's controller) must choose a host among `items` (the
    /// eligible battlefield creatures, public) for the Aura or Equipment it just put onto the
    /// battlefield without casting it. An Aura's host is mandatory — a legal `enchant` target
    /// (CR 303.4f — Songbirds' Blessing, Armored Skyhunter). Equipment's host is optional — any
    /// creature this player controls, and declining leaves it unattached (CR 301.5c, `optional`).
    ChooseAttachHost {
        player: u8,
        attachment: ObjectId,
        items: Vec<ChoiceItem>,
        optional: bool,
    },
}

/// The complete view one player is allowed to see: turn state, per-player facts, and the
/// objects visible to them (all public zones + their own hand; opponents' hands and all
/// libraries are counts only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct VisibleState {
    pub viewer: u8,
    pub active_player: u8,
    /// Step discriminant; see `engine::Step`.
    pub step: u8,
    pub priority: u8,
    pub players: Vec<PlayerView>,
    pub objects: Vec<ObjectView>,
    /// The stack, bottom-first (last entry is the top, which resolves next).
    pub stack: Vec<StackObjectView>,
    /// Declared attackers and blocks this combat (empty outside combat).
    pub combat: CombatView,
    /// Whether the current priority holder has a meaningful action (for "you can act" emphasis);
    /// the server auto-passes when this is false, so the client rarely renders a can't-act turn.
    pub can_act: bool,
    /// Whether THIS viewer has yielded ("don't care") to the current stack — the server's
    /// authoritative flag (set via `POST /yield/v1`, cleared whenever the stack empties), so
    /// the client never has to mirror the clearing rules. Always false for a spectator.
    #[serde(default)]
    pub yielded: bool,
    /// Whether THIS viewer has turn-yielded (auto-pass until their turn / until they act — ADR 0029).
    /// Always false for a spectator.
    #[serde(default)]
    pub turn_yielded: bool,
    /// Milliseconds until an uncontested stack auto-resolves; `0` when no stack-hold is active.
    #[serde(default)]
    pub stack_hold_remaining_ms: u32,
    pub pending_choice: Option<PendingChoiceView>,
    /// The viewer's own legal actions right now (empty for a spectator, or anyone else's seat).
    #[serde(default)]
    pub actions: Vec<ActionView>,
}

// ── Lobby ────────────────────────────────────────────────────────────────────────────
// The pre-game flow: a created table holds up-to-four claimable seats (bound to a
// per-browser token), each choosing a deck, that the host starts. Poll `GET /tables/{table}/lobby`
// for state until `started`, then connect the game stream (ADR-style note: poll-based, a
// push channel if it ever feels laggy).

/// Create a fresh lobby table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CreateTableResponse {
    pub table_id: String,
}

/// Claim the next open seat with one of the caller's saved decks. Identity comes from the
/// session cookie, so no token travels in the body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct JoinRequest {
    pub table_id: String,
    pub deck_id: i64,
}

/// Toggle a seated player's ready flag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReadyRequest {
    pub table_id: String,
    pub ready: bool,
}

/// The host starts the game once ≥2 seats are claimed and all are ready.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct StartRequest {
    pub table_id: String,
}

/// One seat's lobby state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SeatView {
    pub player: u8,
    pub claimed: bool,
    /// The seated account's display name (None if the seat is open).
    pub username: Option<String>,
    /// The chosen deck's name, for the lobby to display (None if the seat is open).
    pub deck_name: Option<String>,
    pub ready: bool,
    pub is_host: bool,
    /// Set only in the reply to the requesting user: whether this seat is theirs.
    pub is_you: bool,
}

/// The full lobby state a client renders (and a mutation's reply). `you` is the seat the
/// requesting token holds, if any.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LobbyView {
    pub table_id: String,
    pub seats: Vec<SeatView>,
    pub you: Option<u8>,
    pub started: bool,
    /// Why the requesting user can't start the game right now (`NotHost`, `NeedTwoPlayers`,
    /// `NotAllReady`, …), or `None` when they can. This replaced a bare `can_start` bool: the
    /// server always knew the reason and dropped it, so a host saw a greyed-out button and had to
    /// guess which of the three things was missing.
    #[serde(default)]
    pub start_error: Option<String>,
    /// Set when a mutation was rejected (e.g. table full, not the host).
    pub error: Option<String>,
}

// ── Accounts ─────────────────────────────────────────────────────────────────────────
// Self-hosted email+password auth; a login/signup binds a session to an HttpOnly cookie.
// Seat ownership and deck ownership are keyed by the authenticated user, not a browser token.

/// Login credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

/// Signup credentials — username is required and not unique across accounts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SignupCredentials {
    pub email: String,
    pub password: String,
    pub username: String,
}

/// The signed-in user (the `GET /auth/me` reply).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Me {
    pub email: String,
    pub username: String,
}

// ── Deck building & card catalog ─────────────────────────────────────────────────────
// A deck is user-authored data (commander + a list of `(card, count)`), replacing the old
// hardcoded `DeckChoice` precons. The catalog exposes the pool so the builder can browse it;
// it carries the engine's *actual simplified* stats/keywords/effect text (not Scryfall
// oracle text, which wouldn't match a simplified card).

/// One line of a decklist: a Card id, how many copies, and the chosen Printing UUID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeckCardEntry {
    /// Card id (Scryfall oracle id).
    pub id: String,
    pub count: u32,
    /// Scryfall card UUID for art (required).
    pub print: String,
}

/// A deck in a list view (no card contents).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeckSummary {
    pub id: i64,
    pub name: String,
    pub commander: String,
    /// Printing UUID for the commander's art (list hover preview).
    #[serde(default)]
    pub commander_print: String,
}

/// A deck with its full contents (the builder's edit view).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeckDetail {
    pub id: i64,
    pub name: String,
    /// Commander Card id (Scryfall oracle id).
    pub commander: String,
    /// Printing UUID for the commander's art.
    pub commander_print: String,
    pub cards: Vec<DeckCardEntry>,
}

/// Body for creating or updating a deck.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SaveDeckRequest {
    pub name: String,
    /// Commander Card id (Scryfall oracle id).
    pub commander: String,
    /// Printing UUID for the commander's art.
    pub commander_print: String,
    pub cards: Vec<DeckCardEntry>,
}

/// Why a deck was rejected as illegal — every problem at once, for the builder to list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeckError {
    pub problems: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_choice_view_pins_the_wire_kind_and_fields_per_variant() {
        // Pins the exact JSON per kind — a typo or dropped field here is a silently dead
        // client modal (a hung game), so this locks the wire contract the enum replaced.
        assert_eq!(
            serde_json::to_value(PendingChoiceView::OrderTriggers {
                player: 0,
                source: 7,
                count: 2,
                labels: vec!["Draw 1".to_string(), "Gain 1 life".to_string()],
            })
            .unwrap(),
            serde_json::json!({
                "kind": "order_triggers", "player": 0, "source": 7, "count": 2,
                "labels": ["Draw 1", "Gain 1 life"],
            }),
        );
        assert_eq!(
            serde_json::to_value(PendingChoiceView::ChooseTarget {
                player: 1,
                source: 7,
                label: "Deal 1 damage to any target".to_string(),
                items: vec![ChoiceItem {
                    id: 4,
                    label: "Bear".to_string(),
                    player: None,
                }],
                optional: false,
            })
            .unwrap(),
            serde_json::json!({
                "kind": "choose_target", "player": 1, "source": 7,
                "label": "Deal 1 damage to any target",
                "items": [{"id": 4, "label": "Bear"}], "optional": false,
            }),
        );
        assert_eq!(
            serde_json::to_value(PendingChoiceView::ChooseTarget {
                player: 0,
                source: 7,
                label: "Exile target player's graveyard".to_string(),
                items: vec![ChoiceItem {
                    id: 0,
                    label: "Player 2".to_string(),
                    player: Some(1),
                }],
                optional: false,
            })
            .unwrap(),
            serde_json::json!({
                "kind": "choose_target", "player": 0, "source": 7,
                "label": "Exile target player's graveyard",
                "items": [{"id": 0, "label": "Player 2", "player": 1}], "optional": false,
            }),
        );
        assert_eq!(
            serde_json::to_value(PendingChoiceView::ChooseSpellTargets {
                player: 1,
                spell: 9,
                label: "Lightning Bolt".to_string(),
                min: 1,
                max: 1,
                items: vec![ChoiceItem {
                    id: 4,
                    label: "Bear".to_string(),
                    player: None,
                }],
            })
            .unwrap(),
            serde_json::json!({
                "kind": "choose_spell_targets", "player": 1, "spell": 9,
                "label": "Lightning Bolt", "min": 1, "max": 1,
                "items": [{"id": 4, "label": "Bear"}],
            }),
        );
        assert_eq!(
            serde_json::to_value(PendingChoiceView::MayYesNo {
                player: 2,
                source: 7,
                label: "Create a Treasure token".to_string(),
            })
            .unwrap(),
            serde_json::json!({
                "kind": "may_yes_no", "player": 2, "source": 7,
                "label": "Create a Treasure token",
            }),
        );
        assert_eq!(
            serde_json::to_value(PendingChoiceView::PayCost {
                player: 3,
                source: 7,
                cost: WireCost {
                    generic: 1,
                    colored: [0, 0, 1, 0, 0],
                    has_x: false,
                },
                label: "Draw a card".to_string(),
            })
            .unwrap(),
            serde_json::json!({
                "kind": "pay_cost", "player": 3, "source": 7,
                "cost": {"generic": 1, "colored": [0, 0, 1, 0, 0], "has_x": false},
                "label": "Draw a card",
            }),
        );
        assert_eq!(
            serde_json::to_value(PendingChoiceView::AssignCombatDamage {
                player: 0,
                source: 5,
                items: vec![ChoiceItem {
                    id: 6,
                    label: "Bear".to_string(),
                    player: None,
                }],
            })
            .unwrap(),
            serde_json::json!({
                "kind": "assign_combat_damage", "player": 0, "source": 5,
                "items": [{"id": 6, "label": "Bear"}],
            }),
        );
        assert_eq!(
            serde_json::to_value(PendingChoiceView::Scry {
                player: 0,
                items: Vec::new()
            })
            .unwrap(),
            serde_json::json!({"kind": "scry", "player": 0, "items": []}),
        );
        assert_eq!(
            serde_json::to_value(PendingChoiceView::Surveil {
                player: 0,
                items: Vec::new()
            })
            .unwrap(),
            serde_json::json!({"kind": "surveil", "player": 0, "items": []}),
        );
        assert_eq!(
            serde_json::to_value(PendingChoiceView::SearchLibrary {
                player: 0,
                items: Vec::new()
            })
            .unwrap(),
            serde_json::json!({"kind": "search_library", "player": 0, "items": []}),
        );
        assert_eq!(
            serde_json::to_value(PendingChoiceView::SacrificeEdict {
                player: 0,
                source: 9,
                keep_one: false,
                items: Vec::new(),
            })
            .unwrap(),
            serde_json::json!({"kind": "sacrifice_edict", "player": 0, "source": 9, "keep_one": false, "items": []}),
        );
        assert_eq!(
            serde_json::to_value(PendingChoiceView::Discard {
                player: 0,
                count: 2,
                items: vec![ChoiceItem {
                    id: 7,
                    label: "Grizzly Bears".into(),
                    player: None,
                }],
            })
            .unwrap(),
            serde_json::json!({
                "kind": "discard", "player": 0, "count": 2,
                "items": [{"id": 7, "label": "Grizzly Bears"}],
            }),
        );
        // A non-owner viewer sees the count but no card ids (the hand is private).
        assert_eq!(
            serde_json::to_value(PendingChoiceView::Discard {
                player: 0,
                count: 2,
                items: Vec::new(),
            })
            .unwrap(),
            serde_json::json!({"kind": "discard", "player": 0, "count": 2, "items": []}),
        );
    }
}
