import { Schema as S } from "effect";
import { DeckBuilderSubmodel, initialDeckBuilderSubmodel } from "./builder/submodel";
import { DeckListSubmodel, initialDeckListSubmodel } from "./list/submodel";

export const DecksSubmodel = S.Struct({
  builder: DeckBuilderSubmodel,
  list: DeckListSubmodel,
});
export type DecksSubmodel = typeof DecksSubmodel.Type;

export function initialDecksSubmodel(): DecksSubmodel {
  return {
    builder: initialDeckBuilderSubmodel(),
    list: initialDeckListSubmodel(),
  };
}
