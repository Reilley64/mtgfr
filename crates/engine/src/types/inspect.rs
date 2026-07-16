use super::Keyword;

/// One inspect-ledger contribution from a source card def.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModifierContribution {
    PowerToughness { power: i32, toughness: i32 },
    SetBasePowerToughness { power: i32, toughness: i32 },
    Keyword(Keyword),
    PlusCounters(i32),
    Goaded,
    Controls,
    ManaAbility,
}

/// Contributions attributed to one source card def, grouped for the inspect ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModifierSourceGroup {
    pub source_name: &'static str,
    /// Card id when a matching battlefield permanent is still present; empty for synthetic labels.
    pub source_card_id: &'static str,
    pub contributions: Vec<ModifierContribution>,
}
