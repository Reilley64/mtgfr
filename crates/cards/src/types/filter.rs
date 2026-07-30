use super::*;
#[cfg(feature = "card-dsl")]
use crate::de;

/// Whose graveyard a [`TargetSpec::CardInGraveyard`] draws from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum GraveyardScope {
    /// The ability's controller's own graveyard (Raise Dead's "your graveyard").
    Yours,
    /// Any player's graveyard (Reanimate's "a graveyard").
    Any,
    /// A living opponent's graveyard, never the controller's own (CR "target card in an
    /// opponent's graveyard" — Nezumi Graverobber's "Exile target card from an opponent's
    /// graveyard").
    Opponents,
}

/// What an ability targets, checked when the spell/ability is put on the stack.
/// ponytail: single-target model; multi-target grows from real cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum TargetSpec {
    /// Takes no target.
    #[default]
    None,
    /// Targets a creature on the battlefield.
    Creature,
    /// Targets a creature the choosing player controls (Twinflame — "target creature you control").
    #[cfg_attr(feature = "card-dsl", serde(rename = "creature_you_control"))]
    CreatureYouControl,
    /// Targets any (living) player.
    Player,
    /// Targets a living player other than the choosing player (CR "target opponent" — Secret
    /// Rendezvous, Witherbloom Command mode 3).
    #[cfg_attr(feature = "card-dsl", serde(rename = "opponent"))]
    OpponentPlayer,
    /// "Any target": a creature, a player, or a planeswalker (modern wording, CR 115.4).
    /// ponytail: battles aren't a modeled permanent type, so creature-or-player-or-planeswalker
    /// is the entire "any target" set this pool can produce — revisit when battles land.
    #[cfg_attr(feature = "card-dsl", serde(rename = "any"))]
    AnyTarget,
    /// A creature or planeswalker on the battlefield (Rip Apart, Lightning Strike-style burn).
    #[cfg_attr(feature = "card-dsl", serde(rename = "creature_or_planeswalker"))]
    CreatureOrPlaneswalker,
    /// A player or a planeswalker (Balefire Liege's "target player or planeswalker").
    #[cfg_attr(feature = "card-dsl", serde(rename = "player_or_planeswalker"))]
    PlayerOrPlaneswalker,
    /// A creature card in the ability controller's own graveyard (Raise Dead).
    #[cfg_attr(feature = "card-dsl", serde(rename = "your_graveyard"))]
    CreatureCardInYourGraveyard,
    /// A creature card in any graveyard (Reanimate).
    #[cfg_attr(feature = "card-dsl", serde(rename = "any_graveyard"))]
    CreatureCardInAnyGraveyard,
    /// A card in a graveyard matching a composable [`CardFilter`] (Sevinne's Reclamation's
    /// "target permanent card with mana value 3 or less from your graveyard"). `whose` scopes
    /// which graveyard(s) are searched. The two creature-card variants above stay as sugar for
    /// their common case rather than migrating onto this general form.
    CardInGraveyard {
        whose: GraveyardScope,
        filter: CardFilter,
        /// "another target creature card" (Deadwood Treefolk) — excludes the ability's own
        /// source card, the same "each other" carve-out [`PermanentFilter::other`] gives
        /// battlefield targets. Needs a source to exclude (the source itself, once it's the
        /// dying/leaving card sitting in the graveyard); without one it restricts nothing.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        other: bool,
    },
    /// An instant or sorcery *spell* currently on the stack (Twincast). Targets the stack
    /// object, not a card in a zone.
    InstantOrSorcerySpellOnStack,
    /// A spell currently on the stack matching a [`SpellFilter`] (Counterspell / Arcane Denial's
    /// unrestricted "counter target spell" is [`SpellFilter::AllSpells`]; Decisive Denial's
    /// "target noncreature spell" and Quandrix Command's "target artifact or enchantment spell"
    /// narrow it). [`Effect::Misc(MiscEffect::CounterTargetSpell)::filter`] supplies the filter.
    SpellOnStack(SpellFilter),
    /// A spell currently on the stack that has exactly one target (Willbender's "target spell …
    /// with a single target", CR 114.6). Targets the stack object; used by
    /// [`Effect::Copy(CopyEffect::ChangeTargetOfTargetSpellOrAbility)`] to pick the spell to bend.
    /// ponytail: CR's "spell or ability" also reaches a single-target activated/triggered ability
    /// on the stack, but stack abilities carry no object identity in this engine (they're keyed by
    /// source, not a chosen `Target`), so only spells are targetable here — see #163's residual gap.
    #[cfg_attr(feature = "card-dsl", serde(rename = "single_target_spell_on_stack"))]
    SingleTargetSpellOnStack,
    /// An *activated* ability currently on the stack (Azorius Guildmage's "Counter target
    /// activated ability", CR 112.7a). Targets the ability's stack item by its `source` id, not a
    /// card in a zone. Mana abilities never reach the stack (CR 605.3b); triggered abilities are
    /// excluded here (only `StackItem::Ability { activated: true }` entries are yielded).
    /// ponytail: keyed by the ability's `source` id — stack abilities carry no object identity of
    /// their own in this engine (same gap #163's `SingleTargetSpellOnStack` note names). If two
    /// activated abilities on the stack shared a source, resolution counters the topmost match; no
    /// pool card produces that, and Azorius counters exactly one. Give stack abilities real object
    /// identity when a card forces the distinction.
    ActivatedAbilityOnStack,
    /// A spell on the stack *or* a permanent on the battlefield, unrestricted — the lace cycle's
    /// "target spell or permanent" (Deathlace). The only spec that spans the two zones; every
    /// other spell spec ([`Self::SpellOnStack`]) and permanent spec ([`Self::Permanent`]) picks
    /// one. Unfiltered because the five cards that print it are unfiltered.
    #[cfg_attr(feature = "card-dsl", serde(rename = "spell_or_permanent"))]
    SpellOrPermanent,
    /// A target artifact, enchantment, or planeswalker on the battlefield (Fracture). The
    /// noncreature-permanent removal set the pool needs; Auras count as enchantments.
    ArtifactEnchantmentOrPlaneswalker,
    /// A target battlefield permanent matching a composable [`PermanentFilter`] (Anguished
    /// Unmaking's "any nonland permanent", Abrade's "artifact", Skyclave Apparition's "nonland
    /// nontoken permanent an opponent controls with mana value 4 or less"). Spelled in TOML as
    /// `target = { permanent = { … } }`. The one target spec that scales to new narrowings
    /// without a new variant; the older unit variants above stay as convenient sugar.
    Permanent(PermanentFilter),
    /// A creature *token* the choosing player controls — the "creature token you control" chosen
    /// by Populate (CR 701.32), used with [`Effect::Token(TokenEffect::CreateCopy)`].
    /// ponytail: populate *chooses* a token, it doesn't *target* one (CR 701.32 is a choice, not a
    /// target); reusing the target machinery is faithful enough — the pool has no card where the
    /// choose/target distinction (hexproof, shroud) matters.
    CreatureTokenYouControl,
    /// The ability's own source, no real choice (Hangarback Walker's "put a +1/+1 counter on
    /// this creature", Gorma's "put a +1/+1 counter on Gorma", Primordial Hydra's "double the
    /// number of +1/+1 counters on this creature"). CR-faithful: these abilities don't say
    /// "target" at all — the source is a fixed reference, not a chosen one — so resolving this
    /// spec never raises a [`PendingChoice`] and skips the shroud/hexproof/protection check that
    /// only applies to true targets (CR 115, 702.11/702.16b/702.18).
    #[cfg_attr(feature = "card-dsl", serde(rename = "this"))]
    ThisPermanent,
    /// The creature this Aura/Equipment is attached to, no real choice (Redemption Arc's "exile
    /// enchanted creature"). Empty (no legal target) if the source isn't currently attached to
    /// anything. Same non-targeted CR treatment as [`ThisPermanent`](Self::ThisPermanent).
    #[cfg_attr(feature = "card-dsl", serde(rename = "enchanted_creature"))]
    EnchantedCreature,
    /// Animate Dead's own ETB reanimation target: the graveyard creature card this Aura was cast
    /// targeting (CR 303.4a's "enchant creature card in a graveyard"), captured on the permanent
    /// as it entered ([`CardDef::enchant_graveyard`], [`Permanent::cast_time_enchant_target`]).
    /// No real choice at resolution — the choice already happened at cast — so this resolves
    /// straight to the stack like [`ThisPermanent`](Self::ThisPermanent)/
    /// [`EnchantedCreature`](Self::EnchantedCreature) rather than pausing on a fresh target
    /// choice; empty (CR 603.3c: the ability is dropped) if the captured card has since left the
    /// graveyard.
    /// ponytail: named for the one card that needs it — no pool card wants a *second* such
    /// look-back, so this isn't generalized into a reusable "this permanent's own cast target"
    /// concept.
    ThisAurasGraveyardTarget,
}

/// A chosen target: either a permanent (by object id) or a player. Spells/abilities target
/// one of these; which are legal is governed by the [`TargetSpec`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Object(ObjectId),
    Player(PlayerId),
}

impl Target {
    /// The object id this target names, if it's a permanent (players have no object id).
    /// Used for the up-front existence check in [`Intent::object_ids`].
    pub fn object_id(self) -> Option<ObjectId> {
        match self {
            Target::Object(id) => Some(id),
            Target::Player(_) => None,
        }
    }
}

/// Which spells a static cost-reducer ([`Effect::Static(StaticEffect::ReduceSpellCost)`]) applies to — the "spells
/// you cast" clause of a "…cost {N} less" ability. Matched against the card being cast.
/// ponytail: the shapes the pool needs; color/tribe filters ("black creature spells", "Goblin
/// spells") grow from a real card that wants one (they'd need a color/subtype read).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "card-dsl", derive(serde::Deserialize))]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum SpellFilter {
    /// Every spell you cast.
    #[default]
    #[cfg_attr(feature = "card-dsl", serde(rename = "all"))]
    AllSpells,
    /// Creature spells you cast.
    #[cfg_attr(feature = "card-dsl", serde(rename = "creature"))]
    CreatureSpells,
    /// Noncreature spells you cast.
    #[cfg_attr(feature = "card-dsl", serde(rename = "noncreature"))]
    NoncreatureSpells,
    /// Spells you cast that target a creature (Killian, Ink Duelist). Matched on the spell's
    /// chosen target being a creature on the battlefield — so an "any target" spell counts only
    /// when it's actually aimed at a creature, matching how the ability reads at cast time.
    #[cfg_attr(feature = "card-dsl", serde(rename = "targets_a_creature"))]
    SpellsThatTargetACreature,
    /// Aura spells you cast (Transcendent Envoy, CR 303.4). An Aura is its own [`CardKind`], so
    /// this is a direct kind check — no subtype axis needed.
    #[cfg_attr(feature = "card-dsl", serde(rename = "aura"))]
    Aura,
    /// Instant and sorcery spells you cast (Stormcatch Mentor).
    #[cfg_attr(feature = "card-dsl", serde(rename = "instant_or_sorcery"))]
    InstantOrSorcery,
    /// Enchantment spells you cast (Starfield Mystic). A type-bit check via [`CardKind::types`],
    /// so an Aura spell matches too (CR 303.4a: an Aura *is* an enchantment) — the pool's white
    /// Auras get Starfield Mystic's discount.
    #[cfg_attr(feature = "card-dsl", serde(rename = "enchantment"))]
    Enchantment,
    /// Artifact or enchantment spells you cast (Quandrix Command's hard counter mode — CR 303/300:
    /// Auras count as enchantments here too, via [`CardKind::types`]).
    #[cfg_attr(feature = "card-dsl", serde(rename = "artifact_or_enchantment"))]
    ArtifactOrEnchantment,
    /// Spells whose card carries any of these printed subtypes (Sram, Senior Edificer's "an Aura,
    /// Equipment, or Vehicle spell" — `["Aura", "Equipment", "Vehicle"]`; an Aura's own subtype
    /// list always includes "Aura", so no separate [`Aura`](Self::Aura) union is needed).
    #[cfg_attr(feature = "card-dsl", serde(rename = "has_subtype"))]
    HasSubtype(
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_str_slice"))]
        &'static [&'static str],
    ),
    /// Spells whose printed cost contains `{X}` (Nev, the Practical Dean / Zimone, Infinite
    /// Analyst / Elementalist's Palette's "a spell with {X} in its mana cost"). Reuses
    /// [`Cost::x`]'s own "this cost contains {X}" predicate (CR 107.3).
    #[cfg_attr(feature = "card-dsl", serde(rename = "has_x"))]
    HasXInCost,
    /// Instant or sorcery spells you cast with `{X}` in their mana cost (Unbound Flourishing's
    /// copy ability: "an instant or sorcery spell … [with] a mana cost that contains {X}") — the
    /// [`InstantOrSorcery`](Self::InstantOrSorcery) and [`HasXInCost`](Self::HasXInCost) checks
    /// combined. No general And-combinator exists yet (see #90); add one and fold this arm into
    /// it when a second card needs a different pair.
    #[cfg_attr(feature = "card-dsl", serde(rename = "instant_or_sorcery_with_x"))]
    InstantOrSorceryWithXInCost,
    /// Historic spells you cast (Teshar, Ancestor's Apostle) — CR 702.135a: an artifact,
    /// legendary, or Saga card is historic.
    #[cfg_attr(feature = "card-dsl", serde(rename = "historic"))]
    Historic,
    /// An Aura spell you cast that targets a modified permanent you control (Pearl-Ear,
    /// Imperial Advisor — CR 701.29 / "Equipment, Auras you control, and counters are
    /// modifications"). Checks the spell's own kind, its chosen target's [`Game::is_modified`],
    /// and that the target's controller is the caster.
    #[cfg_attr(
        feature = "card-dsl",
        serde(rename = "aura_targets_modified_permanent_you_control")
    )]
    AuraTargetsModifiedPermanentYouControl,
    /// Spells you cast from anywhere other than your hand (Advanced Reconstruction's level 3 —
    /// "Spells you cast from anywhere other than your hand cost {2} less to cast"). Matched on
    /// the casting spell's source zone being anything but [`Zone::Hand`] — a flashback/escape
    /// from a graveyard, an impulse-play from exile, a command-zone commander cast (CR 601). The
    /// only [`SpellFilter`] arm that reads the cast-from zone threaded into
    /// [`Game::spell_matches_filter`].
    #[cfg_attr(feature = "card-dsl", serde(rename = "cast_from_non_hand_zone"))]
    CastFromNonHandZone,
    /// Spells you cast of a given color (Balefire Liege — "cast a red spell" / "cast a white
    /// spell"). Reads the spell's own colors (CR 105.1/202.2, [`color_identity`]), so a
    /// multicolored spell matches any of its colors.
    #[cfg_attr(feature = "card-dsl", serde(rename = "color"))]
    Color(Color),
    /// A spell whose mana value equals the `{X}` paid for the spell doing the filtering (Spell
    /// Blast's "Counter target spell with mana value X"). The only X-dependent arm: it is matched
    /// inline in [`Game::legal_targets_for`](crate::Game)'s `SpellOnStack` enumeration — the one
    /// place that knows the filtering spell's X, and the same place the CR 608.2b resolution
    /// re-check reads — so `Game::spell_matches_filter`, which serves trigger and cost-reducer
    /// call sites with no X of their own, never matches it. The spell-side twin of
    /// [`PermanentFilter::mv_eq_x`].
    #[cfg_attr(feature = "card-dsl", serde(rename = "mana_value_equals_x"))]
    ManaValueEqualsX,
}

/// Which library cards a [`Effect::Dig(DigEffect::SearchLibrary)`] may find (CR 701.19 — "search for a card").
/// ponytail: the shapes the pool needs; a color filter ("a black creature card") grows from a
/// real card that wants one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum CardFilter {
    /// A basic land card (fetchlands / basic-land ramp). See [`is_basic_land`].
    BasicLand,
    /// Any land card.
    Land,
    /// A nonland card — the inverse of [`Land`](Self::Land): a creature, artifact, enchantment,
    /// planeswalker, instant, or sorcery card (Creative Technique's "reveal cards from the top
    /// of it until you reveal a nonland card").
    Nonland,
    /// A creature card (a creature tutor).
    Creature,
    /// A card of any kind (Diabolic Tutor).
    AnyCard,
    /// A land card whose type line carries any of these subtypes (Nature's Lore: "a Forest
    /// card" — matches a basic Forest *and* a nonbasic Forest-typed dual like Tangled Islet).
    LandWithSubtype(
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_str_slice"))]
        &'static [&'static str],
    ),
    /// A *basic* land card whose type line carries any of these subtypes (Archaeomancer's Map:
    /// "a basic Plains card" — the Basic supertype, CR 205.4a, excludes a nonbasic Plains-typed
    /// dual like Eclipsed Steppe even though it shares the subtype). [`LandWithSubtype`] minus
    /// the nonbasic case; see [`is_basic_land`] for why the gate reads `CardKind::Land::basic`
    /// rather than the subtype list.
    BasicLandWithSubtype(
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_str_slice"))]
        &'static [&'static str],
    ),
    /// A permanent card (creature/artifact/enchantment/planeswalker/land — anything with a
    /// nonempty [`CardKind::types`]) with mana value at most `N` (Sevinne's Reclamation:
    /// "target permanent card with mana value 3 or less").
    PermanentWithManaValueAtMost(u8),
    /// A nonland permanent card (artifact/creature/enchantment/planeswalker, never a land) with
    /// mana value at most `N` (Sun Titan / Primary Research: "target nonland permanent card with
    /// mana value 3 or less"). [`PermanentWithManaValueAtMost`](Self::PermanentWithManaValueAtMost)
    /// minus the land case.
    NonlandPermanentWithManaValueAtMost(u8),
    /// An artifact or creature card with mana value at most `N` (Lorehold Charm mode 2: "target
    /// artifact or creature card with mana value 2 or less").
    ArtifactOrCreatureWithManaValueAtMost(u8),
    /// A creature card with mana value at most `N` (Teshar, Ancestor's Apostle: "target creature
    /// card with mana value 3 or less").
    CreatureWithManaValueAtMost(u8),
    /// A creature card with mana value at least `N` (Fierce Empath: "search your library for a
    /// creature card with mana value 6 or greater"). The inclusive-lower-bound twin of
    /// [`CreatureWithManaValueAtMost`](Self::CreatureWithManaValueAtMost).
    CreatureWithManaValueAtLeast(u8),
    /// An artifact, creature, or non-Aura enchantment card with mana value at most `N` (Excava,
    /// the Risen Past: "target artifact, creature, or non-Aura enchantment card with mana value 3
    /// or less"). Reads [`CardKind`] directly rather than [`CardKind::types`] so an Aura — which
    /// carries the same enchantment type bit as a plain enchantment — is excluded like the
    /// printed restriction excludes it.
    ArtifactCreatureOrNonAuraEnchantmentWithManaValueAtMost(u8),
    /// An instant or sorcery card (Mystic Sanctuary: "target instant or sorcery card from your
    /// graveyard").
    InstantOrSorcery,
    /// A sorcery card, no instants (Anarchist: "target sorcery card from your graveyard").
    /// [`InstantOrSorcery`](Self::InstantOrSorcery) minus the instant half.
    Sorcery,
    /// A sorcery card of a given color (Nucklavee: "target red sorcery card from your
    /// graveyard"). [`Sorcery`](Self::Sorcery) plus a CR 105.2a color check, read off the card's
    /// own characteristics via [`color_identity`] — a hybrid pip counts as both its colors.
    SorceryWithColor(Color),
    /// An instant card of a given color (Nucklavee: "target blue instant card from your
    /// graveyard"). The instant-half twin of [`SorceryWithColor`](Self::SorceryWithColor).
    InstantWithColor(Color),
    /// An enchantment card, no mana-value bound (Replenish: "return all enchantment cards from
    /// your graveyard to the battlefield" — Eiganjo Dynastorian's back face). Counts an Aura, like
    /// [`CardKind::types`] does.
    Enchantment,
    /// A permanent card (creature/artifact/enchantment/planeswalker/land), no mana-value bound
    /// (Deadly Brew: "return another permanent card from your graveyard to your hand"). The
    /// unbounded twin of [`PermanentWithManaValueAtMost`](Self::PermanentWithManaValueAtMost).
    Permanent,
    /// A permanent card whose type line carries any of these subtypes, no card-type restriction
    /// (Bladewing the Risen: "target Dragon permanent card" — a Dragon creature card qualifies).
    /// [`Permanent`](Self::Permanent) plus a subtype gate; reads the printed subtype line
    /// ([`CardDef::subtypes`]) directly, the same check [`Aura`](Self::Aura) uses.
    PermanentWithSubtype(
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_str_slice"))]
        &'static [&'static str],
    ),
    /// A card that is neither a creature nor a land (Quintorius, Loremaster's "target
    /// noncreature, nonland card") — an instant, sorcery, noncreature artifact, enchantment
    /// (Aura included), or planeswalker.
    NoncreatureNonland,
    /// A creature card with mana value at most the combat damage a `DealsCombatDamageToPlayer`
    /// watcher's source just dealt to a player (Venerable Warsinger: "target creature card with
    /// mana value X or less … where X is the amount of damage this creature dealt to that
    /// player"). ponytail: placeholder only — `matches` never runs live; `fill_combat_damage`
    /// rewrites this to a resolved [`CreatureWithManaValueAtMost`](Self::CreatureWithManaValueAtMost)
    /// at trigger placement (CR 603.10a last-known information), same posture as `Amount::X`
    /// reading 0 outside a cast.
    CreatureWithManaValueAtMostCombatDamage,
    /// A nonland permanent card with mana value at most the source permanent's power (Guardian
    /// Scalelord: "return target nonland permanent card with mana value X or less … where X is
    /// this creature's power"). ponytail: placeholder only — `matches` never runs live;
    /// `fill_source_power` rewrites this to a resolved
    /// [`NonlandPermanentWithManaValueAtMost`](Self::NonlandPermanentWithManaValueAtMost) at
    /// trigger placement (CR 510.2/603.10a last-known information), same posture as
    /// [`CreatureWithManaValueAtMostCombatDamage`](Self::CreatureWithManaValueAtMostCombatDamage).
    NonlandPermanentWithManaValueAtMostSourcePower,
    /// An Aura or Equipment card (Armored Skyhunter's "an Aura or Equipment card from among
    /// them"). Reads the printed subtype line ([`CardDef::subtypes`]) directly, the same check
    /// [`Game::is_modified`]'s Equipment test and [`TargetSpec::HasSubtype`] use — an Aura
    /// card's own subtype list always includes "Aura", so no [`CardKind::Aura`] union is needed.
    AuraOrEquipment,
    /// An Aura card, no Equipment (Herald of Amity's "cast an Aura spell from among them").
    /// [`AuraOrEquipment`](Self::AuraOrEquipment) minus the Equipment half — same subtype-line
    /// read.
    Aura,
    /// An artifact or creature card, no mana-value bound (Restore Relic: "target artifact or
    /// creature card from your graveyard"). The unbounded twin of
    /// [`ArtifactOrCreatureWithManaValueAtMost`](Self::ArtifactOrCreatureWithManaValueAtMost).
    ArtifactOrCreature,
    /// An artifact or enchantment card, no mana-value bound (Enlightened Tutor: "Search your
    /// library for an artifact or enchantment card"). Reads [`CardKind::types`] rather than a raw
    /// [`CardKind`] match, so an Aura counts (it's still an enchantment card, CR 205.4a) the same
    /// way [`Enchantment`](Self::Enchantment) does.
    ArtifactOrEnchantment,
    /// A snow land card (Into the North: "Search your library for a snow land card") — any land
    /// with [`CardDef::snow`], basic or not. Distinct from [`Land`](Self::Land) (no snow gate) and
    /// from a snow *creature* (Ohran Frostfang fails this).
    SnowLand,
}

impl CardFilter {
    /// Whether a card with this definition matches the filter.
    pub fn matches(self, def: &CardDef) -> bool {
        match self {
            CardFilter::AnyCard => true,
            CardFilter::Land => matches!(def.kind, CardKind::Land { .. }),
            CardFilter::Nonland => !matches!(def.kind, CardKind::Land { .. }),
            CardFilter::Creature => matches!(def.kind, CardKind::Creature { .. }),
            CardFilter::BasicLand => is_basic_land(def),
            CardFilter::LandWithSubtype(subtypes) => match def.kind {
                CardKind::Land {
                    subtypes: land_subtypes,
                    ..
                } => land_subtypes.iter().copied().any(|s| subtypes.contains(&s)),
                _ => false,
            },
            CardFilter::BasicLandWithSubtype(subtypes) => match def.kind {
                CardKind::Land {
                    subtypes: land_subtypes,
                    ..
                } => {
                    is_basic_land(def)
                        && land_subtypes.iter().copied().any(|s| subtypes.contains(&s))
                }
                _ => false,
            },
            CardFilter::PermanentWithManaValueAtMost(max) => {
                !def.kind.types().is_empty() && def.mana_value() <= max as u32
            }
            CardFilter::NonlandPermanentWithManaValueAtMost(max) => {
                def.kind.types().intersects(TypeSet::NONLAND) && def.mana_value() <= max as u32
            }
            CardFilter::ArtifactOrCreatureWithManaValueAtMost(max) => {
                def.kind
                    .types()
                    .intersects(TypeSet::ARTIFACT.union(TypeSet::CREATURE))
                    && def.mana_value() <= max as u32
            }
            CardFilter::CreatureWithManaValueAtMost(max) => {
                matches!(def.kind, CardKind::Creature { .. }) && def.mana_value() <= max as u32
            }
            CardFilter::CreatureWithManaValueAtLeast(min) => {
                matches!(def.kind, CardKind::Creature { .. }) && def.mana_value() >= min as u32
            }
            CardFilter::ArtifactCreatureOrNonAuraEnchantmentWithManaValueAtMost(max) => {
                matches!(
                    def.kind,
                    CardKind::Artifact | CardKind::Creature { .. } | CardKind::Enchantment
                ) && def.mana_value() <= max as u32
            }
            CardFilter::InstantOrSorcery => matches!(def.kind, CardKind::Spell { .. }),
            CardFilter::Sorcery => {
                matches!(
                    def.kind,
                    CardKind::Spell {
                        speed: SpellSpeed::Sorcery
                    }
                )
            }
            CardFilter::SorceryWithColor(color) => {
                matches!(
                    def.kind,
                    CardKind::Spell {
                        speed: SpellSpeed::Sorcery
                    }
                ) && color_identity(def)[color.index()]
            }
            CardFilter::InstantWithColor(color) => {
                matches!(
                    def.kind,
                    CardKind::Spell {
                        speed: SpellSpeed::Instant
                    }
                ) && color_identity(def)[color.index()]
            }
            CardFilter::Enchantment => def.kind.types().intersects(TypeSet::ENCHANTMENT),
            CardFilter::Permanent => !def.kind.types().is_empty(),
            CardFilter::PermanentWithSubtype(subtypes) => {
                !def.kind.types().is_empty() && def.subtypes.iter().any(|s| subtypes.contains(s))
            }
            CardFilter::NoncreatureNonland => {
                !matches!(def.kind, CardKind::Creature { .. } | CardKind::Land { .. })
            }
            // ponytail: placeholder, rewritten to `CreatureWithManaValueAtMost` by
            // `fill_combat_damage` before any legality check reads it — see the variant doc.
            CardFilter::CreatureWithManaValueAtMostCombatDamage => false,
            // ponytail: placeholder, rewritten to `NonlandPermanentWithManaValueAtMost` by
            // `fill_source_power` before any legality check reads it — see the variant doc.
            CardFilter::NonlandPermanentWithManaValueAtMostSourcePower => false,
            CardFilter::AuraOrEquipment => {
                def.subtypes.contains(&"Aura") || def.subtypes.contains(&"Equipment")
            }
            CardFilter::Aura => def.subtypes.contains(&"Aura"),
            CardFilter::ArtifactOrCreature => {
                matches!(def.kind, CardKind::Artifact | CardKind::Creature { .. })
            }
            CardFilter::ArtifactOrEnchantment => def
                .kind
                .types()
                .intersects(TypeSet::ARTIFACT.union(TypeSet::ENCHANTMENT)),
            CardFilter::SnowLand => matches!(def.kind, CardKind::Land { .. }) && def.snow,
        }
    }
}

/// Where a found card goes at the end of a [`Effect::Dig(DigEffect::SearchLibrary)`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum SearchDest {
    /// Into the searcher's hand (tutors like Diabolic Tutor).
    Hand,
    /// Onto the battlefield under the searcher's control (ramp / fetchlands), tapped per the
    /// effect's `tapped` flag.
    Battlefield,
    /// Onto the top of the searcher's own library, revealed as it's found (Enlightened Tutor,
    /// Sterling Grove: "reveal it, then shuffle and put that card on top" — CR 701.19). A
    /// same-zone reorder, not a zone change (CR 400.7) — the card never leaves the library, so it
    /// keeps its object id, the same way [`Event::PutOnBottomOfLibrary`] does for the bottom.
    LibraryTop,
    /// Into the searcher's own graveyard (Buried Alive: "put them into your graveyard" — CR
    /// 701.19), routed through the same [`Event::Milled`] library-to-graveyard choke mill effects
    /// use — the arrival is never "put into a graveyard from the battlefield" (CR 700.4), so it
    /// can't fire Dies.
    Graveyard,
    /// Into exile (Trench Gorger: "exile them"), routed through the same [`Event::MovedToExile`]
    /// choke a graveyard-to-exile move uses.
    Exile,
}

/// Where a card selected by [`Effect::Dig(DigEffect::LookAtTop)`] goes (the "put that card into …" destination).
/// `Battlefield`'s `tapped` gate lives as a sibling flag on [`Effect::Dig(DigEffect::LookAtTop)::dest_tapped`]
/// (mirroring [`Effect::Reveal(RevealEffect::Until)`]'s `matched_dest`/`matched_tapped` split) rather than a
/// struct-variant field, so the TOML tag stays a bare `dest = "battlefield"` string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum TopDest {
    /// Into the selecting player's hand (Quandrix Apprentice).
    Hand,
    /// Onto the battlefield under the selecting player's control (Armored Skyhunter's "put an
    /// Aura or Equipment card from among them onto the battlefield"), routed through
    /// [`Event::SearchedToBattlefield`] — the same event [`Effect::Dig(DigEffect::SearchLibrary)`] /
    /// [`Effect::Reveal(RevealEffect::Until)`] use.
    Battlefield,
}

/// Where the *non-matching* revealed/looked-at cards go, shared by [`Effect::Dig(DigEffect::LookAtTop)`],
/// [`Effect::Reveal(RevealEffect::Until)`], and [`Effect::Reveal(RevealEffect::TopCards)`].
/// ponytail: a `Graveyard` arm (a look-then-select whose rest is milled) is the next unlock;
/// add it (routing through [`Event::Milled`], the surveil path) from the first card that needs
/// it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum RestDest {
    /// On the bottom of the selecting player's library (the common case).
    #[default]
    Bottom,
    /// Into the selecting player's hand (Coiling Oracle's "Otherwise, put that card into your
    /// hand").
    Hand,
}

/// Which controller a [`PermanentFilter`] accepts, relative to the effect's controller ("you").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum FilterController {
    /// Any controller (default).
    #[default]
    Any,
    /// A permanent you control.
    You,
    /// A permanent an opponent controls.
    Opponent,
    /// A permanent the *active player* controls (Siren's Call's "creatures the active player
    /// controls" and the "that player" its end-step sweep refers back to). Deliberately not
    /// [`FilterController::Opponent`]: at a four-player table that would reach three opponents'
    /// boards, and every card printing this clause names exactly one of them. Reads whoever is
    /// active right now, so a delayed sweep scheduled this turn still names the same player when
    /// it fires at that turn's end step.
    #[cfg_attr(feature = "card-dsl", serde(rename = "active_player"))]
    ActivePlayer,
    /// A permanent the player *this filter's own source is attacking* controls (Gaea's Liege's
    /// "the number of Forests defending player controls"). Read off
    /// [`Game::defender_of`](crate::Game) rather than off
    /// [`FilterController::Opponent`], which at a four-player table would reach three boards when
    /// the card names exactly one. Matches nothing while the source isn't attacking — which is the
    /// right answer, since every card printing the clause has a second arm for that case.
    #[cfg_attr(feature = "card-dsl", serde(rename = "defending_player"))]
    DefendingPlayer,
}

/// Whether a [`PermanentFilter`] accepts tokens, nontokens, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum TokenFilter {
    /// A token or a nontoken (default).
    #[default]
    Any,
    /// A token only.
    Token,
    /// A nontoken permanent only (Skyclave Apparition, Lorehold Charm's artifact edict).
    Nontoken,
}

/// Power parity gate for a [`PermanentFilter`] (Zimone's Hypothesis's "return each creature with
/// power of the chosen quality" — CR: zero counts as even).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum Parity {
    Even,
    Odd,
}

/// A permanent's color restriction for a [`PermanentFilter`] (CR 105.2).
/// ponytail: only "exactly one color", "is this one specific color", and "is NOT this one
/// specific color" have pool consumers (Vanishing Verse's "monocolored permanent"; Oran-Rief's
/// "green creature"; Terror's "nonblack creature"); add `Multicolored` when a real card needs
/// it (stonecoil's "multicolored") rather than pre-building it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum ColorFilter {
    #[default]
    Any,
    /// Exactly one color (CR 105.2a — colorless has zero and doesn't qualify).
    Monocolored,
    /// Is white (CR 105.2a).
    White,
    /// Is blue (CR 105.2a).
    Blue,
    /// Is black (CR 105.2a).
    Black,
    /// Is red (CR 105.2a).
    Red,
    /// Is green (CR 105.2a) — Oran-Rief, the Vastwood's "each green creature".
    Green,
    /// Does NOT have the named color (CR 105.2a's negation — Terror/Shriekmaw's "nonblack
    /// creature"). Read off `Game::colors_of`; a colorless permanent has no colors, so it always
    /// satisfies a `NotColor` gate.
    NotColor(Color),
}

impl ColorFilter {
    /// Whether an object with these colors (CR 105.2a — the `[bool; Color::COUNT]`
    /// [`Game::colors_of`](crate::Game::colors_of) returns) satisfies this filter. Pure, so a
    /// caller that already holds the colors can ask without re-reading the game.
    pub fn matched_by(self, colors: &[bool; Color::COUNT]) -> bool {
        let named = match self {
            ColorFilter::White => Color::White,
            ColorFilter::Blue => Color::Blue,
            ColorFilter::Black => Color::Black,
            ColorFilter::Red => Color::Red,
            ColorFilter::Green => Color::Green,
            // A colorless object has zero trues, so it fails "exactly one".
            ColorFilter::Monocolored => return colors.iter().filter(|&&c| c).count() == 1,
            // "Nonblack" (Terror): a colorless object has no colors, so it always passes.
            ColorFilter::NotColor(color) => return !colors[color.index()],
            ColorFilter::Any => return true,
        };
        colors[named.index()]
    }

    /// The filter matching exactly `color` — the constructor side of the five color-named
    /// variants, for a caller holding a [`Color`] rather than authored TOML (a CR 612.1 text
    /// change rewriting the color an ability names).
    pub fn of(color: Color) -> Self {
        match color {
            Color::White => ColorFilter::White,
            Color::Blue => ColorFilter::Blue,
            Color::Black => ColorFilter::Black,
            Color::Red => ColorFilter::Red,
            Color::Green => ColorFilter::Green,
        }
    }
}

/// A composable predicate over a battlefield permanent — the one filter behind targeted
/// removal ([`TargetSpec::Permanent`]), mass effects ([`Effect::Destroy(DestroyEffect::DestroyAll)`] /
/// [`Effect::Zone(ZoneEffect::ReturnAllToHand)`]), and sacrifice edicts ([`Effect::Choice(ChoiceEffect::EachPlayerSacrifices)`]).
/// Every axis is independent; an unset axis imposes no restriction. Evaluated by
/// [`Game::permanent_matches`], which reads the axes needing game state. Kept `Copy` because it is
/// a compact authored predicate value.
///
/// In TOML it's a `{ … }` table, or a bare-string shorthand for the common shapes —
/// `"creatures"`, `"nonland"`, `"artifact"`, `"creature_or_planeswalker"` (see the `de` module).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PermanentFilter {
    /// Required card types (empty = any type). A permanent matches if it has *any* of these.
    pub types: TypeSet,
    /// Restrict to permanents carrying any of these subtypes (Goldspan Dragon's "Treasures you
    /// control" — `["Treasure"]`, distinguishing a Treasure from any other artifact); empty
    /// matches every subtype. Same shape/rationale as [`Effect::Static(StaticEffect::Anthem)`]'s own `subtypes`
    /// field (a separate axis there since an anthem always targets creatures specifically).
    /// Matches against [`Game::effective_subtypes`], which reports a land's own type line too
    /// (CR 305.6), so `{ types = "land", subtypes = ["Plains"] }` is Flashfires' "Destroy all
    /// Plains" — the basic and every dual sharing the type.
    /// Deserialized by hand alongside the rest of [`PermanentFilter`]'s table form (see `de.rs`)
    /// rather than a derive attribute — `PermanentFilter` has a hand-written `Deserialize` impl
    /// for its bare-string shorthand, so there's no derive for a field attribute to hang off.
    pub subtypes: &'static [&'static str],
    /// Whose permanents qualify (default any).
    pub controller: FilterController,
    /// Token-ness restriction (default any).
    pub token: TokenFilter,
    /// "another permanent" — excludes the filter's own source (CR: "each other"). Needs a
    /// source to exclude; without one it restricts nothing.
    pub other: bool,
    /// `Some(true)` requires an Aura attached ("enchanted"); `Some(false)` requires none
    /// (Winds of Rath's "creatures that aren't enchanted"); `None` doesn't care.
    pub enchanted: Option<bool>,
    /// `Some(true)` requires the candidate (an Aura) be attached to a permanent that's a
    /// creature (CR 303 — Sage's Reverie's "each Aura you control that's attached to a
    /// creature", distinguishing it from an Aura on the stack/in a graveyard or, theoretically,
    /// attached to a noncreature permanent); `Some(false)` requires the opposite; `None` doesn't
    /// care. The mirror image of `enchanted` (which reads the *host* side).
    pub attached_to_creature: Option<bool>,
    /// Requires an attached Aura controlled by "you" (Eriette of the Charmed Apple's "enchanted
    /// by an Aura you control") — narrower than `enchanted`, which matches any attached Aura.
    /// `false` (default) imposes no restriction. Read against `you` in [`Game::permanent_matches`].
    pub enchanted_by_you: bool,
    /// Mana-value ceiling (Skyclave's "MV 4 or less", Culling Ritual's "MV 2 or less"); `None`
    /// doesn't gate on mana value.
    pub mv_max: Option<u8>,
    /// Mana-value floor (Austere Command's "mana value 4 or greater", the sibling of `mv_max`);
    /// `None` doesn't gate on mana value.
    pub mv_min: Option<u8>,
    /// Mana value exactly equal to the casting spell's chosen `{X}` (Entrancing Melody's
    /// "creature with mana value X"). `false` (default) doesn't gate on it. Resolved against
    /// [`Game::legal_targets_for`]'s own `source` — see that method's doc.
    pub mv_eq_x: bool,
    /// Mana value at most the value a permanent *entered* the battlefield with (Kinetic Ooze's
    /// "artifact or enchantment with mana value X or less", where X is its own entered `{X}`).
    /// `false` (default) doesn't gate on it. Unlike `mv_eq_x` (a cast's chosen X), this reads a
    /// triggered ability's own source permanent's live `+1/+1` counter count as the entered-X
    /// proxy — see [`Game::place_targeted_ability`].
    pub mv_max_x: bool,
    /// `Some(true)` requires the permanent be tapped (Mana Geyser's "tapped land"); `Some(false)`
    /// requires untapped; `None` doesn't care. Ignored in the graveyard zone (cards aren't tapped).
    pub tapped: Option<bool>,
    /// Requires the permanent make mana when tapped (Power Sink's "lands with mana abilities they
    /// control" — which spares a Mishra's Factory or a Maze of Ith). Read live off
    /// [`Game::taps_for_mana`], the same predicate the client's tap-for-mana affordance uses, so a
    /// land whose type line was changed under it is judged by what it produces *now*. `false`
    /// (default) imposes no restriction.
    pub has_mana_ability: bool,
    /// Power ceiling (Silverquill Charm's "creature with power 2 or less"); `None` doesn't gate
    /// on power. Non-creatures have power 0 (see [`Game::power`]), so they always pass a power
    /// gate — no pool card combines `power_max` with a non-creature `types` set.
    pub power_max: Option<u8>,
    /// Power floor (Meekstone's "creatures with power 3 or greater"); `None` doesn't gate on
    /// power. The mirror of [`power_max`](Self::power_max) and read live like it.
    pub power_min: Option<u8>,
    /// Power parity gate (Zimone's Hypothesis's "return each creature with power of the chosen
    /// quality"); `None` doesn't gate on parity.
    pub power_parity: Option<Parity>,
    /// Excluded card types (CR: "noncreature artifact"/"noncreature enchantment" — Haywire
    /// Mite; "nonartifact creature" — Terror/Shriekmaw/Ashes to Ashes). Empty (default) imposes
    /// no restriction; a permanent with *any* type in this set fails, so an Artifact Creature
    /// (a creature with `also = "artifact"`) fails an `exclude` containing artifact even though
    /// it also carries the creature type. `noncreature = true` in TOML is sugar that unions
    /// `TypeSet::CREATURE` into this same field (see `de.rs`).
    pub exclude: TypeSet,
    /// Color-count restriction (Vanishing Verse's "monocolored permanent"); `Any` (default)
    /// doesn't gate on color. Reads [`Game::colors_of`] — color identity derived from cost pips,
    /// overridden live by a runtime color-change layer (an until-end-of-turn SET or ADD) where
    /// one is active.
    pub color: ColorFilter,
    /// "Modified" (CR 701.29 / Silkguard's reminder text "Equipment, Auras you control, and
    /// counters are modifications") — has any counter, is enchanted by an Aura, or is equipped.
    /// `false` (default) imposes no restriction. See [`Game::is_modified`].
    pub modified: bool,
    /// Restrict to creatures declared as attackers this combat (Tajic's Mentor — "target
    /// *attacking* creature"). `false` (default) imposes no restriction.
    pub attacking: bool,
    /// Restrict to creatures attacking *this filter's own controller* (Soul Snare's "creature
    /// that's attacking you") — narrower than `attacking`, which matches an attacker no matter
    /// who its declared defender is. Reads [`Game::defender_of`], the same declared-defender
    /// lookup goad/attack-tax already use. `false` (default) imposes no restriction.
    pub attacking_you: bool,
    /// Restrict to creatures currently blocking some attacker (Righteousness — "target
    /// *blocking* creature"). `false` (default) imposes no restriction. Reads
    /// [`CombatState::blocks`], the same declared-blocks list `anthem_static`'s own
    /// `blocking_only` axis consults.
    pub blocking: bool,
    /// Restrict to creatures that are either attacking or blocking (Tor Wauki's "target
    /// *attacking or blocking* creature" — a Legends idiom, printed on four of that set's archers).
    /// The union of [`attacking`](Self::attacking) and [`blocking`](Self::blocking), which as two
    /// separate flags would instead intersect: setting both would demand one creature do both at
    /// once, which no creature ever does. `false` (default) imposes no restriction.
    pub attacking_or_blocking: bool,
    /// Restrict to creatures that are either tapped or blocking (Tetsuo Umezawa's "target *tapped
    /// or blocking* creature"). The twin of [`attacking_or_blocking`](Self::attacking_or_blocking):
    /// `tapped = Some(true)` and [`blocking`](Self::blocking) as two separate axes would instead
    /// intersect, demanding a creature be both at once, and blocking never taps (CR 509.1).
    /// `false` (default) imposes no restriction.
    pub tapped_or_blocking: bool,
    /// Restrict to attacking creatures no creature is blocking (Forcefield — "an *unblocked*
    /// creature of your choice"). The complement of [`blocking`](Self::blocking) one step over:
    /// that one asks whether this creature blocks something, this one whether anything blocks it.
    /// `false` (default) imposes no restriction. Pair it with [`attacking`](Self::attacking) or
    /// [`attacking_you`](Self::attacking_you) — on its own it also matches every creature sitting
    /// at home, which no attacker is blocking either.
    pub unblocked: bool,
    /// Power strictly less than the filter's own source permanent's power (Mentor, CR 702.121a
    /// "lesser power"). `false` (default) imposes no restriction. Meaningless without a `source`
    /// (see [`Game::permanent_matches`]) — every filter that sets this pairs it with a targeted
    /// ability, which always threads its source.
    pub power_less_than_source: bool,
    /// Toughness strictly less than the filter's own source permanent's *power* (Stone Giant —
    /// "target creature you control with toughness less than this creature's power": what the
    /// Giant can throw is what it can lift). `false` (default) imposes no restriction. Like
    /// `power_less_than_source` just above, meaningless without a `source` (see
    /// [`Game::permanent_matches`]) — every filter that sets it pairs it with a targeted ability,
    /// which always threads its source.
    pub toughness_less_than_source_power: bool,
    /// Requires the permanent entered the battlefield this turn (CR "entered the battlefield
    /// this turn" — Oran-Rief, the Vastwood's "each green creature that entered this turn").
    /// `false` (default) imposes no restriction. Distinct from checking `summoning_sick`, which
    /// clears one step earlier (see [`Permanent::entered_this_turn`]'s doc).
    pub entered_this_turn: bool,
    /// Requires that the permanent's controller has controlled it continuously since this turn
    /// began — CR 302.6's wording, which Nettling Imp's "the active player has controlled
    /// continuously since the beginning of the turn" repeats. `false` (default) imposes no
    /// restriction. Reads [`Permanent::summoning_sick`], which *is* that flag: set on entry and on
    /// a control change, cleared at the controller's own untap step. It therefore only says what
    /// it means about a permanent whose controller has already untapped this turn — the active
    /// player's — and every pool card spelling this clause is restricted to the active player's
    /// turn, so that is always whose permanents it reads.
    pub controlled_since_turn_start: bool,
    /// "…that didn't attack this turn" (CR 508.1 — Siren's Call's end-step sweep). `false`
    /// (default) imposes no restriction. Reads [`Permanent::attacked_this_turn`], which the untap
    /// step clears, so this only means anything within the turn the attack would have happened in.
    /// [`DestroyEffect::Target`](crate::DestroyEffect)'s `attack_rider` asks the same question of
    /// one already-chosen creature; this asks it of a whole board scan, where the answer has to be
    /// re-read per permanent as the sweep fires.
    pub did_not_attack_this_turn: bool,
    /// Excludes basic lands (CR 205.4a's "Basic" supertype — White Orchid Phantom's "target
    /// *nonbasic* land"). `false` (default) imposes no restriction. Read against
    /// [`is_basic_land`] in [`Game::permanent_matches`]; meaningful only alongside a `types` set
    /// that includes land (a nonbasic-land filter is land AND not basic, not "any nonbasic
    /// permanent").
    pub nonbasic: bool,
    /// Requires a basic land (CR 205.4a — the tango-land cycle's "unless you control two or more
    /// basic lands"). `false` (default) imposes no restriction. The positive twin of
    /// [`nonbasic`](Self::nonbasic) and, like it, reads the def's supertype flag via
    /// [`is_basic_land`] rather than subtype strings, so a nonbasic dual sharing a basic's type
    /// line does not qualify. Implies land-ness on its own — every basic *is* a land — so a
    /// counting filter can set it without also naming `types`.
    pub basic: bool,
    /// Restrict to permanents with this exact printed name (Leitmotif Composer's "creatures
    /// *named* Leitmotif Composer can't be blocked this turn" — CR 201.2, matched against
    /// [`CardDef::name`]). `None` (default) doesn't gate on name.
    /// ponytail: printed-name equality only — no card in the pool changes a permanent's name
    /// (CR 707.9), so a copiable-name-vs-current-name distinction doesn't arise yet.
    pub name: Option<&'static str>,
    /// Excludes legendary permanents (CR 205.4a's "Legendary" supertype — Muddle, the
    /// Ever-Changing's "up to one target *nonlegendary* creature you control"). `false` (default)
    /// imposes no restriction. Reads the current (possibly copied) [`CardDef::legendary`].
    pub nonlegendary: bool,
    /// Requires a legendary permanent (CR 205.4a's "Legendary" supertype — Karakas' "target
    /// *legendary* creature", Arena of the Ancients' "legendary creatures", Willow Satyr's "target
    /// legendary creature"). `false` (default) imposes no restriction. The positive twin of
    /// [`nonlegendary`](Self::nonlegendary) and, like it, reads the current (possibly copied)
    /// [`CardDef::legendary`] rather than a printed type line, so a permanent copying a legend
    /// qualifies (CR 706.2).
    pub legendary: bool,
    /// Excludes the "Lair" land subtype (CR 305 — Treva's Ruins' "return a *non-Lair* land you
    /// control"). `false` (default) imposes no restriction. Reads the printed land-type list
    /// directly ([`CardKind::Land::subtypes`], the rules-relevant one — see that field's doc),
    /// not [`CardDef::subtypes`].
    /// ponytail: a single bool covers the pool's one "not this land subtype" need, same shape as
    /// `nonbasic`/`nonlegendary` above; generalize to a `subtypes_exclude` list if a second
    /// land-subtype exclusion turns up.
    pub nonlair: bool,
    /// Excludes creatures with flying (Breath of Darigaaz's "each creature *without flying*").
    /// `false` (default) imposes no restriction. Reads [`Game::has_keyword`].
    /// ponytail: a single bool covers the pool's one keyword-exclusion need, same shape as
    /// `nonbasic`/`nonlegendary`/`nonlair` above; generalize to a `without_keyword: Option<Keyword>`
    /// if a second keyword exclusion turns up.
    pub without_flying: bool,
    /// Requires creatures with flying (Firespout's "each creature *with flying*"). `false`
    /// (default) imposes no restriction. Reads [`Game::has_keyword`], the positive sibling of
    /// `without_flying` above (not a second exclusion, so it stays its own bool rather than
    /// folding into that field's ponytail note).
    pub with_flying: bool,
    /// A *second* keyword exclusion, alongside [`without_flying`](Self::without_flying) — Island
    /// Sanctuary's "can't be attacked except by creatures with flying **and/or islandwalk**"
    /// inverts to a banned set that has to lack both at once, which one bool can't say. `None`
    /// (default) imposes no restriction. Reads [`Game::has_keyword`].
    /// ponytail: two exclusion slots (one bool, one open) cover the pool; a third simultaneous
    /// keyword exclusion wants a small `ArrayVec<Keyword>` here, which
    /// [`PermanentFilter`](Self)'s `Copy` still allows.
    pub without_keyword: Option<Keyword>,
    /// Requires the permanent share at least one card type with the ability's own triggering
    /// dying permanent's last-known card types (CR 603.10a) — a *dynamic* type gate whose type
    /// set isn't known until the ability actually fires (Martyr's Bond's "shares a card type with
    /// it"), unlike the static `types` field above. `false` (default) imposes no restriction.
    /// Only meaningful behind a [`super::Trigger::NonlandPermanentYouControlDiesIncludingThis`]
    /// watch: `contextualize_effect` resolves it by overwriting `types` with
    /// `TriggerContext::dying_permanent_types` before [`Game::permanent_matches`] ever reads this
    /// filter, so [`Game::permanent_matches`] itself never consults this flag.
    pub shares_type_with_dying_permanent: bool,
    /// Counter axis (CR 122.1) narrower than `modified` above — `modified` also matches an
    /// equipped/enchanted permanent with no counter at all, so it can't express Inspiring Call's
    /// "creature you control with a +1/+1 counter on it" or Innkeeper's Talent's "permanents you
    /// control with counters on them" (any kind). `None` (default) doesn't gate on counters.
    pub with_counter: Option<CounterAxis>,
    /// "each permanent you control that's a creature or Vehicle" (Ao, the Dawn Sky mode 2) —
    /// matches a creature **or** a permanent carrying the Vehicle artifact subtype (CR 205.3g;
    /// Vehicle is not a card type). When `true`, the ordinary `types` axis is ignored for the
    /// match (this OR-gate replaces it). `false` (default) imposes no restriction.
    pub creature_or_vehicle: bool,
    /// Restrict to snow permanents (CR 205.4g — Into the North's "snow land" via
    /// [`CardFilter::SnowLand`], or a battlefield "snow permanent" scan). `false` (default)
    /// imposes no restriction. Reads [`CardDef::snow`].
    pub snow: bool,
    /// Excluded subtypes (Keldon Warlord's "non-Wall creatures you control") — a permanent
    /// carrying *any* of these fails. Empty (default) imposes no restriction. Matched against
    /// [`Game::effective_subtypes`], the same layered view the positive `subtypes` axis reads, so
    /// a creature turned into an Insect by Darksteel Mutation is excluded by `["Insect"]` and no
    /// longer by its printed subtype. Distinct from `nonlair` above, which deliberately reads a
    /// land's *printed* type line instead.
    pub exclude_subtypes: &'static [&'static str],
}

/// TOML `with_counter = "any"` / `with_counter = "plus_one_plus_one"` — the two counter shapes
/// the pool currently needs on a [`PermanentFilter`] (CR 122.1's unqualified "counter" vs the
/// +1/+1 kind specifically).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum CounterAxis {
    /// Any counter of any kind (Innkeeper's Talent's "with counters on them").
    Any,
    /// Specifically a +1/+1 counter (Inspiring Call's "with a +1/+1 counter on it").
    PlusOnePlusOne,
}

impl PermanentFilter {
    /// A filter matching every permanent of the given types (the common shape).
    pub const fn of(types: TypeSet) -> PermanentFilter {
        PermanentFilter {
            types,
            subtypes: &[],
            controller: FilterController::Any,
            token: TokenFilter::Any,
            other: false,
            enchanted: None,
            attached_to_creature: None,
            enchanted_by_you: false,
            mv_max: None,
            mv_min: None,
            mv_eq_x: false,
            mv_max_x: false,
            tapped: None,
            has_mana_ability: false,
            power_max: None,
            power_min: None,
            power_parity: None,
            exclude: TypeSet::NONE,
            color: ColorFilter::Any,
            modified: false,
            attacking: false,
            attacking_you: false,
            blocking: false,
            attacking_or_blocking: false,
            tapped_or_blocking: false,
            unblocked: false,
            power_less_than_source: false,
            toughness_less_than_source_power: false,
            entered_this_turn: false,
            controlled_since_turn_start: false,
            did_not_attack_this_turn: false,
            nonbasic: false,
            basic: false,
            name: None,
            nonlegendary: false,
            legendary: false,
            nonlair: false,
            without_flying: false,
            without_keyword: None,
            with_flying: false,
            shares_type_with_dying_permanent: false,
            with_counter: None,
            creature_or_vehicle: false,
            snow: false,
            exclude_subtypes: &[],
        }
    }
}

/// How many distinct targets an effect chooses (CR 601.2c): between `min` and `max`, inclusive.
/// The default `{1, 1}` is the ubiquitous single mandatory target, so every existing effect is
/// untouched. `count = N` in TOML is sugar for `{N, N}` (an exact "N target"); an explicit
/// `{ min, max }` spells "up to"/"one or two" ranges (see `de::TargetCount`).
/// ponytail: scalar `u8`s keep the authored target count tiny and easy to copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetCount {
    pub min: u8,
    pub max: u8,
    /// When `true`, `min`/`max` are placeholders substituted at cast time by the spell's own
    /// chosen `{X}` (CR 601.2b — X is fixed before targets are chosen): `Game::choose_spell_targets`
    /// reads the spell's `x` and overrides the effective count, per the rule on that field's own
    /// doc. `{ min: 0, max: 0, x_scaled: true }` is "up to X target(s)" (Silkguard, "up to X" —
    /// fully declinable); `{ min: 1, max: 1, x_scaled: true }` is "exactly X target(s)" (Curse of
    /// the Swine's "exile X target creatures" — X could be 0, but the caster can't decline once
    /// X is chosen positive since `min` only zeroes out when the printed `min` is 0). Defaults to
    /// `false` (every other multi-target effect keeps a fixed count). Parsed by the hand-written
    /// `Deserialize` impl in `de.rs` (not a derive), so no `serde` attribute belongs here.
    pub x_scaled: bool,
    /// The sibling of [`Self::x_scaled`] for a card whose X is never chosen as `{X}` but is
    /// instead *defined* by an additional cost (CR 601.2b/601.2f) — Immoral Bargain's "As an
    /// additional cost to cast this spell, sacrifice X creatures. Destroy X target nonland
    /// permanents." When `true`, `min`/`max` are placeholders `Game::choose_spell_targets`
    /// substitutes at cast time with [`Game::spell_sacrifice_count`] (always "exactly X", unlike
    /// `x_scaled`'s declinable "up to X" case — no pool card sacrifice-scales an optional count).
    /// Defaults to `false`. Parsed by the hand-written `Deserialize` impl in `de.rs`.
    pub sacrifice_scaled: bool,
    /// Strive's own sibling of [`Self::sacrifice_scaled`] (CR 601.2c/601.2f/702.42) — Twinflame's
    /// "Choose any number of target creatures you control" paired with "This spell costs {2}{R}
    /// more to cast for each target beyond the first." Unlike `sacrifice_scaled` (whose X is the
    /// count of permanents already paid as a cost), Strive's target count is a bare number the
    /// caster commits to *before* the stack (CR 601.2c precedes 601.2f) — carried on
    /// [`crate::Intent::Cast`] and recorded as [`crate::types::card::Spell::strive_count`] (read via
    /// [`crate::Game::spell_strive_count`]). When `true`, `min`/`max` are placeholders
    /// [`Game::choose_spell_targets`](crate::Game::choose_spell_targets) substitutes at cast time
    /// with that declared count (always "exactly N," like `sacrifice_scaled`'s "exactly X").
    /// Defaults to `false`. Parsed by the hand-written `Deserialize` impl in `de.rs`.
    pub strive_scaled: bool,
    /// A set-level cap on the chosen targets' *summed* mana value (CR 601.2c: legality is
    /// evaluated over the whole chosen set, not per target) — Rampaging Yao Guai's "destroy any
    /// number of target artifacts and/or enchantments with total mana value X or less". `None`
    /// (default) imposes no budget, matching every existing multi-target effect. `Some(amount)`
    /// is resolved once, against the choosing ability's own source (its entered `{X}` for a
    /// permanent's own ETB, via [`Game::ability_source_x`](crate::Game::ability_source_x)/
    /// [`Game::resolve_amount`](crate::Game::resolve_amount)), and checked against the sum of
    /// each chosen target's mana value ([`Amount::TargetManaValue`]) in
    /// [`Game::choose_targets`](crate::Game::choose_targets) — an over-budget answer is
    /// [`Reject::IllegalChoice`](crate::Reject::IllegalChoice), not a truncation. Distinct from
    /// [`crate::types::effect::dig::DigEffect::LookAtTop`]'s own `mv_budget` (a per-card dig
    /// filter, already resolved to a bare `u32` before the pause); this is a *target-count* axis
    /// so it composes with `min`/`max`/`x_scaled` on any targeted effect, not just a dig. Parsed
    /// by the hand-written `Deserialize` impl in `de.rs`.
    pub total_mv_max: Option<Amount>,
    /// Multikicker's own sibling (CR 601.2c/702.33c) — Comet Storm's "Choose any target, then
    /// choose another target for each time this spell was kicked." Unlike `x_scaled`/
    /// `sacrifice_scaled`/`strive_scaled` (each "exactly N," substituting the declared/derived
    /// count directly), Multikicker's count is always "one base target, plus one more per kick":
    /// when `true`, `min`/`max` are placeholders
    /// [`Game::choose_spell_targets`](crate::Game::choose_spell_targets) substitutes at cast time
    /// with `1 + `[`Game::spell_multikicker_count`](crate::Game::spell_multikicker_count). Defaults
    /// to `false`. Parsed by the hand-written `Deserialize` impl in `de.rs`.
    pub multikicker_scaled: bool,
    /// Kicker's own sibling (CR 702.33d/702.33g) — Orim's Thunder's "If this spell was kicked, it
    /// deals damage equal to that permanent's mana value to target creature," a *whole second
    /// target clause* (not this clause's own destroy target) present only when kicked. When
    /// `true`, the authored `min`/`max` apply only if
    /// [`Game::spell_was_kicked`](crate::Game::spell_was_kicked) holds for the resolving spell;
    /// otherwise the clause is forced to `(0, 0)` — CR 702.33g's "the spell is cast as if it did
    /// not have those targets." Unlike `x_scaled`/`sacrifice_scaled`/`strive_scaled`/
    /// `multikicker_scaled` above (each substituting a *computed count* for the authored
    /// placeholder), `kicked_scaled` never changes what the authored `min`/`max` mean when the
    /// gate holds — it only zeroes the clause out when it doesn't. Defaults to `false`. Parsed by
    /// the hand-written `Deserialize` impl in `de.rs`.
    pub kicked_scaled: bool,
    /// The timing-conditional sibling of [`Self::kicked_scaled`] (CR 601.2c's general
    /// target-conditionality principle, applied to a cast-timing condition rather than an
    /// additional cost) — Return to Dust's "you may exile up to one other target artifact or
    /// enchantment" only if cast during the caster's main phase. When `true`, the authored `max`
    /// applies only if
    /// [`Game::spell_cast_during_main_phase`](crate::Game::spell_cast_during_main_phase) holds;
    /// otherwise `max` is capped down to `min`. Unlike `kicked_scaled` (whose gated clause
    /// vanishes to `(0, 0)` because it's a wholly separate "target creature" clause),
    /// `main_phase_scaled`'s `min` is never touched: Return to Dust's mandatory first target and
    /// its conditional second target are the *same* "target artifact or enchantment" clause (one
    /// count range, not two), so the same-instance distinctness CR 601.2c already gives a
    /// multi-target clause is what makes the second target "other" for free. Defaults to `false`.
    /// Parsed by the hand-written `Deserialize` impl in `de.rs`.
    pub main_phase_scaled: bool,
}

impl Default for TargetCount {
    fn default() -> Self {
        TargetCount {
            min: 1,
            max: 1,
            x_scaled: false,
            sacrifice_scaled: false,
            strive_scaled: false,
            total_mv_max: None,
            multikicker_scaled: false,
            kicked_scaled: false,
            main_phase_scaled: false,
        }
    }
}

impl TargetCount {
    /// Whether this is the ubiquitous single-mandatory-target count — the fast path that keeps
    /// every existing spell on the untouched single-target plumbing. An `x_scaled`,
    /// `sacrifice_scaled`, `strive_scaled`, or `multikicker_scaled` count is never single even
    /// when its printed `{min, max}` happens to be `{1, 1}` — its *effective* count depends on a
    /// cast-time choice/cost and must go through the multi-target machinery.
    pub fn is_single(self) -> bool {
        self == TargetCount::default()
            && !self.x_scaled
            && !self.sacrifice_scaled
            && !self.strive_scaled
            && !self.multikicker_scaled
            && !self.kicked_scaled
            && !self.main_phase_scaled
    }
}
