use super::*;
#[cfg(feature = "card-dsl")]
use crate::de;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum StaticEffect {
    /// "All Mountains are Plains" (Conversion), "All Swamps are 1/1 black creatures that are
    /// still lands" (Kormus Bell), "All Forests are 1/1 creatures that are still lands" (Living
    /// Lands) — a CR 613.4 type change and CR 613.3 P/T set applied to every land on the
    /// battlefield carrying one of `land_types`, whoever controls it. The one static scoped by a board
    /// sweep rather than by a controller ([`Self::Anthem`]) or an attachment
    /// ([`Self::SetAttachedTypes`]); [`Game::land_type_statics`](crate::Game) is the sweep, and
    /// every characteristic read that can be changed this way consults it.
    ///
    /// Scoped by land type instead of by a general [`PermanentFilter`](crate::PermanentFilter) on
    /// purpose: a filter would have to ask for the candidate's subtypes, which is the very answer
    /// this effect changes. Matching names against the subtype line as accumulated so far keeps
    /// the CR 613.4 timestamp order honest and the recursion impossible.
    ///
    /// `set_subtypes` replaces the whole line as CR 305.7 asks (a Mountain Forest under
    /// Conversion is a Plains and nothing else, and taps only for `{W}` —
    /// [`Game::land_mana_credit`](crate::Game) derives that). Leave it empty to change only what
    /// a land *also* is: Kormus Bell's Swamps stay Swamps.
    AllLandsOfTypeBecome {
        /// The land types this applies to — a land carrying any of them is caught. Each of the
        /// three cards names exactly one.
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_str_slice"))]
        land_types: &'static [&'static str],
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        set_subtypes: &'static [&'static str],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        add_types: TypeSet,
        /// The base P/T the land takes on, read only when `add_types` makes it a creature —
        /// "1/1 black creatures that are still lands". Both default to 0 for a change that
        /// alters no card type (Conversion), where nothing reads them.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        base_power: i32,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        base_toughness: i32,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        add_colors: &'static [Color],
    },

    Anthem {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        power: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        toughness: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        self_only: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        exclude_source: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        tokens_only: bool,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        keywords: &'static [Keyword],
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        subtypes: &'static [&'static str],
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        colors: &'static [Color],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        chosen_subtype: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        attacking_only: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        blocking_only: bool,
        /// Castle's "Untapped creatures you control get +0/+2" — restricts the anthem to
        /// candidates that aren't tapped right now. Live like every other axis here (CR 613.4):
        /// the buff falls off the instant a creature taps, so `characteristics_cache.rs`
        /// invalidates on [`Event::Tapped`](crate::Event)/`Untapped`.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        untapped_only: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        commander_only: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        has_counters: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        condition: Option<Condition>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        from_graveyard: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        all_players: bool,
        /// Restricts to creatures controlled by a player who has made a matching per-player
        /// choice: `Some(true)`/`Some(false)` reads a two-sided as-enters choice recorded on
        /// [`Player`](crate::Player) (Archangel of Strife's "Creatures controlled by players who
        /// chose war/peace"); `None` (default) applies no such restriction.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        war_choice: Option<bool>,
    },

    AttackTax {
        amount: u8,
    },

    /// A characteristic-defining ability (CR 604.3) setting the source's own base power and
    /// toughness to a count — Nightmare's "power and toughness are each equal to the number of
    /// Swamps you control". Applied in layer 7a by [`Game::pt_base`](crate::Game): it replaces the
    /// printed box before every other P/T effect, so a later base-set (Darksteel Mutation) still
    /// overrides it and counters and anthems still sum on top. Resolved live on every recompute,
    /// which is what makes it *defining* rather than a one-shot write — the creature grows the
    /// instant a Swamp arrives.
    ///
    /// `when` narrows the ability to one combat state, so a creature printing two of these gets
    /// whichever count applies right now (Gaea's Liege).
    BasePowerToughnessFromAmount {
        power: Amount,
        toughness: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        when: DefiningPtWhen,
    },

    /// "Non-Eye creatures you control can't attack" (Evil Eye of Orms-by-Gore) / "Except for
    /// creatures named Akron Legionnaire and artifact creatures, creatures you control can't
    /// attack" (Akron Legionnaire): an attack ban on every creature matching `filter`, wherever
    /// that creature is (CR 508.1a). The mirror of [`CantBlockFilter`](Self::CantBlockFilter) —
    /// like it, the whole battlefield is scanned for the static and `filter` is read from the
    /// static's *own* controller's perspective, so `controller = "you"` scopes the ban to the
    /// source's controller and `controller = "any"` makes it board-wide (Moat).
    ///
    /// An "except for X" clause is authored inverted, as the *banned* set — Akron Legionnaire's
    /// exemptions become `exclude = "artifact"` plus `exclude_name = "Akron Legionnaire"` — the
    /// same convention [`CantBeBlockedBy`](Self::CantBeBlockedBy) uses. Folded into
    /// [`Game::can_attack`](crate::Game), so a banned creature is not "able" to attack and goad
    /// cannot demand an attack the card forbids (CR 509.1a).
    CantAttackFilter {
        filter: PermanentFilter,
    },

    /// "Each opponent who cast a spell this turn can't attack with creatures" (Angelic Arbiter):
    /// a blanket per-player attack ban, unlike [`StaticEffect::CantBeAttackedBy`]'s
    /// defender-scoped filter — the gated player can't declare *any* attacker, not just ones
    /// aimed at a specific defender. Checked against `Player::spells_cast_this_turn` in
    /// `Game::declare_attackers`, and only against a static controlled by someone other than the
    /// declaring player (CR: "opponent").
    CantAttackIfCastThisTurn,

    /// "This creature can't attack unless defending player controls an Island" (Sea Serpent,
    /// Pirate Ship): a restriction carried by the *attacker*, satisfied when the defending player
    /// controls at least one permanent matching `filter`. The mirror image of
    /// [`StaticEffect::CantBeAttackedBy`] below, which the defender carries — both are scanned per
    /// declared (attacker, defender) pair in `Game::declare_attackers`. Printed for two players;
    /// at a pod the same creature can be legal against one seat and illegal against the next,
    /// which is why this hangs off the pair rather than off the attacker alone. Also folded into
    /// [`Game::can_attack`](crate::Game), so a creature with no open seat is not "able" and goad
    /// cannot demand an attack the card forbids (CR 509.1a).
    CantAttackUnlessDefenderControls {
        filter: PermanentFilter,
    },

    CantBeAttackedBy {
        filter: PermanentFilter,
    },

    /// Island Sanctuary: "If you would draw a card during your draw step, instead you may skip
    /// that draw. If you do, until your next turn, you can't be attacked except by creatures with
    /// flying and/or islandwalk." An optional replacement on the draw-step draw (CR 614) that
    /// buys a temporary copy of [`CantBeAttackedBy`](Self::CantBeAttackedBy) above.
    ///
    /// `filter` is the *banned* set, exactly as that static's is — the printed "except by" is
    /// inverted when the card is authored, so Island Sanctuary bans creatures lacking both flying
    /// and islandwalk rather than exempting creatures having either. `Game::perform_turn_based_actions`
    /// offers the skip at the draw step; accepting records `(controller, filter)` in
    /// `CombatExtras::repelled_until_next_turn`, which `Game::declare_attackers` reads beside its
    /// scan of the static above and that player's own next untap step clears.
    ///
    /// ponytail: one variant for both halves rather than a nestable "may skip your draw, then
    /// \<effect\>" — no [`Effect`] nests another today, and this is the only card in the pool that
    /// pays a draw for a combat shield. Split it the day a second card skips its draw for
    /// something else.
    MaySkipDrawForCantBeAttackedBy {
        filter: PermanentFilter,
    },

    /// Two-Headed Giant of Foriys' "This creature can block an additional creature each combat"
    /// (CR 509.1b): raises this permanent's own block ceiling from the default one attacker to
    /// `1 + count`. Read by [`Game::max_blocks`](crate::Game), which is where the ceiling is
    /// decided for every blocker, so a creature with no such static is capped at one there.
    CanBlockAdditional {
        count: u8,
    },

    /// Juggernaut's "This creature can't be blocked by Walls" (CR 509.1b): `filter` names the
    /// creatures turned away, so a blocker matching it can never be declared against this
    /// permanent. Invisibility's "can't be blocked *except* by Walls" is the same restriction
    /// authored inverted (`exclude_subtypes = ["Wall"]`), granted to a host by
    /// [`GrantToAttached`](Self::GrantToAttached)'s `cant_be_blocked_by` rather than printed.
    CantBeBlockedBy {
        filter: PermanentFilter,
    },

    /// Ironclaw Orcs' "This creature can't block creatures with power 2 or greater" (CR 509.1b):
    /// the blocker-side restriction that describes the *attacker*. Distinct from
    /// [`CantBlockFilter`](Self::CantBlockFilter) just below, which matches the would-be blocker
    /// and reaches the whole battlefield — this one is printed on the creature it restrains and
    /// reads the attacker it is being declared against.
    CantBlockAttackers {
        filter: PermanentFilter,
    },

    CantBlockFilter {
        filter: PermanentFilter,
    },

    CantCastDuringCombat,

    /// "Each opponent who attacked with a creature this turn can't cast spells" (Angelic
    /// Arbiter): the mirror of [`StaticEffect::CantAttackIfCastThisTurn`] — a blanket per-player
    /// cast ban, checked against `Player::attacked_this_turn` in `Game::cast_timing_ok`, and only
    /// against a static controlled by someone other than the casting player.
    CantCastIfAttackedThisTurn,

    CastXReplacement {
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one"))]
        times: i32,
    },

    ControlAttached,

    /// A counter-placement replacement (CR 614 — Hardened Scales, Doubling Season, Vorinclex).
    /// `add` then `times` then `halve` describe the modification; the remaining fields say which
    /// placements it sees. See [`Game::counters_after_replacements`](crate::Game).
    CounterReplacement {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        add: i32,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one"))]
        times: i32,
        /// Vorinclex's opponent-facing clause: "half that many … rounded down".
        #[cfg_attr(feature = "card-dsl", serde(default))]
        halve: bool,
        /// Benevolent Hydra's "another creature you control": never replaces its own source's
        /// counters.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        other: bool,
        /// "one or more counters" (Winding Constrictor, Vorinclex) rather than the default
        /// "one or more +1/+1 counters" (Hardened Scales, Corpsejack Menace).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        any_kind: bool,
        /// Vorinclex / Innkeeper's Talent Level 3: the clause keys off who *would put* the
        /// counters (CR 614.1), not off whose permanent receives them. `None` keys off the
        /// recipient's side instead (Doubling Season's "a permanent you control", Winding
        /// Constrictor's passive "would be put on … you control").
        #[cfg_attr(feature = "card-dsl", serde(default))]
        placer: Option<CounterPlacer>,
        /// Which recipients the replacement reaches (CR 122.1 — counters sit on permanents and on
        /// players).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        recipients: CounterRecipients,
        /// A type gate on the receiving permanent — Ozolith's "an artifact or creature you
        /// control". `None` is every permanent (Doubling Season, Vorinclex).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        filter: Option<PermanentFilter>,
    },

    CounterScaledAttackTax,

    CreaturesYouControlEnterWithCounters {
        filter: PermanentFilter,
        count: Amount,
    },

    /// "This artifact doesn't untap during your untap step" (Mana Vault, Basalt Monolith) /
    /// "Creatures with power 3 or greater don't untap during their controllers' untap steps"
    /// (Meekstone) — CR 502.2's untap-step exception. Read by
    /// [`Game::doesnt_untap`](crate::Game), which the untap step consults before it untaps
    /// anything; nothing else in the game is affected, so a permanent held down here still
    /// untaps from an ordinary untap *effect* ({3}: Untap this artifact).
    ///
    /// `self_only` is the printed-on-itself form and ignores `filter` entirely — the source is
    /// the only permanent held down. Otherwise `filter` is matched against every permanent
    /// about to untap, and its default [`FilterController::Any`](crate::FilterController) is
    /// what makes Meekstone reach across the table ("their controllers' untap steps"), with no
    /// `all_players` flag of the sort [`Anthem`](Self::Anthem) needs.
    /// Library of Leng's "If an effect causes you to discard a card, discard it, but you may put
    /// it on top of your library instead of into your graveyard" (CR 701.8c). A replacement the
    /// controller's own effect discards consult in [`Game::discard_ids`](crate::Game): the card is
    /// still discarded — [`Event::Discarded`](crate::Event::Discarded) still fires for every
    /// "whenever you discard" watcher — only its destination changes.
    DiscardToLibraryTopInstead,

    DoesntUntap {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        self_only: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        filter: PermanentFilter,
    },

    /// Stasis's "Players skip their untap steps" (CR 703.4a). Read by
    /// [`Game::players_skip_untap_steps`](crate::Game), which the untap step consults before it
    /// phases anything in or untaps anything. Unscoped on purpose — "players" is everyone, its
    /// own controller included, so there is no `all_players` flag of the sort
    /// [`Anthem`](Self::Anthem) needs and no `filter` of the sort
    /// [`DoesntUntap`](Self::DoesntUntap) carries.
    ///
    /// Distinct from `DoesntUntap`, which holds individual permanents down *within* an untap
    /// step that still happens: this one deletes the step's turn-based actions outright, which is
    /// why a phased-out permanent stays phased out under Stasis (CR 502.1) where `DoesntUntap`
    /// would let it back in.
    PlayersSkipUntapSteps,

    /// Revelation's "Players play with their hands revealed." Read by
    /// [`Game::hands_revealed_to_all`](crate::Game) — and *only* by the projection layer in
    /// `crate::schema`, which widens its per-viewer privacy gate. No rules logic branches on it:
    /// a revealed hand is still a hand, so nothing about casting, discarding, or targeting from it
    /// changes (CR 400.2).
    ///
    /// Unscoped like [`PlayersSkipUntapSteps`](Self::PlayersSkipUntapSteps) — "players" is
    /// everyone, the enchantment's own controller included.
    PlayersPlayWithHandsRevealed,

    /// Field of Dreams' "Players play with the top card of their libraries revealed." The library
    /// twin of [`PlayersPlayWithHandsRevealed`](Self::PlayersPlayWithHandsRevealed), read by
    /// [`Game::library_tops_revealed_to_all`](crate::Game): exactly the one card
    /// [`Game::library_top`](crate::Game) names per player, never the cards beneath it, and the
    /// library stays unordered-to-the-client in every other respect.
    PlayersPlayWithLibraryTopsRevealed,

    /// Smoke's "Players can't untap more than one creature during their untap steps" and Winter
    /// Orb's land twin (CR 502.2). Read by
    /// [`Game::untap_at_most_one_filters`](crate::Game), which the untap step consults while
    /// building its untap set: the permanents matching `filter` that were about to untap go into
    /// the [`PendingChoice::DeclineUntap`](crate::PendingChoice) pause instead, and only the one
    /// the active player leaves out of `keep_tapped` comes back up.
    ///
    /// Unscoped like [`PlayersSkipUntapSteps`](Self::PlayersSkipUntapSteps) — "players" is
    /// everyone, so `filter`'s default [`FilterController::Any`](crate::FilterController) is what
    /// makes both cards symmetrical. Winter Orb's "as long as this artifact is untapped" rides on
    /// the ability's own [`Condition::SourceUntapped`](crate::Condition), read once as the untap
    /// step starts — which is why an Orb tapped down in response untaps alongside your lands
    /// without stopping any of them.
    UntapAtMostOne {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        filter: PermanentFilter,
    },

    /// Time Vault's "If you would begin your turn while this artifact is tapped, you may skip that
    /// turn instead. If you do, untap this artifact." — a replacement effect on the turn itself
    /// (CR 614), and the only thing in the pool that can undo the card's own
    /// [`DoesntUntap`](Self::DoesntUntap). Read by
    /// [`Game::may_skip_turn_offer`](crate::Game), which
    /// [`Game::advance_step`](crate::Game) consults as a new turn's untap step begins and before
    /// any of its turn-based actions run: a "yes" untaps the source and hands the turn straight on
    /// to the next player, a "no" runs the untap step the pause stood in front of and takes the
    /// turn with the source still tapped.
    ///
    /// Fieldless: "your turn" is the source's controller's and "this artifact" is the source, so
    /// there is nothing left for a filter to say — unlike
    /// [`UntapAtMostOne`](Self::UntapAtMostOne), whose card says "players".
    MaySkipTurnWhileTapped,

    EntersWithCounters {
        #[cfg_attr(feature = "card-dsl", serde(rename = "count"))]
        amount: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        kind: Option<CounterKind>,
    },

    /// Zombie Master's "Other Zombies have '{B}: Regenerate this permanent.'" — an activated
    /// ability granted to every permanent matching `filter`, wherever it is on the battlefield.
    /// The filter-scoped twin of [`GrantToAttached`](Self::GrantToAttached)'s own
    /// `granted_ability`, which can only reach a host this permanent is attached to, and the
    /// non-mana twin of [`GrantManaAbility`](Self::GrantManaAbility). Both grant kinds are read
    /// back through [`Game::granted_activated_abilities`](crate::Game), so a granted ability
    /// activates at the same indices whichever way it arrived.
    GrantActivatedAbility {
        filter: PermanentFilter,
        #[cfg_attr(
            feature = "card-dsl",
            serde(deserialize_with = "de::opt_static_granted_ability")
        )]
        granted_ability: Option<&'static GrantedAbility>,
    },

    GrantManaAbility {
        filter: PermanentFilter,
        cost: ActivationCost,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::mana_batch")
        )]
        #[cfg_attr(feature = "card-schema", schemars(with = "Vec<crate::Mana>"))]
        mana: ManaPool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        restriction: Option<SpendRestriction>,
        /// "Add N mana of any one color" (CR 106.4 — Goldspan Dragon's granted Treasure ability):
        /// every credit locks to the one color the controller names, so activating pauses on
        /// [`crate::PendingChoice::ChooseManaColor`] rather than producing independent wildcards.
        /// The granted twin of [`ManaEffect::Add`]'s own `single_color`; `false` for a plain grant.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        single_color: bool,
    },

    GrantToAttached {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        power: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        toughness: Amount,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        keywords: &'static [Keyword],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        goad: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        protection_from_chosen_color: bool,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::opt_static_granted_ability")
        )]
        granted_ability: Option<&'static GrantedAbility>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        cant_attack: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        cant_block: bool,
        /// Invisibility's "Enchanted creature can't be blocked except by Walls": the granted twin
        /// of [`CantBeBlockedBy`](Self::CantBeBlockedBy), authored the same way — the filter names
        /// the blockers turned away, so "except by Walls" is spelled `exclude_subtypes = ["Wall"]`.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        cant_be_blocked_by: Option<PermanentFilter>,
        /// Lure's "All creatures able to block enchanted creature do so" (CR 509.1c): a blocking
        /// *requirement* rather than a restriction, gathered by `Game::required_blocks` and met as
        /// far as the declaration legally can be.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        must_be_blocked_by_all: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        cant_attack_controller: bool,
        /// Animate Wall's "Enchanted Wall can attack as though it didn't have defender": the host
        /// ignores the [`Keyword::Defender`](crate::Keyword) attack restriction while this Aura is
        /// on it. The keyword itself stays — only `Game::can_attack`'s check for it is waived — so
        /// anything else reading "has defender" is unaffected.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        may_attack_ignoring_defender: bool,
        /// Instill Energy's "Enchanted creature can attack as though it had haste": the host
        /// ignores the summoning-sickness attack restriction (CR 302.6) while this Aura is on it.
        /// The sibling of `may_attack_ignoring_defender` above, and deliberately narrower than
        /// granting [`Keyword::Haste`](crate::Keyword) — real haste would also free the host's
        /// `{T}` abilities, which "as though it had haste" for attacking does not.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        may_attack_ignoring_summoning_sickness: bool,
        /// Paralyze's "Enchanted creature doesn't untap during its controller's untap step" — the
        /// attachment-scoped form of [`DoesntUntap`](Self::DoesntUntap), which is battlefield-wide
        /// and so can't say "the one this Aura is on". Folded into
        /// [`Game::doesnt_untap`](crate::Game) so the untap step reads one scanner, and like the
        /// battlefield-wide form it is consulted *only* there: an untap effect (Paralyze's own
        /// pay-{4}) frees the host regardless.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        doesnt_untap: bool,
        /// Consecrate Land's "can't be enchanted by other Auras": no *other* Aura may attach to
        /// this host — none can be cast targeting it, and one already there falls off (CR
        /// 704.5n). See [`Game::host_cant_be_enchanted_by`](crate::Game::host_cant_be_enchanted_by).
        #[cfg_attr(feature = "card-dsl", serde(default))]
        cant_be_enchanted: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        activated_abilities: Option<AbilityRestriction>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        legendary_only: bool,
    },

    /// Bartel Runeaxe's "Bartel Runeaxe can't be the target of Aura spells" and Anti-Magic Aura's
    /// "Enchanted creature can't be the target of spells" — a *filtered* targeting restriction (CR
    /// 115.4/115.6), deliberately not [`Keyword::Shroud`](crate::Keyword): shroud turns away
    /// abilities too, and these clauses name only spells (Anti-Magic Aura) or only Aura spells
    /// (Bartel Runeaxe, Tetsuo Umezawa). Enforced alongside
    /// shroud/hexproof/protection in the engine's target enumeration, so a spell that tries anyway
    /// is rejected at target selection like any other illegal target.
    CantBeTargetedBy {
        /// Which spells are turned away — [`SpellFilter::Aura`] for "Aura spells",
        /// [`SpellFilter::AllSpells`] for the unqualified "spells".
        spells: SpellFilter,
        /// The shield lands on the host this Aura is attached to (Anti-Magic Aura's "Enchanted
        /// creature") rather than on the ability's own source (Bartel Runeaxe naming itself).
        /// `false` (default) is the self-shield.
        ///
        /// A flag here rather than a `cant_be_targeted_by` field on
        /// [`GrantToAttached`](Self::GrantToAttached) — where the pool's other "Enchanted creature
        /// …" clauses live — because the self-shielding creatures need this variant regardless, and
        /// one variant carrying both scopes keeps the enforcement to a single battlefield scan.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        attached: bool,
    },
    /// The *filtered* anthem: a continuous grant to every permanent matching a full
    /// [`PermanentFilter`], rather than [`Anthem`](Self::Anthem)'s fixed set of candidate axes.
    /// Reach for it when the affected set needs something `Anthem` can't say — a printed name
    /// (Ivory Guardians' "Creatures named Ivory Guardians") or a per-candidate combat state read
    /// live (Arcades Sabboth's "Each untapped creature you control … as long as it's not
    /// attacking") — and for `Anthem` when the plain "creatures you control" axes suffice.
    ///
    /// The `keyword_anthem` name is what the TOML surface spells and predates the `power` /
    /// `toughness` fields; it grants keywords and/or a P/T delta, either alone.
    ///
    /// Applied per candidate in `Game::anthem_continuous_effects` beside `Anthem`'s own scan, so
    /// both kinds land in layer 7c/6 at the same timestamp choke and are re-read on every
    /// recompute (CR 613.4) — the boost falls off the instant the filter stops matching.
    KeywordAnthem {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        keywords: &'static [Keyword],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        filter: PermanentFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        all_players: bool,
        /// P/T delta granted alongside (or instead of) `keywords`. Both default to 0, so the
        /// pool's keyword-only spellings are unchanged.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        power: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        toughness: Amount,
        /// An "as long as …" board gate on the *whole* anthem, evaluated against the source's own
        /// controller — Ivory Guardians' "as long as an opponent controls a nontoken red
        /// permanent". The candidate-side half of a card's gate belongs in `filter` instead.
        /// Same field and same live re-read as [`Anthem`](Self::Anthem)'s `condition`.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        condition: Option<Condition>,
    },

    /// Crevasse's "Creatures with mountainwalk can be blocked as though they didn't have
    /// mountainwalk" (and its four siblings, plus Gosta Dirk / Lord Magnus / Ur-Drago printing the
    /// same static on a creature): the landwalk evasion for `land` stops being checked, board-wide.
    ///
    /// CR 702.14b's evasion is *checked* when blockers are declared, not a property removed from
    /// the creature — the attacker keeps its [`Keyword::Landwalk`](crate::Keyword), so Island
    /// Sanctuary's "except by creatures with … islandwalk" still sees it. Only
    /// [`Game::can_block`](crate::Game)'s landwalk check is waived, via
    /// [`Game::landwalk_negated`](crate::Game).
    ///
    /// One land type per variant, so Lord Magnus's two statics accumulate instead of one replacing
    /// the other: the board scan finds each on its own.
    LandwalkNegated {
        land: BasicLandType,
    },

    /// Lich's "You don't lose the game for having 0 or less life" — CR 704.5a's exemption, read
    /// by the state-based sweep off every permanent its controller controls. Says nothing about
    /// the other loss conditions: an empty-library draw, ten poison and lethal commander damage
    /// all still eliminate its controller.
    YouDontLoseAtZeroLife,

    /// Lich's "If you would gain life, draw that many cards instead" — a CR 614 replacement, so
    /// the life never arrives at all and nothing that watches life gain sees anything. Applies at
    /// the one funnel every life gain passes through, which is what makes it cover lifelink and
    /// drains as well as a printed "you gain N life".
    LifeGainBecomesDraw,

    LifeGainReplacement {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        plus: i32,
    },

    /// "Creatures attack each combat if able": Avatar of Slaughter's board-wide reading by
    /// default, or Juggernaut's own "This creature attacks each combat if able" with
    /// `self_only = true` — the same split [`DoesntUntap`](Self::DoesntUntap) makes.
    MustAttackEachCombat {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        self_only: bool,
    },

    NoMaximumHandSize,

    OpponentsCantSearchLibraries,

    /// Fastbond's "You may play any number of lands on each of your turns" (CR 305.2 — the
    /// one-land-per-turn rule is an effect-modifiable maximum). Read by
    /// [`Game::land_drop_available`](crate::Game), the single gate both the legality check and the
    /// playability hint route through.
    /// ponytail: fieldless — the pool's only extra-land-play permission is unlimited. Add a
    /// `count` when an Exploration/Azusa "play an additional land" lands.
    PlayAnyNumberOfLands,

    PlayFromGraveyardOncePerTurn,

    /// A permanent's own printed prevention shield (CR 615): "Prevent all damage that would be
    /// dealt to this creature by …". Applies as the damage would be dealt and never uses the
    /// stack, so it is read at the damage chokes rather than placed as a triggered ability.
    ///
    /// `to_self` shields damage dealt *to* the permanent (Guard Gomazoa, every Wall in this
    /// family); `by_self` shields damage it deals to others (Fog Bank's "and dealt by"). The three
    /// gates below narrow *which* damage — an unset gate imposes no restriction, which is the
    /// unqualified "prevent all combat damage" both 2ed cards print.
    PreventDamage {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        to_self: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        by_self: bool,
        /// "Prevent all **combat** damage" (Fog Bank, Guard Gomazoa, Enchanted Being, Marble
        /// Priest). `false` — Wall of Vapor's plain "prevent all damage" — covers combat and
        /// noncombat damage alike. The word is the whole difference between Enchanted Being and
        /// Wall of Putrid Flesh, which otherwise print the same shield.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        combat_only: bool,
        /// "… by enchanted creatures" (Enchanted Being, Wall of Putrid Flesh) / "… by Walls"
        /// (Marble Priest): a gate on the damage's *source*, read as an ordinary permanent
        /// filter from the shielded permanent's controller's perspective. A source that isn't a
        /// permanent (a spell) never matches one — see [`SourceRelation`] for those.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        source_filter: Option<PermanentFilter>,
        /// "… by creatures it's blocking" (Wall of Shadows, Wall of Vapor) / "… by spells that
        /// target it" (Bronze Horse): a *relationship* between the damage's source and the
        /// shielded permanent, which no [`PermanentFilter`] axis can express.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        source_relation: Option<SourceRelation>,
    },

    PreventDamageToSelfRemovingCounter,

    /// Rock Hydra's "for each 1 damage that would be dealt to this creature, if it has a +1/+1
    /// counter on it, remove a +1/+1 counter from it and prevent that 1 damage" (CR 615). Worded
    /// *per point* rather than per event, so unlike its two siblings above it covers only as many
    /// points as the Hydra has counters and the rest of the hit is dealt for real — which is why
    /// it is spent inside the damage choke rather than short-circuiting the whole event.
    PreventDamageToSelfRemovingCounterPerPoint,

    PreventDamageToSelfRemovingCountersGivingRad,

    PreventNoncombatDamageToOtherCreaturesYouControl,

    ProtectionFromChosenColor,

    /// Veteran Bodyguard's "as long as this creature is untapped, all damage that would be dealt
    /// to you by unblocked creatures is dealt to this creature instead" (CR 615.10). A redirection
    /// read live off the permanent — the untapped condition is checked at damage time, not when
    /// the creature entered — rather than a one-shot shield like
    /// [`MiscEffect::PreventNextDamage`](crate::MiscEffect::PreventNextDamage)'s
    /// `redirect_to_controller`, which is armed and spent.
    ///
    /// ponytail: only *combat* damage from an unblocked attacker is moved — the scan sits in
    /// `Game::combat_damage_substep`, which is the one place that knows an attacker went
    /// unblocked. The printed line also covers noncombat damage an unblocked creature deals
    /// (an activated ability, say); no pool card creates that case, and the upgrade path is a
    /// per-turn "went unblocked" set the general damage choke could read.
    RedirectUnblockedDamageToSelf,

    ReduceSpellCost {
        amount: Amount,
        filter: SpellFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        first_x_spell_each_turn: bool,
    },

    /// Gloom's "White spells cost {3} more to cast" (CR 601.2f) — the sign-flipped twin of
    /// [`ReduceSpellCost`](Self::ReduceSpellCost) above, and deliberately a separate variant
    /// rather than a negative `amount` on that one, because the two differ in *scope*: a reducer
    /// only ever discounts its own controller's spells, while a taxer reaches every seat at the
    /// table. Folded into the generic cost ahead of the reduction, per CR 601.2f's
    /// increases-then-reductions order.
    TaxSpellCost {
        amount: Amount,
        filter: SpellFilter,
    },

    /// Gloom's "Activated abilities of white enchantments cost {3} more to activate" (CR 602.2b) —
    /// the same table-wide tax aimed at the *activation* choke instead of the cast choke, keyed to
    /// the ability's source permanent rather than to a spell. `filter` is matched against that
    /// source, so "white enchantments" is `{ types = "enchantment", color = "white" }`.
    TaxActivatedAbility {
        amount: Amount,
        filter: PermanentFilter,
    },

    /// The CR 613.4 layer-7b base P/T an Aura forces onto its host (Darksteel Mutation's "base
    /// power and toughness 0/1"). The amounts resolve with the *host* as source, not the Aura, so
    /// Animate Artifact's "power and toughness each equal to its mana value" is
    /// [`Amount::SourceManaValue`](crate::Amount) reading the enchanted artifact.
    SetAttachedBasePt {
        power: Amount,
        toughness: Amount,
        /// Animate Artifact's "as long as enchanted artifact **isn't a creature**": the base set
        /// applies only while the host is a noncreature, so enchanting an artifact creature
        /// doesn't wipe its printed P/T. Default `false` is Darksteel Mutation's unconditional set.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        noncreature_only: bool,
    },

    SetAttachedTypes {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        add_types: TypeSet,
        /// CR 613.4: when `true`, `add_types` are the host's *complete* card types (replacing its
        /// printed ones — Darksteel Mutation's "loses all other … card types"), not merely unioned
        /// on. Default `false` keeps the additive Angelic-Destiny behavior.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        set_types: bool,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        add_subtypes: &'static [&'static str],
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        set_subtypes: &'static [&'static str],
        /// Phantasmal Terrain's "enchanted land is the chosen type": `set_subtypes` is the one
        /// basic land type this Aura's controller named as it entered
        /// ([`ChoiceEffect::ChooseBasicLandType`](crate::ChoiceEffect)) rather than a printed
        /// list. The whole type change is inert until that choice is answered.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        set_chosen_land_type: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        lose_all_abilities: bool,
    },

    /// Sunglasses of Urza's "You may spend white mana as though it were red mana" (CR 609.4b):
    /// while this permanent's controller pays a cost, each of their `from` credits may pay a `to`
    /// pip as well as its own. Gathered by [`Game::mana_substitutions`](crate::Game) and applied
    /// to the pool by [`ManaPool::substituted`](crate::ManaPool) before the payment planners run.
    SpendManaAsThoughAnotherColor {
        from: Color,
        to: Color,
    },

    TappedForManaBonus {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        scope: LandTapScope,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        bonus_color: LandTapBonusColor,
    },

    TokenReplacement {
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one"))]
        times: i32,
    },

    TriggerDoubling {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        source_subtypes: &'static [&'static str],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        source_other: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        caused_by_instant_or_sorcery_cast: bool,
    },
}

/// When a [`StaticEffect::BasePowerToughnessFromAmount`] applies. Gaea's Liege prints two of
/// them — "as long as this creature isn't attacking" counts your Forests, "as long as it is
/// attacking" counts the defending player's — and a creature carrying both gets exactly one
/// answer at any moment. Modelled as two abilities with a combat-state guard rather than one
/// ability with a ternary `Amount`, because that is how the card prints it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum DefiningPtWhen {
    /// No guard — the count always applies (Nightmare, Keldon Warlord, Plague Rats).
    #[default]
    Always,
    /// Only while the source is a declared attacker (CR 506.3d).
    Attacking,
    /// Only while the source is *not* a declared attacker.
    NotAttacking,
}

/// Which recipients a [`StaticEffect::CounterReplacement`] reaches. Counters sit on permanents and
/// on players (CR 122.1), and a card names one or both: Hardened Scales only permanents, Winding
/// Constrictor's second ability only its controller, Vorinclex "a permanent or player".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum CounterRecipients {
    #[default]
    Permanents,
    Players,
    PermanentsAndPlayers,
}

/// Whose *placement* a [`StaticEffect::CounterReplacement`] replaces (CR 614.1) — the axis
/// Vorinclex's "if **you would put**" / "if an **opponent would put**" reads. Distinct from
/// [`CounterRecipients`] and from the effect's `filter`, which both gate the *recipient*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum CounterPlacer {
    You,
    Opponents,
}

/// How a prevention shield's *source* gate reads the shielded object rather than the source's own
/// characteristics (CR 615) — the two Legends shields whose "by …" clause names a relationship
/// instead of a class of permanents. A [`PermanentFilter`] can say "by Walls"; neither of these
/// can be said that way, because both depend on the shielded object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum SourceRelation {
    /// "by creatures it's blocking" (Wall of Shadows, Wall of Vapor): the shielded permanent is
    /// blocking the damage's source. Read per damage source, not per combat — a creature can
    /// block two attackers (CR 509.1), and the shield stands in front of each of them.
    BlockedByThis,
    /// "by spells that target it" (Bronze Horse) / "a spell or ability that targets that
    /// creature" (Silhouette): the damage's source is a spell on the stack among whose chosen
    /// targets the shielded permanent is.
    SpellTargetingThis,
}
