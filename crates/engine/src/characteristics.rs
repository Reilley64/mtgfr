//! Effective power, toughness, keywords, and targeting legality.
//!
//! Characteristic queries used across combat, SBAs, and cast gates.
//! Also: CR 614 slices (counter replacements, enters-tapped). P/T is a CR 613-ordered layered
//! recompute (`pt_layers`/`apply_pt_layers` — 7b base-set, 7c modifications); keywords/other
//! characteristics stay additive per engine-core-and-event-model spec. Deferred / gaps: per-deck increments under `docs/fidelity/` (fidelity-grind skill).

use std::sync::Arc;

use crate::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ContinuousLayer {
    Type,
    Ability,
    PowerToughnessBase,
    PowerToughnessModifier,
    Keywords,
}

/// One engine-internal CR 613 continuous-effect entry affecting an object's effective
/// characteristics. Built fresh per query from today's board/runtime state; never stored back onto
/// `CardDef` or serialized. `timestamp` is the CR 613.7 same-layer ordering key.
#[derive(Clone, Copy)]
struct ContinuousEffect {
    source: ObjectId,
    timestamp: u64,
    kind: ContinuousEffectKind,
}

#[derive(Clone, Copy)]
enum ContinuousEffectKind {
    /// CR 613.4: type/subtype-changing effect.
    SetTypes {
        add_types: TypeSet,
        /// When `true`, `add_types` *replaces* the host's printed card types (Darksteel Mutation),
        /// rather than being unioned onto them (Angelic Destiny).
        set_types: bool,
        set_subtypes: Option<&'static [&'static str]>,
        add_subtypes: &'static [&'static str],
    },
    /// CR 613.1f/613.1e: "loses all abilities" effect on the host's own printed abilities.
    LoseAllAbilities,
    /// CR 613.3(7b): the creature's base P/T is set.
    BasePtSet { power: i32, toughness: i32 },
    /// CR 613.3(7c): a P/T modification added on top of the base.
    PtDelta { power: i32, toughness: i32 },
    /// Keyword abilities granted by a continuous effect.
    GrantKeywords { keywords: &'static [Keyword] },
}

impl ContinuousEffect {
    fn layer(self) -> ContinuousLayer {
        match self.kind {
            ContinuousEffectKind::SetTypes { .. } => ContinuousLayer::Type,
            ContinuousEffectKind::LoseAllAbilities => ContinuousLayer::Ability,
            ContinuousEffectKind::BasePtSet { .. } => ContinuousLayer::PowerToughnessBase,
            ContinuousEffectKind::PtDelta { .. } => ContinuousLayer::PowerToughnessModifier,
            ContinuousEffectKind::GrantKeywords { .. } => ContinuousLayer::Keywords,
        }
    }
}

impl Game {
    /// Every live sourced modifier on a battlefield permanent, grouped by source card def name
    /// for the Alt-inspect ledger. Empty when `object` is not a battlefield permanent.
    /// Continuous effects are re-derived from the board; timed/stateful batches come from
    /// [`Game::modifier_provenance`]. Additive attribution only — not CR 613 layers (engine-core-and-event-model spec).
    pub fn modifier_sources(&self, object: ObjectId) -> Vec<ModifierSourceGroup> {
        if self.as_permanent(object).is_none() {
            return Vec::new();
        }
        let mut groups: Vec<ModifierSourceGroup> = Vec::new();
        let mut push =
            |source_name: &'static str, contribution: ModifierContribution| {
                if source_name.is_empty() {
                    return;
                }
                if let Some(group) = groups.iter_mut().find(|g| g.source_name == source_name) {
                    match &contribution {
                        ModifierContribution::PlusCounters(n) => {
                            if let Some(existing) = group.contributions.iter_mut().find_map(|c| {
                                if let ModifierContribution::PlusCounters(m) = c {
                                    Some(m)
                                } else {
                                    None
                                }
                            }) {
                                *existing += n;
                                return;
                            }
                        }
                        ModifierContribution::PowerToughness { power, toughness } => {
                            if let Some((ep, et)) = group.contributions.iter_mut().find_map(|c| {
                                if let ModifierContribution::PowerToughness {
                                    power: p,
                                    toughness: t,
                                } = c
                                {
                                    Some((p, t))
                                } else {
                                    None
                                }
                            }) {
                                *ep += power;
                                *et += toughness;
                                return;
                            }
                        }
                        ModifierContribution::Keyword(keyword) => {
                            if group.contributions.iter().any(
                                |c| matches!(c, ModifierContribution::Keyword(k) if k == keyword),
                            ) {
                                return;
                            }
                        }
                        ModifierContribution::Goaded
                        | ModifierContribution::Controls
                        | ModifierContribution::ManaAbility => {
                            if group.contributions.iter().any(|c| {
                                std::mem::discriminant(c) == std::mem::discriminant(&contribution)
                            }) {
                                return;
                            }
                        }
                        ModifierContribution::SetBasePowerToughness { .. } => {}
                    }
                    group.contributions.push(contribution);
                    return;
                }
                groups.push(ModifierSourceGroup {
                    source_name,
                    source_card_id: "",
                    contributions: vec![contribution],
                });
            };

        for &(host, count, source_name) in &self.modifier_provenance.counter_batches {
            if host == object && count > 0 {
                push(source_name, ModifierContribution::PlusCounters(count));
            }
        }
        for &(host, power, toughness, keywords, source_name) in
            &self.modifier_provenance.temp_boosts
        {
            if host != object {
                continue;
            }
            if power != 0 || toughness != 0 {
                push(
                    source_name,
                    ModifierContribution::PowerToughness { power, toughness },
                );
            }
            for &keyword in keywords {
                push(source_name, ModifierContribution::Keyword(keyword));
            }
        }
        for &(host, _, source_name) in &self.combat_extras.goaded {
            if host == object {
                push(source_name, ModifierContribution::Goaded);
            }
        }
        for &(host, _, source_name, _) in &self.play_permissions.control_overrides {
            if host == object {
                push(source_name, ModifierContribution::Controls);
            }
        }
        for &(host, _, condition, _) in &self.play_permissions.conditioned_control_overrides {
            if host == object {
                push(
                    self.source_name_of(condition.source),
                    ModifierContribution::Controls,
                );
            }
        }

        for attachment in self.attachments(object) {
            let name = self.def_of(attachment).name;
            for ability in self.def_of(attachment).abilities.iter().cloned() {
                match (ability.timing, ability.effect.clone()) {
                    (
                        Timing::Static,
                        Effect::Static(StaticEffect::GrantToAttached {
                            power,
                            toughness,
                            keywords,
                            goad,
                            legendary_only,
                            ..
                        }),
                    ) => {
                        if let (Amount::Fixed(power), Amount::Fixed(toughness)) = (power, toughness)
                            && (power != 0 || toughness != 0)
                        {
                            push(
                                name,
                                ModifierContribution::PowerToughness { power, toughness },
                            );
                        }
                        // Champion's Helm's "as long as equipped creature is legendary" gate — no
                        // keyword contribution to explain while the host isn't legendary.
                        if !legendary_only || self.def_of(object).legendary {
                            for &keyword in keywords {
                                push(name, ModifierContribution::Keyword(keyword));
                            }
                        }
                        if goad {
                            push(name, ModifierContribution::Goaded);
                        }
                    }
                    (
                        Timing::Static,
                        Effect::Static(StaticEffect::SetAttachedBasePt { power, toughness }),
                    ) => {
                        push(
                            name,
                            ModifierContribution::SetBasePowerToughness { power, toughness },
                        );
                    }
                    (Timing::Static, Effect::Static(StaticEffect::ControlAttached)) => {
                        push(name, ModifierContribution::Controls);
                    }
                    _ => {}
                }
            }
        }

        // Anthems: re-scan like matching_anthems but keep the source permanent's name.
        if let Some(candidate_permanent) = self.as_permanent(object) {
            let owner = candidate_permanent.owner;
            for &id in &self.battlefield() {
                let Some(p) = self.as_permanent(id) else {
                    continue;
                };
                let def = card_def(p.def);
                for ability in def.abilities.iter().cloned() {
                    let (
                        Timing::Static,
                        Effect::Static(StaticEffect::Anthem {
                            power,
                            toughness,
                            keywords,
                            subtypes,
                            colors,
                            exclude_source,
                            attacking_only,
                            all_players,
                            ..
                        }),
                    ) = (ability.timing, ability.effect.clone())
                    else {
                        continue;
                    };
                    if !all_players && p.owner != owner {
                        continue;
                    }
                    if exclude_source && id == object {
                        continue;
                    }
                    if !colors.is_empty()
                        && !colors.iter().any(|c| self.colors_of(object)[c.index()])
                    {
                        continue;
                    }
                    let candidate_subtypes = self.effective_subtypes(object);
                    if !subtypes.is_empty()
                        && !subtypes.iter().any(|s| candidate_subtypes.contains(s))
                    {
                        continue;
                    }
                    if attacking_only && !self.combat.attackers.contains(&object) {
                        continue;
                    }
                    let name = def.name;
                    if let (Amount::Fixed(power), Amount::Fixed(toughness)) = (power, toughness)
                        && (power != 0 || toughness != 0)
                    {
                        push(
                            name,
                            ModifierContribution::PowerToughness { power, toughness },
                        );
                    }
                    for &keyword in keywords {
                        push(name, ModifierContribution::Keyword(keyword));
                    }
                }
            }
        }

        // Granted mana abilities: same owner-wide scan as granted_mana_abilities.
        if let Some(candidate_permanent) = self.as_permanent(object) {
            let owner = candidate_permanent.owner;
            for &id in &self.battlefield() {
                let p = match self.as_permanent(id) {
                    Some(p) if p.owner == owner => p,
                    _ => continue,
                };
                let def = card_def(p.def);
                for ability in def.abilities.iter().cloned() {
                    let (
                        Timing::Static,
                        Effect::Static(StaticEffect::GrantManaAbility { filter, .. }),
                    ) = (ability.timing, ability.effect.clone())
                    else {
                        continue;
                    };
                    if self.permanent_matches(&filter, object, owner, None) {
                        push(def.name, ModifierContribution::ManaAbility);
                    }
                }
            }
        }

        for group in &mut groups {
            group.source_card_id = self.card_id_for_source_name(group.source_name);
        }
        groups
    }

    /// First battlefield permanent whose def name matches `name`, else `""`.
    fn card_id_for_source_name(&self, name: &'static str) -> &'static str {
        if name.is_empty() {
            return "";
        }
        for &id in &self.battlefield() {
            let def = self.def_of(id);
            if def.name == name {
                return def.id;
            }
        }
        ""
    }

    /// Whether a permanent is tapped.
    pub fn is_tapped(&self, object: ObjectId) -> bool {
        self.as_permanent(object).is_some_and(|p| p.tapped)
    }

    /// Whether a permanent is subject to summoning sickness (CR 302.6): it entered under its
    /// current controller this turn *and* is currently a creature. Noncreatures keep the internal
    /// flag for untap bookkeeping, but they are not summoning sick — they may pay `{T}` the turn
    /// they enter, and must not show the client sick badge.
    pub fn is_summoning_sick(&self, object: ObjectId) -> bool {
        self.as_permanent(object).is_some_and(|p| p.summoning_sick)
            && self.effective_types(object).intersects(TypeSet::CREATURE)
    }

    /// Whether a permanent has haste (so it may attack / tap the turn it enters).
    pub fn has_haste(&self, object: ObjectId) -> bool {
        self.has_keyword(object, Keyword::Haste)
    }

    /// Whether `object` currently has `keyword`: its base keywords ∪ keywords granted by
    /// Auras/Equipment attached to it ∪ any until-end-of-turn keyword grant ∪ a matching
    /// static anthem's keyword grant (engine-core-and-event-model spec — effective keywords are a computed union).
    pub fn has_keyword(&self, object: ObjectId, keyword: Keyword) -> bool {
        self.effective_keywords(object).contains(&keyword)
    }

    /// Every keyword `object` currently has: its base keywords, any until-end-of-turn grant,
    /// those granted by attached Auras/Equipment, and those granted by a matching static
    /// anthem. Used by the parametrized keywords (Ward/ProtectionFrom) that carry a value and
    /// so can't be looked up by [`has_keyword`]'s exact-match.
    pub fn effective_keywords(&self, object: ObjectId) -> Vec<Keyword> {
        if let Some(keywords) = self
            .characteristics_cache
            .read(|cache| cache.keywords(object).map(|k| k.to_vec()))
        {
            return keywords;
        }
        let keywords = self.compute_effective_keywords_uncached(object);
        self.characteristics_cache
            .write(|cache| cache.set_keywords(object, keywords.clone()));
        keywords
    }

    /// The generic amount an opponent must pay to target `object`, if it has Ward {N} (CR 702.21).
    pub(crate) fn ward_amount(&self, object: ObjectId) -> Option<u8> {
        self.effective_keywords(object)
            .into_iter()
            .find_map(|k| match k {
                Keyword::Ward(n) => Some(n),
                _ => None,
            })
    }

    /// How many poison counters `object` gives a player it deals combat damage to (CR 702.164a).
    /// Multiple instances of toxic add (CR 702.164b), so every instance is summed rather than
    /// first-matched the way [`ward_amount`](Self::ward_amount) is.
    pub(crate) fn toxic_amount(&self, object: ObjectId) -> i32 {
        self.effective_keywords(object)
            .into_iter()
            .filter_map(|k| match k {
                Keyword::Toxic(n) => Some(i32::from(n)),
                _ => None,
            })
            .sum()
    }

    /// The [`ProtectionScope`]s `object` currently has (CR 702.16), collected from its
    /// effective keywords.
    pub(crate) fn protection_scopes(
        &self,
        object: ObjectId,
    ) -> impl Iterator<Item = ProtectionScope> {
        self.effective_keywords(object)
            .into_iter()
            .filter_map(|keyword| match keyword {
                Keyword::ProtectionFrom(scope) => Some(scope),
                _ => None,
            })
    }

    /// The colors of `object` — its source card's colored cost pips (CR 105.2), plus any colors
    /// added by a CR 613.4-style type-change layer while it's live (a manland's animated form —
    /// [`Permanent::added_colors_eot`]). Used to test a spell/creature against a protected
    /// permanent (a "red" source has a red pip) and by color-scoped anthems ([`Game::colors_of`]
    /// callers).
    ///
    /// A CR 613.3c layer-5 color-SET ([`Permanent::set_color_eot`] — Wild Mongrel's "becomes the
    /// color of your choice until end of turn") wins ahead of the derived/added colors below: it
    /// *replaces* them rather than unioning, so a green Mongrel that becomes black reads as black
    /// only, never green-and-black.
    pub fn colors_of(&self, object: ObjectId) -> [bool; Color::COUNT] {
        if let Some(color) = self.as_permanent(object).and_then(|p| p.set_color_eot) {
            let mut colors = [false; Color::COUNT];
            colors[color.index()] = true;
            return colors;
        }
        let mut colors = color_identity(&self.def_of(object));
        if let Some(p) = self.as_permanent(object) {
            for color in p.added_colors_eot {
                colors[color.index()] = true;
            }
        }
        colors
    }

    /// The old "Radiance" keyword action's target batch (Cleansing Beam, Bathe in Light): the
    /// chosen `target` creature itself, plus every *other* creature on the battlefield sharing
    /// at least one color with it (CR 105.2). Only `target` itself is a real target — CR 608.2b
    /// legality/protection/hexproof gate only that one chosen creature at cast time; the rest of
    /// the batch is swept in untargeted at resolution, the same way `DamageEffect::EachCreature`
    /// sweeps its filter (protection still independently prevents each swept creature's own
    /// share — checked per-creature by the caller). A colorless `target` shares no color with
    /// anything, so its batch is itself alone.
    pub(crate) fn radiance_batch(&self, target: ObjectId) -> Vec<ObjectId> {
        let target_colors = self.colors_of(target);
        self.battlefield()
            .into_iter()
            .filter(|&id| {
                id == target
                    || (self.is_creature_on_battlefield(id)
                        && Color::ALL
                            .iter()
                            .any(|c| target_colors[c.index()] && self.colors_of(id)[c.index()]))
            })
            .collect()
    }

    /// The color named by a `choose_color` step for `object`, wherever it's stored: a permanent
    /// (Mother of Runes, Flickering Ward's own [`Permanent::chosen_color`]) or a spell mid-
    /// resolution (Bathe in Light's [`Spell::chosen_color`] — a spell isn't a permanent yet, so
    /// it can't share that slot). `None` if `object` is neither, or named no color.
    pub(crate) fn chosen_color_of(&self, object: ObjectId) -> Option<Color> {
        match &self.objects[object as usize] {
            Object::Permanent(p) => p.chosen_color,
            Object::Spell(s) => s.chosen_color,
            _ => None,
        }
    }

    /// `player`'s commander color identity (CR 903.4) — the [`color_identity`] of their
    /// commander card, wherever it currently is (command zone or battlefield). All-`false` if
    /// `player` has no object flagged as a commander (a bare test setup with no designated
    /// commander) — CR 903.4 identity mana wouldn't apply without one.
    pub(crate) fn commander_identity_of(&self, player: PlayerId) -> [bool; Color::COUNT] {
        self.live_object_ids()
            .into_iter()
            .find(|&id| self.is_commander(id) && self.owner_of(id) == player)
            .map_or([false; Color::COUNT], |id| color_identity(&self.def_of(id)))
    }

    /// The mana credit "one mana of any color in your commander's color identity" (CR 903.4 —
    /// Command Tower, Arcane Signet) resolves to for `player`: their single identity color, an
    /// [`Mana::Either`] credit for a two-color identity, a [`Mana::OfColors`] restricted-set
    /// credit for a 3+-color identity (Ruhan of the Fomori/Zedruu the Greathearted/Numot, the
    /// Devastator are all WUR), or `None` for a colorless identity (no commander designated, or a
    /// colorless commander — CR 106.6 has no mana of no color).
    pub(crate) fn commander_identity_credit(&self, player: PlayerId) -> Option<Mana> {
        let identity = self.commander_identity_of(player);
        let mut colors = Color::ALL.iter().copied().filter(|c| identity[c.index()]);
        match (colors.next(), colors.next(), colors.next()) {
            (None, ..) => None,
            (Some(c), None, _) => Some(Mana::Color(c)),
            (Some(a), Some(b), None) => Some(Mana::Either(a, b)),
            (Some(_), Some(_), Some(_)) => {
                let mut mask = 0u8;
                for c in Color::ALL {
                    if identity[c.index()] {
                        mask |= 1 << c.index();
                    }
                }
                Some(Mana::OfColors(mask))
            }
        }
    }

    /// The colors a single land (its base tap-for-one `produces` plus every `add_mana` ability's
    /// fixed batch) could currently produce — the per-land building block of
    /// [`Game::opponent_producible_colors_credit`] (Fellwar Stone, Exotic Orchard). Colorless
    /// `{C}` contributes no color.
    /// ponytail: a land whose own producible colors are themselves "any color a land an opponent
    /// controls could produce" (`LandProduces::OpponentColors`, or an `opponent_colors`-count
    /// ability — no card in the pool authors the latter) contributes nothing here rather than
    /// mutually recursing through the querying player's opponents' own opponents. No two cards
    /// in the pool create that cycle today; revisit if one does.
    fn land_producible_colors(&self, land: ObjectId) -> [bool; Color::COUNT] {
        let mut colors = [false; Color::COUNT];
        let def = self.def_of(land);
        if let CardKind::Land {
            produces: Some(produces),
            ..
        } = def.kind
        {
            match produces {
                LandProduces::Mana(Mana::Color(c)) => colors[c.index()] = true,
                LandProduces::Mana(Mana::Either(a, b)) => {
                    colors[a.index()] = true;
                    colors[b.index()] = true;
                }
                LandProduces::Mana(Mana::Any) => colors = [true; Color::COUNT],
                LandProduces::Mana(Mana::Colorless) => {}
                LandProduces::Mana(Mana::OfColors(mask)) => {
                    for c in Color::ALL {
                        colors[c.index()] |= mask & (1 << c.index()) != 0;
                    }
                }
                // No land in the pool has a spend-restricted `produces` — handled for
                // exhaustiveness, same as its unrestricted `base` kind.
                LandProduces::Mana(Mana::Restricted { base, .. }) => match base {
                    RestrictedManaBase::Color(c) => colors[c.index()] = true,
                    RestrictedManaBase::Colorless => {}
                    RestrictedManaBase::Any => colors = [true; Color::COUNT],
                },
                LandProduces::CommanderIdentity => {
                    let identity = self.commander_identity_of(self.controller_of(land));
                    for i in 0..Color::COUNT {
                        colors[i] |= identity[i];
                    }
                }
                LandProduces::OpponentColors => {}
            }
        }
        for ability in def.abilities.iter().cloned() {
            let Effect::Mana(ManaEffect::Add {
                mana: produced,
                identity,
                ..
            }) = ability.effect
            else {
                continue;
            };
            for (i, on) in colors.iter_mut().enumerate() {
                *on |= produced.colored[i] > 0;
            }
            for (&(a, b), &n) in COLOR_PAIRS.iter().zip(produced.either.iter()) {
                if n > 0 {
                    colors[a.index()] = true;
                    colors[b.index()] = true;
                }
            }
            if produced.any > 0 {
                colors = [true; Color::COUNT];
            }
            if identity > 0 {
                let identity = self.commander_identity_of(self.controller_of(land));
                for i in 0..Color::COUNT {
                    colors[i] |= identity[i];
                }
            }
        }
        colors
    }

    /// The mana credit "one mana of any color that a land an opponent controls could produce"
    /// (Fellwar Stone, Exotic Orchard) resolves to for `player`: the union of
    /// [`Self::land_producible_colors`] over every land each opponent of `player` controls,
    /// collapsed to the cheapest matching shape — `None` for no color, [`Mana::Color`] for
    /// exactly one, [`Mana::Any`] for all five (identical in behavior to the general wildcard),
    /// or the restricted [`Mana::OfColors`] credit for 2–4 (the 3+-color case this exists for —
    /// a 4-player pod's opponents collectively produce 3+ colors the overwhelming majority of
    /// games, so an `Any` fallback there would be zero fidelity gain).
    /// ponytail: [`Game::available_mana`] fixed-points paid filter/karoo taps for *amount*; this is a
    /// color-set union and still only needs one qualifying land to add a color (CR 605, CR 108.3, CR 113).
    pub(crate) fn opponent_producible_colors_credit(&self, player: PlayerId) -> Option<Mana> {
        let filter = PermanentFilter {
            controller: FilterController::Opponent,
            ..PermanentFilter::of(TypeSet::LAND)
        };
        let mut colors = [false; Color::COUNT];
        for id in self.battlefield() {
            if !self.permanent_matches(&filter, id, player, None) {
                continue;
            }
            let land_colors = self.land_producible_colors(id);
            for i in 0..Color::COUNT {
                colors[i] |= land_colors[i];
            }
        }
        match colors.iter().filter(|&&c| c).count() {
            0 => None,
            1 => Color::ALL
                .into_iter()
                .find(|c| colors[c.index()])
                .map(Mana::Color),
            5 => Some(Mana::Any),
            _ => {
                let mut mask = 0u8;
                for c in Color::ALL {
                    if colors[c.index()] {
                        mask |= 1 << c.index();
                    }
                }
                Some(Mana::OfColors(mask))
            }
        }
    }

    /// Whether `target` has protection from a color that a spell (known only by its colors, not
    /// an [`ObjectId`]) is (CR 702.16b/e). Used at the targeting site
    /// ([`Game::legal_targets_for`]), which threads a color bitset rather than a source object.
    /// ponytail: [`ProtectionScope::Creatures`] can't be evaluated here — there's no source
    /// `ObjectId` to test "is it a creature" against, only its colors. No pool card targets a
    /// pro-creatures permanent with a creature-sourced spell/ability; thread the source id
    /// through `legal_targets_for` if one ever does. (CR 702.16, CR 601.2c, CR 601)
    pub(crate) fn protection_blocks_source_colors(
        &self,
        target: ObjectId,
        source_colors: [bool; Color::COUNT],
    ) -> bool {
        self.protection_scopes(target)
            .any(|scope| protection_scope_matches(scope, source_colors, None))
    }

    /// Whether `target` has protection from a quality that `source` (a spell/permanent
    /// `ObjectId`) has — its color(s), a creature type (CR 702.16), or multicolored (CR 105.4).
    /// Used at the blocking ([`Game::can_block`]) and combat-damage sites, which both already
    /// have the source's `ObjectId`.
    pub(crate) fn protection_blocks_source(&self, target: ObjectId, source: ObjectId) -> bool {
        let source_is_creature = matches!(self.def_of(source).kind, CardKind::Creature { .. });
        self.protection_scopes(target).any(|scope| {
            protection_scope_matches(scope, self.colors_of(source), Some(source_is_creature))
        })
    }

    /// Whether damage from `source` to `target` is prevented by protection (CR 702.16d).
    pub(crate) fn damage_prevented_by_protection(
        &self,
        target: ObjectId,
        source: Option<ObjectId>,
    ) -> bool {
        let Some(source) = source else {
            return false;
        };
        self.protection_blocks_source(target, source)
    }

    /// The live permanents currently attached to `host` (its Auras/Equipment).
    pub fn attachments(&self, host: ObjectId) -> Vec<ObjectId> {
        self.permanent_ids(move |p| p.attached_to == Some(host))
            .collect()
    }

    /// The Auras `controller` controls that are currently attached to `host` (CR 303.4) —
    /// narrower than [`attachments`](Self::attachments), which also matches Equipment: Killian,
    /// Decisive Mentor's "creatures ... enchanted by an Aura you control attack" needs "attached
    /// AND an Aura AND controlled by a specific player".
    pub(crate) fn auras_controlled_by_attached_to(
        &self,
        host: ObjectId,
        controller: PlayerId,
    ) -> Vec<ObjectId> {
        self.attachments(host)
            .into_iter()
            .filter(|&id| {
                matches!(self.def_of(id).kind, CardKind::Aura)
                    && self.controller_of(id) == controller
            })
            .collect()
    }

    /// The permanent an Aura/Equipment is attached to, if any (a public read query).
    pub fn attached_to(&self, object: ObjectId) -> Option<ObjectId> {
        self.as_permanent(object).and_then(|p| p.attached_to)
    }

    fn static_continuous_timestamp(&self, source: ObjectId) -> u64 {
        self.as_permanent(source)
            .map_or(source as u64, |p| p.continuous_timestamp)
    }

    fn attachment_type_continuous_effects(&self, host: ObjectId) -> Vec<ContinuousEffect> {
        let mut effects = Vec::new();
        for id in self.attachments(host) {
            if self.is_phased_out(id) {
                continue;
            }
            let timestamp = self.static_continuous_timestamp(id);
            for ability in self.def_of(id).abilities.iter().cloned() {
                let (
                    Timing::Static,
                    Effect::Static(StaticEffect::SetAttachedTypes {
                        add_types,
                        set_types,
                        add_subtypes,
                        set_subtypes,
                        lose_all_abilities,
                    }),
                ) = (ability.timing, ability.effect.clone())
                else {
                    continue;
                };
                effects.push(ContinuousEffect {
                    source: id,
                    timestamp,
                    kind: ContinuousEffectKind::SetTypes {
                        add_types,
                        set_types,
                        set_subtypes: (!set_subtypes.is_empty()).then_some(set_subtypes),
                        add_subtypes,
                    },
                });
                if lose_all_abilities {
                    effects.push(ContinuousEffect {
                        source: id,
                        timestamp,
                        kind: ContinuousEffectKind::LoseAllAbilities,
                    });
                }
            }
        }
        effects
    }

    fn attachment_continuous_effects(&self, host: ObjectId) -> Vec<ContinuousEffect> {
        let host_legendary = self.def_of(host).legendary;
        let mut effects = Vec::new();
        for id in self.attachments(host) {
            if self.is_phased_out(id) {
                continue;
            }
            let controller = self.controller_of(id);
            let timestamp = self.static_continuous_timestamp(id);
            for ability in self.def_of(id).abilities.iter().cloned() {
                match (ability.timing, ability.effect.clone()) {
                    (
                        Timing::Static,
                        Effect::Static(StaticEffect::GrantToAttached {
                            power,
                            toughness,
                            keywords,
                            legendary_only,
                            ..
                        }),
                    ) => {
                        let power = self.resolve_amount(power, controller, id, None, 0);
                        let toughness = self.resolve_amount(toughness, controller, id, None, 0);
                        if power != 0 || toughness != 0 {
                            effects.push(ContinuousEffect {
                                source: id,
                                timestamp,
                                kind: ContinuousEffectKind::PtDelta { power, toughness },
                            });
                        }
                        let keywords = if legendary_only && !host_legendary {
                            &[]
                        } else {
                            keywords
                        };
                        if !keywords.is_empty() {
                            effects.push(ContinuousEffect {
                                source: id,
                                timestamp,
                                kind: ContinuousEffectKind::GrantKeywords { keywords },
                            });
                        }
                    }
                    (
                        Timing::Static,
                        Effect::Static(StaticEffect::SetAttachedBasePt { power, toughness }),
                    ) => effects.push(ContinuousEffect {
                        source: id,
                        timestamp,
                        kind: ContinuousEffectKind::BasePtSet { power, toughness },
                    }),
                    (Timing::Static, Effect::Static(StaticEffect::SetAttachedTypes { .. })) => {}
                    _ => {}
                }
            }
        }
        effects
    }

    fn runtime_continuous_effects(&self, object: ObjectId) -> Vec<ContinuousEffect> {
        let Some(p) = self.as_permanent(object) else {
            return Vec::new();
        };
        let mut effects = Vec::new();
        if let Some((power, toughness)) = p.base_pt_set_eot {
            effects.push(ContinuousEffect {
                source: object,
                timestamp: p.base_pt_set_eot_timestamp,
                kind: ContinuousEffectKind::BasePtSet { power, toughness },
            });
        }
        if p.added_types_eot != TypeSet::NONE || !p.added_subtypes_eot.is_empty() {
            effects.push(ContinuousEffect {
                source: object,
                timestamp: p.added_types_eot_timestamp,
                kind: ContinuousEffectKind::SetTypes {
                    add_types: p.added_types_eot,
                    set_types: false,
                    set_subtypes: None,
                    add_subtypes: p.added_subtypes_eot,
                },
            });
        }
        if let Some((power, toughness)) = p.set_base_pt {
            effects.push(ContinuousEffect {
                source: object,
                timestamp: p.set_base_pt_timestamp,
                kind: ContinuousEffectKind::BasePtSet { power, toughness },
            });
        }
        if p.added_types != TypeSet::NONE || !p.added_subtypes.is_empty() {
            effects.push(ContinuousEffect {
                source: object,
                timestamp: p.added_types_timestamp,
                kind: ContinuousEffectKind::SetTypes {
                    add_types: p.added_types,
                    set_types: false,
                    set_subtypes: None,
                    add_subtypes: p.added_subtypes,
                },
            });
        }
        if p.plus_counters != 0 {
            effects.push(ContinuousEffect {
                source: object,
                timestamp: self.static_continuous_timestamp(object),
                kind: ContinuousEffectKind::PtDelta {
                    power: p.plus_counters,
                    toughness: p.plus_counters,
                },
            });
        }
        let minus_counters = p.kind_counters[CounterKind::MinusOneMinusOne as usize] as i32;
        if minus_counters != 0 {
            effects.push(ContinuousEffect {
                source: object,
                timestamp: self.static_continuous_timestamp(object),
                kind: ContinuousEffectKind::PtDelta {
                    power: -minus_counters,
                    toughness: -minus_counters,
                },
            });
        }
        if p.temp_power != 0 || p.temp_toughness != 0 {
            effects.push(ContinuousEffect {
                source: object,
                timestamp: self.static_continuous_timestamp(object),
                kind: ContinuousEffectKind::PtDelta {
                    power: p.temp_power,
                    toughness: p.temp_toughness,
                },
            });
        }
        if !p.temp_keywords.is_empty() {
            effects.push(ContinuousEffect {
                source: object,
                timestamp: self.static_continuous_timestamp(object),
                kind: ContinuousEffectKind::GrantKeywords {
                    keywords: p.temp_keywords,
                },
            });
        }
        if !p.granted_keywords.is_empty() {
            effects.push(ContinuousEffect {
                source: object,
                timestamp: p.added_types_timestamp,
                kind: ContinuousEffectKind::GrantKeywords {
                    keywords: p.granted_keywords,
                },
            });
        }
        // Copy-effect exception keywords (CR 707.2 — "except it has haste/myriad") are part of
        // the object's copiable characteristics, so they grant the keyword the same as any other
        // continuous grant. `copiable_keywords` reads the same field for the copy-again path.
        if !p.copy_rider_keywords.is_empty() {
            effects.push(ContinuousEffect {
                source: object,
                timestamp: self.static_continuous_timestamp(object),
                kind: ContinuousEffectKind::GrantKeywords {
                    keywords: p.copy_rider_keywords,
                },
            });
        }
        effects
    }

    fn anthem_continuous_effects(&self, candidate: ObjectId) -> Vec<ContinuousEffect> {
        let mut effects = Vec::new();
        let owner = self.owner_of(candidate);
        for (source, effect) in self.matching_anthems(candidate) {
            let timestamp = self.static_continuous_timestamp(source);
            if let Effect::Static(StaticEffect::Anthem {
                power,
                toughness,
                keywords,
                ..
            }) = effect
            {
                let power = self.resolve_amount(power, owner, source, None, 0);
                let toughness = self.resolve_amount(toughness, owner, source, None, 0);
                if power != 0 || toughness != 0 {
                    effects.push(ContinuousEffect {
                        source,
                        timestamp,
                        kind: ContinuousEffectKind::PtDelta { power, toughness },
                    });
                }
                if !keywords.is_empty() {
                    effects.push(ContinuousEffect {
                        source,
                        timestamp,
                        kind: ContinuousEffectKind::GrantKeywords { keywords },
                    });
                }
            }
        }
        if self.as_permanent(candidate).is_none() {
            return effects;
        }
        let candidate_controller = self.controller_of(candidate);
        for (source, object) in self.objects.iter().enumerate() {
            if !matches!(object, Object::Permanent(_)) {
                continue;
            }
            let source = source as ObjectId;
            let timestamp = self.static_continuous_timestamp(source);
            for ability in self.functional_abilities(source).iter().cloned() {
                let (
                    Timing::Static,
                    Effect::Static(StaticEffect::KeywordAnthem {
                        keywords,
                        filter,
                        all_players,
                    }),
                ) = (ability.timing, ability.effect.clone())
                else {
                    continue;
                };
                // "You control" is per-ability, not a pre-filter: an `all_players` grant (Avatar of
                // Slaughter's "All creatures have double strike") reaches creatures its controller
                // doesn't control, so its source has to survive the scan.
                if !all_players && self.controller_of(source) != candidate_controller {
                    continue;
                }
                // A level-gated anthem functions only at or above its level (CR 717.5). A
                // battlefield source has a real level; a graveyard-functional one is trivially 1.
                if ability.min_level > self.as_permanent(source).map_or(1, |p| p.level) {
                    continue;
                }
                if !self.permanent_matches(&filter, candidate, candidate_controller, Some(source)) {
                    continue;
                }
                effects.push(ContinuousEffect {
                    source,
                    timestamp,
                    kind: ContinuousEffectKind::GrantKeywords { keywords },
                });
            }
        }
        effects
    }

    /// [`Keyword::ProtectionFrom`] the chosen color of each attached
    /// `protection_from_chosen_color` [`Effect::Static(StaticEffect::GrantToAttached)`] Aura confers on `host`
    /// (Flickering Ward's "Enchanted creature has protection from the chosen color"). The scope is
    /// the Aura's own runtime [`Permanent::chosen_color`], so it can't ride the static `keywords`
    /// slice of [`Game::attachment_grants`] and is read live here. An Aura whose color choice
    /// hasn't been answered yet (or is phased out) grants nothing.
    fn chosen_color_protection_grants(&self, host: ObjectId) -> Vec<Keyword> {
        self.attachments(host)
            .into_iter()
            .filter(|&id| !self.is_phased_out(id))
            .filter(|&id| {
                self.def_of(id).abilities.iter().any(|a| {
                    matches!(
                        (a.timing, a.effect.clone()),
                        (
                            Timing::Static,
                            Effect::Static(StaticEffect::GrantToAttached {
                                protection_from_chosen_color: true,
                                ..
                            })
                        )
                    )
                })
            })
            .filter_map(|id| self.as_permanent(id).and_then(|p| p.chosen_color))
            .map(|color| Keyword::ProtectionFrom(ProtectionScope::Color(color)))
            .collect()
    }

    /// The controllers of every live Aura attached to `host` whose static
    /// [`Effect::Static(StaticEffect::GrantToAttached)`] carries `goad = true` (CR 701.38a — the Impetus cycle,
    /// Redemption Arc): `host` is goaded by each of them for as long as the Aura stays
    /// attached. A live query over the attachment scan, not an entry in [`Game::goaded`], so
    /// it's continuous with no turn-boundary expiry and vanishes the instant the Aura leaves.
    pub(crate) fn goaded_by_attachment(
        &self,
        host: ObjectId,
    ) -> impl Iterator<Item = PlayerId> + '_ {
        self.attachments(host).into_iter().filter_map(move |id| {
            let goads_host = self.def_of(id).abilities.iter().any(|a| {
                matches!(
                    (a.timing, a.effect.clone()),
                    (
                        Timing::Static,
                        Effect::Static(StaticEffect::GrantToAttached { goad: true, .. })
                    )
                )
            });
            goads_host.then(|| self.controller_of(id))
        })
    }

    /// Whether a live Aura attached to `host` carries a static [`Effect::Static(StaticEffect::GrantToAttached)`]
    /// with `cant_attack = true` (Faith's Fetters/Prison Term — "Enchanted permanent/creature
    /// can't attack"), the reverse of [`Self::goaded_by_attachment`]'s "must attack". Continuous,
    /// read off the attachment scan, so it lifts the instant the Aura leaves.
    pub(crate) fn host_cant_attack(&self, host: ObjectId) -> bool {
        self.attachments(host).into_iter().any(|id| {
            !self.is_phased_out(id)
                && self.def_of(id).abilities.iter().any(|a| {
                    matches!(
                        (a.timing, a.effect.clone()),
                        (
                            Timing::Static,
                            Effect::Static(StaticEffect::GrantToAttached {
                                cant_attack: true,
                                ..
                            })
                        )
                    )
                })
        })
    }

    /// Whether a live Aura attached to `host` carries a static [`Effect::Static(StaticEffect::GrantToAttached)`] with
    /// `cant_attack_controller = true` *and* is controlled by `defender` (the Vow cycle's
    /// "Enchanted creature can't attack you" — scoped to this Aura's own controller, unlike
    /// [`Self::host_cant_attack`]'s blanket ban). Read in `declare_attackers` beside the landed
    /// vow-counter check, off the same attachment scan, so it vanishes the instant the Aura leaves.
    pub(crate) fn host_cant_attack_controller(&self, host: ObjectId, defender: PlayerId) -> bool {
        self.attachments(host).into_iter().any(|id| {
            !self.is_phased_out(id)
                && self.controller_of(id) == defender
                && self.def_of(id).abilities.iter().any(|a| {
                    matches!(
                        (a.timing, a.effect.clone()),
                        (
                            Timing::Static,
                            Effect::Static(StaticEffect::GrantToAttached {
                                cant_attack_controller: true,
                                ..
                            })
                        )
                    )
                })
        })
    }

    /// The block-legality twin of [`Self::host_cant_attack`]: whether a live attached Aura's
    /// [`Effect::Static(StaticEffect::GrantToAttached)`] carries `cant_block = true`.
    pub(crate) fn host_cant_block(&self, host: ObjectId) -> bool {
        self.attachments(host).into_iter().any(|id| {
            !self.is_phased_out(id)
                && self.def_of(id).abilities.iter().any(|a| {
                    matches!(
                        (a.timing, a.effect.clone()),
                        (
                            Timing::Static,
                            Effect::Static(StaticEffect::GrantToAttached {
                                cant_block: true,
                                ..
                            })
                        )
                    )
                })
        })
    }

    /// Whether a live Aura attached to `host` *other than* `aura` carries a static
    /// [`Effect::Static(StaticEffect::GrantToAttached)`] with `cant_be_enchanted = true`
    /// (Consecrate Land — "Enchanted land … can't be enchanted by other Auras"). Skipping `aura`
    /// itself is what makes the restriction apply to *other* Auras: the one granting it never
    /// closes the door on itself. Read by [`Game::attachment_host_legal`] (so an Aura already
    /// there falls off, CR 704.5n) and by the Aura-spell target enumeration in
    /// [`Game::legal_targets_for`] (so one can't be cast there in the first place, CR 303.4a).
    pub(crate) fn host_cant_be_enchanted_by(&self, host: ObjectId, aura: ObjectId) -> bool {
        self.attachments(host).into_iter().any(|id| {
            id != aura
                && !self.is_phased_out(id)
                && self.def_of(id).abilities.iter().any(|a| {
                    matches!(
                        (a.timing, a.effect.clone()),
                        (
                            Timing::Static,
                            Effect::Static(StaticEffect::GrantToAttached {
                                cant_be_enchanted: true,
                                ..
                            })
                        )
                    )
                })
        })
    }

    /// The strictest [`AbilityRestriction`] a live attached Aura's [`Effect::Static(StaticEffect::GrantToAttached)`]
    /// imposes on `host`'s own activated abilities (Faith's Fetters' `mana_only` carve-out vs
    /// Prison Term's unqualified `none`), or `None` if no attached Aura restricts them.
    /// ponytail: takes the first such grant; the pool never stacks two ability-restricting Auras
    /// on one host, so no ordering between competing restrictions is needed yet.
    pub(crate) fn host_activated_ability_restriction(
        &self,
        host: ObjectId,
    ) -> Option<AbilityRestriction> {
        self.attachments(host).into_iter().find_map(|id| {
            if self.is_phased_out(id) {
                return None;
            }
            self.def_of(id)
                .abilities
                .iter()
                .find_map(|a| match (a.timing, a.effect.clone()) {
                    (
                        Timing::Static,
                        Effect::Static(StaticEffect::GrantToAttached {
                            activated_abilities: Some(restriction),
                            ..
                        }),
                    ) => Some(restriction),
                    _ => None,
                })
        })
    }

    /// The latest attached [`Effect::Static(StaticEffect::SetAttachedBasePt)`] continuous-effect
    /// entry affecting `host`, if any.
    fn set_base_pt(&self, host: ObjectId) -> Option<ContinuousEffect> {
        self.attachment_continuous_effects(host)
            .into_iter()
            .filter(|effect| matches!(effect.kind, ContinuousEffectKind::BasePtSet { .. }))
            .max_by_key(|effect| (effect.timestamp, effect.source))
    }

    /// Whether `blocker` may block a creature that has flying (it flies or has reach).
    pub(crate) fn can_block_flyers(&self, blocker: ObjectId) -> bool {
        self.has_keyword(blocker, Keyword::Flying) || self.has_keyword(blocker, Keyword::Reach)
    }

    /// The CR 613.4 type/subtype layer a [`Effect::Static(StaticEffect::SetAttachedTypes)`] Aura forces onto `host`:
    /// `(added_types, set_subtypes, added_subtypes)` — the card types unioned on, the creature
    /// subtypes that *replace* the host's own (when present), and the creature subtypes unioned on.
    /// Empty (`TypeSet::NONE`, `None`, `&[]`) when no such Aura is attached.
    fn attached_type_layer(
        &self,
        host: ObjectId,
    ) -> (
        TypeSet,
        bool,
        Option<&'static [&'static str]>,
        &'static [&'static str],
    ) {
        let mut effects: Vec<_> = self
            .attachment_type_continuous_effects(host)
            .into_iter()
            .filter(|effect| matches!(effect.kind, ContinuousEffectKind::SetTypes { .. }))
            .collect();
        effects.sort_by_key(|effect| (effect.layer(), effect.timestamp, effect.source));
        let mut added_types = TypeSet::NONE;
        let mut set_types = false;
        let mut set_subtypes: Option<&'static [&'static str]> = None;
        let mut added_subtypes: &'static [&'static str] = &[];
        for effect in effects {
            let ContinuousEffectKind::SetTypes {
                add_types,
                set_types: set,
                set_subtypes: set_sub,
                add_subtypes,
            } = effect.kind
            else {
                continue;
            };
            added_types = added_types.union(add_types);
            if set {
                set_types = true;
            }
            if let Some(set_sub) = set_sub {
                set_subtypes = Some(set_sub);
            }
            if !add_subtypes.is_empty() {
                added_subtypes = add_subtypes;
            }
        }
        (added_types, set_types, set_subtypes, added_subtypes)
    }

    /// Whether an attached [`Effect::Static(StaticEffect::SetAttachedTypes)`] Aura with `lose_all_abilities = true`
    /// (Darksteel Mutation's "it loses all other abilities") is stripping `host`'s own printed
    /// abilities and keywords (CR 613.1e/701). Only a battlefield permanent can be a host.
    pub(crate) fn host_loses_all_abilities(&self, host: ObjectId) -> bool {
        if self.as_permanent(host).is_none() {
            return false;
        }
        self.attachment_type_continuous_effects(host)
            .into_iter()
            .any(|effect| matches!(effect.kind, ContinuousEffectKind::LoseAllAbilities))
    }

    /// The abilities that *function* on `id` — its printed abilities, unless an attached Aura is
    /// stripping them ([`Game::host_loses_all_abilities`], CR 613.1e/701 "loses all abilities"), in
    /// which case none of the host's own abilities function. The single choke every
    /// battlefield-permanent ability iteration (trigger placement, activation gate, static scans)
    /// reads so the removal applies uniformly. Grants the Aura layers onto the host (its
    /// `grant_to_attached` keywords, its type/base-P/T sets) are separate and unaffected.
    pub(crate) fn functional_abilities(&self, id: ObjectId) -> Arc<[Ability]> {
        // CR 708.2: a face-down permanent (a manifest) has no abilities.
        if self.is_face_down(id) {
            return empty_slice();
        }
        if self.host_loses_all_abilities(id) {
            return empty_slice();
        }
        self.def_of(id).abilities.clone()
    }

    /// Whether `id` is a bestowed permanent (CR 702.103) currently attached to a host: while so it
    /// is an Aura enchantment and **not** a creature (CR 702.103e). An unattached bestowed
    /// permanent is a creature again (CR 702.103i), so this reads the live "attached?" gate, not the
    /// `bestowed` flag alone.
    pub(crate) fn is_bestowed_and_attached(&self, id: ObjectId) -> bool {
        self.as_permanent(id)
            .is_some_and(|p| p.bestowed && p.attached_to.is_some())
    }

    /// A battlefield permanent's card types after the CR 613.4 type layer: its printed types plus
    /// any added by an attached [`Effect::Static(StaticEffect::SetAttachedTypes)`] Aura (Darksteel Mutation → +Artifact).
    /// Reads printed types for a non-permanent (CR 613 applies only to the permanent).
    pub fn effective_types(&self, id: ObjectId) -> TypeSet {
        // CR 708.2: a face-down permanent (a manifest) is a creature and nothing else — its real
        // card types are hidden, and no type layer applies while it's face down.
        if self.is_face_down(id) {
            return TypeSet::CREATURE;
        }
        // CR 702.103e: a bestowed permanent that's attached is an Aura enchantment, not a creature.
        if self.is_bestowed_and_attached(id) {
            return TypeSet::ENCHANTMENT;
        }
        let printed = self.def_of(id).kind.types();
        if self.as_permanent(id).is_none() {
            return printed;
        }
        let (attached_types, set_types, _, _) = self.attached_type_layer(id);
        // CR 613.4: a set-types Aura (Darksteel Mutation) replaces the host's printed card types
        // outright; an additive Aura (Angelic Destiny) unions onto them.
        let mut types = if set_types {
            attached_types
        } else {
            printed.union(attached_types)
        };
        let mut runtime_effects: Vec<_> = self
            .runtime_continuous_effects(id)
            .into_iter()
            .filter(|effect| matches!(effect.kind, ContinuousEffectKind::SetTypes { .. }))
            .collect();
        runtime_effects.sort_by_key(|effect| (effect.layer(), effect.timestamp, effect.source));
        for effect in runtime_effects {
            let ContinuousEffectKind::SetTypes { add_types, .. } = effect.kind else {
                continue;
            };
            types = types.union(add_types);
        }
        types
    }

    /// A battlefield permanent's creature subtypes after the CR 613.4 subtype layer: its printed
    /// subtypes with an attached [`Effect::Static(StaticEffect::SetAttachedTypes)`] Aura's `add_subtypes` unioned on, or
    /// replaced entirely by its `set_subtypes` when set (Darksteel Mutation → `[Insect]`). Reads
    /// printed subtypes for a non-permanent (CR 613 applies only to the permanent).
    pub fn effective_subtypes(&self, id: ObjectId) -> Vec<&'static str> {
        // CR 708.2: a face-down permanent (a manifest) has no subtypes.
        if self.is_face_down(id) {
            return Vec::new();
        }
        // CR 702.103d/e: while attached, a bestowed permanent is an Aura enchantment — it has the
        // Aura subtype (so it counts for "each Aura you control") and none of its creature subtypes.
        if self.is_bestowed_and_attached(id) {
            return vec!["Aura"];
        }
        let printed = self.def_of(id).subtypes;
        if self.as_permanent(id).is_none() {
            return printed.to_vec();
        }
        let (_, _, set, added) = self.attached_type_layer(id);
        let mut subtypes = match set {
            Some(set) => set.to_vec(),
            None => printed.to_vec(),
        };
        subtypes.extend_from_slice(added);
        let mut runtime_effects: Vec<_> = self
            .runtime_continuous_effects(id)
            .into_iter()
            .filter(|effect| matches!(effect.kind, ContinuousEffectKind::SetTypes { .. }))
            .collect();
        runtime_effects.sort_by_key(|effect| (effect.layer(), effect.timestamp, effect.source));
        for effect in runtime_effects {
            let ContinuousEffectKind::SetTypes {
                set_subtypes,
                add_subtypes,
                ..
            } = effect.kind
            else {
                continue;
            };
            if let Some(set_subtypes) = set_subtypes {
                subtypes = set_subtypes.to_vec();
            }
            subtypes.extend_from_slice(add_subtypes);
        }
        subtypes
    }

    /// Whether a creature is barred from attacking / using tap abilities this turn:
    /// summoning sick and without haste. Summoning sickness's `{T}` restriction is creature-only
    /// (CR 302.6) — an artifact/land (a Treasure, a fetchland) may tap the turn it enters.
    /// Creature-ness is effective types (a manland animated the turn it entered is sick too).
    pub(crate) fn is_sick_without_haste(&self, object: ObjectId) -> bool {
        self.is_summoning_sick(object) && !self.has_keyword(object, Keyword::Haste)
    }

    /// A creature's effective power: its printed base run through the CR 613 P/T layers
    /// ([`Game::pt_layers`]/[`Game::apply_pt_layers`] — 7b base-set, then 7c counters/boosts/
    /// anthems/grants). Non-creatures have power 0.
    pub fn power(&self, object: ObjectId) -> i32 {
        if let Some(power) = self.characteristics_cache.read(|cache| cache.power(object)) {
            return power;
        }
        let power = self.compute_power_uncached(object);
        self.characteristics_cache
            .write(|cache| cache.set_power(object, power));
        power
    }

    /// A creature's effective toughness, computed the same way as [`Game::power`].
    pub fn toughness(&self, object: ObjectId) -> i32 {
        if let Some(toughness) = self
            .characteristics_cache
            .read(|cache| cache.toughness(object))
        {
            return toughness;
        }
        let toughness = self.compute_toughness_uncached(object);
        self.characteristics_cache
            .write(|cache| cache.set_toughness(object, toughness));
        toughness
    }

    fn compute_power_uncached(&self, object: ObjectId) -> i32 {
        let Some((power, toughness)) = self.pt_base(object) else {
            return 0;
        };
        Self::apply_pt_layers(power, toughness, self.pt_layers(object)).0
    }

    fn compute_toughness_uncached(&self, object: ObjectId) -> i32 {
        let Some((power, toughness)) = self.pt_base(object) else {
            return 0;
        };
        Self::apply_pt_layers(power, toughness, self.pt_layers(object)).1
    }

    /// The printed base P/T to feed the CR 613 layers, or `None` if `object` has no P/T (not a
    /// creature). A printed creature contributes its printed base; an *animated* noncreature
    /// (Restless Spire, a creature only via `added_types_eot`) has no printed P/T, so its base is
    /// 0/0 — the animation's until-EOT `BasePtSet` layer then supplies the real numbers (CR 613.3).
    fn pt_base(&self, object: ObjectId) -> Option<(i32, i32)> {
        // Guard on the permanent first: `effective_types` reads `def_of`, which panics on an object
        // that has left the game — P/T is queried through the cache on ids that may already be gone.
        let p = self.as_permanent(object)?;
        // CR 708.2: a face-down permanent (a manifest) has base power and toughness 2/2, whatever
        // its hidden card's printed P/T (7c layers — counters/pumps — still apply on top).
        if p.face_down {
            return Some((2, 2));
        }
        if !self.effective_types(object).intersects(TypeSet::CREATURE) {
            return None;
        }
        // CR 712: a flipped permanent's base P/T comes from its back face — read through `def_of`
        // (not `p.def`) so the flipped numbers feed the CR 613 layers.
        match self.def_of(object).kind {
            CardKind::Creature {
                power, toughness, ..
            } => Some((power, toughness)),
            _ => Some((0, 0)),
        }
    }

    /// Every CR 613 P/T continuous-effect entry currently affecting `object`: base-set 7b entries
    /// plus 7c deltas from counters, pumps, anthems, and attachments. Same-layer ordering is
    /// timestamped where the pool needs it (notably stacked base sets such as Trench Gorger under a
    /// later Darksteel Mutation).
    fn pt_layers(&self, object: ObjectId) -> Vec<ContinuousEffect> {
        let mut effects = Vec::new();
        if let Some(effect) = self.set_base_pt(object) {
            effects.push(effect);
        }
        effects.extend(
            self.attachment_continuous_effects(object)
                .into_iter()
                .filter(|effect| matches!(effect.kind, ContinuousEffectKind::PtDelta { .. })),
        );
        effects.extend(
            self.runtime_continuous_effects(object)
                .into_iter()
                .filter(|effect| {
                    matches!(
                        effect.kind,
                        ContinuousEffectKind::BasePtSet { .. }
                            | ContinuousEffectKind::PtDelta { .. }
                    )
                }),
        );
        effects.extend(
            self.anthem_continuous_effects(object)
                .into_iter()
                .filter(|effect| matches!(effect.kind, ContinuousEffectKind::PtDelta { .. })),
        );
        effects
    }

    /// Apply CR 613-ordered P/T `layers` to a creature's `printed` base, returning its effective
    /// `(power, toughness)`: every 7b `BasePtSet` replaces the running base first, then every 7c
    /// `PtDelta` sums on top. `timestamp`/`source` only break ties (deterministic ordering); with
    /// ≤1 base-set and commutative deltas the result equals the old additive recompute exactly.
    fn apply_pt_layers(
        printed_power: i32,
        printed_toughness: i32,
        mut layers: Vec<ContinuousEffect>,
    ) -> (i32, i32) {
        layers.sort_by_key(|effect| (effect.layer(), effect.timestamp, effect.source));
        let mut power = printed_power;
        let mut toughness = printed_toughness;
        for effect in layers {
            match effect.kind {
                ContinuousEffectKind::BasePtSet {
                    power: base_power,
                    toughness: base_toughness,
                } => {
                    power = base_power;
                    toughness = base_toughness;
                }
                ContinuousEffectKind::PtDelta {
                    power: delta_power,
                    toughness: delta_toughness,
                } => {
                    power += delta_power;
                    toughness += delta_toughness;
                }
                ContinuousEffectKind::SetTypes { .. }
                | ContinuousEffectKind::LoseAllAbilities
                | ContinuousEffectKind::GrantKeywords { .. } => {}
            }
        }
        (power, toughness)
    }

    fn compute_effective_keywords_uncached(&self, object: ObjectId) -> Vec<Keyword> {
        // CR 708.2: a face-down permanent (a manifest) has no abilities, so no keyword abilities.
        if self.is_face_down(object) {
            return Vec::new();
        }
        // CR 613.1e/701 "loses all abilities": a host under an ability-removing Aura (Darksteel
        // Mutation) starts from an empty printed-keyword set, so its printed keyword abilities
        // (flying, …) vanish — but the Aura's own granted keywords (indestructible, added below via
        // `attachment_grants`) still ride it.
        let removes_abilities = self.host_loses_all_abilities(object);
        let mut keywords = if removes_abilities {
            Vec::new()
        } else {
            self.def_of(object).keywords.to_vec()
        };
        for (condition, keyword) in self.def_of(object).conditional_keywords.iter().copied() {
            if removes_abilities {
                break;
            }
            // ponytail: `conditional_keywords` is a static keyword grant (CR 604.3), not a
            // triggered ability's intervening-if — it has no `TriggerContext` to run through the
            // general `Game::condition_holds` evaluator, so each source-object-based `Condition`
            // this axis actually uses gets its own arm here rather than a generic dispatch. Grow
            // this match (not a fallthrough to `condition_holds`, which is unreachable from here)
            // when a future card conditions a keyword on something else.
            let holds = match condition {
                Condition::SourceHasCounters { at_least } => {
                    self.source_has_counters(object, at_least)
                }
                Condition::SourceAttackedThisTurn => self
                    .as_permanent(object)
                    .is_some_and(|p| p.attacked_this_turn),
                _ => false,
            };
            if holds {
                keywords.push(keyword);
            }
        }
        for effect in self
            .attachment_continuous_effects(object)
            .into_iter()
            .chain(self.runtime_continuous_effects(object))
            .chain(self.anthem_continuous_effects(object))
        {
            if let ContinuousEffectKind::GrantKeywords { keywords: granted } = effect.kind {
                keywords.extend_from_slice(granted);
            }
        }
        // Backup / "it gains the following abilities until end of turn" (CR 702.166): a granted
        // source's keyword abilities (Guardian Scalelord's flying) ride the target until cleanup.
        // ponytail: reads the source's *printed* keywords, not its own granted-onto-it keywords —
        // no pool card chains grants, so this needs no recursion. (CR 603.10 / last-known info if
        // the source has since left: the link persists on `abilities_granted_until_eot`.)
        for &(target, source) in &self.abilities_granted_until_eot {
            if target == object {
                keywords.extend_from_slice(&self.def_of(source).keywords);
            }
        }
        // Voice of All's own static "This creature has protection from the chosen color" (paired
        // with its as-enters `choose_color`): the self-grant twin of `chosen_color_protection_grants`
        // below, scoped to `object`'s own abilities rather than an attached Aura's. Suppressed
        // alongside the printed keywords above while the host has lost all abilities.
        if !removes_abilities
            && self.def_of(object).abilities.iter().any(|a| {
                matches!(
                    (a.timing, a.effect.clone()),
                    (
                        Timing::Static,
                        Effect::Static(StaticEffect::ProtectionFromChosenColor)
                    )
                )
            })
            && let Some(color) = self.as_permanent(object).and_then(|p| p.chosen_color)
        {
            keywords.push(Keyword::ProtectionFrom(ProtectionScope::Color(color)));
        }
        keywords.extend(self.chosen_color_protection_grants(object));
        // "Lose ... and can't have" (CR 702.11e/702.18d — arcane_lighthouse): strip these off
        // the fully-unioned set last, so a keyword granted by any source above — including one
        // applied *after* the strip landed this turn — is filtered right back out.
        if let Some(p) = self.as_permanent(object) {
            keywords.retain(|k| !p.temp_lost_keywords.contains(k));
        }
        keywords
    }

    /// Every static [`Effect::Static(StaticEffect::Anthem)`] that applies to `candidate`, paired with the
    /// [`ObjectId`] of the permanent carrying it (its source — needed to resolve a dynamic
    /// `power`/`toughness` [`Amount`] and to honor `self_only`): on a permanent `candidate`'s
    /// owner also owns (or, for an `all_players` anthem, any permanent at all), matching its
    /// `subtype`/`attacking_only`/`blocking_only`/`self_only` filter (`None`/`false` matches everything, same as
    /// the old untyped anthem). The shared scan behind [`Game::anthem_pt_bonus`] and
    /// [`Game::anthem_keywords`] — a filtered anthem has to be tested per candidate creature,
    /// unlike the old controller-wide flat bonus.
    fn matching_anthems(&self, candidate: ObjectId) -> Vec<(ObjectId, Effect)> {
        let Some(candidate_permanent) = self.as_permanent(candidate) else {
            return Vec::new();
        };
        let owner = candidate_permanent.owner;
        let mut matches = Vec::new();
        // Battlefield anthems (`from_graveyard == false`) on every permanent, plus graveyard
        // anthems (`from_graveyard == true`) on the owner's graveyard cards that function there
        // (CR 603.6e continuous-analog — Anger's "as long as this card is in your graveyard …
        // creatures you control have haste"). The `bool` tags which zone each source is in so
        // the two anthem kinds never leak across (a graveyard-only anthem's battlefield copy
        // grants nothing, and vice versa). The "source's controller controls candidate" gate is
        // applied per-ability below (skipped for `all_players`), not by pre-filtering sources
        // here — an `all_players` anthem's source can belong to any player.
        let battlefield_sources = self
            .objects
            .iter()
            .enumerate()
            .filter_map(|(index, object)| match object {
                Object::Permanent(p) => Some((index as ObjectId, false, p.owner)),
                _ => None,
            });
        let graveyard_sources = self
            .graveyard_cards(owner)
            .into_iter()
            .filter(|&id| self.def_of(id).functions_in_graveyard)
            .map(|id| (id, true, owner));
        // Emblem anthems (CR 114.3 — Garruk, Cursed Huntsman's "Creatures you control get +3/+3
        // and have trample"). An emblem's abilities function from the command zone, so it is a
        // third source chain here; it is tagged `false` (not "in a graveyard") because a
        // `from_graveyard` anthem is specifically a card functioning from a graveyard, which an
        // emblem never is.
        let emblem_sources = self.emblems(owner).into_iter().map(|id| (id, false, owner));
        for (source, source_in_graveyard, source_owner) in battlefield_sources
            .chain(graveyard_sources)
            .chain(emblem_sources)
        {
            for ability in self.functional_abilities(source).iter().cloned() {
                let (
                    Timing::Static,
                    effect @ Effect::Static(StaticEffect::Anthem {
                        subtypes,
                        colors,
                        chosen_subtype,
                        attacking_only,
                        blocking_only,
                        commander_only,
                        self_only,
                        exclude_source,
                        tokens_only,
                        has_counters,
                        condition,
                        from_graveyard,
                        all_players,
                        war_choice,
                        ..
                    }),
                ) = (ability.timing, ability.effect.clone())
                else {
                    continue;
                };
                if from_graveyard != source_in_graveyard {
                    continue;
                }
                if !all_players && source_owner != owner {
                    continue;
                }
                // A level-gated anthem functions only at or above its level (CR 717.5). A
                // battlefield source has a real level; a graveyard-functional one is trivially 1.
                if ability.min_level > self.as_permanent(source).map_or(1, |p| p.level) {
                    continue;
                }
                if self_only && source != candidate {
                    continue;
                }
                if exclude_source && source == candidate {
                    continue;
                }
                if tokens_only && !self.as_permanent(candidate).is_some_and(|p| p.token) {
                    continue;
                }
                if !colors.is_empty()
                    && !colors.iter().any(|c| self.colors_of(candidate)[c.index()])
                {
                    continue;
                }
                let candidate_subtypes = self.effective_subtypes(candidate);
                if !subtypes.is_empty() && !subtypes.iter().any(|s| candidate_subtypes.contains(s))
                {
                    continue;
                }
                if chosen_subtype {
                    let Some(named) = self.as_permanent(source).and_then(|p| p.chosen_subtype)
                    else {
                        continue; // no choice made yet — no buff
                    };
                    if !candidate_subtypes.contains(&named) {
                        continue;
                    }
                }
                if attacking_only && !self.combat.attackers.contains(&candidate) {
                    continue;
                }
                if blocking_only
                    && !self
                        .combat
                        .blocks
                        .iter()
                        .any(|&(blocker, _)| blocker == candidate)
                {
                    continue;
                }
                if commander_only && !self.is_commander(candidate) {
                    continue;
                }
                if has_counters && !self.has_any_counter(candidate) {
                    continue;
                }
                // Archangel of Strife's "creatures controlled by players who chose war/peace" —
                // read against the candidate's own seat rather than the anthem source's, through
                // the same `owner`-as-controller proxy this whole scan uses for "you control".
                // Keyed by `source` so a second Archangel's answers don't speak for the first's.
                if let Some(wants_war) = war_choice
                    && !self.players[owner.0 as usize]
                        .war_choices
                        .contains(&(source, wants_war))
                {
                    continue;
                }
                // An "as long as …" gate (tendershoot_dryad's city's blessing) — evaluated
                // against the anthem source's own controller, same as its cost/trigger reads
                // would be.
                if let Some(cond) = condition
                    && !self.condition_holds(cond, TriggerContext::of(source_owner))
                {
                    continue;
                }
                matches.push((source, effect));
            }
        }
        matches
    }

    /// Whether a battlefield static prevents all noncombat damage that would be dealt to `target`
    /// (CR 615 — Tajic, Legion's Edge: "Prevent all noncombat damage that would be dealt to other
    /// creatures you control"). True iff some permanent carries a `(Timing::Static,
    /// PreventNoncombatDamageToOtherCreaturesYouControl)` ability, is controlled by the same player
    /// as `target`, and is a *different* object (CR "**other** creatures you control" — never the
    /// source itself). The static-scan sibling of [`Game::matching_anthems`]; read at every
    /// noncombat creature-damage choke (effect + fight damage). Combat damage never consults it.
    pub(crate) fn noncombat_damage_prevented_to_creature(&self, target: ObjectId) -> bool {
        self.replacement_registry()
            .noncombat_damage_prevented_to_creature(self, target)
    }

    /// Whether `target` itself carries Phantom Centaur's self-shield (CR 615: "If damage would
    /// be dealt to Phantom Centaur, prevent that damage."). Unlike
    /// [`Game::noncombat_damage_prevented_to_creature`]'s "other creatures you control" scan,
    /// this is self-only — true iff `target` has a `(Timing::Static,
    /// PreventDamageToSelfRemovingCounter)` ability of its own — and applies to combat damage
    /// too (Tajic's static skips combat; Phantom Centaur's doesn't).
    pub(crate) fn phantom_shield_active(&self, target: ObjectId) -> bool {
        self.replacement_registry()
            .phantom_shield_active(self, target)
    }

    /// Whether `target` carries a permanent combat-damage-prevention static shielding damage
    /// dealt TO itself (CR 615 — Guard Gomazoa: "Prevent all combat damage that would be dealt
    /// to Guard Gomazoa."; Fog Bank's "to and by" wording sets this half too). True iff `target`
    /// has a `(Timing::Static, PreventCombatDamageStatic { to_self: true, .. })` ability of its
    /// own — combat-only, unlike [`Game::phantom_shield_active`], which covers noncombat damage
    /// too and removes a counter each time.
    pub(crate) fn combat_damage_prevented_to_creature(&self, target: ObjectId) -> bool {
        self.replacement_registry()
            .combat_damage_prevented_to_creature(target)
    }

    /// Whether `source` carries a permanent combat-damage-prevention static shielding damage it
    /// deals TO OTHERS (CR 615 — Fog Bank: "... and dealt by Fog Bank."). True iff `source` has
    /// a `(Timing::Static, PreventCombatDamageStatic { by_self: true, .. })` ability of its own —
    /// the sibling query to [`Game::combat_damage_prevented_to_creature`], keyed on the source
    /// end of a combat-damage instance instead of the target end.
    pub(crate) fn combat_damage_prevented_by_source(&self, source: ObjectId) -> bool {
        self.replacement_registry()
            .combat_damage_prevented_by_source(source)
    }

    /// The events Phantom Centaur's shield or Bloatfly Swarm's scaling variant fire alongside
    /// each prevented damage-dealing event (both CR 615): a `CountersPlaced` removing the
    /// counters taken, plus — for Bloatfly Swarm's rad-counter rider only — one
    /// `PlayerCountersPlaced` rad counter (CR 122.1) per player per counter removed. Empty when
    /// there's no counter left to remove (the shield still applies; it just has nothing to take,
    /// CR 615's replacement effect doesn't create counters from nothing). Phantom Centaur's own
    /// variant always removes exactly one counter regardless of `amount` — "remove a +1/+1
    /// counter" isn't scaled by the damage; Bloatfly Swarm's "remove that many" removes
    /// `min(amount, counters present)`, since it can't remove counters that aren't there.
    pub(crate) fn phantom_shield_counter_removal(
        &self,
        target: ObjectId,
        amount: i32,
    ) -> Vec<Event> {
        let registry = self.replacement_registry();
        if !registry.phantom_shield_active(self, target) {
            return Vec::new();
        }
        let available = self.plus_counters(target);
        if available <= 0 {
            return Vec::new();
        }
        let scales = registry.phantom_shield_scales(target);
        let removed = if scales { amount.min(available) } else { 1 };
        if removed <= 0 {
            return Vec::new();
        }
        let mut events = vec![Event::CountersPlaced {
            object: target,
            count: -removed,
            source_name: self.def_of(target).name,
        }];
        if scales {
            events.extend(
                self.living_players()
                    .map(|player| Event::PlayerCountersPlaced {
                        player,
                        kind: PlayerCounterKind::Rad,
                        count: removed,
                    }),
            );
        }
        events
    }

    /// Every activated mana ability granted to `candidate` by a live static
    /// [`Effect::Static(StaticEffect::GrantManaAbility)`] elsewhere on the battlefield (Goldspan Dragon's "Treasures
    /// you control have '{T}, Sacrifice this artifact: Add two mana of any one color.'"). Mirrors
    /// [`Game::matching_anthems`]'s owner-wide scan — recomputed live off the board, no stored
    /// state, so a grant disappears the instant its source leaves. Read by [`Game::ability_at`],
    /// which addresses these past `candidate`'s own abilities.
    pub(crate) fn granted_mana_abilities(
        &self,
        candidate: ObjectId,
    ) -> Vec<(ActivationCost, ManaPool, bool)> {
        let Some(candidate_permanent) = self.as_permanent(candidate) else {
            return Vec::new();
        };
        let owner = candidate_permanent.owner;
        let mut grants = Vec::new();
        for object in &self.objects {
            let Object::Permanent(p) = object else {
                continue;
            };
            if p.owner != owner {
                continue;
            }
            let def = card_def(p.def);
            for ability in def.abilities.iter().cloned() {
                let (
                    Timing::Static,
                    Effect::Static(StaticEffect::GrantManaAbility {
                        filter,
                        cost,
                        mana,
                        restriction,
                        single_color,
                    }),
                ) = (ability.timing, ability.effect.clone())
                else {
                    continue;
                };
                if self.permanent_matches(&filter, candidate, owner, None) {
                    // Wrapped here, once, so every reader of a granted batch (this ability's
                    // own resolution and the `available_mana` estimate) sees it already
                    // spend-restricted (Galazeth Prismari) — see `ManaPool::restricted_by`.
                    grants.push((cost, mana.restricted_by(restriction), single_color));
                }
            }
        }
        grants
    }

    /// Every *activated* (non-mana) ability granted to `host` by a live
    /// [`Effect::Static(StaticEffect::GrantToAttached)`] on an Aura attached to it (Fallen Ideal's "Sacrifice a
    /// creature: This creature gets +2/+1 until end of turn."), as `(cost, effects)`. The
    /// non-mana twin of [`Game::granted_mana_abilities`], sourced from the attachment scan
    /// ([`Game::attachment_grants`]'s shape) rather than an owner-wide filter. Recomputed live —
    /// the grant disappears the instant the Aura leaves. Read by [`Game::ability_at`], which
    /// addresses these past `host`'s own abilities and its granted mana abilities.
    pub(crate) fn granted_attachment_abilities(
        &self,
        host: ObjectId,
    ) -> Vec<(ActivationCost, &'static [Effect])> {
        self.attachments(host)
            .into_iter()
            // A phased-out Aura grants nothing (CR 702.26e), mirroring `attachment_grants`.
            .filter(|&id| !self.is_phased_out(id))
            .flat_map(|id| {
                let def = self.def_of(id);
                def.abilities
                    .iter()
                    .filter_map(|a| match (a.timing, a.effect.clone()) {
                        (
                            Timing::Static,
                            Effect::Static(StaticEffect::GrantToAttached {
                                granted_ability: Some(g),
                                ..
                            }),
                        ) if g.trigger.is_none() => Some((g.cost, g.effects)),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Every *triggered* ability granted to `host` by a live
    /// [`Effect::Static(StaticEffect::GrantToAttached)`] Aura/Equipment attached to it (Power
    /// Fist's "Whenever this creature deals combat damage to a player, put that many +1/+1
    /// counters on it."), synthesized directly as an [`Ability`] — unlike the activated twin
    /// ([`Game::granted_attachment_abilities`]), there is no `ability_at` index to address, since
    /// a triggered ability isn't activated. Recomputed live off the same attachment scan, so it
    /// disappears the instant the Aura/Equipment leaves (CR 702.26e for a phased-out one).
    /// ponytail: only the combat-damage-to-a-player scanner consults granted triggered abilities —
    /// the pool's one consumer (Power Fist). Move this onto a shared owned-abilities accessor the
    /// moment a second granted trigger flavor lands.
    pub(crate) fn granted_attachment_triggers(&self, host: ObjectId) -> Vec<Ability> {
        self.attachments(host)
            .into_iter()
            // A phased-out Aura/Equipment grants nothing (CR 702.26e), mirroring `attachment_grants`.
            .filter(|&id| !self.is_phased_out(id))
            .flat_map(|id| {
                let def = self.def_of(id);
                def.abilities
                    .iter()
                    .filter_map(|a| match (a.timing, a.effect.clone()) {
                        (
                            Timing::Static,
                            Effect::Static(StaticEffect::GrantToAttached {
                                granted_ability: Some(g),
                                ..
                            }),
                        ) => g.trigger.map(|trigger| {
                            let effect = match g.effects {
                                [single] => single.clone(),
                                steps => Effect::Sequence {
                                    steps: steps.into(),
                                },
                            };
                            Ability {
                                timing: Timing::Triggered(trigger),
                                effect,
                                optional: false,
                                min_level: 0,
                                cost: Cost::FREE,
                                condition: None,
                                once_each_turn: false,
                            }
                        }),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// The ability at `index` on `object`, in a stable order: its own
    /// (`index < def.abilities.len()`), then those granted by a live static
    /// [`Effect::Static(StaticEffect::GrantManaAbility)`] elsewhere on the battlefield
    /// ([`Game::granted_mana_abilities`]), then those granted by an
    /// [`Effect::Static(StaticEffect::GrantToAttached)`] on an Aura attached to it
    /// ([`Game::granted_attachment_abilities`]). Each grant block occupies contiguous indices
    /// immediately past the prior. The one seam [`Game::ability_activation_gate`] and
    /// [`Game::legal_targets`] read so every granted ability activates exactly like an own one.
    /// `None` for an out-of-range index.
    pub fn ability_at(&self, object: ObjectId, index: usize) -> Option<Ability> {
        let def = self.def_of(object);
        if let Some(ability) = def.abilities.get(index) {
            return Some(ability.clone());
        }
        let granted_index = index - def.abilities.len();
        let mana_grants = self.granted_mana_abilities(object);
        if let Some(&(cost, mana, single_color)) = mana_grants.get(granted_index) {
            return Some(Ability {
                timing: Timing::Activated(cost),
                effect: Effect::Mana(ManaEffect::Add {
                    // `mana` is already spend-restricted where applicable — `granted_mana_abilities`
                    // wraps it, so this virtual ability needs no `restriction` of its own.
                    mana,
                    identity: 0,
                    opponent_colors: 0,
                    repeat: Amount::Fixed(1),
                    restriction: None,
                    single_color,
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
            });
        }
        let (cost, effects) = self
            .granted_attachment_abilities(object)
            .into_iter()
            .nth(granted_index - mana_grants.len())?;
        // A one-effect grant is used directly; multiple run as a `Sequence` (the same shape a
        // multi-effect own ability uses).
        let effect = match effects {
            [single] => single.clone(),
            steps => Effect::Sequence {
                steps: steps.into(),
            },
        };
        Some(Ability {
            timing: Timing::Activated(cost),
            effect,
            optional: false,
            min_level: 0,
            cost: Cost::FREE,
            condition: None,
            once_each_turn: false,
        })
    }

    /// Whether `player` has no maximum hand size (CR 402.2): true if any permanent they control
    /// has a live [`Effect::Static(StaticEffect::NoMaximumHandSize)`] static ability (e.g. Reliquary Tower). Read by the
    /// cleanup step's discard-to-hand-size turn-based action; a characteristic-defining continuous
    /// effect (CR 611), so no event is needed — it just stops applying when the source leaves.
    pub(crate) fn has_no_max_hand_size(&self, player: PlayerId) -> bool {
        self.objects.iter().any(|object| {
            let Object::Permanent(p) = object else {
                return false;
            };
            let def = card_def(p.def);
            p.owner == player
                && def.abilities.iter().any(|a| {
                    (a.timing, a.effect.clone())
                        == (
                            Timing::Static,
                            Effect::Static(StaticEffect::NoMaximumHandSize),
                        )
                })
        })
    }

    /// Whether `player` controls a permanent granting Serra Paragon's graveyard-play permission
    /// (CR 118.9 — a live [`Effect::Static(StaticEffect::PlayFromGraveyardOncePerTurn)`] static ability). Read by
    /// [`Game::playable_zone`] to decide whether a land / permanent spell in `player`'s graveyard
    /// is playable this turn; the "once during each of your turns" cap is a separate gate
    /// ([`Player::graveyard_play_used_this_turn`]).
    pub(crate) fn grants_graveyard_recursion(&self, player: PlayerId) -> bool {
        self.objects.iter().any(|object| {
            let Object::Permanent(p) = object else {
                return false;
            };
            let def = card_def(p.def);
            p.owner == player
                && def.abilities.iter().any(|a| {
                    (a.timing, a.effect.clone())
                        == (
                            Timing::Static,
                            Effect::Static(StaticEffect::PlayFromGraveyardOncePerTurn),
                        )
                })
        })
    }

    /// Whether `searcher` is denied library search by an opponent's [`Effect::Static(StaticEffect::OpponentsCantSearchLibraries)`]
    /// static ability (CR 701.19, Stranglehold's "Your opponents can't search libraries"): true if
    /// any *other* player controls a permanent with that static live. The single choke every
    /// library search raises through ([`crate::pending::raise::library::search_library`]), so a
    /// denied search never even offers a `PendingChoice` — no shuffle either (the search and its
    /// tied shuffle are one instruction; Stranglehold skips both, per the printed ruling).
    pub(crate) fn opponent_search_denied(&self, searcher: PlayerId) -> bool {
        self.objects.iter().any(|object| {
            let Object::Permanent(p) = object else {
                return false;
            };
            p.owner != searcher
                && card_def(p.def).abilities.iter().any(|a| {
                    (a.timing, a.effect.clone())
                        == (
                            Timing::Static,
                            Effect::Static(StaticEffect::OpponentsCantSearchLibraries),
                        )
                })
        })
    }

    /// Every "you may spend `from` mana as though it were `to` mana" substitution `player`
    /// controls (Sunglasses of Urza, CR 609.4b), as `(from, to)` color pairs. The payment path
    /// hands these to [`ManaPool::substituted`] before planning — [`Game::plan_payment`] and
    /// [`Game::plan_auto_taps`] (so a cost can actually be paid and auto-tapped that way) and
    /// [`Game::available_mana`] (so the playability/`{X}`-ceiling estimate agrees with them).
    /// Empty — and so free — for every board without one.
    pub(crate) fn mana_substitutions(&self, player: PlayerId) -> Vec<(Color, Color)> {
        let mut subs = Vec::new();
        for (id, object) in self.objects.iter().enumerate() {
            let Object::Permanent(p) = object else {
                continue;
            };
            if self.controller_of(id as ObjectId) != player {
                continue;
            }
            for ability in card_def(p.def).abilities.iter() {
                let (
                    Timing::Static,
                    Effect::Static(StaticEffect::SpendManaAsThoughAnotherColor { from, to }),
                ) = (ability.timing, ability.effect.clone())
                else {
                    continue;
                };
                subs.push((from, to));
            }
        }
        subs
    }

    /// Total generic cost reduction `player`'s static [`Effect::Static(StaticEffect::ReduceSpellCost)`] abilities grant
    /// to a spell they're casting (`def`, aimed at `target`): the sum of every matching reducer
    /// they control (CR 118.9 — reduces generic mana only, so the caller floors generic at 0).
    /// Pure recompute each cast — nothing is stored (engine-core-and-event-model spec, applied to cost).
    pub(crate) fn cost_reduction(
        &self,
        player: PlayerId,
        def: CardDef,
        target: Option<Target>,
        from_zone: Zone,
    ) -> u8 {
        let mut total: u8 = 0;
        for (id, object) in self.objects.iter().enumerate() {
            let Object::Permanent(p) = object else {
                continue;
            };
            if p.owner != player {
                continue;
            }
            let printed = card_def(p.def);
            for ability in printed.abilities.iter().cloned() {
                let (
                    Timing::Static,
                    Effect::Static(StaticEffect::ReduceSpellCost {
                        amount,
                        filter,
                        first_x_spell_each_turn,
                    }),
                ) = (ability.timing, ability.effect.clone())
                else {
                    continue;
                };
                // A level-gated reducer (Advanced Reconstruction's level 3) functions only at or
                // above its level (CR 717.5).
                if ability.min_level > p.level {
                    continue;
                }
                // Zimone, Infinite Analyst: "The first spell you cast with {X} in its mana cost
                // each turn..." — cost_reduction runs before this cast's own SpellCast event
                // increments the tally (CR 601.2f applies the reduction as the spell is cast), so
                // a 0 tally here means this cast IS the turn's first {X} spell.
                if first_x_spell_each_turn
                    && self.players[player.0 as usize].x_spells_cast_this_turn > 0
                {
                    continue;
                }
                if !self.spell_matches_filter(filter, def.clone(), target, player, from_zone) {
                    continue;
                }
                let resolved = self.resolve_amount(amount, player, id as ObjectId, None, 0);
                total = total.saturating_add(resolved.max(0) as u8);
            }
        }
        total
    }

    /// Whether the spell `def` (aimed at `target`, cast by `caster` from `from_zone`) matches a
    /// [`SpellFilter`]. `caster` is only read by
    /// [`SpellFilter::AuraTargetsModifiedPermanentYouControl`] and `from_zone` only by
    /// [`SpellFilter::CastFromNonHandZone`] — every other arm ignores both (callers with no cast
    /// zone in hand pass [`Zone::Hand`], the plain hand-cast default).
    pub(crate) fn spell_matches_filter(
        &self,
        filter: SpellFilter,
        def: CardDef,
        target: Option<Target>,
        caster: PlayerId,
        from_zone: Zone,
    ) -> bool {
        let is_creature = matches!(def.kind, CardKind::Creature { .. });
        match filter {
            SpellFilter::AllSpells => true,
            SpellFilter::CreatureSpells => is_creature,
            SpellFilter::NoncreatureSpells => !is_creature,
            SpellFilter::SpellsThatTargetACreature => {
                matches!(target, Some(Target::Object(id)) if self.is_creature_on_battlefield(id))
            }
            SpellFilter::Aura => matches!(def.kind, CardKind::Aura),
            SpellFilter::InstantOrSorcery => matches!(def.kind, CardKind::Spell { .. }),
            SpellFilter::Enchantment => def.kind.types().intersects(TypeSet::ENCHANTMENT),
            SpellFilter::ArtifactOrEnchantment => def
                .kind
                .types()
                .intersects(TypeSet::ARTIFACT.union(TypeSet::ENCHANTMENT)),
            SpellFilter::HasSubtype(subtypes) => def.subtypes.iter().any(|s| subtypes.contains(s)),
            SpellFilter::HasXInCost => def.cost.x > 0,
            SpellFilter::InstantOrSorceryWithXInCost => {
                matches!(def.kind, CardKind::Spell { .. }) && def.cost.x > 0
            }
            // CR 702.135a: an artifact, legendary, or Saga card is historic.
            SpellFilter::Historic => {
                def.legendary
                    || def.kind.types().intersects(TypeSet::ARTIFACT)
                    || def.subtypes.contains(&"Saga")
            }
            SpellFilter::AuraTargetsModifiedPermanentYouControl => {
                if !matches!(def.kind, CardKind::Aura) {
                    return false;
                }
                let Some(Target::Object(id)) = target else {
                    return false;
                };
                self.is_modified(id, caster) && self.controller_of(id) == caster
            }
            // Advanced Reconstruction's level 3: "Spells you cast from anywhere other than your
            // hand …" — the only arm that reads the cast-from zone (CR 601).
            SpellFilter::CastFromNonHandZone => from_zone != Zone::Hand,
            // Spell Blast's "with mana value X" is matched inline in `legal_targets_for`, the
            // only choke that knows the filtering spell's own X. Never true here: this function's
            // trigger and cost-reducer callers have no X to compare against.
            SpellFilter::ManaValueEqualsX => false,
            // Balefire Liege's "cast a red spell" / "cast a white spell" — CR 105.1/202.2, the
            // spell's own colors (a multicolored spell matches every one of its colors).
            SpellFilter::Color(color) => color_identity(&def)[color.index()],
        }
    }

    /// The number of +1/+1 counters actually placed when `placer` would put `base` on `object`
    /// (CR 614 — Hardened Scales, Doubling Season).
    pub(crate) fn counters_after_replacements(
        &self,
        placer: PlayerId,
        object: ObjectId,
        base: i32,
    ) -> i32 {
        self.replacement_registry().counter_replaced_amount(
            self,
            placer,
            CounterRecipient::Permanent(object),
            true,
            base,
        )
    }

    /// The number of counters of a *named* kind (CR 122.1 — charge, -1/-1, …) actually placed when
    /// `placer` would put `base` on `object`. Only "one or more counters" replacements see these
    /// (Winding Constrictor, Vorinclex); a "+1/+1 counters" replacement does not.
    pub(crate) fn kind_counters_after_replacements(
        &self,
        placer: PlayerId,
        object: ObjectId,
        base: i32,
    ) -> i32 {
        self.replacement_registry().counter_replaced_amount(
            self,
            placer,
            CounterRecipient::Permanent(object),
            false,
            base,
        )
    }

    /// The number of counters actually placed when `placer` would put `base` on `player` — the
    /// player half of CR 122.1 (poison, rad, experience), reached by Winding Constrictor's "if you
    /// would get one or more counters" and Vorinclex's "on a permanent or player".
    pub(crate) fn player_counters_after_replacements(
        &self,
        placer: PlayerId,
        player: PlayerId,
        base: i32,
    ) -> i32 {
        self.replacement_registry().counter_replaced_amount(
            self,
            placer,
            CounterRecipient::Player(player),
            false,
            base,
        )
    }

    /// The total additional +1/+1 counters `entered` receives from every static "creatures you
    /// control enter with additional counters" ability its controller's own *other* permanents
    /// carry (CR 614.1c — Gorma, the Gullet's third ability). Each qualifying static's `count` is
    /// resolved with that static's own permanent as `source` and summed; multiple sources add
    /// together. `entered` is already a live permanent by the time this is called (see
    /// [`Game::resolve_spell`]), but a static housed on `entered` itself is excluded: a permanent's
    /// ETB-modifying replacement never applies to its own entry, because the static isn't
    /// functioning until the permanent is on the battlefield (same ruling as Master Biomancer /
    /// Corpsejack Menace not affecting their own entry).
    pub(crate) fn additional_enter_counters(&self, entered: ObjectId, controller: PlayerId) -> i32 {
        self.replacement_registry()
            .additional_enter_counters(self, entered, controller)
    }

    /// The number of tokens actually created when an effect would create `base` tokens under
    /// `recipient`'s control, after that player's static token-creation replacements (CR 614 —
    /// Doubling Season, "twice that many of those tokens"). Each [`Effect::Static(StaticEffect::TokenReplacement)`]
    /// that `recipient` controls multiplies the count once; the multipliers fold together.
    pub(crate) fn token_count_after_replacements(&self, recipient: PlayerId, base: u32) -> u32 {
        self.replacement_registry()
            .token_replaced_amount(recipient, base)
    }

    /// The life actually gained when `recipient` would gain `base` life, after that player's static
    /// life-gain replacements (CR 614 — Pest Rescuer, "you gain that much life plus 1 instead").
    /// Each [`Effect::Static(StaticEffect::LifeGainReplacement)`] that `recipient` controls adds its `plus`; the addends
    /// fold together. Gaining `base <= 0` is not "gaining life", so no replacement applies.
    pub(crate) fn life_gain_after_replacements(&self, recipient: PlayerId, base: i32) -> i32 {
        self.replacement_registry()
            .life_gain_replaced_amount(recipient, base)
    }

    /// The value of `{X}` a permanent spell actually enters/resolves with when `caster` casts it
    /// for the announced `base`, after that caster's static cast-X modifications (CR 107.3 —
    /// Unbound Flourishing, "double the value of X"). Applies only to *permanent* spells whose cost
    /// contains `{X}`; each [`Effect::Static(StaticEffect::CastXReplacement)`] `caster` controls multiplies the value
    /// once, folding multiplicatively like the token choke. The cost was already paid at `base`, so
    /// only the stored value downstream effects read is changed — not the payment.
    pub(crate) fn cast_x_after_replacements(
        &self,
        caster: PlayerId,
        def: &CardDef,
        base: u32,
    ) -> u32 {
        if base == 0 {
            return 0;
        }
        if def.cost.x == 0 {
            return base;
        }
        // Unbound's first ability is permanent-spells only — its instant/sorcery half is the
        // (unrelated) copy ability. Lands never carry {X}, but exclude them for the same reason.
        if matches!(def.kind, CardKind::Spell { .. } | CardKind::Land { .. }) {
            return base;
        }
        let mut product: u32 = 1;
        for (id, obj) in self.objects.iter().enumerate() {
            let Object::Permanent(p) = obj else {
                continue;
            };
            if self.controller_of(id as ObjectId) != caster {
                continue;
            }
            let def = card_def(p.def);
            for ability in def.abilities.iter().cloned() {
                let (Timing::Static, Effect::Static(StaticEffect::CastXReplacement { times })) =
                    (ability.timing, ability.effect.clone())
                else {
                    continue;
                };
                product *= times.max(0) as u32;
            }
        }
        base * product
    }

    /// Test/setup helper: place a +1/+1 counter on a permanent (raw — bypasses replacements).
    pub fn add_plus_counter(&mut self, object: ObjectId) {
        self.apply(&Event::CountersPlaced {
            object,
            count: 1,
            source_name: self.def_of(object).name,
        });
    }

    /// Test/setup helper: place one named counter on a permanent (raw — bypasses replacements).
    pub fn add_kind_counter(&mut self, object: ObjectId, kind: CounterKind) {
        self.apply(&Event::KindCountersPlaced {
            object,
            kind,
            count: 1,
        });
    }

    /// Test/setup helper: place a finality counter on a permanent directly (raw — the normal
    /// path is a `finality = true` reanimation; see `Event::ReanimatedToBattlefield`).
    pub fn add_finality_counter(&mut self, object: ObjectId) {
        self.permanent_mut(object).finality_counter = true;
    }
}

/// Whether a source with `source_colors` (and, when known, `source_is_creature`) matches a
/// [`ProtectionScope`] — the predicate shared by [`Game::protection_blocks_source_colors`] (no
/// source object, so `source_is_creature` is `None` and `Creatures` never matches) and
/// [`Game::protection_blocks_source`] (`Some`, from the source's actual card type).
fn protection_scope_matches(
    scope: ProtectionScope,
    source_colors: [bool; Color::COUNT],
    source_is_creature: Option<bool>,
) -> bool {
    match scope {
        ProtectionScope::Color(color) => source_colors[color.index()],
        // "Multicolored" is two or more colors (CR 105.4) — a monocolored or colorless source
        // doesn't qualify.
        ProtectionScope::Multicolored => source_colors.iter().filter(|&&c| c).count() >= 2,
        ProtectionScope::Creatures => source_is_creature.unwrap_or(false),
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    const FREE: Cost = Cost {
        generic: 0,
        colored: [0; Color::COUNT],
        colorless: 0,
        x: 0,
        hybrid: &[],
        phyrexian: &[],
        additional: AdditionalCost {
            discard: 0,
            discard_land: false,
            reveal_creature_from_hand: false,
            pay_life_x: false,
            pay_life: 0,
            sacrifice: None,
            kicker: None,
            buyback: None,
            strive: None,
            replicate: None,
            multikicker: None,
        },
        reduce_own_generic: None,
    };

    fn creature(power: i32, toughness: i32) -> CardDef {
        CardDef {
            name: "Test Creature",
            id: "",
            default_print: "",
            cost: FREE,
            kind: CardKind::Creature {
                power,
                toughness,
                also: TypeSet::NONE,
            },
            legendary: false,
            snow: false,
            uncounterable: false,
            modal: false,
            modal_choose: 0,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            keywords: empty_slice(),
            conditional_keywords: empty_slice(),
            abilities: empty_slice(),
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
            subtypes: empty_slice(),
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

    fn anthem() -> CardDef {
        static ABILITIES: &[Ability] = &[Ability {
            timing: Timing::Static,
            effect: Effect::Static(StaticEffect::Anthem {
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(1),
                self_only: false,
                exclude_source: false,
                tokens_only: false,
                keywords: &[],
                subtypes: &[],
                colors: &[],
                chosen_subtype: false,
                attacking_only: false,
                blocking_only: false,
                commander_only: false,
                has_counters: false,
                condition: None,
                from_graveyard: false,
                all_players: false,
                war_choice: None,
            }),
            optional: false,
            min_level: 0,
            cost: Cost::FREE,
            condition: None,
            once_each_turn: false,
        }];
        CardDef {
            name: "Test Anthem",
            id: "",
            default_print: "",
            cost: FREE,
            kind: CardKind::Enchantment,
            legendary: false,
            snow: false,
            uncounterable: false,
            modal: false,
            modal_choose: 0,
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
            subtypes: empty_slice(),
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

    #[test]
    fn cache_populated_and_reused_on_repeated_power_query() {
        let mut game = Game::with_players(2, 0);
        let bear = game.spawn_on_battlefield(PlayerId(0), creature(2, 2));
        assert!(
            game.characteristics_cache
                .read(|cache| cache.power(bear).is_none())
        );

        assert_eq!(game.power(bear), 2);
        assert_eq!(
            game.characteristics_cache.read(|cache| cache.power(bear)),
            Some(2)
        );
        assert_eq!(game.power(bear), 2);
    }

    #[test]
    fn invalidate_on_counters_placed() {
        let mut game = Game::with_players(2, 0);
        let bear = game.spawn_on_battlefield(PlayerId(0), creature(2, 2));
        assert_eq!(game.power(bear), 2);

        game.apply(&Event::CountersPlaced {
            object: bear,
            count: 1,
            source_name: "Test",
        });
        assert!(
            game.characteristics_cache
                .read(|cache| cache.power(bear).is_none()),
            "counter event should drop the cached power"
        );
        assert_eq!(game.power(bear), 3);
    }

    #[test]
    fn invalidate_on_permanent_entered_anthem_owner() {
        let mut game = Game::with_players(2, 0);
        let bear = game.spawn_on_battlefield(PlayerId(0), creature(2, 2));
        assert_eq!(game.power(bear), 2);

        let spell = game.create_object(
            None,
            Object::Spell(Spell {
                def: intern_card_def(anthem()),
                controller: PlayerId(0),
                targets: TargetList::default(),
                targets_second: TargetList::default(),
                commander: false,
                x: 0,
                chosen_color: None,
                modes: Modes::default(),
                copy: false,
                flashback: false,
                escape: false,
                cast_from_hand: false,
                cast_during_main_phase: false,
                damage_division: DamageAssignment::default(),
                damage_division_players: [None; MAX_TARGETS],
                counter_division: DamageAssignment::default(),
                sacrifice_count: 0,
                revealed_creature_mana_value: 0,
                kicked: false,
                bought_back: false,
                strive_count: 0,
                replicate_count: 0,
                multikicker_count: 0,
                serra_recursion: false,
                bestowed: false,
                face_down: false,
                masked: false,
                evoked: false,
                spent_colors: [false; Color::COUNT],
                phyrexian_life_paid: 0,
            }),
        );
        let permanent = game.objects.len() as ObjectId;
        game.apply(&Event::PermanentEntered {
            permanent,
            from: spell,
        });

        assert_eq!(game.power(bear), 3);
    }

    #[test]
    fn cached_keywords_invalidate_on_temp_boost() {
        let mut game = Game::with_players(2, 0);
        let bear = game.spawn_on_battlefield(PlayerId(0), creature(2, 2));
        assert!(!game.has_keyword(bear, Keyword::Flying));
        assert!(
            game.characteristics_cache
                .read(|cache| cache.keywords(bear).is_some())
        );

        game.apply(&Event::TempBoost {
            object: bear,
            power: 0,
            toughness: 0,
            keywords: &[Keyword::Flying],
            source_name: "Test",
        });
        assert!(
            game.characteristics_cache
                .read(|cache| cache.keywords(bear).is_none())
        );
        assert!(game.has_keyword(bear, Keyword::Flying));
    }

    fn forest() -> CardDef {
        CardDef {
            name: "Forest",
            id: "",
            default_print: "",
            cost: Cost::FREE,
            kind: CardKind::Land {
                produces: Some(LandProduces::Mana(Mana::Color(Color::Green))),
                subtypes: &["Forest"],
                basic: true,
            },
            legendary: false,
            snow: false,
            uncounterable: false,
            enchant: None,
            enchant_graveyard: false,
            modal: false,
            modal_choose: 0,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            keywords: empty_slice(),
            conditional_keywords: empty_slice(),
            abilities: empty_slice(),
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
            subtypes: empty_slice(),
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

    #[test]
    fn invalidate_on_land_played() {
        let mut game = Game::with_players(2, 0);
        let bear = game.spawn_on_battlefield(PlayerId(0), creature(2, 2));
        assert_eq!(game.power(bear), 2);
        let from = game.spawn_in_hand(PlayerId(0), forest());
        let permanent = game.next_object_id();
        game.apply(&Event::LandPlayed {
            player: PlayerId(0),
            from,
            permanent,
            tapped: false,
        });
        assert!(
            game.characteristics_cache
                .read(|cache| cache.power(bear).is_none()),
            "LandPlayed through apply should drop the owner's cached power"
        );
    }

    #[test]
    fn invalidate_on_token_created() {
        let mut game = Game::with_players(2, 0);
        let bear = game.spawn_on_battlefield(PlayerId(0), creature(2, 2));
        assert_eq!(game.power(bear), 2);
        let token = game.next_object_id();
        game.apply(&Event::TokenCreated {
            token,
            controller: PlayerId(0),
            def: intern_card_def(creature(1, 1)),
            creator: bear,
        });
        assert!(
            game.characteristics_cache
                .read(|cache| cache.power(bear).is_none()),
            "TokenCreated through apply should drop the controller's cached power"
        );
    }

    #[test]
    fn invalidate_on_combat_cleared() {
        let mut game = Game::with_players(2, 0);
        let bear = game.spawn_on_battlefield(PlayerId(0), creature(2, 2));
        assert_eq!(game.power(bear), 2);
        game.apply(&Event::CombatCleared);
        assert!(
            game.characteristics_cache
                .read(|cache| cache.power(bear).is_none()),
            "CombatCleared through apply should drop battlefield caches"
        );
    }
}

#[cfg(test)]
mod characteristic_query_tests {
    use super::*;

    const P0: PlayerId = PlayerId(0);
    const P1: PlayerId = PlayerId(1);

    const FREE: Cost = Cost {
        generic: 0,
        colored: [0; Color::COUNT],
        colorless: 0,
        x: 0,
        hybrid: &[],
        phyrexian: &[],
        additional: AdditionalCost {
            discard: 0,
            discard_land: false,
            reveal_creature_from_hand: false,
            pay_life_x: false,
            pay_life: 0,
            sacrifice: None,
            kicker: None,
            buyback: None,
            strive: None,
            replicate: None,
            multikicker: None,
        },
        reduce_own_generic: None,
    };

    fn creature_with(keywords: &'static [Keyword]) -> CardDef {
        CardDef {
            name: "Test Creature",
            id: "",
            default_print: "",
            cost: FREE,
            kind: CardKind::Creature {
                power: 2,
                toughness: 2,
                also: TypeSet::NONE,
            },
            legendary: false,
            snow: false,
            uncounterable: false,
            enchant: None,
            enchant_graveyard: false,
            modal: false,
            modal_choose: 0,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            keywords: keywords.into(),
            conditional_keywords: empty_slice(),
            abilities: empty_slice(),
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
            subtypes: empty_slice(),
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

    fn land(produces: LandProduces) -> CardDef {
        CardDef {
            name: "Land",
            id: "",
            default_print: "",
            cost: Cost::FREE,
            kind: CardKind::Land {
                produces: Some(produces),
                subtypes: &[],
                basic: false,
            },
            legendary: false,
            snow: false,
            uncounterable: false,
            enchant: None,
            enchant_graveyard: false,
            modal: false,
            modal_choose: 1,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            keywords: empty_slice(),
            conditional_keywords: empty_slice(),
            abilities: empty_slice(),
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
            subtypes: empty_slice(),
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

    #[test]
    fn ward_amount_reads_parametric_keyword() {
        static KEYWORDS: &[Keyword] = &[Keyword::Ward(2)];
        let mut game = Game::with_players(2, 0);
        let warded = game.spawn_on_battlefield(P0, creature_with(KEYWORDS));
        let plain = game.spawn_on_battlefield(P0, creature_with(&[]));
        assert_eq!(game.ward_amount(warded), Some(2));
        assert_eq!(game.ward_amount(plain), None);
    }

    #[test]
    fn has_haste_reads_from_keywords() {
        static KEYWORDS: &[Keyword] = &[Keyword::Haste];
        let mut game = Game::with_players(2, 0);
        let hastey = game.spawn_on_battlefield(P0, creature_with(KEYWORDS));
        let plain = game.spawn_on_battlefield(P0, creature_with(&[]));
        assert!(game.has_haste(hastey));
        assert!(!game.has_haste(plain));
    }

    #[test]
    fn colors_of_reads_colored_cost_pips() {
        let mut game = Game::with_players(2, 0);
        let black = game.spawn_on_battlefield(
            P0,
            CardDef {
                name: "Black",
                id: "",
                default_print: "",
                cost: Cost {
                    colored: {
                        let mut pips = [0; Color::COUNT];
                        pips[Color::Black.index()] = 1;
                        pips
                    },
                    ..FREE
                },
                kind: CardKind::Creature {
                    power: 1,
                    toughness: 1,
                    also: TypeSet::NONE,
                },
                legendary: false,
                snow: false,
                uncounterable: false,
                enchant: None,
                enchant_graveyard: false,
                modal: false,
                modal_choose: 1,
                modal_choose_max: None,
                modal_choose_max_if_commander: false,
                keywords: empty_slice(),
                conditional_keywords: empty_slice(),
                abilities: empty_slice(),
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
                subtypes: empty_slice(),
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
            },
        );
        let colorless = game.spawn_on_battlefield(P0, creature_with(&[]));
        assert!(game.colors_of(black)[Color::Black.index()]);
        assert!(!game.colors_of(colorless)[Color::Black.index()]);
    }

    #[test]
    fn protection_prevents_damage_from_matching_source_colors() {
        static KEYWORDS: &[Keyword] = &[Keyword::ProtectionFrom(ProtectionScope::Color(
            Color::Black,
        ))];
        let mut game = Game::with_players(2, 0);
        let knight = game.spawn_on_battlefield(P0, creature_with(KEYWORDS));
        let black_source = game.spawn_on_battlefield(
            P1,
            CardDef {
                name: "Black Creature",
                id: "",
                default_print: "",
                cost: Cost {
                    colored: {
                        let mut pips = [0; Color::COUNT];
                        pips[Color::Black.index()] = 1;
                        pips
                    },
                    ..FREE
                },
                kind: CardKind::Creature {
                    power: 1,
                    toughness: 1,
                    also: TypeSet::NONE,
                },
                legendary: false,
                snow: false,
                uncounterable: false,
                enchant: None,
                enchant_graveyard: false,
                modal: false,
                modal_choose: 1,
                modal_choose_max: None,
                modal_choose_max_if_commander: false,
                keywords: empty_slice(),
                conditional_keywords: empty_slice(),
                abilities: empty_slice(),
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
                subtypes: empty_slice(),
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
            },
        );
        assert!(game.damage_prevented_by_protection(knight, Some(black_source)));
        assert!(!game.damage_prevented_by_protection(knight, None));
    }

    #[test]
    fn commander_identity_credit_is_a_single_color_for_monocolored_commanders() {
        let mut game = Game::with_players(2, 0);
        // A non-commander permanent owned by P0 must not steal identity lookup
        // (catches `is_commander && owner` → `||`).
        game.spawn_on_battlefield(P0, creature_with(&[]));
        game.designate_commander(
            P0,
            CardDef {
                name: "Mono-G",
                id: "",
                default_print: "",
                cost: Cost {
                    colored: {
                        let mut pips = [0; Color::COUNT];
                        pips[Color::Green.index()] = 3;
                        pips
                    },
                    ..FREE
                },
                kind: CardKind::Creature {
                    power: 3,
                    toughness: 3,
                    also: TypeSet::NONE,
                },
                legendary: true,
                snow: false,
                uncounterable: false,
                enchant: None,
                enchant_graveyard: false,
                modal: false,
                modal_choose: 1,
                modal_choose_max: None,
                modal_choose_max_if_commander: false,
                keywords: empty_slice(),
                conditional_keywords: empty_slice(),
                abilities: empty_slice(),
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
                subtypes: empty_slice(),
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
            },
        );
        assert_eq!(
            game.commander_identity_credit(P0),
            Some(Mana::Color(Color::Green))
        );
        assert_eq!(game.commander_identity_of(P0), {
            let mut identity = [false; Color::COUNT];
            identity[Color::Green.index()] = true;
            identity
        });
    }

    #[test]
    fn opponent_producible_colors_credit_sees_opponent_lands() {
        let mut game = Game::with_players(2, 0);
        game.spawn_on_battlefield(P0, land(LandProduces::Mana(Mana::Color(Color::Green))));
        assert_eq!(
            game.opponent_producible_colors_credit(P1),
            Some(Mana::Color(Color::Green))
        );
    }

    #[test]
    fn opponent_producible_colors_credit_unions_multiple_colors() {
        let mut game = Game::with_players(2, 0);
        game.spawn_on_battlefield(P0, land(LandProduces::Mana(Mana::Color(Color::Green))));
        game.spawn_on_battlefield(P0, land(LandProduces::Mana(Mana::Color(Color::Blue))));
        match game.opponent_producible_colors_credit(P1) {
            Some(Mana::OfColors(mask)) => {
                assert!(mask & (1 << Color::Green.index()) != 0);
                assert!(mask & (1 << Color::Blue.index()) != 0);
            }
            other => panic!("expected a restricted two-color credit, got {other:?}"),
        }
    }

    #[test]
    fn opponent_producible_colors_credit_reports_any_for_five_colors() {
        let mut game = Game::with_players(2, 0);
        game.spawn_on_battlefield(P0, land(LandProduces::Mana(Mana::Any)));
        assert_eq!(game.opponent_producible_colors_credit(P1), Some(Mana::Any));
    }
}
