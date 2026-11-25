-- An event of the disposition that the plan cannot use — no time, a text the flags
-- do not read — can be set aside: it stays in the disposition (the board keeps
-- showing it above the employees) but asks for nobody and no longer marks its day
-- as unclear.

ALTER TABLE "where_people_dispo_entries"
    ADD COLUMN IF NOT EXISTS "ignored" boolean NOT NULL DEFAULT false;

COMMENT ON COLUMN "where_people_dispo_entries"."ignored" IS
    'The event is kept but asks for nobody and marks no day as unclear.';
