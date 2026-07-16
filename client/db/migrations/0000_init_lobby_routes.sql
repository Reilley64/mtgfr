CREATE TABLE "lobbies" (
	"table_id" text PRIMARY KEY NOT NULL,
	"host_user_id" integer NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"last_activity" timestamp with time zone DEFAULT now() NOT NULL,
	"started_at" timestamp with time zone
);
--> statement-breakpoint
CREATE TABLE "lobby_seats" (
	"table_id" text NOT NULL,
	"seat" integer NOT NULL,
	"user_id" integer NOT NULL,
	"username" text NOT NULL,
	"deck_id" integer NOT NULL,
	"deck_name" text NOT NULL,
	"ready" boolean DEFAULT false NOT NULL,
	CONSTRAINT "lobby_seats_table_id_seat_pk" PRIMARY KEY("table_id","seat")
);
--> statement-breakpoint
CREATE TABLE "table_routes" (
	"table_id" text PRIMARY KEY NOT NULL,
	"pod_dns" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"expires_at" timestamp with time zone NOT NULL
);
--> statement-breakpoint
ALTER TABLE "lobby_seats" ADD CONSTRAINT "lobby_seats_table_id_lobbies_table_id_fk" FOREIGN KEY ("table_id") REFERENCES "public"."lobbies"("table_id") ON DELETE cascade ON UPDATE no action;