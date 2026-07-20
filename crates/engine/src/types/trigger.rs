use super::*;

/// The event a triggered ability watches for. Triggers come in two flavors (see
/// [`Game::enqueue_triggers`]): *self-referential* ones fire when the ability's own source is
/// the subject of the event ([`Etb`](Self::Etb)/[`Attacks`](Self::Attacks)/[`Dies`](Self::Dies)),
/// and *controller-scoped* ones fire on every permanent a given player controls when that
/// player does the thing ([`Upkeep`](Self::Upkeep)/[`EndStep`](Self::EndStep)/
/// [`YouGainLife`](Self::YouGainLife)/[`Magecraft`](Self::Magecraft)). A third, *watch-others*
/// flavor fires on every battlefield permanent when a *different* creature dies
/// ([`CreatureDies`](Self::CreatureDies)/[`CreatureYouControlDies`](Self::CreatureYouControlDies))
/// or when a player attacks one of the watcher's opponents
/// ([`PlayerAttacksYourOpponent`](Self::PlayerAttacksYourOpponent)).
/// The enum grows only as real cards demand it.
///
/// Watch-death triggers honor the CR 603.6c look-back: a watcher that dies alongside other
/// creatures still fires for *their* deaths (see [`Game::queue_watch_death_triggers`]). The plain
/// arms ([`CreatureDies`](Self::CreatureDies)/[`CreatureYouControlDies`](Self::CreatureYouControlDies))
/// don't fire for the watcher's own death ("another creature dies"); the `*IncludingThis` arms
/// additionally self-fire off the dying creature's own last-known information (Blood Artist /
/// Zulaport Cutthroat).
///
/// A fourth, *sacrifice-watch* flavor ([`YouSacrifice`](Self::YouSacrifice)/
/// [`AnyPlayerSacrifices`](Self::AnyPlayerSacrifices)) carries a [`PermanentFilter`], so it isn't
/// spelled from a bare TOML string like the others — `de.rs` deserializes `timing` as a tag and
/// pairs it with a sibling `filter` field to build these two variants by hand (see
/// `de::TriggerTag`).
///
/// A fifth, *every-player* flavor ([`EachUpkeep`](Self::EachUpkeep)/[`EachEndStep`](Self::EachEndStep))
/// fires under its own controller regardless of whose turn it is — unlike the controller-scoped
/// flavor, it isn't gated to the controller's own turn at all.
///
/// A sixth, *combat-damage-watch* flavor ([`DealsCombatDamageToPlayer`](Self::DealsCombatDamageToPlayer))
/// carries a [`CombatDamageScope`], so like the sacrifice-watch flavor it isn't spelled from a
/// bare TOML string — `de.rs` pairs the `timing` tag with a sibling `who` field (see
/// `de::TriggerTag`).
///
/// A seventh, *other-player-only* flavor ([`EachOtherPlayerUntapStep`](Self::EachOtherPlayerUntapStep))
/// is the mirror image of the every-player flavor: it fires under its own controller at the
/// beginning of every *other* player's untap step, explicitly excluding the controller's own
/// (Drumbellower).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// When this permanent enters the battlefield (ETB). Spelled `"etb"` in TOML (`"etb_triggered"`
    /// is an accepted alias — see `de::TriggerTag`).
    Etb,
    /// When this permanent is turned face up (CR 702.37f — a morph/megamorph turned-face-up
    /// trigger). Fires off [`Event::TurnedFaceUp`] by scanning the now-revealed object's own
    /// abilities. Spelled `"turned_face_up"` in TOML.
    TurnedFaceUp,
    /// When this creature is declared as an attacker.
    Attacks,
    /// Whenever this creature blocks or becomes blocked (Goblin Cadets, CR 509/CR 509.1h): fires
    /// off [`Event::BlockerDeclared`] when this creature is named as either the blocker or the
    /// attacker — once per creature per combat, not once per (blocker, attacker) pair, so a
    /// multiply-blocked attacker's "becomes blocked" still fires only once. Placed directly from
    /// [`Game::declare_blockers`] (batch-deduped, like [`Trigger::YouAttackWithCreatures`]'s
    /// [`Game::queue_batch_attack_triggers`]), not [`Game::enqueue_triggers`]'s per-event scan.
    BlocksOrBecomesBlocked,
    /// When this creature dies (moves from the battlefield to the graveyard, or — for a
    /// token — ceases to exist).
    Dies,
    /// Whenever *another* creature dies (a watch-others trigger, self-excluded).
    CreatureDies,
    /// Whenever a creature *this permanent's controller controls* dies, other than itself
    /// (self-excluded).
    CreatureYouControlDies,
    /// Whenever this creature *or another* creature dies (CR: "this creature or another
    /// creature dies") — [`CreatureDies`](Self::CreatureDies) plus a self-fire off the dying
    /// creature's own last-known information (CR 603.6c/603.10). Blood Artist.
    CreatureDiesIncludingThis,
    /// Whenever this creature *or another* creature this permanent's controller controls dies —
    /// [`CreatureYouControlDies`](Self::CreatureYouControlDies) plus the self-fire. Zulaport
    /// Cutthroat.
    CreatureYouControlDiesIncludingThis,
    /// Whenever a *nontoken* creature this permanent's controller controls dies, other than
    /// itself — [`CreatureYouControlDies`](Self::CreatureYouControlDies) plus a token-death
    /// exclusion (Blight Mound: "whenever a nontoken creature you control dies"). A dying token
    /// never fires this arm.
    CreatureYouControlDiesNontoken,
    /// Whenever this creature *or another nontoken* creature this permanent's controller
    /// controls dies — [`CreatureYouControlDiesIncludingThis`](Self::CreatureYouControlDiesIncludingThis)
    /// plus the same token-death exclusion on the *other*-creature half (Pawn of Ulamog: "whenever
    /// this creature or another nontoken creature you control dies"). The watcher's own death
    /// still self-fires unconditionally (Pawn dying fires off itself even though nothing asks
    /// whether Pawn itself was a token).
    CreatureYouControlDiesIncludingThisNontoken,
    /// Whenever a creature *an opponent* of this permanent's controller controls dies
    /// (Yahenni, Undying Partisan) — the opponent-scoped twin of
    /// [`CreatureYouControlDies`](Self::CreatureYouControlDies), self-excluded. No
    /// `*IncludingThis` sibling: "an opponent controls" can never describe the watcher itself.
    CreatureAnOpponentControlsDies,
    /// Whenever an *enchantment* this permanent's controller controls is put into a graveyard
    /// from the battlefield (Starfield Mystic). The enchantment twin of
    /// [`CreatureYouControlDies`](Self::CreatureYouControlDies) — a permanent-type-scoped
    /// leaves-to-graveyard watch, self-excluded like the plain creature arms. No opponent-scoped
    /// or `*IncludingThis` sibling yet — grow those from a real card (flag-don't-force).
    EnchantmentYouControlDies,
    /// Whenever this permanent *or another nonland permanent* this permanent's controller
    /// controls is put into a graveyard from the battlefield (Martyr's Bond: "Whenever a nonland
    /// permanent you control is put into a graveyard...") — a permanent-type-wide, self-including
    /// generalization of [`CreatureYouControlDiesIncludingThis`](Self::CreatureYouControlDiesIncludingThis)/
    /// [`EnchantmentYouControlDies`](Self::EnchantmentYouControlDies): any of the four nonland
    /// types (creature, artifact, enchantment, planeswalker), not creature-only or
    /// enchantment-only, and it self-fires off its own last-known information like the
    /// `*IncludingThis` creature arms. The dying permanent's own last-known card types ride along
    /// on [`TriggerContext::dying_permanent_types`] for a payoff that reads "shares a card type
    /// with it" (`Effect::EachPlayerSacrifices`'s `shares_type_with_dying_permanent` filter axis).
    /// See [`Game::queue_nonland_permanent_death_watchers`].
    NonlandPermanentYouControlDiesIncludingThis,
    /// At the beginning of the controller's upkeep step.
    Upkeep,
    /// At the beginning of *every* player's upkeep, not just the controller's — CR "at the
    /// beginning of each upkeep" (Beledros Witherbloom, Tendershoot Dryad, Ophiomancer). Fires
    /// under the ability's own controller regardless of whose turn it is; contrast
    /// [`Upkeep`](Self::Upkeep), which is scoped to the controller's own turn.
    EachUpkeep,
    /// At the beginning of *every* player's end step, not just the controller's — CR "at the
    /// beginning of each end step" (Relic Retriever). Fires under the ability's own controller
    /// regardless of whose turn it is; contrast [`EndStep`](Self::EndStep), which is scoped to
    /// the controller's own turn — the every-player twin of [`EachUpkeep`](Self::EachUpkeep).
    EachEndStep,
    /// At the beginning of *every* player's draw step, not just the controller's — CR "at the
    /// beginning of each player's draw step" (Howling Mine). Fires under the ability's own
    /// controller regardless of whose turn it is, the draw-step twin of
    /// [`EachUpkeep`](Self::EachUpkeep)/[`EachEndStep`](Self::EachEndStep) — but unlike those two,
    /// its payoff ("**that player** draws an additional card") needs to know whose draw step this
    /// is, not just who controls the permanent, so [`Game::queue_each_draw_step_triggers`] threads
    /// the active player into [`TriggerContext::active_player`].
    EachDrawStep,
    /// At the beginning of every *other* player's untap step — CR "during each other player's
    /// untap step" (Drumbellower). Fires under the ability's own controller, excluding the
    /// controller's own untap step (contrast [`EachUpkeep`](Self::EachUpkeep)/
    /// [`EachEndStep`](Self::EachEndStep), which include the controller's own).
    EachOtherPlayerUntapStep,
    /// At the beginning of the controller's first (precombat) main phase (CR 505/508 —
    /// Advanced Reconstruction's "At the beginning of your first main phase, …"). Fires for the
    /// active player's own permanents only, scoped to the controller's own turn like
    /// [`Upkeep`](Self::Upkeep). Distinct from `BeginCombat` — this is `Step::Main1`, not
    /// `Step::BeginCombat`. Spelled `"first_main_phase"` in TOML.
    FirstMainPhase,
    /// At the beginning of combat on the controller's turn (Leonin Vanguard). Fires for the
    /// active player's own permanents only — an "each player" variant (Combat Celebrant-style)
    /// is a distinct, unlanded trigger.
    BeginCombat,
    /// At the beginning of the controller's end step.
    EndStep,
    /// Whenever the controller gains life.
    YouGainLife,
    /// Whenever an opponent of the controller gains life (Punishing Fire) — the opponent-scoped
    /// twin of [`YouGainLife`](Self::YouGainLife), fired for every player other than the
    /// gainer. Functions from the graveyard (CR 603.6e) for a card that sets
    /// [`CardDef::functions_in_graveyard`].
    OpponentGainsLife,
    /// Whenever the controller casts (or copies) an instant or sorcery spell.
    Magecraft,
    /// Whenever a player attacks one of the ability's controller's opponents (Breena, the
    /// Demagogue). A watch-others *attack* trigger: fires on every battlefield permanent whose
    /// controller is *not* the attacked player. The attacking player and the attacked opponent
    /// come off the [`Event::AttackerDeclared`] and reach the effect via the triggering context.
    PlayerAttacksYourOpponent,
    /// Whenever this permanent's controller attacks with `at_least` or more creatures this
    /// combat, regardless of defender (CR 508.1, Firemane Commando's "whenever you attack with
    /// two or more creatures"). A batch-count trigger: fires once per combat off the full
    /// attacker set [`Game::declare_attackers`] commits, not per single [`Event::AttackerDeclared`]
    /// (a per-event fire can't see "two or more"). See [`Game::queue_batch_attack_triggers`].
    YouAttackWithCreatures { at_least: u8 },
    /// Whenever an opponent attacks this permanent's controller (and/or planeswalkers they
    /// control) with `at_least` or more creatures this combat (CR 508.1, Mangara/Tomik's "an
    /// opponent attacks with creatures, if two or more of those creatures are attacking you").
    /// The [`OpponentAttacksYouWithCreatures`](Self::OpponentAttacksYouWithCreatures) batch twin
    /// of [`PlayerAttacksYourOpponent`](Self::PlayerAttacksYourOpponent): fires once per
    /// attacking opponent, gated on that opponent's own attacker count against this controller —
    /// counts don't combine across different attacking opponents. The attacking opponent rides
    /// in [`TriggerContext`]'s `attack` tuple (Tomik's punisher addresses "that opponent"). See
    /// [`Game::queue_batch_attack_triggers`].
    // ponytail: "and/or planeswalkers you control" reduces to "attacking you" — no pool card
    // attacks a planeswalker (`Event::AttackerDeclared.defender` is always a `PlayerId`, per the
    // existing creature-or-player ponytail note on `TargetSpec`), so the planeswalker half is
    // moot until planeswalkers-as-attack-targets land.
    OpponentAttacksYouWithCreatures { at_least: u8 },
    /// Whenever another player attacks with `at_least` or more creatures this combat,
    /// regardless of defender, *if none of those creatures attacked this permanent's
    /// controller* (CR 508.1, Firemane Commando's second ability: "whenever another player
    /// attacks with two or more creatures, they draw a card if none of those creatures
    /// attacked you"). The batch-count watch-others twin of
    /// [`YouAttackWithCreatures`](Self::YouAttackWithCreatures): the "if none attacked you"
    /// clause is the gate, not a scope restriction — an attacking player who *did* attack the
    /// watcher's controller just doesn't trigger it. The attacking player rides in
    /// [`TriggerContext`]'s `attack` tuple so the payoff effect can address "they". See
    /// [`Game::queue_batch_attack_triggers`].
    AnotherPlayerAttacksWithCreatures { at_least: u8 },
    /// Whenever the creature this Aura is attached to is declared as an attacker (CR 508.1, the
    /// Impetus cycle: "Whenever enchanted creature attacks, …"). A watch-attached trigger: placed
    /// on the Aura, but its controller is the Aura's own controller — not the enchanted
    /// creature's — so it fires *for* the host's attack while belonging to whoever cast the Aura
    /// (goaded-onto-an-opponent is the cycle's usual home). The enchanted creature's controller
    /// and the defended player ride along in the [`TriggerContext`]'s `attack` tuple, the same
    /// slot [`PlayerAttacksYourOpponent`](Self::PlayerAttacksYourOpponent) uses; see
    /// [`Game::queue_enchanted_creature_attacks_triggers`].
    EnchantedCreatureAttacks,
    /// Whenever the creature this Aura is attached to dies (CR "When enchanted creature dies…",
    /// Angelic Destiny). The death twin of [`EnchantedCreatureAttacks`](Self::EnchantedCreatureAttacks):
    /// placed on the Aura, controlled by *that Aura's own controller*, not the dying creature's.
    /// By the time this fires the Aura has itself already been put into its owner's graveyard by
    /// a state-based action (CR 704.5m) triggered by the host's death — the pre-move attachment
    /// is captured in `Game::apply`'s death-event handling and read back by
    /// [`Game::queue_enchanted_creature_dies_triggers`].
    EnchantedCreatureDies,
    /// Whenever the creature this Aura is attached to deals damage, combat or noncombat alike (CR
    /// 609.7/702, Armadillo Cloak: "Whenever enchanted creature deals damage, you gain that much
    /// life"). A second attached-host watch fired straight off [`Self::EnchantedCreatureAttacks`]'s
    /// `self.attachments(host)` shape: placed on the Aura, controlled by *that Aura's own
    /// controller* — not the host's — so it fires *for* the host's damage while belonging to
    /// whoever cast the Aura. Distinct from lifelink (CR 702.15e, a static replacement bundled
    /// into the damage event itself): this is a separate triggered ability that goes on the stack,
    /// so it stacks additively with real lifelink on the same creature rather than doubling into
    /// it. The dealt amount rides in [`TriggerContext::triggering_damage_dealt`]
    /// (`Amount::TriggeringDamageDealt`); see [`Game::queue_enchanted_creature_deals_damage_triggers`].
    EnchantedCreatureDealsDamage,
    /// Whenever *any* enchanted creature dies (CR 603.6c, Hateful Eidolon: "Whenever an
    /// enchanted creature dies, draw a card for each Aura you controlled that was attached to
    /// it.") — a watch-others twin of [`EnchantedCreatureDies`](Self::EnchantedCreatureDies):
    /// placed on any battlefield permanent, not just the dying creature's own Auras, and gated
    /// on the *watcher's controller* having controlled at least one of the Auras attached to the
    /// dying creature (read from the same pre-move attachment snapshot
    /// [`EnchantedCreatureDies`](Self::EnchantedCreatureDies) uses). The count of Auras the
    /// watcher's controller controlled is baked into
    /// [`Amount::AurasYouControlledAttachedToDyingCreature`] at placement — CR 603.10a
    /// last-known information, same shape as `dying_source_stats`. See
    /// [`Game::queue_an_enchanted_creature_dies_triggers`].
    /// ponytail: gates the trigger's very firing on the count being nonzero, rather than always
    ///   firing "an enchanted creature died" and drawing zero cards for an all-opponent-Auras
    ///   case — the two are observably identical (zero cards drawn, no other pool effect reacts
    ///   to the ability merely being on the stack), so this stays a same-behavior shortcut, not
    ///   a fidelity gap.
    AnEnchantedCreatureDies,
    /// Whenever one or more creatures enchanted by an Aura the controller controls attack (CR
    /// 508.1, Killian, Decisive Mentor's second ability: "Whenever one or more creatures that
    /// are enchanted by an Aura you control attack, draw a card.") — the attachment-aware twin
    /// of [`YouAttackWithCreatures`](Self::YouAttackWithCreatures): fires once per combat off
    /// the full committed attacker set when `at_least` or more of them are each enchanted by an
    /// Aura the watcher's controller controls, not a per-attacker fire. See
    /// [`Game::queue_batch_attack_triggers`].
    CreatureEnchantedByYourAuraAttacks { at_least: u8 },
    /// Whenever this permanent's controller sacrifices a permanent matching `filter` (Smothering
    /// Abomination: "whenever you sacrifice a creature, draw a card"). Fires off
    /// [`Event::Sacrificed`]; see [`Game::queue_sacrifice_triggers`].
    YouSacrifice { filter: PermanentFilter },
    /// Whenever *any* player sacrifices a permanent matching `filter` — a watch-others trigger
    /// (Mazirek, Kraul Death Priest: "whenever a player sacrifices another permanent";
    /// `filter.other` is what excludes the ability's own source sacrificing itself). Fires off
    /// [`Event::Sacrificed`]; see [`Game::queue_sacrifice_triggers`].
    AnyPlayerSacrifices { filter: PermanentFilter },
    /// Whenever this permanent's controller discards a card (CR 701.8) — Containment Construct's
    /// "whenever you discard a card". Fieldless (no filter — every discard qualifies) and
    /// controller-scoped like [`Upkeep`](Self::Upkeep); fires off [`Event::Discarded`], a marker
    /// pushed alongside every discard's `MovedToGraveyard` (an effect discard and the cleanup
    /// hand-size trim both count, CR 701.8). The discarded card's graveyard-object id rides in
    /// the [`TriggerContext`]'s `discarded` field so the effect can act on "that card"; see
    /// [`Game::queue_discard_triggers`].
    YouDiscard,
    /// Whenever a creature deals combat damage to a player (CR 510.2), scoped by `who`:
    /// [`CombatDamageScope::This`] (Leitmotif Composer — only this permanent's own damage),
    /// [`CombatDamageScope::YourCreatures`] (Ohran Frostfang, Defiling Daemogoth — any creature
    /// this permanent's controller controls), or [`CombatDamageScope::YourTokens`] (Curiosity
    /// Crafter — any creature *token* this permanent's controller controls). A sixth,
    /// bespoke-queued watch flavor like [`YouSacrifice`](Self::YouSacrifice): fires off
    /// [`Event::CombatDamageDealtToPlayer`], not `LifeChanged` (non-combat life loss — drain,
    /// pay-life — must not fire it); see [`Game::queue_combat_damage_triggers`].
    DealsCombatDamageToPlayer { who: CombatDamageScope },
    /// Whenever this permanent deals combat damage to a *creature* (CR 510.2, Stinkweed Imp:
    /// "Whenever this creature deals combat damage to a creature, destroy that creature").
    /// Self-scoped and fieldless, like [`DealsDamageToOpponent`](Self::DealsDamageToOpponent) —
    /// the pool's only consumer needs no `who`-style scope (flag-don't-force; widen to a sibling
    /// scope field if a second card needs "your creatures"/"your tokens" the way
    /// [`DealsCombatDamageToPlayer`](Self::DealsCombatDamageToPlayer) does). Fires off
    /// [`Event::CombatDamageDealtToCreature`], never plain [`Event::DamageMarked`] alone — a fight
    /// (CR 701.12) or other noncombat creature damage only emits that, not this marker. The
    /// damaged creature's id rides in [`TriggerContext::damaged_creature`] (CR 603.10a
    /// last-known information) so
    /// [`Effect::DestroyTriggeringDamagedCreature`](crate::Effect::DestroyTriggeringDamagedCreature)
    /// can act on "that creature". See the `Event::CombatDamageDealtToCreature` arm of
    /// [`Game::enqueue_triggers`].
    DealsCombatDamageToCreature,
    /// Whenever a creature dealt damage by this permanent *this turn* dies (CR 603.10a
    /// last-known information, Vampiric Dragon: "Whenever a creature dealt damage by this
    /// creature this turn dies, put a +1/+1 counter on this creature."). Self-scoped and
    /// fieldless, like [`DealsCombatDamageToCreature`](Self::DealsCombatDamageToCreature) — the
    /// pool's only consumer needs no `who`-style scope (flag-don't-force). Unlike
    /// `DealsCombatDamageToCreature`'s single-fire `damaged_creature` context slot, this reads
    /// the persistent turn-scoped tally in [`Game::damaged_this_turn`], populated at *every*
    /// creature-damage choke — combat or noncombat alike (CR 510.2 / 120.3/506) — not just
    /// combat damage, and cleared at the same turn boundary as
    /// [`Game::permanents_died_this_turn`]. See the `Event::DamageMarked` arm of
    /// [`Game::enqueue_triggers`] for where the tally is recorded and the creature-death arms for
    /// where this fires.
    CreatureDealtDamageByThisDies,
    /// Whenever this permanent deals damage to an *opponent* of its controller, combat or
    /// noncombat alike (CR 603.3, Looter il-Kor: "Whenever this creature deals damage to an
    /// opponent, draw a card, then discard a card."). Self-scoped and fieldless — contrast
    /// [`DealsCombatDamageToPlayer`](Self::DealsCombatDamageToPlayer), which is combat-only and
    /// scope-carrying. Fires off [`Event::CombatDamageDealtToPlayer`] (the combat half) and
    /// [`Event::DamageDealtToPlayer`] (the noncombat half — a marker distinct from the
    /// `LifeChanged` it accompanies, since non-damage life loss also emits `LifeChanged` with a
    /// `source`); every player other than the controller is an opponent (CR 102.3). See
    /// [`Game::queue_deals_damage_to_opponent_triggers`].
    DealsDamageToOpponent,
    /// Whenever a player casts a spell matching `filter` (CR: the general form behind
    /// [`Magecraft`](Self::Magecraft) and its kin) — a data-driven cast-watch. `caster` scopes
    /// whose cast counts, relative to the ability's own controller ([`CasterScope::You`] default,
    /// [`CasterScope::Opponent`] — Monologue Tax/Mangara's "an opponent casts", or
    /// [`CasterScope::AnyPlayer`]); `nth_each_turn` restricts to exactly the caster's Nth spell
    /// that turn (CR "their second spell each turn" — `Some(2)`), read off
    /// [`Player::spells_cast_this_turn`] (`None` = every matching cast). A seventh,
    /// bespoke-queued watch flavor like [`DealsCombatDamageToPlayer`](Self::DealsCombatDamageToPlayer):
    /// fires off [`Event::SpellCast`]; see [`Game::queue_cast_spell_triggers`]. Distinct from
    /// [`Magecraft`](Self::Magecraft) — which stays its own fixed instant/sorcery-only, self-only,
    /// every-cast watch and also fires off `SpellCopied`, which this doesn't — rather than folding
    /// Magecraft into this shape, since no `CastSpell` consumer needs the copy half.
    CastSpell {
        filter: SpellFilter,
        caster: CasterScope,
        nth_each_turn: Option<u8>,
        /// Restrict to a spell cast from its controller's hand (CR 601's default cast zone) —
        /// Dirgur Focusmage's "you cast … from your hand": `false` (the default) fires on a cast
        /// from *any* zone (flashback/escape from a graveyard, the command zone, an impulse-play
        /// permission from exile); `true` excludes all of those. Read off the triggering spell's
        /// own [`Spell::cast_from_hand`](crate::Spell::cast_from_hand) — see
        /// [`Game::queue_cast_spell_triggers`].
        from_hand: bool,
    },
    /// Whenever a player draws their Nth card this turn (Faerie Mastermind's "an opponent draws
    /// their second card each turn, you draw a card") — the draw-side twin of
    /// [`CastSpell`](Self::CastSpell): `drawer` scopes whose draw counts relative to the
    /// ability's own controller, `nth_each_turn` restricts to exactly that player's Nth draw
    /// this turn (`None` = every matching draw), read off [`Player::draws_this_turn`]. Fires off
    /// [`Event::CardDrawn`]; see [`Game::queue_player_draws_triggers`].
    /// ponytail: reuses [`CasterScope`] rather than a parallel `PlayerScope` — the enum name says
    /// "caster" but the you/opponent/any-player scope math is identical for draws, and no other
    /// pool card needs a second name for the same three variants.
    PlayerDraws {
        drawer: CasterScope,
        nth_each_turn: Option<u8>,
    },
    /// Whenever a player activates an ability whose activation cost contains `{X}` (CR 707.10 —
    /// Unbound Flourishing's "or activate an ability, if that … ability's activation cost contains
    /// {X}, copy that … ability"), scoped by `caster` relative to this ability's own controller
    /// ([`CasterScope::You`] for Unbound). Fired directly off the activated ability's stack
    /// placement (`{X}`-gated) in [`Game::activate_ability`]; see
    /// [`Game::queue_activate_ability_triggers`]. The triggering ability's source rides in
    /// [`TriggerContext::triggering_ability`] so the payoff can copy it.
    ActivateAbility { caster: CasterScope },
    /// Whenever *another* permanent matching `filter` enters the battlefield, scoped by
    /// `controller` relative to this ability's own controller — the shape behind constellation
    /// (CR 702.76a: "whenever an enchantment you control enters" — [`EnterController::You`],
    /// Ajani's Chosen/Archon of Sun's Grace) and landfall (CR 704.5n's kin: "whenever a land
    /// enters"; [`EnterController::Opponent`] for "a land an opponent controls enters",
    /// Archaeomancer's Map). An eighth, bespoke-queued watch flavor like
    /// [`YouSacrifice`](Self::YouSacrifice): fires off any of the entering-permanent events
    /// ([`Event::PermanentEntered`], `TokenCreated`, `LandPlayed`, `SearchedToBattlefield`,
    /// `ReanimatedToBattlefield`, `PutOntoBattlefieldFromHand`); see
    /// [`Game::queue_permanent_enters_triggers`]. Self-excluded: this is the watch-others
    /// companion to [`Etb`](Self::Etb) — a permanent's own entry never fires its own
    /// `PermanentEnters` ability, only every *other* battlefield permanent's.
    PermanentEnters {
        filter: PermanentFilter,
        controller: EnterController,
    },
    /// Whenever this permanent *or another* permanent matching `filter` enters the battlefield —
    /// [`PermanentEnters`](Self::PermanentEnters) plus a self-fire off the watcher's own entry
    /// (CR 603.6a's kin for "this permanent or another … enters"). Doomwake Giant's
    /// constellation ("this creature or another enchantment you control enters"): mirrors
    /// [`CreatureDiesIncludingThis`](Self::CreatureDiesIncludingThis)'s "plain arm plus self-fire"
    /// shape, but the entering permanent is still on the battlefield (unlike a death's
    /// last-known-information snapshot), so the self-fire reads it directly rather than off a
    /// caller-supplied snapshot — see `Game::queue_self_permanent_enters_trigger`.
    PermanentEntersIncludingThis {
        filter: PermanentFilter,
        controller: EnterController,
    },
    /// Whenever one or more cards leave the controller's graveyard (Quintorius Field Historian /
    /// Lorehold's mechanic) — a controller-scoped trigger like [`Upkeep`](Self::Upkeep), but
    /// batch-once: reanimating several cards, or emptying a graveyard with one effect, is a
    /// single "cards leave" event, not one fire per card (CR "one or more"). Fires off
    /// [`Game::create_object`]'s graveyard-exit detection, drained once per event batch by
    /// [`Game::enqueue_triggers`]; see `Game::graveyard_exits_this_batch`.
    CardsLeaveYourGraveyard,
    /// Whenever one or more cards are put into exile from the controller's library and/or their
    /// graveyard (Laelia, the Blade Reforged's growth trigger) — the exile-destination twin of
    /// [`CardsLeaveYourGraveyard`](Self::CardsLeaveYourGraveyard), same batch-once controller-scoped
    /// shape (CR "one or more"). Fires off [`Game::create_object`]'s exile-destination detection,
    /// drained once per event batch by [`Game::enqueue_triggers`]; see
    /// `Game::library_or_graveyard_exits_this_batch`.
    CardsExiledFromYourLibraryOrGraveyard,
    /// Whenever the controller creates one or more creature tokens (CR 603.3b's "one or more" —
    /// Staff of the Storyteller's "whenever you create one or more creature tokens, put a story
    /// counter on this artifact") — the token twin of
    /// [`CardsLeaveYourGraveyard`](Self::CardsLeaveYourGraveyard)'s batch-once controller-scoped
    /// shape. Fires off [`Event::TokenCreated`], gated to creature tokens and deduped to once per
    /// event batch by [`Game::enqueue_triggers`]; see
    /// `Game::creature_tokens_created_this_batch`.
    /// ponytail: fieldless — "you" is the only scope any pool card needs (flag-don't-force; add
    /// an opponent/any-player scope the moment a second consumer wants one).
    YouCreateToken,
    /// Whenever this permanent becomes the target of a spell (CR 603.2c "becomes the
    /// target"), fired under the *targeted permanent's own controller* — Goldspan Dragon.
    /// A self-referential watch, like [`Etb`](Self::Etb)/[`Attacks`](Self::Attacks): scoped
    /// to the permanent itself, not "any permanent"/"another permanent", since the only pool
    /// consumer needs no filter (flag-don't-force — add one the moment a second consumer
    /// wants it). Fires off [`Event::SpellCast`]'s carried `target`; see
    /// [`Game::queue_becomes_targeted_triggers`].
    /// ponytail: the engine's spells carry a single [`Target`] (multi-target spells are
    /// unlanded), so this fires at most once per spell cast — faithful for Goldspan, but a
    /// hypothetical spell targeting *this* permanent among several targets would need
    /// per-target firing once multi-target lands.
    BecomesTargeted,
    /// "When you cast this spell" (CR 601.2i/603.3): a triggered ability on the spell's *own*
    /// text that goes on the stack above the spell the moment it's cast, controlled by the
    /// caster — Hydroid Krasis's "you gain half X life and draw half X cards, rounded down."
    /// Scanned off the cast card's own `def` (not a battlefield watcher) at `Event::SpellCast`;
    /// see [`Game::queue_trigger_group`]'s call site in `enqueue_triggers`. Because it's a
    /// separate object from the spell, it resolves independently — including if the spell is
    /// later countered (CR 702.137a for Hydroid specifically). Fieldless: every pool consumer is
    /// self-only ("this spell"), so no filter/scope axis exists yet (flag-don't-force).
    YouCastThis,
    /// When this permanent is put into a graveyard from the battlefield (CR: "is put into a
    /// graveyard from the battlefield" — Fallen Ideal's Aura-death rider). A self-referential
    /// twin of [`Dies`](Self::Dies) that isn't creature-scoped: [`Dies`] gates on
    /// `CardKind::Creature`, which is exactly why an Aura's own graveyard-bound trigger doesn't
    /// use it. Fires off [`Event::MovedToGraveyard`] (and [`Event::TokenCeasedToExist`] for
    /// symmetry, though no pool token authors this timing yet), guarded to objects that were a
    /// live battlefield [`Object::Permanent`] the instant they moved — captured at
    /// `Game::apply`'s `MovedToGraveyard` choke point (by the time `Game::enqueue_triggers` runs
    /// the pre-move object has already been overwritten into `Object::Moved`), so a milled or
    /// discarded copy of the same card does not fire it. Fieldless: only Fallen Ideal authors
    /// this timing, so the def-scan can't misfire on any other card.
    ThisAuraLeaves,
    /// When this permanent leaves the battlefield to *any* zone (destroy/exile → graveyard/exile,
    /// bounce → hand, tuck → library — not just the graveyard-only
    /// [`ThisAuraLeaves`](Self::ThisAuraLeaves)) — Animate Dead's "When this Aura leaves the
    /// battlefield, that creature's controller sacrifices it." A self-referential trigger, like
    /// [`ThisAuraLeaves`](Self::ThisAuraLeaves), but scoped to whichever permanent this was
    /// attached to at the instant it left (CR 603.10a last-known information), carried in
    /// [`TriggerContext::left_battlefield_host`] rather than read live (the attachment is gone by
    /// the time the trigger resolves). Fires off [`Event::MovedToGraveyard`], [`Event::MovedToExile`],
    /// [`Event::ReturnedToHand`], [`Event::TuckedToLibrary`], and [`Event::TokenCeasedToExist`] —
    /// see `Game::queue_leaves_battlefield_triggers`. Fieldless: only Animate Dead authors this
    /// timing so far (flag-don't-force).
    ThisPermanentLeavesBattlefield,
    /// Whenever one or more creatures the ability's own controller controls, each with **base**
    /// power 0, deal combat damage to a player (CR 510.2/603.3b, Primo, the Unbounded: "Whenever
    /// one or more creatures you control with base power 0 deal combat damage to a player,
    /// create a … Fractal … token. Put a number of +1/+1 counters on it equal to the damage
    /// dealt.") — a batch trigger like [`YouCreateToken`](Self::YouCreateToken): fires once per
    /// defending player this combat, not once per qualifying attacker, with every qualifying
    /// attacker's damage to that player summed into [`TriggerContext::combat_damage`] (CR
    /// 603.10a last-known information, filled by [`fill_combat_damage`](super::fill_combat_damage)
    /// same as [`DealsCombatDamageToPlayer`](Self::DealsCombatDamageToPlayer)). See
    /// [`Game::queue_zero_base_power_combat_damage_triggers`].
    /// ponytail: the base-power-0 filter is hard-coded to Primo's own predicate (no sibling
    /// `PermanentFilter`/power-threshold field) — the pool's only consumer. Widen to a filter
    /// field the moment a second card needs a different power/type predicate. (CR 510, CR 111, CR 108.3)
    ZeroBasePowerCreaturesYouControlDealCombatDamage,
    /// Whenever this permanent's controller spends mana this permanent produced to cast a
    /// qualifying spell (CR — "When you spend this mana to cast …", Study Hall / Path of Ancestry /
    /// Opal Palace). A source-scoped, provenance-driven watch: unlike the battlefield-scan triggers,
    /// this fires only for the *specific* land whose tagged mana ([`Player::mana_provenance`](crate::state))
    /// paid, and only when that mana funds a cast the [`SpendToCastPredicate`] accepts. Placed at
    /// the cast payment choke off [`Event::SpellCast`] (the preceding [`Event::ManaSpent`] carries
    /// the spend multiset); see [`Game::queue_spend_to_cast_triggers`].
    SpendManaToCast { predicate: SpendToCastPredicate },
    /// Whenever the ability's controller loses life for the first time each turn (Intermediate
    /// Chirography's level-2 "whenever you lose life for the first time each turn, put a +1/+1
    /// counter on target creature you control"). A controller-scoped trigger like
    /// [`Upkeep`](Self::Upkeep), fired off a life-loss [`Event::LifeChanged`] (a *decrease*, CR
    /// 118.9/119.3) only when that loss is the turn's first for the player — see
    /// [`Player::life_losses_this_turn`](crate::state) and the queueing in
    /// [`Game::enqueue_triggers`]. Fieldless: "you" is the only scope the pool needs
    /// (flag-don't-force).
    YouLoseLifeFirstTimeEachTurn,
    /// Whenever a player casts a spell matching `filter` whose only target is this permanent
    /// (CR 603.2c "becomes the target" narrowed to "targets only" — Mirrorwing Dragon: "Whenever
    /// a player casts an instant or sorcery spell that targets only this creature, that player
    /// copies that spell for each other creature they control that the spell could target.").
    /// A self-referential watch like [`BecomesTargeted`](Self::BecomesTargeted), reusing the same
    /// single-[`Target`] [`Event::SpellCast`] field (so, per `BecomesTargeted`'s own ponytail, a
    /// multi-target spell's post-cast target selection isn't visible here — it fires only for a
    /// spell whose spec is itself single-target). Fires under the *targeted permanent's own
    /// controller*, same as `BecomesTargeted`; see [`Game::queue_spell_targets_this_only_triggers`].
    SpellTargetsThisOnly { filter: SpellFilter },
    /// "When you cycle this card" (CR 702.29e): a triggered ability on the cycled card's *own*
    /// text that fires off the cycling activation's discard, stacking above the fixed "draw a
    /// card" (Krosan Tusker's "you may search your library for a basic land card … (Do this
    /// before you draw.)"; Decree of Justice's "you may pay {X}. If you do, create X 1/1 white
    /// Soldier creature tokens"). Scanned off the cycled card's own `def` at the cycling choke —
    /// not a battlefield watcher — mirroring [`YouCastThis`](Self::YouCastThis)'s self-scan; see
    /// [`Game::cycle`](crate::Game::cycle). Fieldless: every pool consumer is self-only ("this
    /// card"), so no filter/scope axis exists yet (flag-don't-force).
    Cycled,
}

/// Which cast a [`Trigger::SpendManaToCast`] watch accepts as "this mana was spent to cast …",
/// relative to the source's own controller. Only the two predicates the three real cards print
/// (flag-don't-force).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum SpendToCastPredicate {
    /// "…to cast your commander" (Study Hall, Opal Palace) — the cast spell is the controller's
    /// own commander.
    Commander,
    /// "…to cast a creature spell that shares a creature type with your commander" (Path of
    /// Ancestry).
    CreatureSharingTypeWithCommander,
}

/// Whose permanent a [`Trigger::PermanentEnters`] watch cares about, relative to the ability's
/// own controller (mirrors [`CasterScope`]'s shape).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum EnterController {
    /// The ability's own controller (default) — constellation's "an enchantment you control".
    #[default]
    You,
    /// Any opponent of the ability's controller — Archaeomancer's Map's "a land an opponent
    /// controls".
    Opponent,
    /// Any player, including the ability's own controller — plain landfall's "a land enters".
    AnyPlayer,
}

/// Whose cast a [`Trigger::CastSpell`] watch cares about, relative to the ability's own
/// controller (mirrors [`CombatDamageScope`]'s shape).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum CasterScope {
    /// The ability's own controller (default).
    #[default]
    You,
    /// Any opponent of the ability's controller.
    Opponent,
    /// Any player, including the ability's own controller.
    AnyPlayer,
}

/// Whose combat damage a [`Trigger::DealsCombatDamageToPlayer`] watch cares about, relative to
/// the ability's own source/controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum CombatDamageScope {
    /// Only the ability's own source (default).
    #[default]
    This,
    /// Any creature the ability's controller controls.
    YourCreatures,
    /// Any creature *token* the ability's controller controls.
    YourTokens,
}

/// What a triggering event exposes to an intervening-if condition and to a watch-others effect:
/// the ability's controller, and — for a `PlayerAttacksYourOpponent` trigger — the attacking and
/// attacked players. Most triggers only need the controller.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TriggerContext {
    pub(crate) controller: PlayerId,
    /// The active player, for a [`Trigger::EachDrawStep`] ability's "**that player** draws an
    /// additional card" payoff (Howling Mine) — the player whose draw step this is, which need
    /// not be this ability's controller. `None` for every other trigger; set by
    /// [`Game::queue_each_draw_step_triggers`].
    pub(crate) active_player: Option<PlayerId>,
    /// `(attacking player, attacked player)` for a `PlayerAttacksYourOpponent` trigger.
    pub(crate) attack: Option<(PlayerId, PlayerId)>,
    /// The graveyard-object id of the card just discarded, for a `YouDiscard` trigger (so its
    /// effect can act on "that card" — Containment Construct). See
    /// [`Game::queue_discard_triggers`].
    pub(crate) discarded: Option<ObjectId>,
    /// The id of the permanent that just entered, for a `PermanentEnters`/
    /// `PermanentEntersIncludingThis` trigger (so its effect can act on "it" — Marauding
    /// Raptor's "this creature deals 2 damage to it"). See
    /// [`Game::queue_permanent_enters_triggers`].
    pub(crate) entering: Option<ObjectId>,
    /// CR 603.10a last-known information: the dying source's power/+1/+1-counter count the
    /// instant before it died, for a `Dies` trigger's `Amount::SourcePower`/
    /// `Amount::PerCounterOnSource` reads (Lifeblood Hydra, Hangarback Walker). `None` for every
    /// non-death trigger, so a living source's amount reads stay live. See
    /// `Game::dying_creature_stats` for where this is captured.
    pub(crate) dying_source_stats: Option<(i32, i32)>,
    /// The triggering spell's mana value (CR 202.3), for a `Trigger::CastSpell` (magecraft)
    /// ability's `Amount::TriggeringSpellManaValue`/`Condition::TriggeringSpellManaValueAtLeast`
    /// reads (Prismari Pianist's "if that spell's mana value is 5 or greater"; Renegade Bull's
    /// "+X/+0 … where X is that spell's mana value"). `None` for every non-`CastSpell` trigger,
    /// so other triggers' amount/condition reads are unaffected. Locked in at trigger placement,
    /// same CR 603.4 reasoning as `dying_source_stats` above. See
    /// `Game::queue_cast_spell_triggers` for where this is captured.
    pub(crate) cast_mana_value: Option<u32>,
    /// The mana actually spent to cast the triggering spell (CR 601.2h/202.3), for a
    /// `Trigger::CastSpell` ability's `Amount::TriggeringSpellManaSpent` reads (Manaform
    /// Hellkite's "X is the amount of mana spent to cast that spell") — the mana-*spent* sibling
    /// of `cast_mana_value` above (which reads the printed mana value, treating `{X}` as 0).
    /// `None` for every non-`CastSpell` trigger. Locked in at trigger placement from the
    /// preceding `Event::ManaSpent` in the same batch, same CR 603.4 last-known-information
    /// reasoning as `cast_mana_value`. See `Game::queue_cast_spell_triggers` for where this is
    /// captured.
    pub(crate) cast_mana_spent: Option<u32>,
    /// The casting spell's chosen `{X}` (CR 603.4), for a [`Trigger::YouCastThis`] self-cast
    /// ability's `Amount::X`/`Amount::HalfXRoundedDown` reads (Hydroid Krasis's "you gain half X
    /// life and draw half X cards, rounded down"), a [`Trigger::CastSpell`] watcher's own `X`
    /// read (Nev's "put X +1/+1 counters"), or a self [`Trigger::Etb`]'s `Amount::X`/`HalfX`
    /// read off the entering permanent's own already-placed counters (The Goose Mother's
    /// "create half X Food tokens" — see [`Game::queue_self_trigger`]). `None` for every other
    /// trigger. Locked in at placement — the ability is a separate stack object from the spell
    /// (CR 601.2i), so it resolves even if the spell is later countered. See
    /// `Game::enqueue_triggers`'s `Event::SpellCast` arm, `Game::queue_cast_spell_triggers`, and
    /// `Game::queue_self_trigger` for where this is captured.
    pub(crate) cast_x: Option<u32>,
    /// How many Auras the watcher's controller controlled that were attached to the dying
    /// creature, for a [`Trigger::AnEnchantedCreatureDies`] watch's
    /// `Amount::AurasYouControlledAttachedToDyingCreature` reads (Hateful Eidolon). `None` for
    /// every other trigger. Locked in at placement — same CR 603.10a last-known-information
    /// reasoning as `dying_source_stats` above. See
    /// [`Game::queue_an_enchanted_creature_dies_triggers`] for where this is captured.
    pub(crate) auras_you_controlled_attached_to_dying_creature: Option<u32>,
    /// CR 510.2/603.10a last-known information: the amount of combat damage the source just
    /// dealt to a player, for a [`Trigger::DealsCombatDamageToPlayer`] watch's reanimation target
    /// bound (Venerable Warsinger: "return target creature card with mana value X or less …
    /// where X is the amount of damage this creature dealt to that player"). `None` for every
    /// other trigger, same shape as `dying_source_stats` above. See
    /// [`Game::queue_combat_damage_triggers`] for where this is captured.
    pub(crate) combat_damage: Option<i32>,
    /// CR 609.7/603.10a last-known information: the amount of damage the enchanted host just
    /// dealt (combat or noncombat alike), for a [`Trigger::EnchantedCreatureDealsDamage`] watch's
    /// `Amount::TriggeringDamageDealt` reads (Armadillo Cloak's "you gain that much life"). `None`
    /// for every other trigger, same shape as `combat_damage` above (which this doesn't reuse: that
    /// field is specifically CR 510.2 combat damage *to a player*). See
    /// [`Game::queue_enchanted_creature_deals_damage_triggers`] for where this is captured.
    pub(crate) triggering_damage_dealt: Option<i32>,
    /// The dying creature's graveyard-object id, for a [`Trigger::EnchantedCreatureDies`]
    /// ability's look-back reanimation payoff (Changing Loyalty's "return it to the battlefield
    /// under your control", Gift of Immortality's "return that card … under its owner's
    /// control") — CR 603.10a last-known information's "that card". `None` for every other
    /// trigger. See [`Game::queue_enchanted_creature_dies_triggers`] for where this is captured;
    /// `def_of`/`owner_of`/`zone_of` all still resolve this id correctly after the death (they
    /// follow the object's `Moved` lineage into its new graveyard card, and on into wherever it
    /// moves next).
    pub(crate) dying_enchanted_creature: Option<ObjectId>,
    /// CR 510.2/603.10a last-known information: the creature a [`Trigger::DealsCombatDamageToCreature`]
    /// watch's source just dealt combat damage to (Stinkweed Imp's "destroy that creature"),
    /// named separately from `dying_enchanted_creature`/`dead_creature` above since the damaged
    /// creature need not be dead or even still on the battlefield. `None` for every other
    /// trigger. Feeds [`Effect::DestroyTriggeringDamagedCreature`] via `contextualize_effect`;
    /// `def_of`/`owner_of`/`zone_of` all still resolve it whether it's still a live permanent or
    /// has since left. See the `Event::CombatDamageDealtToCreature` arm of
    /// [`Game::enqueue_triggers`] for where this is captured. Named so a future "all damage this
    /// source dealt this turn" generalization (fidelity increment #194) can extend it.
    pub(crate) damaged_creature: Option<ObjectId>,
    /// The triggering spell's stack object id, for a delayed [`Trigger::CastSpell`] one-shot
    /// armed by [`Effect::ScheduleNextCastTrigger`] whose `then` copies that spell (Thunderclap
    /// Drake's "when you next cast an instant or sorcery spell this turn, copy it") — CR 603.4
    /// last-known information, same shape as `cast_x` above but naming the spell itself rather
    /// than its `{X}`. `None` for every other trigger. See
    /// [`Game::fire_next_cast_triggers`] for where this is captured.
    pub(crate) triggering_spell: Option<ObjectId>,
    /// CR 702.40a's storm count: the game-wide tally of spells cast before this one this turn
    /// (every player, not just the caster), for a [`Trigger::YouCastThis`] Storm ability's
    /// `Amount::SpellsCastBeforeThisThisTurn` reads (Reaping the Graves). Locked in when the
    /// trigger goes on the stack — same CR 603.4 last-known-information shape as `cast_x` above,
    /// so a response cast in front of the storm trigger (or the storm spell's own later-minted
    /// copies) can't inflate it. `None` for every other trigger. See `Game::enqueue_triggers`'s
    /// `Event::SpellCast` arm for where this is captured.
    pub(crate) spells_cast_before_this: Option<u32>,
    /// CR 510.2/603.10a last-known information: the trigger's own source permanent's power at the
    /// instant the trigger goes on the stack, for an [`Trigger::Attacks`] ability's reanimation
    /// target bound (Guardian Scalelord: "return target nonland permanent card with mana value X
    /// or less … where X is this creature's power"). `None` for every trigger that doesn't read
    /// its source's power, same shape as `combat_damage` above. See the `Event::AttackerDeclared`
    /// arm of `Game::enqueue_triggers` for where this is captured.
    pub(crate) source_power: Option<i32>,
    /// CR 603.10a last-known information: the just-dead creature's graveyard-object id, for a
    /// [`Trigger::CreatureYouControlDies`]-family watch's exile-and-copy payoff (Hofri Ghostforge's
    /// "exile it. If you do, create a token that's a copy of that creature"). `None` for every
    /// trigger that doesn't read the dead creature's id. Feeds
    /// [`Effect::ExileDeadCreatureCreateCopyWithSubtype`] via `contextualize_effect`; `def_of`/
    /// `owner_of`/`zone_of` all still resolve it after the death (following the `Moved` lineage into
    /// its new graveyard card). See [`Game::queue_death_watcher`] for where this is captured.
    pub(crate) dead_creature: Option<ObjectId>,
    /// CR 603.10a last-known information: the dying permanent's own card types, for a
    /// [`Trigger::NonlandPermanentYouControlDiesIncludingThis`] watch's dynamic edict payoff
    /// (Martyr's Bond's "each opponent sacrifices a permanent that shares a card type with it").
    /// `None` for every other trigger. Feeds `Effect::EachPlayerSacrifices`'s
    /// `shares_type_with_dying_permanent`-marked filter via `contextualize_effect`; see
    /// [`Game::queue_nonland_permanent_death_watchers`] for where this is captured.
    pub(crate) dying_permanent_types: Option<TypeSet>,
    /// CR 603.10a last-known information: the graveyard-object ids of the cards that left this
    /// batch, for a [`Trigger::CardsLeaveYourGraveyard`] payoff that becomes a copy of one of them
    /// (Spirit of Resilience's "become a copy of an artifact or creature card from among those
    /// cards"). `&[]` for every other trigger. Feeds
    /// [`Effect::PutCounterThenMayBecomeCopyOfCardFromList`] via `contextualize_effect`'s
    /// `fill_cards_left_graveyard`; `def_of` still resolves each id after the move (following the
    /// `Moved` lineage). See `Game::queue_cards_leave_graveyard_triggers` for where this is set.
    /// ponytail: a leaked `&'static [ObjectId]` interned per fire so [`TriggerContext`] stays
    ///   `Copy`; move to a runtime carrier if a long game's repeated fires make the leak matter.
    pub(crate) cards_left_graveyard: &'static [ObjectId],
    /// CR 603.10a last-known information: the permanent this object was attached to the instant
    /// it left the battlefield, for a [`Trigger::ThisPermanentLeavesBattlefield`] ability's
    /// payoff (Animate Dead's "that creature's controller sacrifices it"). `None` for every
    /// other trigger, and also `None` for a `ThisPermanentLeavesBattlefield` fire off a permanent
    /// that wasn't itself attached to anything. See `Game::queue_leaves_battlefield_triggers` for
    /// where this is captured.
    pub(crate) left_battlefield_host: Option<ObjectId>,
    /// The source permanent of the activated ability that fired a [`Trigger::ActivateAbility`]
    /// watch (Unbound Flourishing), for its [`Effect::CopyTriggeringAbility`] payoff — the copy
    /// finds that ability still on the stack (its trigger sits directly above it, CR 603.3b) by
    /// this source and copies its effect/target/`{X}`. `None` for every other trigger. See
    /// [`Game::queue_activate_ability_triggers`] for where this is captured.
    pub(crate) triggering_ability: Option<ObjectId>,
    /// The player who cast the triggering spell, for a [`Trigger::CastSpell`] watch's own
    /// "unless that player pays" payoff (Rhystic Study's "you may draw a card unless that player
    /// pays {1}" — [`Effect::MayDrawUnlessPays`]). Distinct from `TriggerContext::controller`
    /// (the watcher's own controller) precisely when `caster: CasterScope::Opponent`/`AnyPlayer`
    /// — `controller` alone can't name the payer for those scopes. `None` for every other
    /// trigger. See [`Game::queue_cast_spell_triggers`] for where this is captured.
    pub(crate) triggering_caster: Option<PlayerId>,
}

impl TriggerContext {
    /// Context for a trigger whose only relevant player is its controller.
    pub(crate) fn of(controller: PlayerId) -> Self {
        Self {
            controller,
            active_player: None,
            attack: None,
            discarded: None,
            entering: None,
            dying_source_stats: None,
            cast_mana_value: None,
            cast_mana_spent: None,
            cast_x: None,
            auras_you_controlled_attached_to_dying_creature: None,
            combat_damage: None,
            triggering_damage_dealt: None,
            dying_enchanted_creature: None,
            damaged_creature: None,
            triggering_spell: None,
            spells_cast_before_this: None,
            source_power: None,
            dead_creature: None,
            dying_permanent_types: None,
            cards_left_graveyard: &[],
            left_battlefield_host: None,
            triggering_ability: None,
            triggering_caster: None,
        }
    }
}
