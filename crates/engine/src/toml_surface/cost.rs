use serde::Deserialize;
use serde::de::{self, Deserializer};

use crate::de::intern;
use crate::{AdditionalCost, Amount, Color, Cost};

/// `[cost]`'s `x` key: the common case `x = true` (a single `{X}`) or an integer count of
/// `{X}` symbols (`x = 3` for Astral Cornucopia's `{X}{X}{X}`, CR 107.3). `false`/absent means
/// no `{X}`. Untagged so TOML's own scalar type picks the arm.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum XPips {
    Bool(bool),
    Count(u8),
}

impl Default for XPips {
    fn default() -> Self {
        XPips::Bool(false)
    }
}

impl From<XPips> for u8 {
    fn from(pips: XPips) -> u8 {
        match pips {
            XPips::Bool(false) => 0,
            XPips::Bool(true) => 1,
            XPips::Count(n) => n,
        }
    }
}

/// A `[cost]` table spells each color by name (`white = 1`) rather than as the
/// [`Cost::colored`] WUBRG array; every field is optional.
#[derive(Debug, Clone, Deserialize, Default)]
#[cfg_attr(
    feature = "card-schema",
    derive(schemars::JsonSchema),
    schemars(deny_unknown_fields)
)]
#[serde(default, deny_unknown_fields)]
pub struct CostToml {
    /// Generic mana pips such as `{2}`.
    pub generic: u8,
    /// White mana pips (`{W}`).
    pub white: u8,
    /// Blue mana pips (`{U}`).
    pub blue: u8,
    /// Black mana pips (`{B}`).
    pub black: u8,
    /// Red mana pips (`{R}`).
    pub red: u8,
    /// Green mana pips (`{G}`).
    pub green: u8,
    /// Colorless mana pips (`{C}`), payable only by colorless mana. This is not a color.
    pub colorless: u8,
    /// `{X}` pips. `true` means one `{X}`; an integer gives the count of `{X}` symbols.
    pub x: XPips,
    /// Hybrid mana pips (CR 107.4e — `{a/b}`): a list of two-color arrays, one per
    /// hybrid symbol (`hybrid = [["black", "green"]]` for one `{B/G}`).
    pub hybrid: Vec<[Color; 2]>,
    /// Phyrexian mana pips (CR 107.4f — `{a/P}`): a list of colors, one per Phyrexian
    /// symbol (`phyrexian = ["black"]` for one `{B/P}`, Vraska, Betrayal's Sting's cost).
    pub phyrexian: Vec<Color>,
    /// `[cost.additional]` — an additional cost paid alongside mana (CR 601.2f).
    pub additional: AdditionalCost,
    /// A spell's own board-derived generic reduction (Blasphemous Act's "costs {1} less
    /// ... for each creature on the battlefield"), e.g.
    /// `reduce_own_generic = "per_creature_on_battlefield"`.
    pub reduce_own_generic: Option<Amount>,
}

impl CostToml {
    pub(crate) fn validate_hybrid<E: de::Error>(&self) -> Result<(), E> {
        for [a, b] in &self.hybrid {
            if a == b {
                return Err(de::Error::custom(
                    "a hybrid pip's two colors must differ (spell a mono pip as a colored cost)",
                ));
            }
        }
        Ok(())
    }
}

pub(crate) fn deserialize_cost_toml<'de, D>(d: D) -> Result<CostToml, D::Error>
where
    D: Deserializer<'de>,
{
    let cost = CostToml::deserialize(d)?;
    cost.validate_hybrid()?;
    Ok(cost)
}

impl From<CostToml> for Cost {
    fn from(cost: CostToml) -> Self {
        let mut hybrid = Vec::with_capacity(cost.hybrid.len());
        for [a, b] in cost.hybrid {
            // Normalize to WUBRG order so either spelling interns identically, mirroring
            // Mana::Either's dual-symbol normalization.
            hybrid.push(if a.index() < b.index() {
                (a, b)
            } else {
                (b, a)
            });
        }

        Cost {
            generic: cost.generic,
            colored: [cost.white, cost.blue, cost.black, cost.red, cost.green],
            colorless: cost.colorless,
            x: cost.x.into(),
            hybrid: intern(hybrid),
            phyrexian: intern(cost.phyrexian),
            additional: cost.additional,
            reduce_own_generic: cost.reduce_own_generic,
        }
    }
}
