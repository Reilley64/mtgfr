-- Wipe user decks (identity rekey; no live data to migrate — ADR 0031).
TRUNCATE TABLE "decks";
-- #[toasty::breakpoint]
ALTER TABLE "decks" ADD COLUMN "commander_print" TEXT NOT NULL DEFAULT '';
-- #[toasty::breakpoint]
-- Rekey catalog projection by Card id.
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
