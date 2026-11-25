-- Locations of the disposition ("Großes Haus", "Podium", "Podium.bar", …) and the
-- events that take place there.
--
-- The locations are master data like departments and positions: the PeM import
-- fills them, and `Stammdaten → Personal → Abteilungen und Positionen` maintains
-- them by hand. Every event of a location is its own row, so the time and the
-- text stay editable — the board shows them above the employees.

CREATE TABLE IF NOT EXISTS "where_people_locations" (
    "id" bigserial PRIMARY KEY,
    "name" text NOT NULL,
    "sort_order" integer NOT NULL DEFAULT 0,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "changed_at" timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT "where_people_locations_name_ck"
        CHECK (length(btrim("name")) BETWEEN 1 AND 100)
);

CREATE UNIQUE INDEX IF NOT EXISTS "where_people_locations_name_idx"
    ON "where_people_locations" (lower(btrim("name")));

CREATE TABLE IF NOT EXISTS "where_people_dispo_entries" (
    "id" bigserial PRIMARY KEY,
    "location_id" bigint NOT NULL
        REFERENCES "where_people_locations" ("id") ON DELETE CASCADE,
    "day" date NOT NULL,
    "starts_at" time NULL,
    "title" text NOT NULL,
    "sort_order" integer NOT NULL DEFAULT 0,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "changed_at" timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT "where_people_dispo_entries_title_ck"
        CHECK (length(btrim("title")) BETWEEN 1 AND 200)
);

CREATE INDEX IF NOT EXISTS "where_people_dispo_entries_day_idx"
    ON "where_people_dispo_entries" ("day", "location_id");

COMMENT ON TABLE "where_people_locations" IS
    'Locations of the disposition (stage, foyer, bar); master data in the personnel list.';
COMMENT ON TABLE "where_people_dispo_entries" IS
    'One event of one location on one day, shown above the employees of the duty plan.';
COMMENT ON COLUMN "where_people_dispo_entries"."starts_at" IS
    'Start of the event; empty when the export carried no time.';
COMMENT ON COLUMN "where_people_dispo_entries"."title" IS
    'Event without its time, at most 200 characters.';
