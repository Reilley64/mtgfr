# Product

## Register

product

## Users

A fixed group of friends (2–4 per table) playing Commander (EDH) together in the
browser, mostly desktop, mostly evening game sessions. Phones and tablets are
supported in **landscape only** — the board and builder assume horizontal space;
portrait gets a rotate prompt, not a reflowed vertical layout. One player hosts a
table; others claim seats with their own saved decks. Eliminated players stay as
spectators. They are fluent Magic players — the interface can assume the game's
vocabulary (priority, stack, tap, commander damage) and must never fight their
paper-Magic muscle memory.

## Product Purpose

A browser-based 4-player Commander table: authoritative rules engine on the
server, an MTGA-style board in the client, plus auth, lobby, and deck builder over
the card pool. The north star is to support *any* card, built faithfully — grown
from real cards with gaps flagged rather than forced (ADR 0014). The soc precons
(~389 unique cards) are the first faithful decks; success = games worth replaying
with rules that resolve correctly without manual bookkeeping.

## Brand Personality

MTGA-lite polish. Game-client energy — glowing priority indicators, snappy zone
transitions, drama when the stack grows — but with none of the free-to-play chrome.
Three words: **polished, snappy, focused**.

## Anti-references

- **F2P game chrome** — currency badges, battle-pass glow, reward-popup maximalism.
- **Generic SaaS dashboard** — stat tiles, cool-gray defaults; the game reduced to an admin panel.
- **Hobby-tool jank** — mismatched controls, debug-looking dense panels, unstyled defaults.

## Design Principles

1. **Polish conveys game state, never reward.** Glow and emphasis on priority, stack, combat, legality.
2. **The board is the hero; chrome recedes.** Auth, lobby, and builder are quiet surfaces.
3. **Readable from across the room.** Whose turn, priority, stack, and "you can act" at a glance.
4. **Paper-Magic grammar.** Drag to play, tapped is sideways, piles stack, arrows point at targets.
5. **One vocabulary, every screen.** Same buttons, panels, inputs across all surfaces.

## Accessibility & Inclusion

Best effort for a known friend group; no formal WCAG target. Readable contrast on dark felt, `prefers-reduced-motion` where motion exists, mana pips carry symbols not just hue.
