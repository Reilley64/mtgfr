/** Project one Scryfall printing into the flavor fields owned by printing TOML records. */
export function printingFlavors(card) {
  if (!Array.isArray(card.card_faces)) {
    return { flavor: typeof card.flavor_text === "string" ? card.flavor_text : "", faces: [] };
  }
  return {
    flavor: typeof card.card_faces[0]?.flavor_text === "string" ? card.card_faces[0].flavor_text : "",
    faces: card.card_faces
      .filter((face) => typeof face.name === "string")
      .map((face) => ({
        name: face.name,
        flavor: typeof face.flavor_text === "string" ? face.flavor_text : null,
      })),
  };
}
