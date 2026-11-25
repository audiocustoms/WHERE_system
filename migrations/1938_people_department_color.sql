-- Colour of a department in the duty plan.
--
-- The name column of every employee of that department is tinted with it, so a
-- board with two departments shows at a glance who belongs where. The colour is
-- maintained with the department (`Stammdaten → Abteilungen und Positionen`);
-- empty means no tint.

ALTER TABLE "where_people_departments"
    ADD COLUMN IF NOT EXISTS "color" text;

ALTER TABLE "where_people_departments"
    DROP CONSTRAINT IF EXISTS "where_people_departments_color_ck";

ALTER TABLE "where_people_departments"
    ADD CONSTRAINT "where_people_departments_color_ck"
    CHECK ("color" IS NULL OR "color" ~ '^#[0-9A-F]{6}$');

COMMENT ON COLUMN "where_people_departments"."color" IS
    'Colour of the department in the name column of the duty plan (#RRGGBB, empty = no tint).';
