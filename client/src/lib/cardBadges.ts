// Arena-style status + keyword badge specs for battlefield cards.
// Glyphs come from the Mana icon font (Arena ability recreations).

/** Evergreen keywords painted as icons, in combat-readability order. */
export const BADGE_KEYWORDS = [
  "flying",
  "first_strike",
  "double_strike",
  "vigilance",
  "haste",
  "trample",
  "deathtouch",
  "lifelink",
  "menace",
  "reach",
  "defender",
  "unblockable",
  "indestructible",
  "hexproof",
  "shroud",
  "flash",
  "prowess",
] as const;

/** How many keyword icons fit on a card face before collapsing into "+N". */
export const MAX_KEYWORD_BADGES = 4;

/** Mana font codepoints for wire keyword ids (from mana-font / `.ms-ability-*`). */
const ABILITY_GLYPH: Record<string, string> = {
  flying: "\ue952",
  first_strike: "\ue950",
  double_strike: "\ue94d",
  vigilance: "\ue968",
  haste: "\ue953",
  trample: "\ue964",
  deathtouch: "\ue94b",
  lifelink: "\uea4b",
  menace: "\ue95d",
  reach: "\ue960",
  defender: "\ue94c",
  unblockable: "\uea5c",
  indestructible: "\ue95a",
  hexproof: "\ue954",
  shroud: "\uea88",
  flash: "\ue951",
  prowess: "\ue982",
  ward: "\ue992",
  "protection:white": "\uea83",
  "protection:blue": "\uea80",
  "protection:black": "\uea7f",
  "protection:red": "\uea82",
  "protection:green": "\uea81",
  summoning_sick: "\ue96a",
  goaded: "\ue9c9",
};

/** Mana-font tap symbol (`.ms-tap`) — auto-tap payment preview on battlefield cards. */
export const TAP_GLYPH = "\ue61a";

/** Private-use glyph for a wire keyword, or null if Mana has no matching ability symbol. */
export function abilityGlyph(keyword: string): string | null {
  if (keyword.startsWith("ward:")) return ABILITY_GLYPH.ward;
  return ABILITY_GLYPH[keyword] ?? null;
}

/** Ordered, deduped keyword badge ids for a permanent — combat-relevant first, capped with overflow. */
export function keywordBadges(keywords: readonly string[]): {
  shown: string[];
  overflow: number;
} {
  const present = new Set(keywords);
  const ordered: string[] = [];
  for (const id of BADGE_KEYWORDS) {
    if (!present.has(id)) continue;
    ordered.push(id);
  }
  // Parametrized keywords after evergreens (ward:N, protection:color).
  for (const raw of keywords) {
    if (ordered.includes(raw)) continue;
    if (raw.startsWith("ward:") || raw.startsWith("protection:")) ordered.push(raw);
  }
  if (ordered.length <= MAX_KEYWORD_BADGES) return { shown: ordered, overflow: 0 };
  return {
    shown: ordered.slice(0, MAX_KEYWORD_BADGES),
    overflow: ordered.length - MAX_KEYWORD_BADGES,
  };
}

/** How many keywords are hidden after painting `painted` of `shown` (plus pre-cap overflow). */
export function hiddenKeywordCount(shownLen: number, painted: number, overflow: number): number {
  return overflow + Math.max(0, shownLen - painted);
}

/** True when the permanent can't attack yet (sick without haste). */
export function showsSummoningSick(summoningSick: boolean, hasHaste: boolean): boolean {
  return summoningSick && !hasHaste;
}

/**
 * A donated / stolen / exchanged permanent sits under a controller different from its owner
 * (CR 108.3 — ownership never changes; CR 800.4a control layers do). It renders in its
 * controller's row, so it needs an owner badge to show whose card it really is. Returns the owner
 * seat when it differs from the controller, else null (no badge on a normally-controlled permanent).
 */
export function foreignOwnerSeat(owner: number, controller: number): number | null {
  return owner === controller ? null : owner;
}
