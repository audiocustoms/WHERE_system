-- Flags of a free-text row of the duty plan.
--
-- A row stands for what the day is about ("Premiere Bühne links"); its flags say
-- which kind of day it is in the vocabulary of the disposition (`Vst`, `Probe`,
-- `TE`). The row keeps any number of them, they are master data of the module, so
-- a flag that is deleted leaves the rows without it.

CREATE TABLE IF NOT EXISTS "where_people_duty_note_flags" (
    "row_id" bigint NOT NULL
        REFERENCES "where_people_duty_note_rows" ("id") ON DELETE CASCADE,
    "flag_id" bigint NOT NULL
        REFERENCES "where_people_flags" ("id") ON DELETE CASCADE,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("row_id", "flag_id")
);

CREATE INDEX IF NOT EXISTS "where_people_duty_note_flags_flag_idx"
    ON "where_people_duty_note_flags" ("flag_id");

COMMENT ON TABLE "where_people_duty_note_flags" IS
    'Flags of a free-text row of the duty plan; a row may carry any number of them.';
COMMENT ON COLUMN "where_people_duty_note_flags"."row_id" IS
    'The free-text row the flag belongs to.';
COMMENT ON COLUMN "where_people_duty_note_flags"."flag_id" IS
    'A flag of the disposition (`where_people_flags`).';
