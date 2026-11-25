-- A cell of the duty plan may carry a note of its own.
--
-- The note belongs to the cell, not to the shift: it stands under the short name and
-- the time on the board and on the printed sheet — "Tausch mit Kollegen", "fährt den
-- Bus", "Einweisung". The cell, not the shift, keeps it, so it stays with the day
-- when the shift changes and it goes away with the cell.

ALTER TABLE "where_people_duty_entries"
    ADD COLUMN IF NOT EXISTS "note" text;

ALTER TABLE "where_people_duty_entries"
    DROP CONSTRAINT IF EXISTS "where_people_duty_note_len_ck";
ALTER TABLE "where_people_duty_entries"
    ADD CONSTRAINT "where_people_duty_note_len_ck"
    CHECK ("note" IS NULL OR length("note") <= 200);

COMMENT ON COLUMN "where_people_duty_entries"."note" IS
    'Free text of one cell of the plan, shown under the shift, at most 200 characters.';
