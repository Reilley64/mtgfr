import * as Combobox from "@foldkit/ui/combobox";

/** The "name a card" typeahead. Items are whatever the catalog search returned for the current
 *  query, so the item type is the card name itself. */
export const CardNameCombobox = Combobox.create<string>();

/** Document-unique id. Combobox keys its input id, ARIA wiring, focus, and anchoring commands
 *  on it. */
export const CARD_NAME_COMBOBOX_ID = "prompt-name";
