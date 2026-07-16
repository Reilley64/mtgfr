CREATE UNIQUE INDEX "lobby_seats_table_user_uidx" ON "lobby_seats" USING btree ("table_id","user_id");
