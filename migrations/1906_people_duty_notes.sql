-- Free-text notes above the employees of the duty plan.
--
-- They carry events or reminders ("Premiere Bühne links") and optionally belong to
-- a period; without a period they are shown in every view.

CREATE TABLE IF NOT EXISTS "where_people_duty_notes" (
    "id" bigserial PRIMARY KEY,
    "note" text NOT NULL,
    "starts_on" date,
    "ends_on" date,
    "sort_order" integer NOT NULL DEFAULT 0,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "changed_at" timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT "where_people_duty_notes_text_ck"
        CHECK (length(btrim("note")) BETWEEN 1 AND 200),
    CONSTRAINT "where_people_duty_notes_period_ck"
        CHECK (
            ("starts_on" IS NULL AND "ends_on" IS NULL)
            OR (
                "starts_on" IS NOT NULL
                AND "ends_on" IS NOT NULL
                AND "starts_on" <= "ends_on"
            )
        )
);

CREATE INDEX IF NOT EXISTS "where_people_duty_notes_period_idx"
    ON "where_people_duty_notes" ("starts_on", "ends_on");

COMMENT ON TABLE "where_people_duty_notes" IS
    'Free-text rows of the duty plan, shown above the employees.';
COMMENT ON COLUMN "where_people_duty_notes"."note" IS
    'The text of the row (1-200 characters).';
COMMENT ON COLUMN "where_people_duty_notes"."starts_on" IS
    'First day the note is shown; NULL means always.';
COMMENT ON COLUMN "where_people_duty_notes"."ends_on" IS
    'Last day the note is shown; NULL means always.';
