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
pub enum GraveyardScope {
    /// The ability's controller's own graveyard (Raise Dead's "your graveyard").
    Yours,
    /// Any player's graveyard (Reanimate's "a graveyard").
    Any,
}

/// What an ability targets, checked when the spell/ability is put on the stack.
/// ponytail: single-target model; multi-target grows from real cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
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
    },
    /// An instant or sorcery *spell* currently on the stack (Twincast). Targets the stack
    /// object, not a card in a zone.
    InstantOrSorcerySpellOnStack,
    /// A spell currently on the stack matching a [`SpellFilter`] (Counterspell / Arcane Denial's
    /// unrestricted "counter target spell" is [`SpellFilter::AllSpells`]; Decisive Denial's
    /// "target noncreature spell" and Quandrix Command's "target artifact or enchantment spell"
    /// narrow it). [`Effect::CounterTargetSpell::filter`] supplies the filter.
    SpellOnStack(SpellFilter),
    /// A spell currently on the stack that has exactly one target (Willbender's "target spell …
    /// with a single target", CR 114.6). Targets the stack object; used by
    /// [`Effect::ChangeTargetOfTargetSpellOrAbility`] to pick the spell to bend.
    /// ponytail: CR's "spell or ability" also reaches a single-target activated/triggered ability
    /// on the stack, but stack abilities carry no object identity in this engine (they're keyed by
    /// source, not a chosen `Target`), so only spells are targetable here — see #163's residual gap.
    #[cfg_attr(feature = "card-dsl", serde(rename = "single_target_spell_on_stack"))]
    SingleTargetSpellOnStack,
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
    /// by Populate (CR 701.32), used with [`Effect::CreateTokenCopy`].
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
    pub(crate) fn object_id(self) -> Option<ObjectId> {
        match self {
            Target::Object(id) => Some(id),
            Target::Player(_) => None,
        }
    }
}

/// Which spells a static cost-reducer ([`Effect::ReduceSpellCost`]) applies to — the "spells
/// you cast" clause of a "…cost {N} less" ability. Matched against the card being cast.
/// ponytail: the shapes the pool needs; color/tribe filters ("black creature spells", "Goblin
/// spells") grow from a real card that wants one (they'd need a color/subtype read).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "card-dsl", derive(serde::Deserialize))]
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
}

/// Which library cards a [`Effect::SearchLibrary`] may find (CR 701.19 — "search for a card").
/// ponytail: the shapes the pool needs; a color filter ("a black creature card") grows from a
/// real card that wants one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
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
    /// An artifact, creature, or non-Aura enchantment card with mana value at most `N` (Excava,
    /// the Risen Past: "target artifact, creature, or non-Aura enchantment card with mana value 3
    /// or less"). Reads [`CardKind`] directly rather than [`CardKind::types`] so an Aura — which
    /// carries the same enchantment type bit as a plain enchantment — is excluded like the
    /// printed restriction excludes it.
    ArtifactCreatureOrNonAuraEnchantmentWithManaValueAtMost(u8),
    /// An instant or sorcery card (Mystic Sanctuary: "target instant or sorcery card from your
    /// graveyard").
    InstantOrSorcery,
    /// An enchantment card, no mana-value bound (Replenish: "return all enchantment cards from
    /// your graveyard to the battlefield" — Eiganjo Dynastorian's back face). Counts an Aura, like
    /// [`CardKind::types`] does.
    Enchantment,
    /// A permanent card (creature/artifact/enchantment/planeswalker/land), no mana-value bound
    /// (Deadly Brew: "return another permanent card from your graveyard to your hand"). The
    /// unbounded twin of [`PermanentWithManaValueAtMost`](Self::PermanentWithManaValueAtMost).
    Permanent,
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
}

impl CardFilter {
    /// Whether a card with this definition matches the filter.
    pub(crate) fn matches(self, def: CardDef) -> bool {
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
            CardFilter::ArtifactCreatureOrNonAuraEnchantmentWithManaValueAtMost(max) => {
                matches!(
                    def.kind,
                    CardKind::Artifact | CardKind::Creature { .. } | CardKind::Enchantment
                ) && def.mana_value() <= max as u32
            }
            CardFilter::InstantOrSorcery => matches!(def.kind, CardKind::Spell { .. }),
            CardFilter::Enchantment => def.kind.types().intersects(TypeSet::ENCHANTMENT),
            CardFilter::Permanent => !def.kind.types().is_empty(),
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
        }
    }
}

/// Where a found card goes at the end of a [`Effect::SearchLibrary`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
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
}

/// Where a card selected by [`Effect::LookAtTop`] goes (the "put that card into …" destination).
/// `Battlefield`'s `tapped` gate lives as a sibling flag on [`Effect::LookAtTop::dest_tapped`]
/// (mirroring [`Effect::RevealUntil`]'s `matched_dest`/`matched_tapped` split) rather than a
/// struct-variant field, so the TOML tag stays a bare `dest = "battlefield"` string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum TopDest {
    /// Into the selecting player's hand (Quandrix Apprentice).
    Hand,
    /// Onto the battlefield under the selecting player's control (Armored Skyhunter's "put an
    /// Aura or Equipment card from among them onto the battlefield"), routed through
    /// [`Event::SearchedToBattlefield`] — the same event [`Effect::SearchLibrary`] /
    /// [`Effect::RevealUntil`] use.
    Battlefield,
}

/// Where the *non-matching* revealed/looked-at cards go, shared by [`Effect::LookAtTop`],
/// [`Effect::RevealUntil`], and [`Effect::RevealTopCards`].
/// ponytail: a `Graveyard` arm (a look-then-select whose rest is milled) is the next unlock;
/// add it (routing through [`Event::Milled`], the surveil path) from the first card that needs
/// it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum RestDest {
    /// On the bottom of the selecting player's library (the common case).
    #[default]
    Bottom,
    /// Into the selecting player's hand (Coiling Oracle's "Otherwise, put that card into your
    /// hand").
    Hand,
}

/// Whose library a [`Effect::SearchLibrary`] searches (CR 701.19 — "search their library").
/// Most search effects are self-tutors/ramp; a few (Path to Exile, Assassin's Trophy) hand the
/// search to the *affected permanent's* controller as compensation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum SearchScope {
    /// The ability's own controller (tutors, ramp, fetchlands).
    #[default]
    You,
    /// The ability's shared target's controller (Path to Exile's/Assassin's Trophy's ramp
    /// rider — read via [`Game::controller_of`], which follows the owner chain even after the
    /// target has left the battlefield).
    TargetController,
}

/// Which controller a [`PermanentFilter`] accepts, relative to the effect's controller ("you").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum FilterController {
    /// Any controller (default).
    #[default]
    Any,
    /// A permanent you control.
    You,
    /// A permanent an opponent controls.
    Opponent,
}

/// Whether a [`PermanentFilter`] accepts tokens, nontokens, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
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
pub enum Parity {
    Even,
    Odd,
}

/// A permanent's color restriction for a [`PermanentFilter`] (CR 105.2).
/// ponytail: only "exactly one color" and "is this one specific color" have pool consumers
/// (Vanishing Verse's "monocolored permanent"; Oran-Rief's "green creature"); add
/// `Multicolored` when a real card needs it (stonecoil's "multicolored") rather than
/// pre-building it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
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
}

/// A composable predicate over a battlefield permanent — the one filter behind targeted
/// removal ([`TargetSpec::Permanent`]), mass effects ([`Effect::DestroyAll`] /
/// [`Effect::ReturnAllToHand`]), and sacrifice edicts ([`Effect::EachPlayerSacrifices`]).
/// Every axis is independent; an unset axis imposes no restriction. Evaluated by
/// [`Game::permanent_matches`], which reads the axes needing game state. Kept `Copy` so
/// [`CardDef`] stays `Copy`.
///
/// In TOML it's a `{ … }` table, or a bare-string shorthand for the common shapes —
/// `"creatures"`, `"nonland"`, `"artifact"`, `"creature_or_planeswalker"` (see the `de` module).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PermanentFilter {
    /// Required card types (empty = any type). A permanent matches if it has *any* of these.
    pub types: TypeSet,
    /// Restrict to permanents carrying any of these subtypes (Goldspan Dragon's "Treasures you
    /// control" — `["Treasure"]`, distinguishing a Treasure from any other artifact); empty
    /// matches every subtype. Same shape/rationale as [`Effect::AnthemStatic`]'s own `subtypes`
    /// field (a separate axis there since an anthem always targets creatures specifically).
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
    /// Power ceiling (Silverquill Charm's "creature with power 2 or less"); `None` doesn't gate
    /// on power. Non-creatures have power 0 (see [`Game::power`]), so they always pass a power
    /// gate — no pool card combines `power_max` with a non-creature `types` set.
    pub power_max: Option<u8>,
    /// Power parity gate (Zimone's Hypothesis's "return each creature with power of the chosen
    /// quality"); `None` doesn't gate on parity.
    pub power_parity: Option<Parity>,
    /// Excludes creature-typed permanents (CR: "noncreature artifact"/"noncreature enchantment"
    /// — Haywire Mite). `false` (default) imposes no restriction.
    /// ponytail: a single bool covers the pool's one "not creature" need; generalize to an
    /// `exclude: TypeSet` if a future card needs to exclude a different type.
    pub noncreature: bool,
    /// Color-count restriction (Vanishing Verse's "monocolored permanent"); `Any` (default)
    /// doesn't gate on color. Reads [`Game::colors_of`] — color identity derived from cost
    /// pips, exact for every pool card (no color-indicator cards yet).
    pub color: ColorFilter,
    /// "Modified" (CR 701.29 / Silkguard's reminder text "Equipment, Auras you control, and
    /// counters are modifications") — has any counter, is enchanted by an Aura, or is equipped.
    /// `false` (default) imposes no restriction. See [`Game::is_modified`].
    pub modified: bool,
    /// Restrict to creatures declared as attackers this combat (Tajic's Mentor — "target
    /// *attacking* creature"). `false` (default) imposes no restriction.
    pub attacking: bool,
    /// Power strictly less than the filter's own source permanent's power (Mentor, CR 702.121a
    /// "lesser power"). `false` (default) imposes no restriction. Meaningless without a `source`
    /// (see [`Game::permanent_matches`]) — every filter that sets this pairs it with a targeted
    /// ability, which always threads its source.
    pub power_less_than_source: bool,
    /// Requires the permanent entered the battlefield this turn (CR "entered the battlefield
    /// this turn" — Oran-Rief, the Vastwood's "each green creature that entered this turn").
    /// `false` (default) imposes no restriction. Distinct from checking `summoning_sick`, which
    /// clears one step earlier (see [`Permanent::entered_this_turn`]'s doc).
    pub entered_this_turn: bool,
    /// Excludes basic lands (CR 205.4a's "Basic" supertype — White Orchid Phantom's "target
    /// *nonbasic* land"). `false` (default) imposes no restriction. Read against
    /// [`is_basic_land`] in [`Game::permanent_matches`]; meaningful only alongside a `types` set
    /// that includes land (a nonbasic-land filter is land AND not basic, not "any nonbasic
    /// permanent").
    pub nonbasic: bool,
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
    /// Excludes the "Lair" land subtype (CR 305 — Treva's Ruins' "return a *non-Lair* land you
    /// control"). `false` (default) imposes no restriction. Reads the printed land-type list
    /// directly ([`CardKind::Land::subtypes`], the rules-relevant one — see that field's doc),
    /// not [`CardDef::subtypes`].
    /// ponytail: a single bool covers the pool's one "not this land subtype" need, same shape as
    /// `nonbasic`/`nonlegendary` above; generalize to a `subtypes_exclude` list if a second
    /// land-subtype exclusion turns up.
    pub nonlair: bool,
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
            mv_eq_x: false,
            mv_max_x: false,
            tapped: None,
            power_max: None,
            power_parity: None,
            noncreature: false,
            color: ColorFilter::Any,
            modified: false,
            attacking: false,
            power_less_than_source: false,
            entered_this_turn: false,
            nonbasic: false,
            name: None,
            nonlegendary: false,
            nonlair: false,
        }
    }
}

/// Which players a multi-player sacrifice edict ([`Effect::EachPlayerSacrifices`]) affects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum EdictScope {
    /// "Each player" (Deadly Brew, Promise of Loyalty) — everyone, the edict's controller included.
    AllPlayers,
    /// "Each opponent" (Witch of the Moors, Lorehold Charm) — every player other than the
    /// controller.
    EachOpponent,
    /// "Any number of target players" (Priest of Forgotten Gods, CR 601.2c/608.2b: choosing zero
    /// is legal) — the controller's own chosen subset of living players, picked via a
    /// [`PendingChoice::ChooseTargetPlayers`](super::PendingChoice::ChooseTargetPlayers) pause
    /// before the edict's per-player sacrifice fan-out begins.
    TargetedPlayers,
}

/// Who controls a token minted by [`Effect::CreateToken`] (CR 111.4's "under its controller's
/// control" default is the ability's own controller; some effects hand the token to a different
/// player instead).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum TokenController {
    /// The ability's own controller (the default — most "create a token" effects).
    #[default]
    You,
    /// The controller of this effect's target (Beast Within's "its controller creates a 3/3
    /// Beast" — the destroyed/exiled permanent's controller, read via [`Game::controller_of`]
    /// even after the target has left the battlefield, since a moved object's controller chain
    /// still resolves back to its owner).
    TargetController,
    /// One token minted per opponent of the ability's controller, each under that opponent
    /// (Eccentric Pestfinder's "for each opponent, you create a...").
    EachOpponent,
    /// One token minted per opponent of the ability's controller, each under the ability's own
    /// controller (Eccentric Pestfinder's Turn Stones back face, "For each opponent, you
    /// create a..."). Distinct from [`TokenController::EachOpponent`], which mints one per
    /// opponent under *each opponent*. CR 111.4.
    OnePerOpponent,
    /// The ability's own chosen Player target (Shadrix Silverquill's begin-combat "Target player
    /// creates a 2/1 ... Inkling ... token" — CR 111.4). Makes [`Effect::target`](super::Effect::target)
    /// report [`TargetSpec::Player`](super::TargetSpec::Player) for this `CreateToken`, unlike
    /// every other `TokenController` variant (which take their recipient from context, not a
    /// target of their own).
    TargetPlayer,
    /// The ability's own chosen Player target, restricted to an opponent (CR "target opponent" —
    /// Questing Phelddagrif's "Target opponent creates a 1/1 ... Hippo ... token", CR 111.4). The
    /// opponent-restricted twin of [`TargetPlayer`](Self::TargetPlayer): same [`Target::Player`]
    /// resolution, narrower [`TargetSpec::OpponentPlayer`](super::TargetSpec::OpponentPlayer)
    /// legal-target set.
    TargetOpponent,
}

/// Who acts when a [`Effect::ScheduleAtNextUpkeep`] delayed trigger fires (CR 603.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum DelayController {
    /// The scheduling ability's own controller (Arcane Denial's "you draw a card").
    #[default]
    You,
    /// The controller of the scheduling ability's shared target spell (Arcane Denial's "its
    /// controller may draw up to two cards" — the just-countered spell's controller, read via
    /// [`Game::controller_of`], which resolves through the [`Object::Moved`] chain even after
    /// the spell has left the stack for the graveyard).
    TargetSpellController,
}
