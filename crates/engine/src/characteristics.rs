//! Effective power, toughness, keywords, and targeting legality.
//!
//! Characteristic queries used across combat, SBAs, and cast gates.
//! Also: CR 614 slices (counter replacements, enters-tapped). P/T is a CR 613-ordered layered
//! recompute (`pt_layers`/`apply_pt_layers` — 7b base-set, 7c modifications); keywords/other
//! characteristics stay additive per ADR 0003. Deferred / gaps: see `docs/FIDELITY_BACKLOG.md`.

use crate::*;

/// One CR 613 continuous-effect entry contributing to a creature's power/toughness, built fresh
/// per recompute in [`Game::pt_layers`] and applied in order by [`Game::apply_pt_layers`].
/// Engine-internal — NOT a `CardDef`/TOML surface and never stored (a runtime `Vec`, so `CardDef`
/// stays `Copy`). `source`/`timestamp` are forward-compat: they only break application ties today
/// (see [`Game::apply_pt_layers`]) — real CR 613.7 dependency ordering + per-effect timestamps
/// arrive with the slice that needs them (Quandrix Charm / stacked base-sets — none in the pool).
struct PtLayer {
    source: ObjectId,
    timestamp: u64,
    kind: PtLayerKind,
}

enum PtLayerKind {
    /// CR 613.3(7b): the creature's base P/T is set (today's `SetAttachedBasePT` Aura).
    BasePtSet { power: i32, toughness: i32 },
    /// CR 613.3(7c): a P/T modification added on top of the base (counters, until-EOT boosts,
    /// anthems, `grant_to_attached`).
    PtDelta { power: i32, toughness: i32 },
}

impl Game {
    /// Every live sourced modifier on a battlefield permanent, grouped by source card def name
    /// for the Alt-inspect ledger. Empty when `object` is not a battlefield permanent.
    /// Continuous effects are re-derived from the board; timed/stateful batches come from
    /// [`Game::modifier_provenance`]. Additive attribution only — not CR 613 layers (ADR 0003).
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
        for &(host, _, source_name) in &self.play_permissions.control_overrides {
            if host == object {
                push(source_name, ModifierContribution::Controls);
            }
        }

        for attachment in self.attachments(object) {
            let name = self.def_of(attachment).name;
            for ability in self.def_of(attachment).abilities {
                match (ability.timing, ability.effect) {
                    (
                        Timing::Static,
                        Effect::GrantToAttached {
                            power,
                            toughness,
                            keywords,
                            goad,
                            ..
                        },
                    ) => {
                        if let (Amount::Fixed(power), Amount::Fixed(toughness)) = (power, toughness)
                        {
                            if power != 0 || toughness != 0 {
                                push(
                                    name,
                                    ModifierContribution::PowerToughness { power, toughness },
                                );
                            }
                        }
                        for &keyword in keywords {
                            push(name, ModifierContribution::Keyword(keyword));
                        }
                        if goad {
                            push(name, ModifierContribution::Goaded);
                        }
                    }
                    (Timing::Static, Effect::SetAttachedBasePT { power, toughness }) => {
                        push(
                            name,
                            ModifierContribution::SetBasePowerToughness { power, toughness },
                        );
                    }
                    (Timing::Static, Effect::ControlAttached) => {
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
                let p = match self.as_permanent(id) {
                    Some(p) if p.owner == owner => p,
                    _ => continue,
                };
                for ability in p.def.abilities {
                    let (
                        Timing::Static,
                        Effect::AnthemStatic {
                            power,
                            toughness,
                            keywords,
                            subtypes,
                            colors,
                            exclude_source,
                            attacking_only,
                            ..
                        },
                    ) = (ability.timing, ability.effect)
                    else {
                        continue;
                    };
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
                    let name = p.def.name;
                    if let (Amount::Fixed(power), Amount::Fixed(toughness)) = (power, toughness) {
                        if power != 0 || toughness != 0 {
                            push(
                                name,
                                ModifierContribution::PowerToughness { power, toughness },
                            );
                        }
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
                for ability in p.def.abilities {
                    let (Timing::Static, Effect::GrantManaAbility { filter, .. }) =
                        (ability.timing, ability.effect)
                    else {
                        continue;
                    };
                    if self.permanent_matches(&filter, object, owner, None) {
                        push(p.def.name, ModifierContribution::ManaAbility);
                    }
                }
            }
        }

        groups
    }

    /// Whether a permanent is tapped.
    pub fn is_tapped(&self, object: ObjectId) -> bool {
        self.as_permanent(object).is_some_and(|p| p.tapped)
    }

    /// Whether a permanent is summoning sick (entered this turn, no haste yet).
    pub fn is_summoning_sick(&self, object: ObjectId) -> bool {
        self.as_permanent(object).is_some_and(|p| p.summoning_sick)
    }

    /// Whether a permanent has haste (so it may attack / tap the turn it enters).
    pub fn has_haste(&self, object: ObjectId) -> bool {
        self.has_keyword(object, Keyword::Haste)
    }

    /// Whether `object` currently has `keyword`: its base keywords ∪ keywords granted by
    /// Auras/Equipment attached to it ∪ any until-end-of-turn keyword grant ∪ a matching
    /// static anthem's keyword grant (ADR 0003 — effective keywords are a computed union).
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
    pub fn colors_of(&self, object: ObjectId) -> [bool; Color::COUNT] {
        let mut colors = color_identity(self.def_of(object));
        if let Some(p) = self.as_permanent(object) {
            for color in p.added_colors_eot {
                colors[color.index()] = true;
            }
        }
        colors
    }

    /// `player`'s commander color identity (CR 903.4) — the [`color_identity`] of their
    /// commander card, wherever it currently is (command zone or battlefield). All-`false` if
    /// `player` has no object flagged as a commander (a bare test setup with no designated
    /// commander) — CR 903.4 identity mana wouldn't apply without one.
    pub(crate) fn commander_identity_of(&self, player: PlayerId) -> [bool; Color::COUNT] {
        self.live_object_ids()
            .into_iter()
            .find(|&id| self.is_commander(id) && self.owner_of(id) == player)
            .map_or([false; Color::COUNT], |id| color_identity(self.def_of(id)))
    }

    /// The mana credit "one mana of any color in your commander's color identity" (CR 903.4 —
    /// Command Tower, Arcane Signet) resolves to for `player`: their single identity color, an
    /// [`Mana::Either`] credit for a two-color identity (exact for every soc commander, all
    /// two-color), or `None` for a colorless identity (no commander designated, or a colorless
    /// commander — CR 106.6 has no mana of no color).
    /// ponytail: a 3+-color identity has no restricted-credit shape yet (only `Either` exists
    /// beyond a single color); it falls back to [`Mana::Any`]. No soc-pool commander is 3+
    /// colors — upgrade to a color-set credit only if one becomes a fidelity target.
    pub(crate) fn commander_identity_credit(&self, player: PlayerId) -> Option<Mana> {
        let identity = self.commander_identity_of(player);
        let mut colors = Color::ALL.iter().copied().filter(|c| identity[c.index()]);
        match (colors.next(), colors.next(), colors.next()) {
            (None, ..) => None,
            (Some(c), None, _) => Some(Mana::Color(c)),
            (Some(a), Some(b), None) => Some(Mana::Either(a, b)),
            (Some(_), Some(_), Some(_)) => Some(Mana::Any),
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
        for ability in def.abilities {
            let Effect::AddMana {
                mana: produced,
                identity,
                ..
            } = ability.effect
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

    /// Each [`Effect::GrantToAttached`] granted to `host` by a permanent attached to it,
    /// as `(power, toughness, keywords)`. Drives the additive P/T and keyword recompute.
    /// `power`/`toughness` are an [`Amount`], resolved live off the attached permanent as the
    /// effect's controller/source (Sage's Reverie's "+1/+1 for each Aura you control that's
    /// attached to a creature" — a board-derived grant, mirroring how
    /// [`Game::anthem_pt_bonus`] resolves [`Effect::AnthemStatic`]'s amounts).
    pub(crate) fn attachment_grants(
        &self,
        host: ObjectId,
    ) -> impl Iterator<Item = (i32, i32, &'static [Keyword])> + '_ {
        self.attachments(host)
            .into_iter()
            // A phased-out Aura/Equipment grants nothing (CR 702.26e — treated as though it
            // doesn't exist); `attachments` is unfiltered so the phase-in cascade can still find it.
            .filter(move |&id| !self.is_phased_out(id))
            .flat_map(move |id| {
                let controller = self.controller_of(id);
                self.def_of(id)
                    .abilities
                    .iter()
                    .filter_map(move |a| match (a.timing, a.effect) {
                        (
                            Timing::Static,
                            Effect::GrantToAttached {
                                power,
                                toughness,
                                keywords,
                                ..
                            },
                        ) => Some((
                            self.resolve_amount(power, controller, id, None, 0),
                            self.resolve_amount(toughness, controller, id, None, 0),
                            keywords,
                        )),
                        _ => None,
                    })
            })
    }

    /// [`Keyword::ProtectionFrom`] the chosen color of each attached
    /// `protection_from_chosen_color` [`Effect::GrantToAttached`] Aura confers on `host`
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
                        (a.timing, a.effect),
                        (
                            Timing::Static,
                            Effect::GrantToAttached {
                                protection_from_chosen_color: true,
                                ..
                            }
                        )
                    )
                })
            })
            .filter_map(|id| self.as_permanent(id).and_then(|p| p.chosen_color))
            .map(|color| Keyword::ProtectionFrom(ProtectionScope::Color(color)))
            .collect()
    }

    /// The controllers of every live Aura attached to `host` whose static
    /// [`Effect::GrantToAttached`] carries `goad = true` (CR 701.38a — the Impetus cycle,
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
                    (a.timing, a.effect),
                    (Timing::Static, Effect::GrantToAttached { goad: true, .. })
                )
            });
            goads_host.then(|| self.controller_of(id))
        })
    }

    /// The `(power, toughness)` a [`Effect::SetAttachedBasePT`] Aura forces onto `host`'s base,
    /// if any is attached — the CR 613.3(7b) base-P/T-set entry [`Game::pt_layers`] emits, applied
    /// before the 7c counters/pumps/anthems/grants.
    /// ponytail: takes the first such grant; the pool never stacks two on one creature.
    pub(crate) fn set_base_pt(&self, host: ObjectId) -> Option<(i32, i32)> {
        self.attachments(host).into_iter().find_map(|id| {
            self.def_of(id)
                .abilities
                .iter()
                .find_map(|a| match (a.timing, a.effect) {
                    (Timing::Static, Effect::SetAttachedBasePT { power, toughness }) => {
                        Some((power, toughness))
                    }
                    _ => None,
                })
        })
    }

    /// Whether `blocker` may block a creature that has flying (it flies or has reach).
    pub(crate) fn can_block_flyers(&self, blocker: ObjectId) -> bool {
        self.has_keyword(blocker, Keyword::Flying) || self.has_keyword(blocker, Keyword::Reach)
    }

    /// The CR 613.4 type/subtype layer a [`Effect::SetAttachedTypes`] Aura forces onto `host`:
    /// `(added_types, set_subtypes, added_subtypes)` — the card types unioned on, the creature
    /// subtypes that *replace* the host's own (when present), and the creature subtypes unioned on.
    /// Empty (`TypeSet::NONE`, `None`, `&[]`) when no such Aura is attached.
    /// ponytail: takes the first grant per axis; the pool never stacks two type-changing Auras on
    /// one creature, so CR 613.7 dependency/timestamp ordering is deferred to the slice needing it.
    fn attached_type_layer(
        &self,
        host: ObjectId,
    ) -> (
        TypeSet,
        Option<&'static [&'static str]>,
        &'static [&'static str],
    ) {
        let mut added_types = TypeSet::NONE;
        let mut set_subtypes: Option<&'static [&'static str]> = None;
        let mut added_subtypes: &'static [&'static str] = &[];
        for id in self.attachments(host) {
            for ability in self.def_of(id).abilities {
                let (
                    Timing::Static,
                    Effect::SetAttachedTypes {
                        add_types,
                        add_subtypes,
                        set_subtypes: set,
                        ..
                    },
                ) = (ability.timing, ability.effect)
                else {
                    continue;
                };
                added_types = added_types.union(add_types);
                if !set.is_empty() {
                    set_subtypes = Some(set);
                }
                if !add_subtypes.is_empty() {
                    added_subtypes = add_subtypes;
                }
            }
        }
        (added_types, set_subtypes, added_subtypes)
    }

    /// Whether an attached [`Effect::SetAttachedTypes`] Aura with `lose_all_abilities = true`
    /// (Darksteel Mutation's "it loses all other abilities") is stripping `host`'s own printed
    /// abilities and keywords (CR 613.1e/701). Only a battlefield permanent can be a host.
    /// ponytail: ≤1 ability-removing Aura per host in the pool, so no CR 613.7 timestamp/dependency
    /// ordering between competing removals — grow it from a card that stacks two.
    pub(crate) fn host_loses_all_abilities(&self, host: ObjectId) -> bool {
        if self.as_permanent(host).is_none() {
            return false;
        }
        for id in self.attachments(host) {
            for ability in self.def_of(id).abilities {
                if let (
                    Timing::Static,
                    Effect::SetAttachedTypes {
                        lose_all_abilities: true,
                        ..
                    },
                ) = (ability.timing, ability.effect)
                {
                    return true;
                }
            }
        }
        false
    }

    /// The abilities that *function* on `id` — its printed abilities, unless an attached Aura is
    /// stripping them ([`Game::host_loses_all_abilities`], CR 613.1e/701 "loses all abilities"), in
    /// which case none of the host's own abilities function. The single choke every
    /// battlefield-permanent ability iteration (trigger placement, activation gate, static scans)
    /// reads so the removal applies uniformly. Grants the Aura layers onto the host (its
    /// `grant_to_attached` keywords, its type/base-P/T sets) are separate and unaffected.
    pub(crate) fn functional_abilities(&self, id: ObjectId) -> &'static [Ability] {
        // CR 708.2: a face-down permanent (a manifest) has no abilities.
        if self.is_face_down(id) {
            return &[];
        }
        if self.host_loses_all_abilities(id) {
            return &[];
        }
        self.def_of(id).abilities
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
    /// any added by an attached [`Effect::SetAttachedTypes`] Aura (Darksteel Mutation → +Artifact).
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
        let Some(p) = self.as_permanent(id) else {
            return printed;
        };
        // Type-layer sources: an attached Aura (Darksteel Mutation), an until-EOT self-animation
        // (Restless Spire → Creature), and an indefinite reanimation set (Excava → Creature). No
        // pool card stacks two on one permanent, so their order is unobservable (CR 613.7 deferred
        // — see `pt_layers`).
        printed
            .union(self.attached_type_layer(id).0)
            .union(p.added_types_eot)
            .union(p.added_types)
    }

    /// A battlefield permanent's creature subtypes after the CR 613.4 subtype layer: its printed
    /// subtypes with an attached [`Effect::SetAttachedTypes`] Aura's `add_subtypes` unioned on, or
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
        let (_, set, added) = self.attached_type_layer(id);
        let mut subtypes = match set {
            Some(set) => set.to_vec(),
            None => printed.to_vec(),
        };
        subtypes.extend_from_slice(added);
        // A self-animation (Restless Spire → "Elemental") and an indefinite reanimation set
        // (Excava → "Spirit") add subtypes on top of the printed/Aura set — same union axis as the
        // Aura's `add_subtypes`.
        if let Some(p) = self.as_permanent(id) {
            subtypes.extend_from_slice(p.added_subtypes_eot);
            subtypes.extend_from_slice(p.added_subtypes);
        }
        subtypes
    }

    /// Whether a creature is barred from attacking / using tap abilities this turn:
    /// summoning sick and without haste. Summoning sickness's `{T}` restriction is creature-only
    /// (CR 302.6) — an artifact/land (a Treasure, a fetchland) may tap the turn it enters.
    pub(crate) fn is_sick_without_haste(&self, object: ObjectId) -> bool {
        matches!(self.def_of(object).kind, CardKind::Creature { .. })
            && self.is_summoning_sick(object)
            && !self.has_keyword(object, Keyword::Haste)
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
        match p.def.kind {
            CardKind::Creature {
                power, toughness, ..
            } => Some((power, toughness)),
            _ => Some((0, 0)),
        }
    }

    /// Every CR 613 P/T layer entry currently affecting `object` — the enchanted-base-set Aura
    /// (7b) plus the 7c modifications (counters, until-EOT boosts, anthems, `grant_to_attached`).
    /// A re-expression of the additive contributors, not a re-derivation: it reuses the same scans
    /// ([`Game::set_base_pt`], [`Game::anthem_pt_bonus`], [`Game::attachment_grants`]).
    /// ponytail: every entry's `source`/`timestamp` is the host `object` as a stand-in. This can
    /// now push TWO `BasePtSet` layers on one host (a `SetAttachedBasePT` Aura + an until-EOT set),
    /// but with ≤1 base-set *observed at once* in the pool (no card combines them) and commutative
    /// 7c deltas, application is order-independent, so any deterministic timestamp is exact. Real
    /// per-effect timestamps + CR 613.7 dependency ordering land with the slice that stacks two.
    fn pt_layers(&self, object: ObjectId) -> Vec<PtLayer> {
        let mut layers = Vec::new();
        let stamp = |kind| PtLayer {
            source: object,
            timestamp: object as u64,
            kind,
        };
        if let Some((power, toughness)) = self.set_base_pt(object) {
            layers.push(stamp(PtLayerKind::BasePtSet { power, toughness }));
        }
        if let Some((power, toughness)) = self.as_permanent(object).and_then(|p| p.base_pt_set_eot)
        {
            layers.push(stamp(PtLayerKind::BasePtSet { power, toughness }));
        }
        // An indefinite reanimation set (Excava → base 1/1), the same 7b base-set as above but not
        // cleared at cleanup (CR 611.2c).
        if let Some((power, toughness)) = self.as_permanent(object).and_then(|p| p.set_base_pt) {
            layers.push(stamp(PtLayerKind::BasePtSet { power, toughness }));
        }
        if let Some(p) = self.as_permanent(object) {
            layers.push(stamp(PtLayerKind::PtDelta {
                power: p.plus_counters,
                toughness: p.plus_counters,
            }));
            layers.push(stamp(PtLayerKind::PtDelta {
                power: p.temp_power,
                toughness: p.temp_toughness,
            }));
        }
        let (anthem_power, anthem_toughness) = self.anthem_pt_bonus(object);
        layers.push(stamp(PtLayerKind::PtDelta {
            power: anthem_power,
            toughness: anthem_toughness,
        }));
        for (power, toughness, _keywords) in self.attachment_grants(object) {
            layers.push(stamp(PtLayerKind::PtDelta { power, toughness }));
        }
        layers
    }

    /// Apply CR 613-ordered P/T `layers` to a creature's `printed` base, returning its effective
    /// `(power, toughness)`: every 7b `BasePtSet` replaces the running base first, then every 7c
    /// `PtDelta` sums on top. `timestamp`/`source` only break ties (deterministic ordering); with
    /// ≤1 base-set and commutative deltas the result equals the old additive recompute exactly.
    fn apply_pt_layers(
        printed_power: i32,
        printed_toughness: i32,
        mut layers: Vec<PtLayer>,
    ) -> (i32, i32) {
        layers.sort_by_key(|l| {
            (
                matches!(l.kind, PtLayerKind::PtDelta { .. }),
                l.timestamp,
                l.source,
            )
        });
        let mut power = printed_power;
        let mut toughness = printed_toughness;
        for layer in layers {
            match layer.kind {
                PtLayerKind::BasePtSet {
                    power: base_power,
                    toughness: base_toughness,
                } => {
                    power = base_power;
                    toughness = base_toughness;
                }
                PtLayerKind::PtDelta {
                    power: delta_power,
                    toughness: delta_toughness,
                } => {
                    power += delta_power;
                    toughness += delta_toughness;
                }
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
        for (condition, keyword) in self.def_of(object).conditional_keywords {
            if removes_abilities {
                break;
            }
            if let Condition::SourceHasCounters { at_least } = condition
                && self.source_has_counters(object, *at_least)
            {
                keywords.push(*keyword);
            }
        }
        if let Some(p) = self.as_permanent(object) {
            keywords.extend_from_slice(p.temp_keywords);
            // Indefinite reanimation grant (Excava → flying), the same union axis as `temp_keywords`
            // but not cleared at cleanup (CR 611.2c).
            keywords.extend_from_slice(p.granted_keywords);
        }
        for (_, _, granted) in self.attachment_grants(object) {
            keywords.extend_from_slice(granted);
        }
        // Backup / "it gains the following abilities until end of turn" (CR 702.166): a granted
        // source's keyword abilities (Guardian Scalelord's flying) ride the target until cleanup.
        // ponytail: reads the source's *printed* keywords, not its own granted-onto-it keywords —
        // no pool card chains grants, so this needs no recursion. (CR 603.10 / last-known info if
        // the source has since left: the link persists on `abilities_granted_until_eot`.)
        for &(target, source) in &self.abilities_granted_until_eot {
            if target == object {
                keywords.extend_from_slice(self.def_of(source).keywords);
            }
        }
        keywords.extend(self.chosen_color_protection_grants(object));
        keywords.extend(self.anthem_keywords(object));
        // "Lose ... and can't have" (CR 702.11e/702.18d — arcane_lighthouse): strip these off
        // the fully-unioned set last, so a keyword granted by any source above — including one
        // applied *after* the strip landed this turn — is filtered right back out.
        if let Some(p) = self.as_permanent(object) {
            keywords.retain(|k| !p.temp_lost_keywords.contains(k));
        }
        keywords
    }

    /// Every static [`Effect::AnthemStatic`] that applies to `candidate`, paired with the
    /// [`ObjectId`] of the permanent carrying it (its source — needed to resolve a dynamic
    /// `power`/`toughness` [`Amount`] and to honor `self_only`): on a permanent `candidate`'s
    /// owner also owns, matching its `subtype`/`attacking_only`/`self_only` filter (`None`/
    /// `false` matches everything, same as the old untyped anthem). The shared scan behind
    /// [`Game::anthem_pt_bonus`] and [`Game::anthem_keywords`] — a filtered anthem has to be
    /// tested per candidate creature, unlike the old controller-wide flat bonus.
    fn matching_anthems(&self, candidate: ObjectId) -> Vec<(ObjectId, Effect)> {
        let Some(candidate_permanent) = self.as_permanent(candidate) else {
            return Vec::new();
        };
        let owner = candidate_permanent.owner;
        let mut matches = Vec::new();
        // Battlefield anthems (`from_graveyard == false`) on permanents the owner controls,
        // plus graveyard anthems (`from_graveyard == true`) on the owner's graveyard cards that
        // function there (CR 603.6e continuous-analog — Anger's "as long as this card is in your
        // graveyard … creatures you control have haste"). The `bool` tags which zone each
        // source is in so the two anthem kinds never leak across (a graveyard-only anthem's
        // battlefield copy grants nothing, and vice versa).
        let battlefield_sources = self
            .objects
            .iter()
            .enumerate()
            .filter(|(_, object)| matches!(object, Object::Permanent(p) if p.owner == owner))
            .map(|(index, _)| (index as ObjectId, false));
        let graveyard_sources = self
            .graveyard_cards(owner)
            .into_iter()
            .filter(|&id| self.def_of(id).functions_in_graveyard)
            .map(|id| (id, true));
        for (source, source_in_graveyard) in battlefield_sources.chain(graveyard_sources) {
            for ability in self.functional_abilities(source) {
                let (
                    Timing::Static,
                    effect @ Effect::AnthemStatic {
                        subtypes,
                        colors,
                        chosen_subtype,
                        attacking_only,
                        commander_only,
                        self_only,
                        exclude_source,
                        tokens_only,
                        has_counters,
                        condition,
                        from_graveyard,
                        ..
                    },
                ) = (ability.timing, ability.effect)
                else {
                    continue;
                };
                if from_graveyard != source_in_graveyard {
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
                if commander_only && !self.is_commander(candidate) {
                    continue;
                }
                if has_counters && !self.has_any_counter(candidate) {
                    continue;
                }
                // An "as long as …" gate (tendershoot_dryad's city's blessing) — evaluated
                // against the anthem source's own controller, same as its cost/trigger reads
                // would be.
                if let Some(cond) = condition
                    && !self.condition_holds(cond, TriggerContext::of(owner))
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
        let target_controller = self.controller_of(target);
        for source in self.battlefield() {
            if source == target {
                continue;
            }
            if self.controller_of(source) != target_controller {
                continue;
            }
            let prevents = self.functional_abilities(source).iter().any(|ability| {
                ability.timing == Timing::Static
                    && matches!(
                        ability.effect,
                        Effect::PreventNoncombatDamageToOtherCreaturesYouControl
                    )
            });
            if prevents {
                return true;
            }
        }
        false
    }

    /// The total (power, toughness) bonus [`Game::matching_anthems`] grants to `candidate`.
    pub(crate) fn anthem_pt_bonus(&self, candidate: ObjectId) -> (i32, i32) {
        let owner = self.owner_of(candidate);
        self.matching_anthems(candidate)
            .into_iter()
            .fold((0, 0), |(pw, tf), (source, effect)| match effect {
                Effect::AnthemStatic {
                    power, toughness, ..
                } => (
                    pw + self.resolve_amount(power, owner, source, None, 0),
                    tf + self.resolve_amount(toughness, owner, source, None, 0),
                ),
                _ => (pw, tf),
            })
    }

    /// Every keyword [`Game::matching_anthems`] grants to `candidate` (Ohran Frostfang's
    /// deathtouch, CR 702.2).
    fn anthem_keywords(&self, candidate: ObjectId) -> Vec<Keyword> {
        self.matching_anthems(candidate)
            .into_iter()
            .flat_map(|(_, effect)| match effect {
                Effect::AnthemStatic { keywords, .. } => keywords.to_vec(),
                _ => Vec::new(),
            })
            .collect()
    }

    /// Every activated mana ability granted to `candidate` by a live static
    /// [`Effect::GrantManaAbility`] elsewhere on the battlefield (Goldspan Dragon's "Treasures
    /// you control have '{T}, Sacrifice this artifact: Add two mana of any one color.'"). Mirrors
    /// [`Game::matching_anthems`]'s owner-wide scan — recomputed live off the board, no stored
    /// state, so a grant disappears the instant its source leaves. Read by [`Game::ability_at`],
    /// which addresses these past `candidate`'s own abilities.
    pub(crate) fn granted_mana_abilities(
        &self,
        candidate: ObjectId,
    ) -> Vec<(ActivationCost, ManaPool)> {
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
            for ability in p.def.abilities {
                let (
                    Timing::Static,
                    Effect::GrantManaAbility {
                        filter,
                        cost,
                        mana,
                        restriction,
                    },
                ) = (ability.timing, ability.effect)
                else {
                    continue;
                };
                if self.permanent_matches(&filter, candidate, owner, None) {
                    // Wrapped here, once, so every reader of a granted batch (this ability's
                    // own resolution and the `available_mana` estimate) sees it already
                    // spend-restricted (Galazeth Prismari) — see `ManaPool::restricted_by`.
                    grants.push((cost, mana.restricted_by(restriction)));
                }
            }
        }
        grants
    }

    /// Every *activated* (non-mana) ability granted to `host` by a live
    /// [`Effect::GrantToAttached`] on an Aura attached to it (Fallen Ideal's "Sacrifice a
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
                self.def_of(id)
                    .abilities
                    .iter()
                    .filter_map(|a| match (a.timing, a.effect) {
                        (
                            Timing::Static,
                            Effect::GrantToAttached {
                                granted_ability: Some(g),
                                ..
                            },
                        ) => Some((g.cost, g.effects)),
                        _ => None,
                    })
            })
            .collect()
    }

    /// The ability at `index` on `object`, in a stable order: its own
    /// (`index < def.abilities.len()`), then those granted by a live static
    /// [`Effect::GrantManaAbility`] elsewhere on the battlefield
    /// ([`Game::granted_mana_abilities`]), then those granted by an
    /// [`Effect::GrantToAttached`] on an Aura attached to it
    /// ([`Game::granted_attachment_abilities`]). Each grant block occupies contiguous indices
    /// immediately past the prior. The one seam [`Game::ability_activation_gate`] and
    /// [`Game::legal_targets`] read so every granted ability activates exactly like an own one.
    /// `None` for an out-of-range index.
    pub fn ability_at(&self, object: ObjectId, index: usize) -> Option<Ability> {
        let def = self.def_of(object);
        if let Some(&ability) = def.abilities.get(index) {
            return Some(ability);
        }
        let granted_index = index - def.abilities.len();
        let mana_grants = self.granted_mana_abilities(object);
        if let Some(&(cost, mana)) = mana_grants.get(granted_index) {
            return Some(Ability {
                timing: Timing::Activated(cost),
                effect: Effect::AddMana {
                    // `mana` is already spend-restricted where applicable — `granted_mana_abilities`
                    // wraps it, so this virtual ability needs no `restriction` of its own.
                    mana,
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
            });
        }
        let (cost, effects) = self
            .granted_attachment_abilities(object)
            .into_iter()
            .nth(granted_index - mana_grants.len())?;
        // A one-effect grant is used directly; multiple run as a `Sequence` (the same shape a
        // multi-effect own ability uses).
        let effect = match effects {
            [single] => *single,
            steps => Effect::Sequence { steps },
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
    /// has a live [`Effect::NoMaximumHandSize`] static ability (e.g. Reliquary Tower). Read by the
    /// cleanup step's discard-to-hand-size turn-based action; a characteristic-defining continuous
    /// effect (CR 611), so no event is needed — it just stops applying when the source leaves.
    pub(crate) fn has_no_max_hand_size(&self, player: PlayerId) -> bool {
        self.objects.iter().any(|object| {
            let Object::Permanent(p) = object else {
                return false;
            };
            p.owner == player
                && p.def
                    .abilities
                    .iter()
                    .any(|a| (a.timing, a.effect) == (Timing::Static, Effect::NoMaximumHandSize))
        })
    }

    /// Whether `player` controls a permanent granting Serra Paragon's graveyard-play permission
    /// (CR 118.9 — a live [`Effect::PlayFromGraveyardOncePerTurn`] static ability). Read by
    /// [`Game::playable_zone`] to decide whether a land / permanent spell in `player`'s graveyard
    /// is playable this turn; the "once during each of your turns" cap is a separate gate
    /// ([`Player::graveyard_play_used_this_turn`]).
    pub(crate) fn grants_graveyard_recursion(&self, player: PlayerId) -> bool {
        self.objects.iter().any(|object| {
            let Object::Permanent(p) = object else {
                return false;
            };
            p.owner == player
                && p.def.abilities.iter().any(|a| {
                    (a.timing, a.effect) == (Timing::Static, Effect::PlayFromGraveyardOncePerTurn)
                })
        })
    }

    /// Total generic cost reduction `player`'s static [`Effect::ReduceSpellCost`] abilities grant
    /// to a spell they're casting (`def`, aimed at `target`): the sum of every matching reducer
    /// they control (CR 118.9 — reduces generic mana only, so the caller floors generic at 0).
    /// Pure recompute each cast — nothing is stored (ADR 0003, applied to cost).
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
            for ability in p.def.abilities {
                let (
                    Timing::Static,
                    Effect::ReduceSpellCost {
                        amount,
                        filter,
                        first_x_spell_each_turn,
                    },
                ) = (ability.timing, ability.effect)
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
                if !self.spell_matches_filter(filter, def, target, player, from_zone) {
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
                self.is_modified(id) && self.controller_of(id) == caster
            }
            // Advanced Reconstruction's level 3: "Spells you cast from anywhere other than your
            // hand …" — the only arm that reads the cast-from zone (CR 601).
            SpellFilter::CastFromNonHandZone => from_zone != Zone::Hand,
            // Balefire Liege's "cast a red spell" / "cast a white spell" — CR 105.1/202.2, the
            // spell's own colors (a multicolored spell matches every one of its colors).
            SpellFilter::Color(color) => color_identity(def)[color.index()],
        }
    }

    /// The number of +1/+1 counters actually placed when `base` would be put on `object`, after
    /// its controller's static replacement effects (CR 614 — Hardened Scales, a "twice that many"
    /// doubler). Each [`Effect::CounterReplacement`] that controller controls applies once.
    ///
    /// ponytail: fixed order — all additions, then all multipliers: `(base + Σadd) × Πtimes`.
    /// CR 616.1 lets the *affected player* order simultaneous replacements; every counter
    /// replacement in the pool is that player's own adder/doubler, and add-then-multiply maximizes
    /// the result — the choice they'd make — so a single order is documented rather than offered as
    /// a choice. Grow into a real ordering choice if a card ever makes another order preferable.
    pub(crate) fn counters_after_replacements(&self, object: ObjectId, base: i32) -> i32 {
        if base <= 0 {
            return base;
        }
        let controller = self.controller_of(object);
        let mut add = 0;
        let mut times = 1;
        for (id, obj) in self.objects.iter().enumerate() {
            let Object::Permanent(p) = obj else {
                continue;
            };
            if p.owner != controller {
                continue;
            }
            for ability in p.def.abilities {
                let (
                    Timing::Static,
                    Effect::CounterReplacement {
                        add: a,
                        times: t,
                        other,
                    },
                ) = (ability.timing, ability.effect)
                else {
                    continue;
                };
                // CR "another creature you control": a replacement that excludes its own
                // source doesn't apply when the permanent receiving the counters IS that
                // source (Benevolent Hydra doesn't double its own counters).
                if other && id as ObjectId == object {
                    continue;
                }
                add += a;
                times *= t;
            }
        }
        (base + add) * times
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
        let mut total = 0;
        for (id, obj) in self.objects.iter().enumerate() {
            let Object::Permanent(p) = obj else {
                continue;
            };
            let source = id as ObjectId;
            // A permanent's own ETB-modifying static doesn't modify its own entry (see doc above).
            if source == entered {
                continue;
            }
            if self.controller_of(source) != controller {
                continue;
            }
            for ability in p.def.abilities {
                let (
                    Timing::Static,
                    Effect::CreaturesYouControlEnterWithCounters { filter, count },
                ) = (ability.timing, ability.effect)
                else {
                    continue;
                };
                if !self.permanent_matches(&filter, entered, controller, Some(source)) {
                    continue;
                }
                total += self.resolve_count(count, controller, source, None, 0) as i32;
            }
        }
        total
    }

    /// The number of tokens actually created when an effect would create `base` tokens under
    /// `recipient`'s control, after that player's static token-creation replacements (CR 614 —
    /// Doubling Season, "twice that many of those tokens"). Each [`Effect::TokenReplacement`]
    /// that `recipient` controls multiplies the count once; the multipliers fold together.
    pub(crate) fn token_count_after_replacements(&self, recipient: PlayerId, base: u32) -> u32 {
        if base == 0 {
            return 0;
        }
        let mut product: u32 = 1;
        for obj in &self.objects {
            let Object::Permanent(p) = obj else {
                continue;
            };
            if p.owner != recipient {
                continue;
            }
            for ability in p.def.abilities {
                let (Timing::Static, Effect::TokenReplacement { times }) =
                    (ability.timing, ability.effect)
                else {
                    continue;
                };
                product *= times.max(0) as u32;
            }
        }
        base * product
    }

    /// The life actually gained when `recipient` would gain `base` life, after that player's static
    /// life-gain replacements (CR 614 — Pest Rescuer, "you gain that much life plus 1 instead").
    /// Each [`Effect::LifeGainReplacement`] that `recipient` controls adds its `plus`; the addends
    /// fold together. Gaining `base <= 0` is not "gaining life", so no replacement applies.
    pub(crate) fn life_gain_after_replacements(&self, recipient: PlayerId, base: i32) -> i32 {
        if base <= 0 {
            return base;
        }
        let mut total = 0;
        for (id, obj) in self.objects.iter().enumerate() {
            let Object::Permanent(p) = obj else {
                continue;
            };
            if self.controller_of(id as ObjectId) != recipient {
                continue;
            }
            for ability in p.def.abilities {
                let (Timing::Static, Effect::LifeGainReplacement { plus }) =
                    (ability.timing, ability.effect)
                else {
                    continue;
                };
                total += plus;
            }
        }
        base + total
    }

    /// The value of `{X}` a permanent spell actually enters/resolves with when `caster` casts it
    /// for the announced `base`, after that caster's static cast-X modifications (CR 107.3 —
    /// Unbound Flourishing, "double the value of X"). Applies only to *permanent* spells whose cost
    /// contains `{X}`; each [`Effect::CastXReplacement`] `caster` controls multiplies the value
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
            for ability in p.def.abilities {
                let (Timing::Static, Effect::CastXReplacement { times }) =
                    (ability.timing, ability.effect)
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
        additional: AdditionalCost {
            discard: 0,
            discard_land: false,
            pay_life_x: false,
            pay_life: 0,
            sacrifice: None,
            kicker: None,
            strive: None,
            replicate: None,
        },
        reduce_own_generic: None,
    };

    fn creature(power: i32, toughness: i32) -> CardDef {
        CardDef {
            name: "Test Creature",
            cost: FREE,
            kind: CardKind::Creature {
                power,
                toughness,
                also: TypeSet::NONE,
            },
            legendary: false,
            uncounterable: false,
            modal: false,
            modal_choose: 0,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            keywords: &[],
            conditional_keywords: &[],
            abilities: &[],
            identity_pips: &[],
            colors: &[],
            enters_tapped: false,
            enters_tapped_unless: None,
            approximates: None,
            oracle: None,
            set: "",
            subtypes: &[],
            otags: &[],
            cycling: None,
            flashback: None,
            echo: None,
            bestow: None,
            morph: None,
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
        }
    }

    fn anthem() -> CardDef {
        static ABILITIES: &[Ability] = &[Ability {
            timing: Timing::Static,
            effect: Effect::AnthemStatic {
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
                commander_only: false,
                has_counters: false,
                condition: None,
                from_graveyard: false,
            },
            optional: false,
            min_level: 0,
            cost: Cost::FREE,
            condition: None,
            once_each_turn: false,
        }];
        CardDef {
            name: "Test Anthem",
            cost: FREE,
            kind: CardKind::Enchantment,
            legendary: false,
            uncounterable: false,
            modal: false,
            modal_choose: 0,
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
            subtypes: &[],
            otags: &[],
            cycling: None,
            flashback: None,
            echo: None,
            bestow: None,
            morph: None,
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
                def: anthem(),
                controller: PlayerId(0),
                targets: TargetList::default(),
                targets_second: TargetList::default(),
                commander: false,
                x: 0,
                modes: Modes::default(),
                copy: false,
                flashback: false,
                escape: false,
                cast_from_hand: false,
                damage_division: DamageAssignment::default(),
                damage_division_players: [None; MAX_TARGETS],
                counter_division: DamageAssignment::default(),
                sacrifice_count: 0,
                kicked: false,
                strive_count: 0,
                replicate_count: 0,
                serra_recursion: false,
                bestowed: false,
                face_down: false,
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
            cost: Cost::FREE,
            kind: CardKind::Land {
                produces: Some(LandProduces::Mana(Mana::Color(Color::Green))),
                subtypes: &["Forest"],
                basic: true,
            },
            legendary: false,
            uncounterable: false,
            enchant: None,
            enchant_graveyard: false,
            modal: false,
            modal_choose: 0,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            keywords: &[],
            conditional_keywords: &[],
            abilities: &[],
            identity_pips: &[],
            colors: &[],
            enters_tapped: false,
            enters_tapped_unless: None,
            approximates: None,
            oracle: None,
            set: "",
            subtypes: &[],
            otags: &[],
            cycling: None,
            flashback: None,
            echo: None,
            bestow: None,
            morph: None,
            delve: false,
            escape: None,
            retrace: false,
            graveyard_cast_cost: None,
            cascade: false,
            functions_in_graveyard: false,
            back: None,
            adventure: None,
            suspend: None,
            devour: None,
            demonstrate: false,
            enter_as_copy: None,
            encore: None,
            hand_ability: None,
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
            def: creature(1, 1),
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
        additional: AdditionalCost {
            discard: 0,
            discard_land: false,
            pay_life_x: false,
            pay_life: 0,
            sacrifice: None,
            kicker: None,
            strive: None,
            replicate: None,
        },
        reduce_own_generic: None,
    };

    fn creature_with(keywords: &'static [Keyword]) -> CardDef {
        CardDef {
            name: "Test Creature",
            cost: FREE,
            kind: CardKind::Creature {
                power: 2,
                toughness: 2,
                also: TypeSet::NONE,
            },
            legendary: false,
            uncounterable: false,
            enchant: None,
            enchant_graveyard: false,
            modal: false,
            modal_choose: 0,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            keywords,
            conditional_keywords: &[],
            abilities: &[],
            identity_pips: &[],
            colors: &[],
            enters_tapped: false,
            enters_tapped_unless: None,
            approximates: None,
            oracle: None,
            set: "",
            subtypes: &[],
            otags: &[],
            cycling: None,
            flashback: None,
            echo: None,
            bestow: None,
            morph: None,
            delve: false,
            escape: None,
            retrace: false,
            graveyard_cast_cost: None,
            cascade: false,
            functions_in_graveyard: false,
            back: None,
            adventure: None,
            suspend: None,
            devour: None,
            demonstrate: false,
            enter_as_copy: None,
            encore: None,
            hand_ability: None,
        }
    }

    fn land(produces: LandProduces) -> CardDef {
        CardDef {
            name: "Land",
            cost: Cost::FREE,
            kind: CardKind::Land {
                produces: Some(produces),
                subtypes: &[],
                basic: false,
            },
            legendary: false,
            uncounterable: false,
            enchant: None,
            enchant_graveyard: false,
            modal: false,
            modal_choose: 1,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            keywords: &[],
            conditional_keywords: &[],
            abilities: &[],
            identity_pips: &[],
            colors: &[],
            enters_tapped: false,
            enters_tapped_unless: None,
            approximates: None,
            oracle: None,
            set: "",
            subtypes: &[],
            otags: &[],
            cycling: None,
            flashback: None,
            echo: None,
            bestow: None,
            morph: None,
            delve: false,
            escape: None,
            retrace: false,
            graveyard_cast_cost: None,
            cascade: false,
            functions_in_graveyard: false,
            back: None,
            adventure: None,
            suspend: None,
            devour: None,
            demonstrate: false,
            enter_as_copy: None,
            encore: None,
            hand_ability: None,
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
                uncounterable: false,
                enchant: None,
                enchant_graveyard: false,
                modal: false,
                modal_choose: 1,
                modal_choose_max: None,
                modal_choose_max_if_commander: false,
                keywords: &[],
                conditional_keywords: &[],
                abilities: &[],
                identity_pips: &[],
                colors: &[],
                enters_tapped: false,
                enters_tapped_unless: None,
                approximates: None,
                oracle: None,
                set: "",
                subtypes: &[],
                otags: &[],
                cycling: None,
                flashback: None,
                echo: None,
                bestow: None,
                morph: None,
                delve: false,
                escape: None,
                retrace: false,
                graveyard_cast_cost: None,
                cascade: false,
                functions_in_graveyard: false,
                back: None,
                adventure: None,
                suspend: None,
                devour: None,
                demonstrate: false,
                enter_as_copy: None,
                encore: None,
                hand_ability: None,
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
                uncounterable: false,
                enchant: None,
                enchant_graveyard: false,
                modal: false,
                modal_choose: 1,
                modal_choose_max: None,
                modal_choose_max_if_commander: false,
                keywords: &[],
                conditional_keywords: &[],
                abilities: &[],
                identity_pips: &[],
                colors: &[],
                enters_tapped: false,
                enters_tapped_unless: None,
                approximates: None,
                oracle: None,
                set: "",
                subtypes: &[],
                otags: &[],
                cycling: None,
                flashback: None,
                echo: None,
                bestow: None,
                morph: None,
                delve: false,
                escape: None,
                retrace: false,
                graveyard_cast_cost: None,
                cascade: false,
                functions_in_graveyard: false,
                back: None,
                adventure: None,
                suspend: None,
                devour: None,
                demonstrate: false,
                enter_as_copy: None,
                encore: None,
                hand_ability: None,
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
                uncounterable: false,
                enchant: None,
                enchant_graveyard: false,
                modal: false,
                modal_choose: 1,
                modal_choose_max: None,
                modal_choose_max_if_commander: false,
                keywords: &[],
                conditional_keywords: &[],
                abilities: &[],
                identity_pips: &[],
                colors: &[],
                enters_tapped: false,
                enters_tapped_unless: None,
                approximates: None,
                oracle: None,
                set: "",
                subtypes: &[],
                otags: &[],
                cycling: None,
                flashback: None,
                echo: None,
                bestow: None,
                morph: None,
                delve: false,
                escape: None,
                retrace: false,
                graveyard_cast_cost: None,
                cascade: false,
                functions_in_graveyard: false,
                back: None,
                adventure: None,
                suspend: None,
                devour: None,
                demonstrate: false,
                enter_as_copy: None,
                encore: None,
                hand_ability: None,
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
