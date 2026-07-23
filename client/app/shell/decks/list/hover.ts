import { Schema as S } from "effect";

export const DeckListHover = S.Struct({
  id: S.String,
  print: S.String,
  x: S.Number,
  y: S.Number,
});
export type DeckListHover = typeof DeckListHover.Type;
