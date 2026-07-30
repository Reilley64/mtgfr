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
        // Only the until-EOT boosts are ledgered: they're the ones whose minting event carries a
        // source name. The other registered modifiers (a lace's color set, a self-animation)
        // would show up as an unattributed row.
        for modifier in self.modifiers_on(object) {
            let ModifierKind::Boost {
                power,
                toughness,
                keywords,
            } = modifier.kind
            else {
                continue;
            };
            if power != 0 || toughness != 0 {
                push(
                    modifier.source_name,
                    ModifierContribution::PowerToughness { power, toughness },
                );
            }
            for &keyword in keywords {
                push(modifier.source_name, ModifierContribution::Keyword(keyword));
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
            let aura_controller = self.controller_of(attachment);
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
                        Effect::Static(StaticEffect::SetAttachedBasePt {
                            power,
                            toughness,
                            noncreature_only,
                        }),
                    ) => {
                        if noncreature_only && self.host_is_printed_creature(object) {
                            continue;
                        }
                        push(
                            name,
                            ModifierContribution::SetBasePowerToughness {
                                power: self.resolve_amount(power, aura_controller, object, None, 0),
                                toughness: self.resolve_amount(
                                    toughness,
                                    aura_controller,
                                    object,
                                    None,
                                    0,
                                ),
                            },
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
                            untapped_only,
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
                    if untapped_only && self.as_permanent(object).is_some_and(|p| p.tapped) {
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

    /// Every instance of Rampage N `object` currently has, as its N (CR 702.23). Multiple
    /// instances trigger separately (CR 702.23c), so this yields one entry per instance rather
    /// than a sum.
    pub(crate) fn rampage_amounts(&self, object: ObjectId) -> impl Iterator<Item = u8> {
        self.effective_keywords(object)
            .into_iter()
            .filter_map(|keyword| match keyword {
                Keyword::Rampage(n) => Some(n),
                _ => None,
            })
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

    /// The colors of `object` — its source card's colored cost pips (CR 105.2), then every CR
    /// 613.3c layer-5 effect on it applied in timestamp order (CR 613.7). Used to test a
    /// spell/creature against a protected permanent (a "red" source has a red pip) and by
    /// color-scoped anthems.
    ///
    /// A layer-5 *SET* (Wild Mongrel's "becomes the color of your choice until end of turn",
    /// Deathlace's "becomes black") replaces every color established before it rather than
    /// unioning, so a green Mongrel that becomes black reads as black only. A layer-5 *ADD* (a
    /// manland's animated form) unions onto whatever the effects before it left — so an animation
    /// resolving after a lace keeps both the lace's color and its own.
    pub fn colors_of(&self, object: ObjectId) -> [bool; Color::COUNT] {
        // Fork's "except that the copy is red" — a spell has no duration to sweep and ceases to
        // exist as it resolves, so its set is a plain field rather than a registered modifier.
        if let Object::Spell(s) = &self.objects[object as usize]
            && let Some(color) = s.set_color
        {
            let mut colors = [false; Color::COUNT];
            colors[color.index()] = true;
            return colors;
        }
        let mut colors = color_identity(&self.def_of(object));
        // Kormus Bell's "All Swamps are 1/1 **black** creatures" — a layer-5 add like a manland's,
        // just scoped to a land type rather than to the permanent itself.
        // ponytail: folded in ahead of the registered effects rather than at its own timestamp,
        // so a lace resolving *before* the Bell entered still wipes the Bell's black. An add only
        // loses to a set, and no pool card laces a Swamp.
        for (_, _, effect) in self.land_type_statics_on(object) {
            let StaticEffect::AllLandsOfTypeBecome { add_colors, .. } = effect else {
                continue;
            };
            for color in add_colors {
                colors[color.index()] = true;
            }
        }
        for modifier in self.modifiers_on(object) {
            match modifier.kind {
                ModifierKind::SetColor(color) => {
                    colors = [false; Color::COUNT];
                    colors[color.index()] = true;
                }
                ModifierKind::Became { colors: added, .. } => {
                    for color in added {
                        colors[color.index()] = true;
                    }
                }
                _ => {}
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

    /// The CR 612.1 text change riding `object`, wherever it's stored: a permanent
    /// ([`Permanent::text_swap`]) or a spell still on the stack ([`Spell::text_swap`] — Magical
    /// Hack targets "spell or permanent"). `None` if `object` is neither, or was never changed.
    pub(crate) fn text_swap_of(&self, object: ObjectId) -> Option<TextSwap> {
        match &self.objects[object as usize] {
            Object::Permanent(p) => p.text_swap,
            Object::Spell(s) => s.text_swap,
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
        Self::mana_credit_for_colors(self.commander_identity_of(player))
    }

    /// The cheapest single-mana credit that covers exactly `present`: nothing for no color, a
    /// plain [`Mana::Color`] for one, [`Mana::Either`] for two, a [`Mana::OfColors`] restricted
    /// set for three or more.
    fn mana_credit_for_colors(present: [bool; Color::COUNT]) -> Option<Mana> {
        let mut colors = Color::ALL.iter().copied().filter(|c| present[c.index()]);
        match (colors.next(), colors.next(), colors.next()) {
            (None, ..) => None,
            (Some(c), None, _) => Some(Mana::Color(c)),
            (Some(a), Some(b), None) => Some(Mana::Either(a, b)),
            (Some(_), Some(_), Some(_)) => {
                let mut mask = 0u8;
                for c in Color::ALL {
                    if present[c.index()] {
                        mask |= 1 << c.index();
                    }
                }
                Some(Mana::OfColors(mask))
            }
        }
    }

    /// Which basic land types (CR 305.6) a type line carries, as the colors they tap for —
    /// [`BASIC_LAND_TYPES`] is in WUBRG order, so its index *is* the color's.
    fn basic_land_types(subtypes: &[&str]) -> [bool; Color::COUNT] {
        let mut colors = [false; Color::COUNT];
        for (color, basic) in colors.iter_mut().zip(BASIC_LAND_TYPES) {
            *color = subtypes.contains(basic);
        }
        colors
    }

    /// The one mana a land's free base tap produces for `player` — the printed `produces` sugar,
    /// resolved for the two credits that depend on the board (Command Tower, Exotic Orchard).
    ///
    /// A land whose basic land types have been *changed* (Evil Presence, CR 305.7) taps for those
    /// types instead: it lost its old ones and the mana ability they granted along with them, and
    /// gained the ones its new types grant. Every read of `produces` outside deserialization goes
    /// through here so that holds at the tap intent and in the auto-tap planner alike.
    pub(crate) fn land_mana_credit(&self, land: ObjectId, player: PlayerId) -> Option<Mana> {
        let CardKind::Land {
            produces, subtypes, ..
        } = self.def_of(land).kind
        else {
            return None;
        };
        // CR 708.2: a face-down land has no subtypes, which is an absence rather than a
        // type-changing effect. Its callers already refuse to tap it at all.
        if !self.is_face_down(land) {
            let effective = Self::basic_land_types(&self.effective_subtypes(land));
            if effective != Self::basic_land_types(subtypes) {
                return Self::mana_credit_for_colors(effective);
            }
        }
        match produces? {
            LandProduces::Mana(m) => Some(m),
            LandProduces::CommanderIdentity => self.commander_identity_credit(player),
            LandProduces::OpponentColors => self.opponent_producible_colors_credit(player),
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
        // CR 305.7: a land whose basic types were changed produces what those types produce, and
        // nothing its printed rules text used to say.
        if let CardKind::Land { subtypes, .. } = def.kind
            && !self.is_face_down(land)
        {
            let effective = Self::basic_land_types(&self.effective_subtypes(land));
            if effective != Self::basic_land_types(subtypes) {
                return effective;
            }
        }
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
                        set_chosen_land_type,
                        lose_all_abilities,
                    }),
                ) = (ability.timing, ability.effect.clone())
                else {
                    continue;
                };
                // Phantasmal Terrain names its one type as it enters instead of printing it.
                // `BASIC_LAND_TYPES` doubles as the static home for that single-element slice, so
                // the answer needs no allocation; an unanswered choice changes nothing at all.
                let set_subtypes = if set_chosen_land_type {
                    let chosen = self.as_permanent(id).and_then(|p| p.chosen_subtype);
                    let Some(i) =
                        chosen.and_then(|c| BASIC_LAND_TYPES.iter().position(|&t| t == c))
                    else {
                        continue;
                    };
                    &BASIC_LAND_TYPES[i..=i]
                } else {
                    set_subtypes
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
                        Effect::Static(StaticEffect::SetAttachedBasePt {
                            power,
                            toughness,
                            noncreature_only,
                        }),
                    ) => {
                        if noncreature_only && self.host_is_printed_creature(host) {
                            continue;
                        }
                        // Resolved against the *host*, not the Aura: Animate Artifact's "equal to
                        // its mana value" is the enchanted artifact's mana value. Every other
                        // attachment amount here (`GrantToAttached` above) reads the Aura, because
                        // "gets +1/+1 for each Forest you control" is the Aura's own count.
                        effects.push(ContinuousEffect {
                            source: id,
                            timestamp,
                            kind: ContinuousEffectKind::BasePtSet {
                                power: self.resolve_amount(power, controller, host, None, 0),
                                toughness: self
                                    .resolve_amount(toughness, controller, host, None, 0),
                            },
                        });
                    }
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
        if let Some((power, toughness)) = p.set_base_pt {
            effects.push(ContinuousEffect {
                source: object,
                timestamp: p.set_base_pt_timestamp,
                kind: ContinuousEffectKind::BasePtSet { power, toughness },
            });
        }
        // "Until this creature leaves the battlefield" (Gaea's Liege): the duration is a live
        // lookup, not a scheduled cleanup — the moment `source` is no longer a permanent this
        // entry stops being produced and the land's printed line is back.
        if let Some((subtypes, source, timestamp)) = p.subtypes_set_while_source_remains
            && self.as_permanent(source).is_some()
        {
            effects.push(ContinuousEffect {
                source,
                timestamp,
                kind: ContinuousEffectKind::SetTypes {
                    add_types: TypeSet::NONE,
                    set_types: false,
                    set_subtypes: Some(subtypes),
                    add_subtypes: &[],
                },
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
        // Clockwork Beast's +1/+0 counters (CR 121.1): power only, so this can't fold into the
        // scalar `plus_counters` above — it is its own layer-7d delta, like the -1/-1 read.
        let plus_one_plus_zero = p.kind_counters[CounterKind::PlusOnePlusZero as usize] as i32;
        if plus_one_plus_zero != 0 {
            effects.push(ContinuousEffect {
                source: object,
                timestamp: self.static_continuous_timestamp(object),
                kind: ContinuousEffectKind::PtDelta {
                    power: plus_one_plus_zero,
                    toughness: 0,
                },
            });
        }
        // Spirit Shackle's -0/-2 counters (CR 121.1): toughness only, the mirror of the +1/+0 read
        // above. A separate kind from -1/-1, so both can sit here and both apply.
        let minus_zero_minus_two = p.kind_counters[CounterKind::MinusZeroMinusTwo as usize] as i32;
        if minus_zero_minus_two != 0 {
            effects.push(ContinuousEffect {
                source: object,
                timestamp: self.static_continuous_timestamp(object),
                kind: ContinuousEffectKind::PtDelta {
                    power: 0,
                    toughness: -2 * minus_zero_minus_two,
                },
            });
        }
        // Every registered continuous modification of this object, one layer entry each at the
        // timestamp it was stamped with — rather than one pre-summed aggregate per layer. Layer
        // 7c and the keyword layer are additive, so N entries and one summed entry agree, and
        // nothing has to union keyword slices into a leaked `&'static` to fit a single field.
        // The color kinds are absent because color isn't a layer this pipeline models — see
        // [`Game::colors_of`], which folds them itself.
        for modifier in self.modifiers_on(object) {
            let timestamp = modifier.timestamp;
            match modifier.kind {
                ModifierKind::Boost {
                    power,
                    toughness,
                    keywords,
                } => {
                    if power != 0 || toughness != 0 {
                        effects.push(ContinuousEffect {
                            source: object,
                            timestamp,
                            kind: ContinuousEffectKind::PtDelta { power, toughness },
                        });
                    }
                    if !keywords.is_empty() {
                        effects.push(ContinuousEffect {
                            source: object,
                            timestamp,
                            kind: ContinuousEffectKind::GrantKeywords { keywords },
                        });
                    }
                }
                ModifierKind::BasePtSet { power, toughness } => effects.push(ContinuousEffect {
                    source: object,
                    timestamp,
                    kind: ContinuousEffectKind::BasePtSet { power, toughness },
                }),
                ModifierKind::Became {
                    types, subtypes, ..
                } => {
                    if types != TypeSet::NONE || !subtypes.is_empty() {
                        effects.push(ContinuousEffect {
                            source: object,
                            timestamp,
                            kind: ContinuousEffectKind::SetTypes {
                                add_types: types,
                                set_types: false,
                                set_subtypes: None,
                                add_subtypes: subtypes,
                            },
                        });
                    }
                }
                ModifierKind::LoseKeywords(_)
                | ModifierKind::SetColor(_)
                | ModifierKind::RevertsToDef(_) => {}
            }
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
                        power,
                        toughness,
                        condition,
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
                // An "as long as …" board gate (Ivory Guardians' "as long as an opponent controls a
                // nontoken red permanent") — read against the source's own controller, the same way
                // `matching_anthems` reads `Anthem`'s. The candidate-side half of a gate rides
                // `filter` instead, so this is only ever the global half.
                let source_controller = self.controller_of(source);
                if let Some(cond) = condition
                    && !self.ability_condition_holds(
                        cond,
                        source,
                        TriggerContext::of(source_controller),
                    )
                {
                    continue;
                }
                let power = self.resolve_amount(power, source_controller, source, None, 0);
                let toughness = self.resolve_amount(toughness, source_controller, source, None, 0);
                if power != 0 || toughness != 0 {
                    effects.push(ContinuousEffect {
                        source,
                        timestamp,
                        kind: ContinuousEffectKind::PtDelta { power, toughness },
                    });
                }
                if keywords.is_empty() {
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

    /// Whether a live Aura attached to `host` waives the [`Keyword::Defender`] attack
    /// restriction — Animate Wall's "Enchanted Wall can attack as though it didn't have defender."
    /// The host keeps the keyword; only `Game::can_attack`'s check for it is skipped, so this is
    /// "as though", not a keyword loss. The mirror image of [`Self::host_cant_attack`], read off
    /// the same attachment scan, so it ends the instant the Aura leaves.
    pub(crate) fn host_may_attack_ignoring_defender(&self, host: ObjectId) -> bool {
        self.attachments(host).into_iter().any(|id| {
            !self.is_phased_out(id)
                && self.def_of(id).abilities.iter().any(|a| {
                    matches!(
                        (a.timing, a.effect.clone()),
                        (
                            Timing::Static,
                            Effect::Static(StaticEffect::GrantToAttached {
                                may_attack_ignoring_defender: true,
                                ..
                            })
                        )
                    )
                })
        })
    }

    /// Whether a live Aura attached to `host` waives the summoning-sickness attack restriction
    /// (CR 302.6) — Instill Energy's "Enchanted creature can attack as though it had haste."
    /// Read only by `Game::can_attack`, so the host stays sick for everything else: its `{T}`
    /// abilities are as locked as they were, which is the difference between this and haste.
    pub(crate) fn host_may_attack_ignoring_summoning_sickness(&self, host: ObjectId) -> bool {
        self.attachments(host).into_iter().any(|id| {
            !self.is_phased_out(id)
                && self.def_of(id).abilities.iter().any(|a| {
                    matches!(
                        (a.timing, a.effect.clone()),
                        (
                            Timing::Static,
                            Effect::Static(StaticEffect::GrantToAttached {
                                may_attack_ignoring_summoning_sickness: true,
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

    /// The "can't be blocked by \[filter\]" restrictions live attached Auras grant `host`
    /// (Invisibility), the granted twin of the printed
    /// [`Effect::Static(StaticEffect::CantBeBlockedBy)`] and read alongside it by
    /// [`Game::can_block`]. Several Auras may each contribute one, so this yields rather than
    /// answering yes/no the way its `cant_block` sibling above does.
    pub(crate) fn host_cant_be_blocked_by(&self, host: ObjectId) -> Vec<PermanentFilter> {
        self.attachments(host)
            .into_iter()
            .filter(|&id| !self.is_phased_out(id))
            .flat_map(|id| {
                self.def_of(id)
                    .abilities
                    .iter()
                    .filter_map(|a| match (a.timing, a.effect.clone()) {
                        (
                            Timing::Static,
                            Effect::Static(StaticEffect::GrantToAttached {
                                cant_be_blocked_by,
                                ..
                            }),
                        ) => cant_be_blocked_by,
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Whether a live attached Aura makes `host` a creature that "all creatures able to block …
    /// do so" (CR 509.1c — Lure). The requirement twin of [`Self::host_cant_be_blocked_by`]: that
    /// one says who may not block, this one says who must.
    pub(crate) fn host_must_be_blocked_by_all(&self, host: ObjectId) -> bool {
        self.attachments(host).into_iter().any(|id| {
            !self.is_phased_out(id)
                && self.def_of(id).abilities.iter().any(|a| {
                    matches!(
                        (a.timing, a.effect.clone()),
                        (
                            Timing::Static,
                            Effect::Static(StaticEffect::GrantToAttached {
                                must_be_blocked_by_all: true,
                                ..
                            })
                        )
                    )
                })
        })
    }

    /// Whether any permanent on the battlefield holds `id` down with a live
    /// [`Effect::Static(StaticEffect::DoesntUntap)`] (CR 502.2 — Mana Vault's own
    /// `self_only` printing, Meekstone's power-3 filter over the whole table). Consulted by the
    /// untap step's turn-based action and nowhere else: an untap *effect* ({3}: Untap this
    /// artifact) ignores this entirely, which is exactly how those cards get free again.
    /// Battlefield-wide like `Game::cant_block_filter`, not controller-scoped — Meekstone reads
    /// "their controllers' untap steps", so who controls the Meekstone never enters into it.
    pub(crate) fn doesnt_untap(&self, id: ObjectId) -> bool {
        // Paralyze's attachment-scoped form. Folded in here rather than given its own scanner so
        // the untap step keeps reading exactly one, and so "doesn't untap" means the same thing
        // whether the source is an Aura on the permanent or a Meekstone across the table.
        if self.attachments(id).into_iter().any(|aura| {
            !self.is_phased_out(aura)
                && self.def_of(aura).abilities.iter().any(|a| {
                    matches!(
                        (a.timing, a.effect.clone()),
                        (
                            Timing::Static,
                            Effect::Static(StaticEffect::GrantToAttached {
                                doesnt_untap: true,
                                ..
                            })
                        )
                    )
                })
        }) {
            return true;
        }
        self.battlefield().into_iter().any(|source| {
            self.functional_abilities(source)
                .iter()
                .any(|a| match (a.timing, a.effect.clone()) {
                    (
                        Timing::Static,
                        Effect::Static(StaticEffect::DoesntUntap { self_only, filter }),
                    ) => {
                        if self_only {
                            return source == id;
                        }
                        self.permanent_matches(
                            &filter,
                            id,
                            self.controller_of(source),
                            Some(source),
                        )
                    }
                    _ => false,
                })
        })
    }

    /// Every live cap on how many permanents of a class may untap in one untap step (CR 502.2 —
    /// Smoke's "no more than one creature", Winter Orb's "no more than one land"), as the
    /// (source, filter) pairs the untap step matches its candidates against.
    ///
    /// Battlefield-wide and unscoped like [`Game::players_skip_untap_steps`] — both cards say
    /// "players", so who controls them never enters into it. Unlike `doesnt_untap` this one honors
    /// the ability's own `condition`, because Winter Orb's cap is gated on the Orb being untapped;
    /// the read happens once as the untap step starts, so an Orb that untaps in that same step
    /// (having been tapped down in response) never gets to stop the lands untapping beside it.
    pub(crate) fn untap_at_most_one_filters(&self) -> Vec<(ObjectId, PermanentFilter)> {
        let mut caps = Vec::new();
        for source in self.battlefield() {
            for ability in self.functional_abilities(source).iter() {
                let Effect::Static(StaticEffect::UntapAtMostOne { filter }) = &ability.effect
                else {
                    continue;
                };
                if ability.timing != Timing::Static {
                    continue;
                }
                if !ability.condition.is_none_or(|condition| {
                    self.ability_condition_holds(
                        condition,
                        source,
                        TriggerContext::of(self.controller_of(source)),
                    )
                }) {
                    continue;
                }
                caps.push((source, *filter));
            }
        }
        caps
    }

    /// Whether anything on the battlefield is telling the table to skip its untap steps
    /// (CR 703.4a — Stasis's "Players skip their untap steps"). Battlefield-wide and unscoped:
    /// "players" is everyone, the Stasis controller included, so who controls it never enters
    /// into it.
    ///
    /// Consulted by the untap step's two real turn-based actions — phasing in (CR 502.1) and
    /// untapping (CR 502.2) — and by nothing else. Losing summoning sickness and freeing a
    /// planeswalker's loyalty are per-*turn* durations that this engine merely bookkeeps in the
    /// same arm; they are not untap-step actions, so they survive the skip and a creature cast
    /// under Stasis can still attack on its controller's next turn.
    pub(crate) fn players_skip_untap_steps(&self) -> bool {
        self.battlefield().into_iter().any(|source| {
            self.functional_abilities(source).iter().any(|a| {
                matches!(
                    (a.timing, &a.effect),
                    (
                        Timing::Static,
                        Effect::Static(StaticEffect::PlayersSkipUntapSteps)
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

    /// Whether `host` is a creature for a `noncreature_only` attachment gate (Animate Artifact's
    /// "as long as enchanted artifact isn't a creature").
    ///
    /// ponytail: reads *printed* types, not [`Game::effective_types`]. The gate has to ignore the
    /// gating Aura's own creature-adding layer, and asking for effective types from inside the
    /// attachment scan that feeds those layers would recurse. Printed types answer the case the
    /// pool has — enchanting a printed artifact creature — and miss an artifact animated by
    /// something else (a Karn, another Aura). The upgrade path is an `effective_types` variant that
    /// takes a source to exclude.
    fn host_is_printed_creature(&self, host: ObjectId) -> bool {
        self.def_of(host).kind.types().intersects(TypeSet::CREATURE)
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
        let printed = self.def_of(id).abilities.clone();
        // CR 612.1 layer 3: a text change rewrites the words this object's own printed abilities
        // name before every later read of them — see [`TextSwap::ability`] for how far that goes.
        let Some(swap) = self.text_swap_of(id) else {
            return printed;
        };
        printed
            .iter()
            .map(|ability| swap.ability(ability))
            .collect()
    }

    /// Whether `id` is a bestowed permanent (CR 702.103) currently attached to a host: while so it
    /// is an Aura enchantment and **not** a creature (CR 702.103e). An unattached bestowed
    /// permanent is a creature again (CR 702.103i), so this reads the live "attached?" gate, not the
    /// `bestowed` flag alone.
    pub(crate) fn is_bestowed_and_attached(&self, id: ObjectId) -> bool {
        self.as_permanent(id)
            .is_some_and(|p| p.bestowed && p.attached_to.is_some())
    }

    /// Every live [`StaticEffect::AllLandsOfTypeBecome`] on the battlefield ("All Mountains are
    /// Plains"), oldest source first. Whole-battlefield like [`Game::matching_anthems`] and for
    /// the same reason — these say "All Mountains", not "Mountains you control" — and recomputed
    /// per query rather than cached, the same as every other continuous-effect scan here.
    ///
    /// The order is CR 613.4's timestamp order, which is why the payloads come back as a list
    /// rather than a merged answer: callers apply them in turn, so a second conversion acts on
    /// what the first one left behind. Each entry keeps its `(timestamp, source)` for the one
    /// caller that has to hand them on — a base-P/T set is a layer-7b entry like any other.
    fn land_type_statics(&self) -> Vec<(u64, ObjectId, StaticEffect)> {
        let mut statics = Vec::new();
        for (index, object) in self.objects.iter().enumerate() {
            let source = index as ObjectId;
            if !matches!(object, Object::Permanent(_)) || self.is_phased_out(source) {
                continue;
            }
            for ability in self.functional_abilities(source).iter() {
                let (
                    Timing::Static,
                    Effect::Static(effect @ StaticEffect::AllLandsOfTypeBecome { .. }),
                ) = (ability.timing, &ability.effect)
                else {
                    continue;
                };
                statics.push((self.static_continuous_timestamp(source), source, *effect));
            }
        }
        statics.sort_by_key(|&(timestamp, source, _)| (timestamp, source));
        statics
    }

    /// The live "all lands of a type are …" statics that apply to `id` right now, oldest first —
    /// the handle for every characteristic these change *except* the subtype line itself, which
    /// is mid-computation when it asks and so does its own matching against the line it is
    /// building ([`Game::effective_subtypes`]).
    fn land_type_statics_on(&self, id: ObjectId) -> Vec<(u64, ObjectId, StaticEffect)> {
        // ponytail: a full battlefield sweep per characteristic read, so the empty-board case —
        // every game with none of these three cards in it — has to cost nothing. Upgrade path if
        // a board with one ever gets slow is a `Game`-level count maintained as permanents enter
        // and leave, which buys nothing until then.
        let statics = self.land_type_statics();
        if statics.is_empty() {
            return statics;
        }
        let subtypes = self.effective_subtypes(id);
        statics
            .into_iter()
            .filter(|(_, _, effect)| {
                let StaticEffect::AllLandsOfTypeBecome { land_types, .. } = effect else {
                    return false;
                };
                land_types.iter().any(|ty| subtypes.contains(ty))
            })
            .collect()
    }

    /// A battlefield permanent's card types after the CR 613.4 type layer: its printed types plus
    /// any added by an attached [`Effect::Static(StaticEffect::SetAttachedTypes)`] Aura (Darksteel Mutation → +Artifact),
    /// plus the card types a global land-type change grants every land of a type (Kormus Bell's
    /// "All Swamps are 1/1 black creatures that are still lands").
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
        // "All Swamps are 1/1 black creatures that are **still lands**" — always additive, so
        // there is no set-types global to order against the layer above.
        for (_, _, effect) in self.land_type_statics_on(id) {
            let StaticEffect::AllLandsOfTypeBecome { add_types, .. } = effect else {
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
        let def = self.def_of(id);
        // CR 305.6: a land's types are printed on its type line like any other subtype, but they
        // live under [`CardKind::Land`]'s own `subtypes` (see [`CardDef::subtypes`]). Union them in
        // here, the one read every subtype check routes through, so "Destroy all Plains"
        // (Flashfires) catches the basic and every dual that shares the type. A `set_subtypes`
        // layer below still replaces the whole line, land types included (CR 613.4).
        let printed = def.printed_subtypes();
        // CR 612.1 layer 3, and the reason a Hacked Swamp taps for {W}: `land_mana_credit`
        // re-derives a land's mana from its *effective* basic land types, so rewriting the word
        // here moves the mana with it.
        let printed = match self.text_swap_of(id) {
            Some(swap) => printed.into_iter().map(|s| swap.subtype(s)).collect(),
            None => printed,
        };
        if self.as_permanent(id).is_none() {
            return printed;
        }
        let (_, _, set, added) = self.attached_type_layer(id);
        let mut subtypes = match set {
            Some(set) => set.to_vec(),
            None => printed,
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
        // "All Mountains are Plains" (Conversion) matches on the line built so far rather than
        // through `land_type_statics_on`, which would ask this function for the answer it is
        // still computing. Matching here is also what makes CR 613.4 timestamp order mean
        // something: a second conversion sees the type the first one left.
        //
        // Nothing but a basic land type is ever matched, so a line without one — every creature,
        // every artifact, every spell — skips the battlefield sweep entirely.
        if subtypes.iter().any(|s| BASIC_LAND_TYPES.contains(s)) {
            for (_, _, effect) in self.land_type_statics() {
                let StaticEffect::AllLandsOfTypeBecome {
                    land_types,
                    set_subtypes,
                    ..
                } = effect
                else {
                    continue;
                };
                // CR 305.7: taking on a basic land type costs a land every land type it had, so
                // this replaces the line rather than extending it. An empty `set_subtypes` is a
                // global that changes something other than the type (Kormus Bell).
                if set_subtypes.is_empty() || !land_types.iter().any(|ty| subtypes.contains(ty)) {
                    continue;
                }
                subtypes = set_subtypes.to_vec();
            }
        }
        // "That land is a Swamp for as long as it has a mire counter on it" (Cyclopean Tomb,
        // CR 305.7 — a type *set*, so the land keeps nothing else). Read off the counter rather
        // than off a continuous effect on purpose: the Tomb can be long gone and the land is a
        // Swamp still, so the counter is the only thing left holding the effect.
        // ponytail: applied last, after any global conversion, instead of by CR 613.4 timestamp —
        // the counter carries no timestamp to sort by. Give `Permanent` a mire timestamp if a
        // board ever has both a mired land and a Conversion on it.
        if self.counters_of_kind(id, CounterKind::Mire) > 0 {
            subtypes = vec!["Swamp"];
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
    /// (Restless Spire, a creature only via a registered `Became`) has no printed P/T, so its base is
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
            } => Some(self.defined_base_pt(object).unwrap_or((power, toughness))),
            _ => Some((0, 0)),
        }
    }

    /// The CR 604.3 layer-7a base P/T `object`'s own printed characteristic-defining ability
    /// declares (Nightmare's "power and toughness are each equal to the number of Swamps you
    /// control"), or `None` when it has no such ability. Both counts are resolved against today's
    /// board on every recompute, so the creature tracks the count live; because this feeds
    /// [`Game::pt_base`] rather than [`Game::pt_layers`], everything else — a base-set Aura, a
    /// counter, an anthem — still applies on top in its own layer.
    fn defined_base_pt(&self, object: ObjectId) -> Option<(i32, i32)> {
        let controller = self.controller_of(object);
        self.def_of(object).abilities.iter().find_map(|ability| {
            let Effect::Static(StaticEffect::BasePowerToughnessFromAmount {
                power,
                toughness,
                when,
            }) = &ability.effect
            else {
                return None;
            };
            // A creature printing two of these (Gaea's Liege) keeps only the one whose combat
            // state holds right now; the `find_map` then stops at it.
            let attacking = self.combat.attackers.contains(&object);
            match when {
                DefiningPtWhen::Always => {}
                DefiningPtWhen::Attacking if !attacking => return None,
                DefiningPtWhen::NotAttacking if attacking => return None,
                _ => {}
            }
            Some((
                self.resolve_amount(*power, controller, object, None, 0),
                self.resolve_amount(*toughness, controller, object, None, 0),
            ))
        })
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
        // Kormus Bell's "All Swamps are 1/1 …" — a layer-7b base set like any other, timestamped
        // to the Bell so a later base set on the land itself still wins.
        for (timestamp, source, effect) in self.land_type_statics_on(object) {
            let StaticEffect::AllLandsOfTypeBecome {
                add_types,
                base_power,
                base_toughness,
                ..
            } = effect
            else {
                continue;
            };
            // "All Mountains are Plains" sets no P/T; only the globals that make a land a
            // creature print one.
            if !add_types.intersects(TypeSet::CREATURE) {
                continue;
            }
            effects.push(ContinuousEffect {
                source,
                timestamp,
                kind: ContinuousEffectKind::BasePtSet {
                    power: base_power,
                    toughness: base_toughness,
                },
            });
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
            // A static keyword grant (CR 604.3), not a triggered ability's intervening-if, so
            // there is no triggering event to describe — but the source-object-based conditions
            // this axis uses (Primordial Hydra's ten-counter trample, Agent Frank Horrigan's
            // attacked-this-turn indestructible) read `TriggerContext::source`, so a context
            // naming this object and its controller is all the general evaluator needs.
            let holds = self.condition_holds(
                condition,
                TriggerContext {
                    source: Some(object),
                    ..TriggerContext::of(self.controller_of(object))
                },
            );
            if holds {
                keywords.push(keyword);
            }
        }
        // CR 612.1 layer 3: "you may change 'swampwalk' to 'plainswalk'" is Magical Hack's own
        // reminder text, and protection's colour is what Sleight of Mind is played for. Only this
        // object's *printed* keywords move — a keyword granted below is the granting object's
        // text, not this one's.
        if let Some(swap) = self.text_swap_of(object) {
            for keyword in &mut keywords {
                *keyword = swap.keyword(*keyword);
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
        for modifier in self.modifiers_on(object) {
            let ModifierKind::LoseKeywords(lost) = modifier.kind else {
                continue;
            };
            keywords.retain(|k| !lost.contains(k));
        }
        // "Enchanted creature loses flying" (Earthbind), an ability the Aura gained rather than one
        // printed on it: stripped last for the same reason as the losses above — it beats every
        // grant regardless of source — but indefinitely, and only for as long as the Aura is still
        // attached here, since the loss lives on the Aura.
        // ponytail: no CR 613 timestamp ordering against the grants it undoes; no pool card wants a
        // grant applied later to win.
        for aura in self.attachments(object) {
            let Some(lost) = self.as_permanent(aura).map(|p| p.attachment_lost_keywords) else {
                continue;
            };
            keywords.retain(|k| !lost.contains(k));
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
                        untapped_only,
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
                if untapped_only && self.as_permanent(candidate).is_some_and(|p| p.tapped) {
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
                    && !self.ability_condition_holds(cond, source, TriggerContext::of(source_owner))
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

    /// Rock Hydra's per-point shield (CR 615): "for each 1 damage that would be dealt to this
    /// creature, if it has a +1/+1 counter on it, remove a +1/+1 counter from it and prevent that
    /// 1 damage." Unlike the two whole-event shields below it covers only as many points as the
    /// Hydra has counters — the returned `i32` is the rest of the hit, which is dealt for real.
    pub(crate) fn per_point_counter_shield(
        &self,
        target: ObjectId,
        amount: i32,
    ) -> (Vec<Event>, i32) {
        if amount <= 0 || !self.replacement_registry().phantom_shield_per_point(target) {
            return (Vec::new(), amount);
        }
        let paid = amount.min(self.plus_counters(target).max(0));
        if paid <= 0 {
            return (Vec::new(), amount);
        }
        (
            vec![Event::CountersPlaced {
                object: target,
                count: -paid,
                source_name: self.def_of(target).name,
            }],
            amount - paid,
        )
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

    /// Every *activated* (non-mana) ability granted to `host`, as `(cost, effects)` — the
    /// non-mana twin of [`Game::granted_mana_abilities`]. Two grant kinds land here: an
    /// [`Effect::Static(StaticEffect::GrantToAttached)`] on an Aura attached to `host` (Fallen
    /// Ideal's "Sacrifice a creature: This creature gets +2/+1 until end of turn.") and an
    /// [`Effect::Static(StaticEffect::GrantActivatedAbility)`] anywhere on the battlefield whose
    /// filter `host` matches (Zombie Master's "Other Zombies have '{B}: Regenerate this
    /// permanent.'"). Attachment grants come first so an existing grant keeps its index when a
    /// lord arrives. Recomputed live off the board — a grant disappears the instant its source
    /// leaves. Read by [`Game::ability_at`], which addresses these past `host`'s own abilities
    /// and its granted mana abilities.
    pub(crate) fn granted_activated_abilities(
        &self,
        host: ObjectId,
    ) -> Vec<(ActivationCost, &'static [Effect])> {
        let mut grants: Vec<(ActivationCost, &'static [Effect])> = self
            .attachments(host)
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
            .collect();
        for (source, object) in self.objects.iter().enumerate() {
            let Object::Permanent(p) = object else {
                continue;
            };
            let source = source as ObjectId;
            for ability in self.functional_abilities(source).iter().cloned() {
                let (
                    Timing::Static,
                    Effect::Static(StaticEffect::GrantActivatedAbility {
                        filter,
                        granted_ability: Some(g),
                    }),
                ) = (ability.timing, ability.effect.clone())
                else {
                    continue;
                };
                // `you` is the *granting* permanent's controller, so a `controller = "you"` filter
                // reads off the lord (as its printed text does), not off the candidate.
                if !self.permanent_matches(&filter, host, p.owner, Some(source)) {
                    continue;
                }
                grants.push((g.cost, g.effects));
            }
        }
        grants
    }

    /// Every *triggered* ability granted to `host` by a live
    /// [`Effect::Static(StaticEffect::GrantToAttached)`] Aura/Equipment attached to it (Power
    /// Fist's "Whenever this creature deals combat damage to a player, put that many +1/+1
    /// counters on it."), synthesized directly as an [`Ability`] — unlike the activated twin
    /// ([`Game::granted_activated_abilities`]), there is no `ability_at` index to address, since
    /// a triggered ability isn't activated. Recomputed live off the same attachment scan, so it
    /// disappears the instant the Aura/Equipment leaves (CR 702.26e for a phased-out one).
    /// Read by [`Game::queue_trigger_group`], the shared choke most trigger flavors route
    /// through, and separately by the combat-damage-to-a-player scanner, which is bespoke and
    /// doesn't route through it.
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
                                optional: g.optional,
                                min_level: 0,
                                // Only the mana half of the grant's `cost` is a *triggered*
                                // ability's cost (Farmstead's "you may pay {W}{W}") — an
                                // `Ability::cost` is a `Cost`, and the rest of an
                                // `ActivationCost` (tapping, sacrificing) has no meaning for
                                // something that was never activated.
                                cost: g.cost.mana,
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
    /// [`Effect::Static(StaticEffect::GrantToAttached)`] on an Aura attached to it or by a
    /// filter-scoped [`Effect::Static(StaticEffect::GrantActivatedAbility)`]
    /// ([`Game::granted_activated_abilities`]). Each grant block occupies contiguous indices
    /// immediately past the prior. The one seam [`Game::ability_activation_gate`] and
    /// [`Game::legal_targets`] read so every granted ability activates exactly like an own one.
    /// `None` for an out-of-range index.
    pub fn ability_at(&self, object: ObjectId, index: usize) -> Option<Ability> {
        let def = self.def_of(object);
        if let Some(ability) = def.abilities.get(index) {
            // CR 612.1 layer 3: activating reads this object's text, not its card's — a Circle of
            // Protection: Red that Sleight of Mind rewrote arms against the colour it says now.
            let Some(swap) = self.text_swap_of(object) else {
                return Some(ability.clone());
            };
            return Some(swap.ability(ability));
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
            .granted_activated_abilities(object)
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

    /// Whether any permanent `player` controls carries `wanted` as a live static ability — the
    /// shared scan behind the two fieldless player-scoped statics Lich prints.
    fn controls_static(&self, player: PlayerId, wanted: StaticEffect) -> bool {
        self.battlefield().into_iter().any(|id| {
            let Some(permanent) = self.as_permanent(id) else {
                return false;
            };
            permanent.owner == player
                && card_def(permanent.def).abilities.iter().any(|a| {
                    (a.timing, a.effect.clone()) == (Timing::Static, Effect::Static(wanted))
                })
        })
    }

    /// Whether `player` survives 0 or less life (Lich's "You don't lose the game for having 0 or
    /// less life") — the CR 704.5a exemption, read live by the state-based sweep, so the turn the
    /// Lich leaves the battlefield is the turn its controller's negative life total catches up
    /// with them.
    pub(crate) fn ignores_zero_life(&self, player: PlayerId) -> bool {
        self.controls_static(player, StaticEffect::YouDontLoseAtZeroLife)
    }

    /// Whether life `player` would gain becomes that many cards instead (Lich's "If you would
    /// gain life, draw that many cards instead") — a CR 614 replacement read at the single
    /// life-gain funnel, so lifelink, drains and plain "you gain N" all route through it.
    pub(crate) fn life_gain_becomes_draw(&self, player: PlayerId) -> bool {
        self.controls_static(player, StaticEffect::LifeGainBecomesDraw)
    }

    /// The permanent standing between `player` and an unblocked attacker (Veteran Bodyguard,
    /// CR 615.10): the first untapped permanent they control with a live
    /// [`Effect::Static(StaticEffect::RedirectUnblockedDamageToSelf)`]. Both halves are read here
    /// and not cached, because the untapped condition is a live one — a bodyguard tapped after
    /// blockers were declared protects nothing by the time damage is dealt.
    pub(crate) fn unblocked_damage_bodyguard(&self, player: PlayerId) -> Option<ObjectId> {
        self.battlefield().into_iter().find(|&id| {
            let Some(permanent) = self.as_permanent(id) else {
                return false;
            };
            permanent.owner == player
                && !self.is_tapped(id)
                && card_def(permanent.def).abilities.iter().any(|a| {
                    (a.timing, a.effect.clone())
                        == (
                            Timing::Static,
                            Effect::Static(StaticEffect::RedirectUnblockedDamageToSelf),
                        )
                })
        })
    }

    /// Whether an *effect* discard by `player` lands on top of their library instead of in their
    /// graveyard (CR 701.8c — Library of Leng): true if any permanent they control has a live
    /// [`Effect::Static(StaticEffect::DiscardToLibraryTopInstead)`] static. Read by
    /// [`Game::discard_ids`], the shared tail every effect discard routes through.
    /// ponytail: the printed "you *may*" is taken as always-yes — the engine has no pause it can
    ///   raise mid-discard, since `discard_ids` returns into six callers that keep working. Take
    ///   the choice when a resumable discard path exists (see the 2ed increments backlog).
    pub(crate) fn discards_to_library_top(&self, player: PlayerId) -> bool {
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
                            Effect::Static(StaticEffect::DiscardToLibraryTopInstead),
                        )
                })
        })
    }

    /// Whether `player` may still play a land this turn (CR 305.2): one per turn, unless a
    /// permanent they control lifts the cap with a live
    /// [`Effect::Static(StaticEffect::PlayAnyNumberOfLands)`] (Fastbond). The single gate both the
    /// legality check in [`Game::play_land`] and the playability hint in `Game::land_actions`
    /// route through, so an offered land play is always a legal one.
    pub(crate) fn land_drop_available(&self, player: PlayerId) -> bool {
        if self.players[player.0 as usize].lands_played < 1 {
            return true;
        }
        self.objects.iter().enumerate().any(|(id, object)| {
            let Object::Permanent(p) = object else {
                return false;
            };
            // Controller, not owner: a stolen Fastbond gives its permission to the thief.
            self.controller_of(id as ObjectId) == player
                && card_def(p.def).abilities.iter().any(|a| {
                    (a.timing, a.effect.clone())
                        == (
                            Timing::Static,
                            Effect::Static(StaticEffect::PlayAnyNumberOfLands),
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

    /// Total generic mana a spell `player` is casting (`def`, aimed at `target`) owes to the
    /// [`Effect::Static(StaticEffect::TaxSpellCost)`] statics on the battlefield (CR 601.2f —
    /// Gloom's "White spells cost {3} more to cast"). Unlike [`Game::cost_reduction`] above this
    /// scan is *not* scoped to `player`: a taxer reaches every seat, which is the whole reason the
    /// tax is its own variant rather than a negative reduction.
    pub(crate) fn cost_increase(
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
            for ability in card_def(p.def).abilities.iter().cloned() {
                let (Timing::Static, Effect::Static(StaticEffect::TaxSpellCost { amount, filter })) =
                    (ability.timing, ability.effect.clone())
                else {
                    continue;
                };
                if ability.min_level > p.level {
                    continue;
                }
                if !self.spell_matches_filter(filter, def.clone(), target, player, from_zone) {
                    continue;
                }
                // The taxer's own controller resolves the amount — it is their ability, even
                // though the spell being taxed is somebody else's.
                let resolved = self.resolve_amount(
                    amount,
                    self.controller_of(id as ObjectId),
                    id as ObjectId,
                    None,
                    0,
                );
                total = total.saturating_add(resolved.max(0) as u8);
            }
        }
        total
    }

    /// Total generic mana an activated ability of `source` owes to the
    /// [`Effect::Static(StaticEffect::TaxActivatedAbility)`] statics on the battlefield (CR 602.2b
    /// — Gloom's "Activated abilities of white enchantments cost {3} more to activate"). The
    /// activation-choke twin of [`Game::cost_increase`], and table-wide for the same reason;
    /// `filter` is matched against `source` itself.
    pub(crate) fn activation_tax(&self, source: ObjectId) -> u8 {
        let mut total: u8 = 0;
        for (id, object) in self.objects.iter().enumerate() {
            let Object::Permanent(p) = object else {
                continue;
            };
            for ability in card_def(p.def).abilities.iter().cloned() {
                let (
                    Timing::Static,
                    Effect::Static(StaticEffect::TaxActivatedAbility { amount, filter }),
                ) = (ability.timing, ability.effect.clone())
                else {
                    continue;
                };
                if ability.min_level > p.level {
                    continue;
                }
                let taxer = self.controller_of(id as ObjectId);
                if !self.permanent_matches(&filter, source, taxer, Some(id as ObjectId)) {
                    continue;
                }
                let resolved = self.resolve_amount(amount, taxer, id as ObjectId, None, 0);
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
            SpellFilter::Instant => matches!(
                def.kind,
                CardKind::Spell {
                    speed: SpellSpeed::Instant
                }
            ),
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
            // Avoid Fate / Ring of Immortals's "targets a permanent you control" is matched
            // inline in `legal_targets_for`'s `SpellOnStack` enumeration, the only choke that
            // holds the filtering spell's own controller separately from `caster` here (which is
            // the *candidate* spell's controller for this call's other callers). Never true here:
            // no trigger or cost-reducer filters on this shape.
            SpellFilter::InstantOrAuraTargetsPermanentYouControl => false,
            // Invoke Prejudice's "doesn't share a color with a creature you control" is matched
            // inline in `queue_cast_spell_triggers`, the only choke that holds the *watching
            // permanent's* controller separately from `caster` here (which is the casting player).
            // Never true here: no cost-reducer filters on this shape.
            SpellFilter::CreatureNotSharingColorWithCreatureYouControl => false,
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
        x_color: None,
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
            world: false,
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
            cast_only_before_blockers: false,
            cast_only_during_opponents_turn: false,
            cast_only_before_combat_damage: false,
            cast_only_during_declare_blockers: false,
            cast_only_during_declare_attackers: false,
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
                untapped_only: false,
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
            world: false,
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
            cast_only_before_blockers: false,
            cast_only_during_opponents_turn: false,
            cast_only_before_combat_damage: false,
            cast_only_during_declare_blockers: false,
            cast_only_during_declare_attackers: false,
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
                set_color: None,
                text_swap: None,
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
                sacrificed_mana_value: 0,
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
            world: false,
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
            cast_only_before_blockers: false,
            cast_only_during_opponents_turn: false,
            cast_only_before_combat_damage: false,
            cast_only_during_declare_blockers: false,
            cast_only_during_declare_attackers: false,
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
        x_color: None,
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
            world: false,
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
            cast_only_before_blockers: false,
            cast_only_during_opponents_turn: false,
            cast_only_before_combat_damage: false,
            cast_only_during_declare_blockers: false,
            cast_only_during_declare_attackers: false,
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
            world: false,
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
            cast_only_before_blockers: false,
            cast_only_during_opponents_turn: false,
            cast_only_before_combat_damage: false,
            cast_only_during_declare_blockers: false,
            cast_only_during_declare_attackers: false,
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
                world: false,
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
                cast_only_before_blockers: false,
                cast_only_during_opponents_turn: false,
                cast_only_before_combat_damage: false,
                cast_only_during_declare_blockers: false,
                cast_only_during_declare_attackers: false,
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
                world: false,
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
                cast_only_before_blockers: false,
                cast_only_during_opponents_turn: false,
                cast_only_before_combat_damage: false,
                cast_only_during_declare_blockers: false,
                cast_only_during_declare_attackers: false,
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
                world: false,
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
                cast_only_before_blockers: false,
                cast_only_during_opponents_turn: false,
                cast_only_before_combat_damage: false,
                cast_only_during_declare_blockers: false,
                cast_only_during_declare_attackers: false,
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
