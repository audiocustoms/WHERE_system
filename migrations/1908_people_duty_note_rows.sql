-- Free-text rows above the employees: each row carries one text per day.
--
-- The board adds a row with its `+` button and removes it with `-`; every day
-- column of that row holds its own text.

DROP TABLE IF EXISTS "where_people_duty_day_notes";

CREATE TABLE IF NOT EXISTS "where_people_duty_note_rows" (
    "id" bigserial PRIMARY KEY,
    "sort_order" integer NOT NULL DEFAULT 0,
    "created_at" timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS "where_people_duty_note_cells" (
    "row_id" bigint NOT NULL
        REFERENCES "where_people_duty_note_rows" ("id") ON DELETE CASCADE,
    "day" date NOT NULL,
    "note" text NOT NULL,
    "changed_at" timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("row_id", "day"),
    CONSTRAINT "where_people_duty_note_cells_text_ck"
        CHECK (length(btrim("note")) BETWEEN 1 AND 200)
);

CREATE INDEX IF NOT EXISTS "where_people_duty_note_cells_day_idx"
    ON "where_people_duty_note_cells" ("day");

COMMENT ON TABLE "where_people_duty_note_rows" IS
    'Free-text rows of the duty plan, shown between the dates and the employees.';
COMMENT ON TABLE "where_people_duty_note_cells" IS
    'Text of one day within a free-text row.';
COMMENT ON COLUMN "where_people_duty_note_cells"."note" IS
    'The text under one date (1-200 characters).';
