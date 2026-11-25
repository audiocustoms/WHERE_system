-- One free-text note per day, shown under the date of the duty plan.
--
-- The first shape (rows spanning the whole period) did not match the use case, so
-- it is replaced by one text per day.

DROP TABLE IF EXISTS "where_people_duty_notes";

CREATE TABLE IF NOT EXISTS "where_people_duty_day_notes" (
    "id" bigserial PRIMARY KEY,
    "day" date NOT NULL UNIQUE,
    "note" text NOT NULL,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "changed_at" timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT "where_people_duty_day_notes_text_ck"
        CHECK (length(btrim("note")) BETWEEN 1 AND 200)
);

COMMENT ON TABLE "where_people_duty_day_notes" IS
    'Free-text note of a single day of the duty plan.';
COMMENT ON COLUMN "where_people_duty_day_notes"."day" IS
    'The day the note belongs to (at most one note per day).';
COMMENT ON COLUMN "where_people_duty_day_notes"."note" IS
    'The text under the date (1-200 characters).';
