-- Repair: web code since gravatar seats (#212) SELECTs gravatar_hash. If 0002 never
-- applied on mtgfr_web (Argo image roll without terraform edh-web-migrate), Host
-- create succeeds but join/lobby GET 500 → client "Couldn't reach the table".
ALTER TABLE "lobby_seats" ADD COLUMN IF NOT EXISTS "gravatar_hash" text DEFAULT '' NOT NULL;
