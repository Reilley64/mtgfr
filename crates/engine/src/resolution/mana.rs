//! Mana-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (ADR 0002 / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    pub(crate) fn mint_mana_family(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Vec<Event> {
        let _source_name = self.source_name_of(source);
        match effect {
            // Add `repeat` copies of the mana batch — one ManaAdded event per mana kind.
            // ponytail: a pool holds at most 255 of any one mana (u8); a burst this large never
            // arises in the soc pool, so an over-255 repeat saturates rather than widening the type.
            // `single_color` is handled by `Game::activate_ability` before a mana ability ever (CR 605, CR 113)
            // reaches here (it pauses on `ChooseManaColor` instead) — ignored via `..`.
            Effect::AddMana {
                mana: produced,
                identity,
                opponent_colors,
                repeat,
                restriction,
                persist_until_end_of_turn,
                ..
            } => {
                // Wrap the static batch as [`Mana::Restricted`] if this ability's mana is
                // spend-restricted (Troyan, Gutsy Explorer) — a no-op otherwise. A granted mana
                // ability's batch (Galazeth Prismari) already arrives pre-wrapped from
                // `Game::granted_mana_abilities` with `restriction: None` here, so this is
                // harmless to call regardless.
                let produced = produced.restricted_by(restriction);
                let repeat = self
                    .resolve_count(repeat, controller, source, target, x)
                    .min(u8::MAX as u32) as u8;
                let mut events = Vec::new();
                let mut push = |mana: Mana, amount: u8| {
                    let amount = amount.saturating_mul(repeat);
                    if amount > 0 {
                        events.push(Event::ManaAdded {
                            player: controller,
                            mana,
                            amount,
                            persist: persist_until_end_of_turn,
                        });
                    }
                };
                for (color, &n) in Color::ALL.iter().zip(produced.colored.iter()) {
                    push(Mana::Color(*color), n);
                }
                push(Mana::Colorless, produced.colorless);
                push(Mana::Any, produced.any);
                // Dual credits (filter lands' "{W}{W}/{W}{B}/{B}{B}", a painland's colored mode).
                for (&(a, b), &n) in COLOR_PAIRS.iter().zip(produced.either.iter()) {
                    push(Mana::Either(a, b), n);
                }
                // Fixed 2-4 color-choice credits (Treva's Ruins' "{T}: Add {G}, {W}, or {U}"),
                // keyed by their WUBRG bitmask — the static-batch twin of the `either` loop above.
                for (mask, &n) in produced.of_colors.iter().enumerate() {
                    push(Mana::OfColors(mask as u8), n);
                }
                // Spend-restricted credits (Troyan's own restriction above, or a granted mana
                // ability's pre-wrapped batch — Galazeth's Treasures-style grant).
                for slot in produced.restricted {
                    if let Some((base, restriction)) = slot.key {
                        push(Mana::Restricted { base, restriction }, slot.amount);
                    }
                }
                // "One mana of any color in your commander's color identity" (CR 903.4, Arcane
                // Signet): resolved to a real credit now, since the identity depends on
                // `controller`'s commander — it can't be baked into the static `mana` batch above.
                if identity > 0
                    && let Some(credit) = self.commander_identity_credit(controller)
                {
                    push(credit, identity);
                }
                // "One mana of any color that a land an opponent controls could produce"
                // (Fellwar Stone, Exotic Orchard): resolved to a real credit now, since the
                // producible set depends on the current board — it can't be baked into the
                // static `mana` batch above. No credit at all (`None`) if no opponent land
                // produces a color.
                if opponent_colors > 0
                    && let Some(credit) = self.opponent_producible_colors_credit(controller)
                {
                    push(credit, opponent_colors);
                }
                events
            }

            _ => unreachable!("mana family mint received a non-family effect"),
        }
    }
}
