-- For databases that applied a partial 0001 (commander_print only, before history
-- registration). Idempotent: safe after the full 0001 as well.
DROP TABLE IF EXISTS "catalog_cards";
-- #[toasty::breakpoint]
CREATE TABLE "catalog_cards" (
    "id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "search_blob" TEXT NOT NULL,
    "card_json" TEXT NOT NULL,
    PRIMARY KEY ("id")
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_catalog_cards_by_name" ON "catalog_cards" ("name");
