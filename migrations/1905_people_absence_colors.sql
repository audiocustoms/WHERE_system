-- Colours of the absences in the duty plan.
--
-- `Krank` and `Urlaub` are their own cell kinds of the board; their colours can
-- be chosen in the duty plan settings.

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "vacation_color" text NOT NULL DEFAULT '#0369A1';

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "sick_color" text NOT NULL DEFAULT '#B91C1C';

ALTER TABLE "where_people_planning"
    DROP CONSTRAINT IF EXISTS "where_people_planning_colors_ck";

ALTER TABLE "where_people_planning"
    ADD CONSTRAINT "where_people_planning_colors_ck"
    CHECK (
        "vacation_color" ~ '^#[0-9A-F]{6}$'
        AND "sick_color" ~ '^#[0-9A-F]{6}$'
    );

COMMENT ON COLUMN "where_people_planning"."vacation_color" IS
    'Colour of the vacation cells in the duty plan (#RRGGBB).';
COMMENT ON COLUMN "where_people_planning"."sick_color" IS
    'Colour of the sick cells in the duty plan (#RRGGBB).';
