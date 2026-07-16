//! Static English labels for effects — the human phrase shown on the stack panel, the game
//! log, and the deck-builder catalog. A label is a property of the [`Effect`] *data* (like
//! [`Effect::target`]), so it lives here in the engine beside the effect; the wire layer only
//! assembles per-viewer render text around these labels.
//!
//! No CR chapter ownership — presentation only.

use crate::*;

/// An effect amount for a label: a plain number, `X`, or a phrase for a derived value.
fn amount_label(amount: Amount) -> String {
    match amount {
        Amount::Fixed(n) => n.to_string(),
        Amount::X => "X".to_string(),
        Amount::HalfX => "half X".to_string(),
        Amount::HalfXRoundedDown => "half X, rounded down".to_string(),
        Amount::TwiceX => "twice X".to_string(),
        Amount::PerCreatureYouControl => "1 per creature you control".to_string(),
        Amount::PerCreatureOnBattlefield => "1 per creature on the battlefield".to_string(),
        Amount::PerPermanentMatching { filter, zone } => {
            let where_ = match zone {
                AmountZone::Battlefield => "on the battlefield",
                AmountZone::Graveyard => "in the graveyard",
            };
            format!("1 per {} {where_}", permanent_filter_label(filter))
        }
        Amount::SourcePower => "its power".to_string(),
        Amount::SourceToughness => "its toughness".to_string(),
        Amount::TargetPower => "target's power".to_string(),
        Amount::TargetToughness => "target's toughness".to_string(),
        Amount::TargetManaValue => "target's mana value".to_string(),
        Amount::PerCounterOnSource => "1 per +1/+1 counter on it".to_string(),
        Amount::PerCounterOfKindOnSource { kind } => {
            let kind_name = match kind {
                CounterKind::Charge => "charge",
                CounterKind::Story => "story",
                CounterKind::Study => "study",
                CounterKind::Vow => "vow",
                CounterKind::Time => "time",
            };
            format!("1 per {kind_name} counter on it")
        }
        Amount::LifeGainedThisTurn => "life gained this turn".to_string(),
        Amount::SpellsCastThisTurn => "spells cast this turn".to_string(),
        Amount::CardsInTargetPlayerHand => "1 per card in target opponent's hand".to_string(),
        Amount::CardsInYourHand => "1 per card in your hand".to_string(),
        Amount::CommanderCastsFromCommandZone => {
            "the number of times they've cast their commander from the command zone".to_string()
        }
        Amount::CreaturesDiedThisTurn => {
            "1 per creature that died under your control this turn".to_string()
        }
        Amount::NontokenCreaturesEnteredThisTurn => {
            "1 per nontoken creature that entered the battlefield under your control this turn"
                .to_string()
        }
        Amount::SacrificedCreaturePower => "the sacrificed creature's power".to_string(),
        Amount::SacrificedCreatureToughness => "the sacrificed creature's toughness".to_string(),
        Amount::CommanderColorCount => {
            "the number of colors in your commander's color identity".to_string()
        }
        Amount::TotalPowerYouControl => "the total power of creatures you control".to_string(),
        Amount::IfCondition { then, .. } => amount_label(*then),
        Amount::TriggeringSpellManaValue => "that spell's mana value".to_string(),
        Amount::TriggeringSpellManaSpent => {
            "the amount of mana spent to cast that spell".to_string()
        }
        Amount::SpellSacrificeCount => "1 per creature sacrificed this way".to_string(),
        Amount::PermanentsDiedThisTurn => {
            "1 per permanent put into a graveyard from the battlefield this turn".to_string()
        }
        Amount::PermanentsDestroyedThisWay { filter } => {
            format!(
                "1 per {} destroyed this way",
                permanent_filter_label(filter)
            )
        }
        Amount::NonlandCardsExiledThisWay => "1 per nonland card exiled this way".to_string(),
        Amount::PastVotes => "1 per past vote".to_string(),
        Amount::PresentVotes => "1 per present vote".to_string(),
        Amount::TotalManaValueMilledThisWay => {
            "the total mana value of cards milled this way".to_string()
        }
        Amount::ExiledCardManaValueThisWay => "that card's mana value".to_string(),
        Amount::AurasYouControlledAttachedToDyingCreature => {
            "1 per Aura you controlled that was attached to it".to_string()
        }
        Amount::IfSpellKicked { then, else_ } => {
            format!(
                "{} if kicked, otherwise {}",
                amount_label(*then),
                amount_label(*else_)
            )
        }
        Amount::GreatestInstantOrSorceryManaValueCastThisTurn => {
            "the greatest mana value among instant and sorcery spells you've cast this turn"
                .to_string()
        }
        Amount::OnePlusInstantsAndSorceriesCastThisTurn => {
            "one plus the number of instant and sorcery spells you've cast this turn".to_string()
        }
        Amount::AurasAttachedToSource => "1 per Aura attached to it".to_string(),
        Amount::InstantOrSorceryCardsInYourGraveyard => {
            "1 per instant or sorcery card in your graveyard".to_string()
        }
        Amount::CombatDamageDealt => "the damage dealt".to_string(),
        Amount::TriggeringDamageDealt => "that much".to_string(),
    }
}

/// A comma-joined human label for a keyword grant (`indestructible`, `flying, vigilance`).
fn keyword_list_label(keywords: &[Keyword]) -> String {
    keywords
        .iter()
        .map(|k| format!("{k:?}").to_lowercase())
        .collect::<Vec<_>>()
        .join(", ")
}

/// A human label for a cost-reducer's spell filter (the "spells you cast" clause).
fn spell_filter_label(filter: SpellFilter) -> &'static str {
    match filter {
        SpellFilter::AllSpells => "Spells you cast",
        SpellFilter::CreatureSpells => "Creature spells you cast",
        SpellFilter::NoncreatureSpells => "Noncreature spells you cast",
        SpellFilter::SpellsThatTargetACreature => "Spells you cast that target a creature",
        SpellFilter::Aura => "Aura spells you cast",
        SpellFilter::InstantOrSorcery => "Instant and sorcery spells you cast",
        SpellFilter::Enchantment => "Enchantment spells you cast",
        SpellFilter::ArtifactOrEnchantment => "Artifact and enchantment spells you cast",
        SpellFilter::HasSubtype(_) => "Spells you cast",
        SpellFilter::HasXInCost => "Spells you cast with {X} in their mana cost",
        SpellFilter::InstantOrSorceryWithXInCost => {
            "Instant and sorcery spells you cast with {X} in their mana cost"
        }
        SpellFilter::Historic => "Historic spells you cast",
        SpellFilter::AuraTargetsModifiedPermanentYouControl => {
            "Aura spells you cast that target a modified permanent you control"
        }
        SpellFilter::CastFromNonHandZone => "Spells you cast from anywhere other than your hand",
        SpellFilter::Color(Color::White) => "White spells you cast",
        SpellFilter::Color(Color::Blue) => "Blue spells you cast",
        SpellFilter::Color(Color::Black) => "Black spells you cast",
        SpellFilter::Color(Color::Red) => "Red spells you cast",
        SpellFilter::Color(Color::Green) => "Green spells you cast",
    }
}

/// The noun phrase for a [`Effect::CounterTargetSpell`]'s `filter` ("spell", "noncreature
/// spell", "artifact or enchantment spell") — the object of "Counter target …".
fn counter_target_spell_noun(filter: SpellFilter) -> String {
    match filter {
        SpellFilter::AllSpells => "spell".to_string(),
        SpellFilter::CreatureSpells => "creature spell".to_string(),
        SpellFilter::NoncreatureSpells => "noncreature spell".to_string(),
        SpellFilter::SpellsThatTargetACreature => "spell that targets a creature".to_string(),
        SpellFilter::Aura => "Aura spell".to_string(),
        SpellFilter::InstantOrSorcery => "instant or sorcery spell".to_string(),
        SpellFilter::Enchantment => "enchantment spell".to_string(),
        SpellFilter::ArtifactOrEnchantment => "artifact or enchantment spell".to_string(),
        SpellFilter::HasSubtype(subtypes) => format!("{} spell", subtypes.join("/")),
        SpellFilter::HasXInCost => "spell with {X} in its mana cost".to_string(),
        SpellFilter::InstantOrSorceryWithXInCost => {
            "instant or sorcery spell with {X} in its mana cost".to_string()
        }
        SpellFilter::Historic => "historic spell".to_string(),
        SpellFilter::AuraTargetsModifiedPermanentYouControl => {
            "Aura spell that targets a modified permanent you control".to_string()
        }
        SpellFilter::CastFromNonHandZone => {
            "spell cast from anywhere other than your hand".to_string()
        }
        SpellFilter::Color(color) => format!("{color:?} spell").to_lowercase(),
    }
}

/// A short human phrase for a composable permanent filter (for effect descriptions like
/// "Destroy all …"). Names the type set, then any controller/token/enchanted/mana-value axes.
fn permanent_filter_label(filter: PermanentFilter) -> String {
    let types = filter.types;
    let mut base = if types.is_empty() {
        "permanents".to_string()
    } else if types == TypeSet::NONLAND {
        "nonland permanents".to_string()
    } else {
        let mut names = Vec::new();
        for (bit, name) in [
            (TypeSet::CREATURE, "creature"),
            (TypeSet::ARTIFACT, "artifact"),
            (TypeSet::ENCHANTMENT, "enchantment"),
            (TypeSet::PLANESWALKER, "planeswalker"),
            (TypeSet::LAND, "land"),
        ] {
            if types.intersects(bit) {
                names.push(name);
            }
        }
        format!("{}s", names.join("/"))
    };

    if let TokenFilter::Nontoken = filter.token {
        base = format!("nontoken {base}");
    } else if let TokenFilter::Token = filter.token {
        base = format!("{base} tokens");
    }
    match filter.controller {
        FilterController::You => base = format!("{base} you control"),
        FilterController::Opponent => base = format!("{base} an opponent controls"),
        FilterController::Any => {}
    }
    if filter.enchanted == Some(true) {
        base = format!("enchanted {base}");
    } else if filter.enchanted == Some(false) {
        base = format!("{base} that aren't enchanted");
    }
    if let Some(max) = filter.mv_max {
        base = format!("{base} with mana value {max} or less");
    }
    base
}

/// A short human-readable description of a [`CardFilter`], shared by a library search's payoff
/// (`SearchLibrary`) and a mass graveyard-return's ("Return all …") disambiguation.
fn card_filter_label(filter: CardFilter) -> String {
    match filter {
        CardFilter::BasicLand => "a basic land".to_string(),
        CardFilter::Land => "a land".to_string(),
        CardFilter::Nonland => "a nonland card".to_string(),
        CardFilter::Creature => "a creature".to_string(),
        CardFilter::AnyCard => "a card".to_string(),
        CardFilter::LandWithSubtype(subtypes) => format!("a {} card", subtypes.join(" or ")),
        CardFilter::BasicLandWithSubtype(subtypes) => {
            format!("a basic {} card", subtypes.join(" or "))
        }
        CardFilter::PermanentWithManaValueAtMost(max) => {
            format!("a permanent card with mana value {max} or less")
        }
        CardFilter::NonlandPermanentWithManaValueAtMost(max) => {
            format!("a nonland permanent card with mana value {max} or less")
        }
        CardFilter::ArtifactOrCreatureWithManaValueAtMost(max) => {
            format!("an artifact or creature card with mana value {max} or less")
        }
        CardFilter::CreatureWithManaValueAtMost(max) => {
            format!("a creature card with mana value {max} or less")
        }
        CardFilter::ArtifactCreatureOrNonAuraEnchantmentWithManaValueAtMost(max) => format!(
            "an artifact, creature, or non-Aura enchantment card with mana value {max} or less"
        ),
        CardFilter::InstantOrSorcery => "an instant or sorcery card".to_string(),
        CardFilter::Enchantment => "enchantment cards".to_string(),
        CardFilter::Permanent => "a permanent card".to_string(),
        CardFilter::NoncreatureNonland => "a noncreature, nonland card".to_string(),
        // ponytail: placeholder — always rewritten to `CreatureWithManaValueAtMost` at trigger
        // placement before a label is ever read for it (see the variant doc).
        CardFilter::CreatureWithManaValueAtMostCombatDamage => {
            "a creature card with mana value equal to the combat damage dealt or less".to_string()
        }
        // ponytail: placeholder — always rewritten to `NonlandPermanentWithManaValueAtMost` at
        // trigger placement before a label is ever read for it (see the variant doc).
        CardFilter::NonlandPermanentWithManaValueAtMostSourcePower => {
            "a nonland permanent card with mana value equal to this creature's power or less"
                .to_string()
        }
        CardFilter::AuraOrEquipment => "an Aura or Equipment card".to_string(),
        CardFilter::Aura => "an Aura card".to_string(),
        CardFilter::ArtifactOrCreature => "an artifact or creature card".to_string(),
        CardFilter::ArtifactOrEnchantment => "an artifact or enchantment card".to_string(),
    }
}

impl Effect {
    /// A short human-readable description of this effect, for the stack panel / log / catalog.
    ///
    /// The match is intentionally exhaustive (no `_` arm): every new [`Effect`] variant must
    /// stop here so its author writes a label, the same way it must supply an [`Effect::target`].
    pub fn label(self) -> String {
        match self {
            Effect::DealDamage { amount, .. } => format!("Deal {} damage", amount_label(amount)),
            Effect::DrawCards { count } => format!("Draw {}", amount_label(count)),
            Effect::TargetPlayerDraws { count, .. } => {
                format!("Target player draws {}", amount_label(count))
            }
            Effect::RevealTopToHand { filter, .. } => format!(
                "Defending player reveals the top card of their library; if it's {}, put it into their hand",
                card_filter_label(filter)
            ),
            Effect::RevealTopAndDrainMutual => {
                "You and target opponent each reveal the top card of your library, lose life equal to the mana value of the other's, and put it into your hand".to_string()
            }
            Effect::RevealUntil {
                filter,
                count,
                matched_dest,
                ..
            } => {
                let what = card_filter_label(filter);
                let dest = match matched_dest {
                    SearchDest::Hand => "into your hand",
                    SearchDest::Battlefield => "onto the battlefield",
                    SearchDest::LibraryTop => "on top of your library",
                };
                format!(
                    "Reveal cards from the top of your library until you reveal {} {}, put them {}, and put the rest on the bottom of your library",
                    amount_label(count),
                    what,
                    dest
                )
            }
            Effect::RevealUntilMayDeploy { filter } => format!(
                "Reveal cards from the top of your library until you reveal {}. You may put that card onto the battlefield. If you don't, put it into your hand. Put the rest on the bottom of your library",
                card_filter_label(filter)
            ),
            Effect::RevealUntilExileCastFree { filter } => format!(
                "Reveal cards from the top of your library until you reveal {}. Exile that card and put the rest on the bottom of your library. You may cast the exiled card without paying its mana cost",
                card_filter_label(filter)
            ),
            Effect::ShuffleLibrary => "Shuffle your library".to_string(),
            Effect::ExileTopUntilStopCastFreeUnderBudget { budget } => format!(
                "As many times as you choose, you may exile the top card of your library. If the \
                 total mana value of the cards exiled this way is {budget} or less, you may cast \
                 any number of spells from among those cards without paying their mana costs"
            ),
            Effect::RevealTopCards {
                filter,
                count,
                matched_dest,
                ..
            } => {
                let what = card_filter_label(filter);
                let dest = match matched_dest {
                    SearchDest::Hand => "into your hand",
                    SearchDest::Battlefield => "onto the battlefield",
                    SearchDest::LibraryTop => "on top of your library",
                };
                format!(
                    "Reveal the top {} cards of your library, put all cards among them that are {} {}, and put the rest on the bottom of your library",
                    amount_label(count),
                    what,
                    dest
                )
            }
            Effect::GainLife { amount } => format!("Gain {} life", amount_label(amount)),
            Effect::LoseLife { amount } => format!("Lose {} life", amount_label(amount)),
            Effect::GainLifeTargetController { amount } => {
                format!("Target's controller gains {} life", amount_label(amount))
            }
            Effect::Manifest => "Its controller manifests the top card of their library".to_string(),
            Effect::AddMana { .. } => "Add mana".to_string(),
            Effect::GrantManaAbility { filter, .. } => match filter.subtypes {
                [] => "Artifacts you control gain a mana ability".to_string(),
                _ => format!(
                    "{} you control gain a mana ability",
                    filter.subtypes.join("/")
                ),
            },
            Effect::PumpUntilEndOfTurn {
                power,
                toughness,
                keywords,
                ..
            } => {
                let (power, toughness) = (amount_label(power), amount_label(toughness));
                if keywords.is_empty() {
                    format!("+{power}/+{toughness} until end of turn")
                } else {
                    format!(
                        "+{power}/+{toughness} and gains {} until end of turn",
                        keyword_list_label(keywords)
                    )
                }
            }
            Effect::PumpSelfUntilEndOfTurn {
                power,
                toughness,
                keywords,
            } => {
                let (power, toughness) = (amount_label(power), amount_label(toughness));
                if keywords.is_empty() {
                    format!("+{power}/+{toughness} until end of turn")
                } else {
                    format!(
                        "+{power}/+{toughness} and gains {} until end of turn",
                        keyword_list_label(keywords)
                    )
                }
            }
            Effect::PumpCreaturesYouControlUntilEndOfTurn {
                power,
                toughness,
                keywords,
                filter: _,
            } => {
                if keywords.is_empty() {
                    format!(
                        "Creatures you control get +{}/+{} until end of turn",
                        amount_label(power),
                        amount_label(toughness)
                    )
                } else {
                    format!(
                        "Creatures you control get +{}/+{} and gain {} until end of turn",
                        amount_label(power),
                        amount_label(toughness),
                        keyword_list_label(keywords)
                    )
                }
            }
            Effect::GrantKeywordsToPermanentsYouControlUntilEndOfTurn { keywords, filter: _ } => {
                format!(
                    "Permanents you control gain {} until end of turn",
                    keyword_list_label(keywords)
                )
            }
            Effect::KeywordAnthemStatic { keywords, filter } => {
                let scope = if filter.other {
                    "Other permanents you control have"
                } else {
                    "Permanents you control have"
                };
                format!("{scope} {}", keyword_list_label(keywords))
            }
            Effect::SetBasePtCreaturesYouControlUntilEndOfTurn {
                power,
                toughness,
                other,
            } => {
                let scope = if other { "Other creatures" } else { "Creatures" };
                format!(
                    "{scope} you control have base power and toughness {}/{} until end of turn",
                    amount_label(power),
                    amount_label(toughness)
                )
            }
            Effect::SetBasePtTargetUntilEndOfTurn {
                power, toughness, ..
            } => {
                format!(
                    "Target creature has base power and toughness {}/{} until end of turn",
                    amount_label(power),
                    amount_label(toughness)
                )
            }
            Effect::AnimateSelfUntilEndOfTurn {
                base_power,
                base_toughness,
                ..
            } => {
                format!(
                    "Becomes a {base_power}/{base_toughness} creature until end of turn"
                )
            }
            Effect::PumpOtherAttackersAttackingYourOpponents { power, toughness } => {
                format!(
                    "Each other creature that's attacking one of your opponents gets \
                     +{power}/+{toughness} until end of turn"
                )
            }
            Effect::EnchantedAttackerPumpAttackingOpponentElseControllerLosesLife {
                power,
                toughness,
                life,
            } => format!(
                "It gets +{power}/+{toughness} until end of turn if it's attacking one of your \
                 opponents. Otherwise, its controller loses {life} life"
            ),
            Effect::StripKeywordsFromOpponentsCreatures { keywords } => {
                format!(
                    "Creatures your opponents control lose {} until end of turn and can't have \
                     {} this turn",
                    keyword_list_label(keywords),
                    keyword_list_label(keywords)
                )
            }
            Effect::AnthemStatic {
                power,
                toughness,
                self_only,
                exclude_source,
                tokens_only,
                keywords,
                subtypes,
                colors,
                chosen_subtype,
                attacking_only,
                commander_only,
                has_counters,
                condition: _,
                from_graveyard: _,
                all_players,
            } => {
                let scope = match subtypes {
                    _ if all_players => "All creatures".to_string(),
                    _ if chosen_subtype => "Creatures you control of the chosen type".to_string(),
                    [] if self_only => "This creature".to_string(),
                    [] => "Creatures you control".to_string(),
                    _ => format!("{} you control", subtypes.join("/")),
                };
                let scope = if colors.is_empty() {
                    scope
                } else {
                    let names: Vec<String> =
                        colors.iter().map(|c| format!("{c:?}").to_lowercase()).collect();
                    format!("{} {}", names.join("/"), scope.to_lowercase())
                };
                let scope = if exclude_source {
                    format!("Other {}", scope.to_lowercase())
                } else {
                    scope
                };
                let scope = if tokens_only {
                    format!("Token {}", scope.to_lowercase())
                } else {
                    scope
                };
                let scope = if commander_only {
                    format!("Commander {}", scope.to_lowercase())
                } else {
                    scope
                };
                let scope = if attacking_only {
                    format!("Attacking {}", scope.to_lowercase())
                } else {
                    scope
                };
                let scope = if has_counters {
                    format!("{scope} with counters on them")
                } else {
                    scope
                };
                if keywords.is_empty() {
                    format!(
                        "{scope} get{} +{}/+{}",
                        if self_only { "s" } else { "" },
                        amount_label(power),
                        amount_label(toughness)
                    )
                } else {
                    format!("{scope} have {}", keyword_list_label(keywords))
                }
            }
            Effect::TriggerDoublingStatic { .. } => {
                "That triggered ability triggers an additional time".to_string()
            }
            Effect::PreventNoncombatDamageToOtherCreaturesYouControl => {
                "Prevent all noncombat damage that would be dealt to other creatures you control"
                    .to_string()
            }
            Effect::NoMaximumHandSize => "You have no maximum hand size".to_string(),
            Effect::PlayFromGraveyardOncePerTurn => {
                "Once during each of your turns, you may play a land or cast a permanent spell with \
                 mana value 3 or less from your graveyard"
                    .to_string()
            }
            Effect::ReduceSpellCost {
                amount,
                filter,
                first_x_spell_each_turn,
            } => {
                let scope = if first_x_spell_each_turn {
                    "The first spell you cast with {X} in its mana cost each turn"
                } else {
                    spell_filter_label(filter)
                };
                format!("{scope} cost {{{}}} less", amount_label(amount))
            }
            Effect::AttackTax { amount } => format!(
                "Creatures can't attack you unless their controller pays {{{amount}}} for each creature they control that's attacking you"
            ),
            Effect::CounterScaledAttackTax => {
                "Creatures with counters on them can't attack you unless their controller pays generic mana equal to their counters".to_string()
            }
            Effect::CantBeAttackedBy { filter } => {
                format!("{} can't attack you", permanent_filter_label(filter))
            }
            Effect::PreventCombatDamageToYouCreatingTokens { .. } => {
                "Prevent all combat damage that would be dealt to you this turn, creating a token per point prevented".to_string()
            }
            Effect::PreventAllCombatDamageThisTurn => {
                "Prevent all combat damage that would be dealt this turn".to_string()
            }
            Effect::PlaceVowCounters { .. } => "Put a vow counter on each surviving creature".to_string(),
            Effect::DestroyTarget { .. } => "Destroy target".to_string(),
            Effect::RegenerateShield { .. } => "Regenerate target".to_string(),
            Effect::DestroyAll { filter } => {
                format!("Destroy all {}", permanent_filter_label(filter))
            }
            Effect::ExileAll { filter } => {
                format!("Exile all {}", permanent_filter_label(filter))
            }
            Effect::DamageEachCreature {
                amount,
                opponents_only,
            } => {
                let subject = if opponents_only {
                    "each creature your opponents control"
                } else {
                    "each creature"
                };
                format!("Deal {} damage to {subject}", amount_label(amount))
            }
            Effect::WeakenEachCreature {
                power,
                toughness,
                opponents_only,
            } => {
                let subject = if opponents_only {
                    "Creatures your opponents control"
                } else {
                    "Each creature"
                };
                format!(
                    "{subject} get{} -{}/-{} until end of turn",
                    if opponents_only { "" } else { "s" },
                    amount_label(power),
                    amount_label(toughness)
                )
            }
            Effect::PutCounters {
                count, kind: None, ..
            } => {
                format!("Put {} +1/+1 counters", amount_label(count))
            }
            Effect::PutCounters {
                count,
                kind: Some(kind),
                ..
            } => {
                let kind_name = match kind {
                    CounterKind::Charge => "charge",
                    CounterKind::Story => "story",
                    CounterKind::Study => "study",
                    CounterKind::Vow => "vow",
                    CounterKind::Time => "time",
                };
                format!("Put {} {kind_name} counters", amount_label(count))
            }
            Effect::DoubleCounters { .. } => "Double its +1/+1 counters".to_string(),
            Effect::DoubleCountersOnAttachedCreature => {
                "Double the +1/+1 counters on equipped creature".to_string()
            }
            Effect::PutCountersEach { count, .. } => {
                format!("Put {} +1/+1 counters on each", amount_label(count))
            }
            Effect::Proliferate { times } => format!("Proliferate {} times", amount_label(times)),
            Effect::MoveCounters { all_kinds, .. } => {
                if all_kinds {
                    "Move all counters onto another permanent".to_string()
                } else {
                    "Move +1/+1 counters onto another permanent".to_string()
                }
            }
            Effect::RemoveAllCountersThenDraw { .. } => {
                "Remove all counters, draw a card for each removed".to_string()
            }
            Effect::CounterReplacement { add, times, .. } => {
                format!("+1/+1 counters placed: (n + {add}) x {times}")
            }
            Effect::TokenReplacement { times } => {
                format!("tokens created: n x {times}")
            }
            Effect::LifeGainReplacement { plus } => {
                format!("life gained: n + {plus}")
            }
            Effect::CastXReplacement { times } => {
                format!("value of X: X x {times}")
            }
            Effect::EntersWithCounters { amount, kind: None } => {
                format!("Enters with {} +1/+1 counters", amount_label(amount))
            }
            Effect::EntersWithCounters {
                amount,
                kind: Some(kind),
            } => {
                let kind_name = match kind {
                    CounterKind::Charge => "charge",
                    CounterKind::Story => "story",
                    CounterKind::Study => "study",
                    CounterKind::Vow => "vow",
                    CounterKind::Time => "time",
                };
                format!("Enters with {} {kind_name} counters", amount_label(amount))
            }
            Effect::CreaturesYouControlEnterWithCounters { filter, count } => format!(
                "{} you control enter with {} additional +1/+1 counters",
                permanent_filter_label(filter),
                amount_label(count)
            ),
            Effect::CreateToken { token, count, .. } => {
                format!("Create {} {} token(s)", amount_label(count), token.name)
            }
            Effect::CreateTreasure {
                count,
                target_player: false,
                ..
            } => format!("Create {} Treasure token(s)", amount_label(count)),
            Effect::CreateTreasure {
                count,
                target_player: true,
                ..
            } => format!(
                "Target player creates {} Treasure token(s)",
                amount_label(count)
            ),
            Effect::CreateTokenCopy {
                count,
                sacrifice_at_next_end_step,
                exile_at_next_end_step,
                ..
            } => {
                let mut label = format!(
                    "Create {} token copy/copies of target creature",
                    amount_label(count)
                );
                if sacrifice_at_next_end_step {
                    label.push_str("; sacrifice it at the beginning of the next end step");
                }
                if exile_at_next_end_step {
                    label.push_str("; exile it at the beginning of the next end step");
                }
                label
            }
            Effect::CopyEachEnteredThisTurnTokenTappedAttacking { .. } => {
                "For each creature token you control that entered this turn, create a tapped \
                 and attacking copy of it; sacrifice those tokens at the beginning of the next \
                 end step"
                    .to_string()
            }
            Effect::GrantToAttached {
                power, toughness, ..
            } => format!(
                "Attached creature gets +{}/+{}",
                amount_label(power),
                amount_label(toughness)
            ),
            Effect::SetAttachedBasePT { power, toughness } => {
                format!("Attached creature has base power and toughness {power}/{toughness}")
            }
            Effect::SetAttachedTypes {
                set_subtypes,
                add_subtypes,
                ..
            } => {
                let subtypes = if set_subtypes.is_empty() {
                    add_subtypes
                } else {
                    set_subtypes
                };
                format!("Attached creature is a {}", subtypes.join(" "))
            }
            Effect::ControlAttached => "You control enchanted creature".to_string(),
            Effect::Equip => "Equip".to_string(),
            Effect::AttachTriggeringAuraToMintedToken { .. } => {
                "Attach it to the token".to_string()
            }
            // The reflexive trigger's label is that of its own (separately placed) ability.
            Effect::ReflexiveTrigger { then } => {
                then.first().map(|e| e.label()).unwrap_or_default()
            }
            Effect::ReturnFromGraveyardAttachedToToken { filter, .. } => format!(
                "Return up to one {} from your graveyard to the battlefield attached to that \
                 token",
                card_filter_label(filter)
            ),
            Effect::AttachSelfToEntering { .. } => "Attach this to that creature".to_string(),
            Effect::AttachSelfToReanimated => "Attach this to it".to_string(),
            Effect::AttachSelfToMintedToken => "Attach this to the token".to_string(),
            Effect::AttachMintedAuraToTarget { .. } => {
                "Attach the token to target creature an opponent controls".to_string()
            }
            Effect::ScheduleReturnThisAuraAttachedToReanimated => {
                "Return this to the battlefield attached to that creature at the beginning of \
                 the next end step"
                    .to_string()
            }
            Effect::ReturnThisAuraAttachedTo { .. } => {
                "Return this to the battlefield attached to that creature".to_string()
            }
            Effect::ReturnThisAuraFromGraveyardAttachedToChosenHost => {
                "Return this from your graveyard to the battlefield".to_string()
            }
            Effect::ScheduleReturnThisAuraFromGraveyardAttachedToChosenHost => {
                "Return this to the battlefield at the beginning of the next end step".to_string()
            }
            Effect::ExileTarget { .. } => "Exile target".to_string(),
            Effect::ExileUntilSourceLeaves { .. } => {
                "Exile target until this leaves the battlefield".to_string()
            }
            Effect::ExileTargetMintingIllusionOnLeave { .. } => "Exile target".to_string(),
            Effect::FlickerTarget {
                return_at: None, ..
            } => "Exile target creature, then return it to the battlefield under its owner's \
                  control"
                .to_string(),
            Effect::FlickerTarget {
                return_at: Some(_), ..
            } => "Exile target creature. Return that card to the battlefield under its owner's \
                  control at the beginning of the next end step"
                .to_string(),
            Effect::ReturnFlickeredCard { .. } => {
                "Return that card to the battlefield under its owner's control".to_string()
            }
            Effect::ExileTopMayPlay {
                count,
                until_next_turn,
            } => {
                let duration = if until_next_turn {
                    "until the end of your next turn"
                } else {
                    "until end of turn"
                };
                format!(
                    "Exile the top {} card(s); play {duration}",
                    amount_label(count)
                )
            }
            Effect::ExileTopCastMatchingFree { count, filter } => {
                format!(
                    "Exile the top {count} card(s); you may cast {} from among them without \
                     paying its mana cost. Put the rest on the bottom of your library",
                    card_filter_label(filter)
                )
            }
            Effect::Cascade { .. } => "Cascade".to_string(),
            Effect::OpponentSplitsExilePiles => {
                "Exile the top four cards in one pile, then the top four in a second pile. An \
                 opponent chooses one pile; put it into your graveyard. You may cast a card from \
                 the other pile without paying its mana cost; put the rest into your hand"
                    .to_string()
            }
            Effect::RevealTopSplitPiles => {
                "Reveal the top five cards of your library. An opponent separates those cards \
                 into two piles. Put one pile into your hand and the other into your graveyard"
                    .to_string()
            }
            Effect::EachPlayerExilesUntilNonlandOpponentPicks => {
                "Each player exiles cards from the top of their library until they exile a nonland \
                 card. An opponent chooses a nonland card exiled this way. You may cast up to two \
                 of the other exiled cards without paying their mana costs"
                    .to_string()
            }
            Effect::ExileFromGraveyardMayPlay { .. } => {
                "Exile that card from your graveyard; play it this turn".to_string()
            }
            Effect::ExileRandomFromGraveyardMayPlay => {
                "Exile a card from your graveyard at random; you may play it this turn"
                    .to_string()
            }
            Effect::ExileDiscardedWithThis { .. } => {
                "Exile that card from your graveyard with this".to_string()
            }
            Effect::ExileTargetFromGraveyardWithThis => {
                "Exile target noncreature, nonland card from your graveyard".to_string()
            }
            Effect::ExileTargetFromGraveyardCreateTokenCopy { filter } => {
                format!(
                    "Exile target {} from your graveyard. Create a token that's a copy of it",
                    card_filter_label(filter)
                )
            }
            Effect::ExileTargetGraveyardSpellCastFree { filter, .. } => {
                format!(
                    "Exile up to one target {} from your graveyard; you may cast it without \
                     paying its mana cost",
                    card_filter_label(filter)
                )
            }
            Effect::ExileTargetGraveyardCardRecordManaValue { filter } => {
                format!("Exile target {} from your graveyard", card_filter_label(filter))
            }
            Effect::ExileTargetGraveyardCardThenIfCreature { then } => format!(
                "Exile target card from a graveyard. If a creature card is exiled this way, {}",
                then.iter()
                    .map(|&s| s.label())
                    .collect::<Vec<_>>()
                    .join(", then ")
            ),
            Effect::ScheduleThisTurnCombatDamageCopy => {
                "Whenever a creature you control deals combat damage to a player this turn, \
                 copy the exiled card; you may cast the copy without paying its mana cost"
                    .to_string()
            }
            Effect::MintFreeCopyOfExiledCard { .. } => {
                "Copy the exiled card; you may cast the copy without paying its mana cost"
                    .to_string()
            }
            Effect::CashOutExiledWithThis => {
                "Put a card exiled with this into its owner's graveyard".to_string()
            }
            Effect::CastExiledWithThisFree => {
                "Choose target card exiled with this; you may cast it this turn without paying \
                 its mana cost"
                    .to_string()
            }
            Effect::ReturnToHand { .. } => "Return to owner's hand".to_string(),
            Effect::ReturnThisToHand => "Return this card to its owner's hand".to_string(),
            Effect::ReturnThisFromGraveyardToBattlefield { tapped: true } => {
                "Return this card from your graveyard to the battlefield tapped".to_string()
            }
            Effect::ReturnThisFromGraveyardToBattlefield { tapped: false } => {
                "Return this card from your graveyard to the battlefield".to_string()
            }
            Effect::ReturnAllToHand { filter } => {
                format!(
                    "Return all {} to their owners' hands",
                    permanent_filter_label(filter)
                )
            }
            Effect::ReturnFromGraveyardToHand { .. } => "Return from graveyard to hand".to_string(),
            Effect::ReanimateToBattlefield { .. } => "Reanimate to battlefield".to_string(),
            Effect::TuckFromGraveyard { to_top: true, .. } => {
                "Put graveyard card on top of library".to_string()
            }
            Effect::TuckFromGraveyard { to_top: false, .. } => {
                "Put graveyard card on bottom of library".to_string()
            }
            Effect::MassReturnFromGraveyard { filter } => format!(
                "Return all {} from your graveyard to the battlefield",
                card_filter_label(filter)
            ),
            Effect::ShuffleTargetCardsFromGraveyardIntoLibrary { max, target_player } => {
                let count = if max == 0 {
                    "any number of".to_string()
                } else {
                    format!("up to {max}")
                };
                if target_player {
                    format!(
                        "Target player shuffles {count} target cards from their graveyard into their library"
                    )
                } else {
                    format!("Shuffle {count} target cards from your graveyard into your library")
                }
            }
            Effect::ShuffleTargetPermanentIntoLibraryThenReveal { .. } => {
                "The owner of target permanent shuffles it into their library, then reveals the \
                 top card of their library. If it's a permanent card, they put it onto the \
                 battlefield"
                    .to_string()
            }
            Effect::TuckPermanentIntoLibrary { to_top: true, .. } => {
                "Put target permanent on top of its owner's library".to_string()
            }
            Effect::TuckPermanentIntoLibrary { to_top: false, .. } => {
                "Put target permanent on the bottom of its owner's library".to_string()
            }
            Effect::Mill { count, .. } => format!("Target player mills {}", amount_label(count)),
            Effect::ExileGraveyard => "Exile target player's graveyard".to_string(),
            Effect::ExileAllGraveyards => "Exile all graveyards".to_string(),
            Effect::DrainTarget { amount, .. } => {
                format!("Target player loses {amount}, you gain {amount}")
            }
            Effect::TargetPlayerGainsLife { amount, .. } => {
                format!("Target player gains {amount} life")
            }
            Effect::TargetPlayerMayDraw { count, .. } => {
                format!("Target player may draw {}", amount_label(count))
            }
            Effect::EachOpponentDrain { amount, sum_gain } => {
                let amount_str = amount_label(amount);
                if sum_gain {
                    format!("Each opponent loses {amount_str}, you gain life equal to the life lost this way")
                } else {
                    format!("Each opponent loses {amount_str}, you gain {amount_str}")
                }
            }
            Effect::EachOpponentLosesLife { amount } => {
                format!("Each opponent loses {}", amount_label(amount))
            }
            Effect::Scry { count } => format!("Scry {}", amount_label(count)),
            Effect::Surveil { count } => format!("Surveil {count}"),
            Effect::LookAtTop {
                count, up_to, dest, ..
            } => {
                let where_to = match dest {
                    TopDest::Hand => "into your hand",
                    TopDest::Battlefield => "onto the battlefield",
                };
                format!(
                    "Look at the top {count} cards, put up to {up_to} {where_to}, rest on the bottom"
                )
            }
            Effect::DistributeTop {
                count,
                to_hand,
                to_bottom,
                to_exile_may_play,
            } => format!(
                "Look at the top {count} cards, put {to_hand} into your hand, {to_bottom} on the \
                 bottom, and exile {to_exile_may_play} (you may play it this turn)"
            ),
            Effect::Discard {
                count,
                target_player: false,
                or_one_matching: None,
            } => format!("Discard {count}"),
            Effect::Discard {
                count,
                target_player: true,
                or_one_matching: None,
            } => format!("Target player discards {count}"),
            Effect::Discard {
                count,
                target_player: false,
                or_one_matching: Some(_),
            } => format!("Discard {count} unless you discard a land card"),
            Effect::Discard {
                count,
                target_player: true,
                or_one_matching: Some(_),
            } => format!("Target player discards {count} unless they discard a land card"),
            Effect::PutLandFromHand { tapped } => {
                let suffix = if tapped { " tapped" } else { "" };
                format!("Put a land from hand onto the battlefield{suffix}")
            }
            Effect::CastCreatureFaceDown => {
                "Cast a creature card from hand face down as a 2/2".to_string()
            }
            // ponytail: generic pips only (Rupture Spire's {1}) — no pool card needs a colored
            // sacrifice-unless-pay cost yet; extend if one does.
            Effect::SacrificeSelfUnlessPay { cost } => {
                format!("Sacrifice this unless you pay {{{}}}", cost.generic)
            }
            Effect::SacrificeSelfUnlessReturnLand { .. } => {
                "Sacrifice this unless you return a non-Lair land you control".to_string()
            }
            // A sequence reads as its steps joined by ", then " (Faithless Looting's "Draw 2, then
            // discard 2").
            Effect::Sequence { steps } => steps
                .iter()
                .map(|&s| s.label())
                .collect::<Vec<_>>()
                .join(", then "),
            // A "choose one —" trigger reads as its modes joined by " • " (Atsushi's dies trigger). (CR 603.6, CR 603)
            Effect::ChooseOne { modes } => format!(
                "Choose one — {}",
                modes
                    .iter()
                    .map(|&m| m.label())
                    .collect::<Vec<_>>()
                    .join(" • ")
            ),
            Effect::GoadTarget { .. } => "Goad target creature".to_string(),
            Effect::PhaseOut => {
                "Any number of other target creatures you control phase out".to_string()
            }
            Effect::DoubleCountersOnTargetCreatures { .. } => {
                "Double the number of +1/+1 counters on any number of other target creatures"
                    .to_string()
            }
            Effect::CopyTargetSpell => "Copy target spell".to_string(),
            Effect::CopyThisSpell { .. } => "Copy this spell".to_string(),
            Effect::RetargetSpellCopy { .. } => "Choose new targets for the copy".to_string(),
            Effect::ChangeTargetOfTargetSpellOrAbility { .. } => {
                "Change the target of target spell or ability with a single target".to_string()
            }
            Effect::CopyTriggeringSpell { count, .. } => {
                format!("Copy it {} times", amount_label(count))
            }
            Effect::CopyTriggeringSpellForEachOtherCreatureYouControl { .. } => {
                "Copy it for each other creature you control it could target".to_string()
            }
            Effect::CopyTriggeringAbility { .. } => "Copy that ability".to_string(),
            Effect::Demonstrate { .. } => "Demonstrate".to_string(),
            Effect::CommanderEntersWithBonusCounters { count, .. } => format!(
                "It enters with {} additional +1/+1 counters on it",
                amount_label(count)
            ),
            Effect::Fight {
                ally_is_shared_target: false,
                ..
            } => "Target creature you control fights target creature you don't control"
                .to_string(),
            Effect::Fight {
                ally_is_shared_target: true,
                ..
            } => "Then it fights up to one target creature you don't control".to_string(),
            Effect::CounterTargetSpell {
                unless_pays: None,
                filter,
                countered_dest: None,
            } => format!("Counter target {}", counter_target_spell_noun(filter)),
            Effect::CounterTargetSpell {
                unless_pays: None,
                filter,
                countered_dest: Some(CounteredDest::LibraryTopOrBottom),
            } => format!(
                "Counter target {}. If that spell is countered this way, put that card on the \
                 top or bottom of its owner's library instead of into that player's graveyard",
                counter_target_spell_noun(filter)
            ),
            Effect::CounterTargetSpell {
                unless_pays: Some(amount),
                filter,
                ..
            } => format!(
                "Counter target {} unless its controller pays {}",
                counter_target_spell_noun(filter),
                amount_label(amount)
            ),
            Effect::CounterTargetActivatedAbility => {
                "Counter target activated ability".to_string()
            }
            Effect::ScheduleAtNextUpkeep { then, fire_at, .. } => {
                let when = match fire_at {
                    Step::End => "the next end step",
                    _ => "the next upkeep",
                };
                format!("Delayed: {} at the beginning of {when}", then.label())
            }
            // A next-cast delayed one-shot reads as its steps joined the same way a `Sequence`
            // does (see the arm above) — `then` is a plain effect list, not itself a `Sequence`.
            Effect::ScheduleNextCastTrigger { filter, then } => format!(
                "When you next cast a {} this turn: {}",
                counter_target_spell_noun(filter),
                then.iter()
                    .map(|e| e.label())
                    .collect::<Vec<_>>()
                    .join(", then ")
            ),
            Effect::AttackerDrawsControllerCounters { counters, .. } => {
                format!("Attacking player draws; put {counters} +1/+1 counters on a creature")
            }
            Effect::AttackerLosesLifeYouGain { amount, .. } => {
                format!("Enchanted creature's controller loses {amount} life; you gain {amount}")
            }
            Effect::AttackerLosesLifeYouDraw { life_loss, .. } => {
                format!("That opponent loses {life_loss} life; you draw a card")
            }
            Effect::AttackingPlayerDraws { count, .. } => {
                format!("The attacking player draws {count}")
            }
            Effect::DealDamageToEnteringPermanent { amount, .. } => {
                format!("Deal {amount} damage to the permanent that entered")
            }
            Effect::ReanimateDyingEnchantedCreature { under_owner, .. } => {
                if under_owner {
                    "Return that card to the battlefield under its owner's control".to_string()
                } else {
                    "Return it to the battlefield under your control".to_string()
                }
            }
            Effect::ExileDeadCreatureCreateCopyWithSubtype { add_subtypes, .. } => {
                match add_subtypes.first() {
                    Some(subtype) => format!(
                        "Exile it, then create a token that's a copy of it that's a {subtype}"
                    ),
                    None => "Exile it, then create a token that's a copy of it".to_string(),
                }
            }
            Effect::ReturnExiledCardToOwnersGraveyard { .. } => {
                "Return the exiled card to its owner's graveyard".to_string()
            }
            Effect::SearchLibrary {
                filter, to_zone, ..
            } => {
                let what = card_filter_label(filter);
                let dest = match to_zone {
                    SearchDest::Hand => "into your hand",
                    SearchDest::Battlefield => "onto the battlefield",
                    SearchDest::LibraryTop => "on top of your library, revealing it",
                };
                format!("Search your library for {what}, put it {dest}")
            }
            Effect::EachPlayerSacrifices {
                scope, keep_one, ..
            } => {
                let who = match scope {
                    EdictScope::AllPlayers => "Each player",
                    EdictScope::EachOpponent => "Each opponent",
                    EdictScope::TargetedPlayers => "Any number of target players",
                };
                if keep_one {
                    format!("{who} keeps one creature and sacrifices the rest")
                } else {
                    format!("{who} sacrifices a permanent")
                }
            }
            Effect::EachPlayerExilesFromGraveyard => {
                "Each player exiles a card from their graveyard".to_string()
            }
            Effect::TargetPlayerExilesFromGraveyard { .. } => {
                "Target player exiles a card from their graveyard".to_string()
            }
            Effect::CasterKeepsOneOfEachTypePerPlayer => {
                "For each player, you choose an artifact, a creature, an enchantment, and a \
                 planeswalker they control; each player sacrifices their other nonland permanents"
                    .to_string()
            }
            Effect::EachPlayerControllerChoosesCounterTarget => {
                "For each player, put a +1/+1 counter on up to one creature that player controls"
                    .to_string()
            }
            Effect::CouncilsDilemmaVote { options } => {
                format!("Starting with you, each player votes for {}", options.join(" or "))
            }
            Effect::EachPlayerCreatesFractalFromExiledPower { token } => format!(
                "Each player creates a {} token with +1/+1 counters equal to the total power of \
                 creatures they controlled that were exiled this way",
                token.name
            ),
            Effect::EachPlayerDiscardsHandThenDraws { count } => format!(
                "Each player discards their hand, then draws {}",
                amount_label(count)
            ),
            Effect::EachOtherTokenBecomesCopyOfChosen => {
                "You may choose a token you control; if you do, each other token you control \
                 becomes a copy of that token"
                    .to_string()
            }
            Effect::PutCounterThenMayBecomeCopyOfCardFromList { .. } => {
                "Put a +1/+1 counter on this creature, then you may have this creature become a \
                 copy of an artifact or creature card from among those cards until end of turn"
                    .to_string()
            }
            Effect::BecomeCopyOfTargetCreatureGainingMyriad { .. } => {
                "This creature becomes a copy of up to one target nonlegendary creature you \
                 control until end of turn, except it has myriad"
                    .to_string()
            }
            Effect::MyriadTokenCopies { .. } => {
                "For each opponent other than the defending player, create a token copy that's \
                 tapped and attacking that opponent; exile the tokens at the end of combat"
                    .to_string()
            }
            Effect::MaySacrifice { filter, .. } => {
                format!("You may sacrifice {}", permanent_filter_label(filter))
            }
            Effect::MayReturnFromGraveyard { filter, .. } => format!(
                "You may return {} from your graveyard to your hand",
                card_filter_label(filter)
            ),
            Effect::MayDiscard { .. } => "You may discard a card".to_string(),
            Effect::MayDrawUnlessPays { cost, .. } => {
                format!("You may draw a card unless that player pays {}", amount_label(cost))
            }
            Effect::TapTarget { .. } => "Tap target".to_string(),
            Effect::UntapTarget { .. } => "Untap target".to_string(),
            Effect::GainControlUntilEndOfTurn { .. } => {
                "Gain control of target creature until end of turn".to_string()
            }
            Effect::GainControl { .. } => "Gain control of target creature".to_string(),
            Effect::GainControlWhile { .. } => {
                "Gain control of target creature for as long as you control this and it remains tapped"
                    .to_string()
            }
            Effect::GrantSourceAbilitiesUntilEndOfTurn => {
                "It gains this creature's other abilities until end of turn".to_string()
            }
            Effect::UntapAll { filter } => {
                format!("Untap all {} you control", permanent_filter_label(filter))
            }
            Effect::EachPlayerDraws { count } => format!("Each player draws {count}"),
            Effect::TargetPlayerLosesLife { amount } => {
                format!("Target player loses {amount} life")
            }
            Effect::SacrificeOwn { filter, count } => {
                format!("Sacrifice {count} {}", permanent_filter_label(filter))
            }
            Effect::DefendingPlayerSacrifices { count, .. } => {
                format!("Defending player sacrifices {count} permanents of their choice")
            }
            Effect::SacrificeObject { .. } => "Sacrifice it".to_string(),
            Effect::SacrificeSource => "Sacrifice it".to_string(),
            Effect::SacrificeEnchantedCreature { .. } => {
                "That creature's controller sacrifices it".to_string()
            }
            Effect::ExileObject { .. } => "Exile it".to_string(),
            Effect::MillSelf { count } => format!("Mill {}", amount_label(count)),
            Effect::ExileSelfWithTimeCounters { counters } => {
                format!("Exile this with {counters} time counters on it")
            }
            Effect::BecomePrepared => "Become prepared".to_string(),
            Effect::LevelUp { level } => format!("Level {level}"),
            Effect::ArmCombatDamageWatch => {
                "Arm a delayed watch: this creature becomes prepared when target creature deals \
                 combat damage to a player this combat"
                    .to_string()
            }
            Effect::ChooseCreatureType => "Choose a creature type".to_string(),
            Effect::ChooseColor => "Choose a color".to_string(),
            Effect::RemoveCounterFromSelf => "Remove a +1/+1 counter from it".to_string(),
            Effect::GrantFlashThisTurn => {
                "You may cast spells this turn as though they had flash".to_string()
            }
            Effect::GrantChannelColorlessManaThisTurn => {
                "Until end of turn, any time you could activate a mana ability, you may pay 1 \
                 life. If you do, add {C}"
                    .to_string()
            }
            // A conditional step reads as its `then` steps — no consumer renders `Condition`
            // prose today (an activation gate's `condition` isn't labeled either).
            Effect::Conditional { then, .. } => then
                .iter()
                .map(|&s| s.label())
                .collect::<Vec<_>>()
                .join(", then "),
            Effect::UntapSearchedLand => "Untap the searched land".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    // Pin a few labels byte-for-byte, including a Sequence (which recurses into step labels).
    #[test]
    fn labels_are_stable() {
        assert_eq!(
            Effect::DrawCards {
                count: Amount::Fixed(2)
            }
            .label(),
            "Draw 2"
        );
        assert_eq!(
            Effect::GainLife {
                amount: Amount::Fixed(1)
            }
            .label(),
            "Gain 1 life"
        );
        assert_eq!(
            Effect::Scry {
                count: Amount::Fixed(3)
            }
            .label(),
            "Scry 3"
        );
        assert_eq!(
            Effect::Sequence {
                steps: &[
                    Effect::DrawCards {
                        count: Amount::Fixed(2)
                    },
                    Effect::Discard {
                        count: 2,
                        target_player: false,
                        or_one_matching: None,
                    },
                ],
            }
            .label(),
            "Draw 2, then Discard 2"
        );
    }
}
