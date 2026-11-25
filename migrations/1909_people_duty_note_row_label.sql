-- A label in front of the remove symbol of a free-text row.
--
-- The label names the row ("Technik", "Bühne links"); it is optional and belongs
-- to the row, not to a single day.

ALTER TABLE "where_people_duty_note_rows"
    ADD COLUMN IF NOT EXISTS "label" text NOT NULL DEFAULT '';

ALTER TABLE "where_people_duty_note_rows"
    DROP CONSTRAINT IF EXISTS "where_people_duty_note_rows_label_ck";

ALTER TABLE "where_people_duty_note_rows"
    ADD CONSTRAINT "where_people_duty_note_rows_label_ck"
    CHECK (length(btrim("label")) <= 200);

COMMENT ON COLUMN "where_people_duty_note_rows"."label" IS
    'Optional name of the row, shown left of the remove symbol (up to 200 characters).';
