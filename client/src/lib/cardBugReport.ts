// Build a GitHub "new issue" URL for the card-bug form, prefilled from in-game inspect.

const ISSUE_NEW = "https://github.com/reilley64/mtgfr/issues/new";
const TEMPLATE = "card-bug.yml";

export type CardBugReportFields = {
  cardName: string;
  tableId: string;
  /** Catalog Card (oracle) id when known — finds the script under `crates/cards/data/`. */
  cardId?: string;
  /** Battlefield object id when Alt-pinning a permanent. */
  objectId?: number;
};

/** Issue form URL with `card_name` / `table_id` (and optional ids) as query prefills. */
export function cardBugReportUrl(fields: CardBugReportFields): string {
  const params = new URLSearchParams({
    template: TEMPLATE,
    title: `card: ${fields.cardName}`,
    card_name: fields.cardName,
    table_id: fields.tableId,
  });
  const cardId = fields.cardId?.trim();
  if (cardId) params.set("card_id", cardId);
  if (fields.objectId != null) params.set("object_id", String(fields.objectId));
  return `${ISSUE_NEW}?${params.toString()}`;
}
