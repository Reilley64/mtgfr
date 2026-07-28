//! Text-changing effects (CR 612) — the one-word substitutions Magical Hack and Sleight of Mind
//! make.
//!
//! CR 612.1 replaces *words*, and this engine has no words: a card is a [`CardDef`] of enums. What
//! it can replace is the enum the word would have named, wherever the card model already stores
//! one. So the vocabulary here is exactly the two printed cards' ("one basic land type", "one
//! color word") and the reach is exactly the fields that hold it — read back at CR 613.4 layer 3
//! by [`Game::effective_subtypes`](crate::Game::effective_subtypes),
//! [`Game::effective_keywords`](crate::Game::effective_keywords) and `Game::functional_abilities`.
//! Nothing is parsed, and a word the card model doesn't store as an enum doesn't move.

use crate::*;

/// The five color words, in WUBRG order — the vocabulary Sleight of Mind replaces one of, and the
/// color twin of [`BASIC_LAND_TYPES`]. Offered through [`PendingChoice::ChooseCreatureType`]'s
/// picker like the land types are, so both text-changers ask their two questions the same way.
pub const COLOR_WORDS: &[&str] = &["White", "Blue", "Black", "Red", "Green"];

/// Which enumerated vocabulary a text-changing spell replaces a word from — Magical Hack's basic
/// land types, Sleight of Mind's color words. Names the *vocabulary* only; which word becomes
/// which is picked as [`Effect::Choice(ChoiceEffect::ChangeText)`] resolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum TextWords {
    /// "…replacing all instances of one basic land type with another" (Magical Hack).
    BasicLandType,
    /// "…replacing all instances of one color word with another" (Sleight of Mind).
    Color,
}

impl TextWords {
    /// The words this vocabulary offers, in printed order — the picker's candidate list, asked
    /// once for the word being replaced and once for its replacement.
    pub fn options(self) -> &'static [&'static str] {
        match self {
            TextWords::BasicLandType => BASIC_LAND_TYPES,
            TextWords::Color => COLOR_WORDS,
        }
    }

    /// What this vocabulary is called in the printed sentence — "one **basic land type**", "one
    /// **color word**" — for rendering the effect.
    pub fn label(self) -> &'static str {
        match self {
            TextWords::BasicLandType => "basic land type",
            TextWords::Color => "color word",
        }
    }

    /// The substitution two picked words describe, or `None` if either isn't one of
    /// [`options`](Self::options). The picker only ever offers those, so this is the defensive
    /// read at the answer boundary rather than a case a real answer reaches.
    pub fn swap(self, from: &str, to: &str) -> Option<TextSwap> {
        match self {
            TextWords::BasicLandType => Some(TextSwap::LandType {
                from: BasicLandType::from_subtype(from)?,
                to: BasicLandType::from_subtype(to)?,
            }),
            TextWords::Color => Some(TextSwap::Color {
                from: crate::Color::from_word(from)?,
                to: crate::Color::from_word(to)?,
            }),
        }
    }
}

/// One resolved "replace all instances of one word with another" (CR 612.1), riding the object
/// whose text it changed ([`Permanent::text_swap`] / [`Spell::text_swap`]).
///
/// `Copy` so it fits in those `Copy` structs — which is also what gives it the right duration for
/// nothing: "this effect lasts indefinitely" means as long as *this* object does, and a card that
/// changes zones is a new object (CR 400.7) that starts with no swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextSwap {
    /// Magical Hack: "swampwalk" becomes "plainswalk", and a Swamp becomes a Plains.
    LandType {
        from: BasicLandType,
        to: BasicLandType,
    },
    /// Sleight of Mind: "protection from black" becomes "protection from blue".
    Color {
        from: crate::Color,
        to: crate::Color,
    },
}

impl TextSwap {
    /// The two words this swap reads as, replaced first — for the log line ("Swamp becomes
    /// Plains") and nothing else; every rule read goes through the typed methods below.
    pub fn words(self) -> (&'static str, &'static str) {
        match self {
            TextSwap::LandType { from, to } => (from.as_str(), to.as_str()),
            TextSwap::Color { from, to } => (from.word(), to.word()),
        }
    }

    /// This swap applied to one printed land subtype — the layer-3 read behind
    /// [`Game::effective_subtypes`](crate::Game::effective_subtypes), which is in turn what makes
    /// a Hacked basic tap for its new type ([`Game::land_mana_credit`](crate::Game)) and answer
    /// to "Destroy all Plains". Any other subtype, basic or not, passes through.
    pub(crate) fn subtype(self, subtype: &'static str) -> &'static str {
        let TextSwap::LandType { from, to } = self else {
            return subtype;
        };
        if subtype == from.as_str() {
            return to.as_str();
        }
        subtype
    }

    /// This swap applied to one printed keyword ability: landwalk's land type (Magical Hack's own
    /// reminder text — "you may change 'swampwalk' to 'plainswalk'") and protection's color
    /// (Sleight of Mind on a White Knight). Every other keyword names no word either card can
    /// replace, and passes through.
    pub(crate) fn keyword(self, keyword: Keyword) -> Keyword {
        match (self, keyword) {
            (TextSwap::LandType { from, to }, Keyword::Landwalk(land)) if land == from => {
                Keyword::Landwalk(to)
            }
            (
                TextSwap::Color { from, to },
                Keyword::ProtectionFrom(ProtectionScope::Color(color)),
            ) if color == from => Keyword::ProtectionFrom(ProtectionScope::Color(to)),
            _ => keyword,
        }
    }

    /// This swap applied to one printed ability.
    ///
    /// ponytail: reaches the color a Circle of Protection names — the interaction Sleight of Mind
    /// is printed for — through whatever `Sequence` nesting sits above it, and nothing else. Every
    /// other enumerated color or land type buried in an [`Effect`] (a filter's color, a land-type
    /// count, an Aura's keyword *grant*) passes through unchanged: `Effect` is a wide tree whose
    /// leaves are mostly `&'static` slices, so rewriting it wholesale would mean leaking a fresh
    /// slice on every read of a swapped object's abilities. Grow this match one shape at a time,
    /// from a card that wants one.
    pub(crate) fn ability(self, ability: &Ability) -> Ability {
        Ability {
            effect: self.effect(&ability.effect),
            ..ability.clone()
        }
    }

    fn effect(self, effect: &Effect) -> Effect {
        match effect {
            Effect::Sequence { steps } => Effect::Sequence {
                steps: steps.iter().map(|step| self.effect(step)).collect(),
            },
            // "The next time a red source of your choice would deal damage to you this turn" — the
            // one enumerated color the pool's abilities hold that Sleight of Mind is played to move.
            Effect::Misc(MiscEffect::PreventNextDamage { from_color, .. }) => {
                let TextSwap::Color { from, to } = self else {
                    return effect.clone();
                };
                if *from_color != ColorFilter::of(from) {
                    return effect.clone();
                }
                let mut swapped = effect.clone();
                if let Effect::Misc(MiscEffect::PreventNextDamage { from_color, .. }) = &mut swapped
                {
                    *from_color = ColorFilter::of(to);
                }
                swapped
            }
            _ => effect.clone(),
        }
    }
}

/// The half-finished state of a text-changing spell's two-word pick: "replacing all instances of
/// one basic land type with another" is two questions, and both are asked through the one
/// [`PendingChoice::ChooseCreatureType`] picker rather than a bespoke two-word prompt — the
/// picker already offers a `&'static [&'static str]` candidate list, which is all either question
/// needs. Carried on that choice as its `then` tail, the same way
/// [`SplittingContinuation`] carries what an answered opponent-pick should do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSwapPick {
    /// The spell or permanent whose text is being changed — the resolving spell's target.
    pub object: ObjectId,
    /// Which vocabulary both words come from.
    pub words: TextWords,
    /// The word picked first ("all instances of *this*"), `None` while that first pick is the
    /// question still open.
    pub from: Option<&'static str>,
}
