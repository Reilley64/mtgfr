/** Zone discriminants — must match `engine::Zone`'s declaration order. */
export const ZONE = {
  Library: 0,
  Hand: 1,
  Battlefield: 2,
  Graveyard: 3,
  Exile: 4,
  Command: 5,
  Stack: 6,
} as const;
