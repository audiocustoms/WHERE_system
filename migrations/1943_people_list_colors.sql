-- Colours for sub-departments and positions.
--
-- Every list of the employee master now carries a colour: a sub-department and a
-- position are tinted like a department and a location. The colour is maintained
-- with the entry (`Stammdaten → Abteilungen und Positionen`); empty means none.

ALTER TABLE "where_people_subdepartments"
    ADD COLUMN IF NOT EXISTS "color" text;

ALTER TABLE "where_people_subdepartments"
    DROP CONSTRAINT IF EXISTS "where_people_subdepartments_color_ck";

ALTER TABLE "where_people_subdepartments"
    ADD CONSTRAINT "where_people_subdepartments_color_ck"
    CHECK ("color" IS NULL OR "color" ~ '^#[0-9A-F]{6}$');

COMMENT ON COLUMN "where_people_subdepartments"."color" IS
    'Colour of the sub-department (#RRGGBB, empty = none).';

ALTER TABLE "where_people_positions"
    ADD COLUMN IF NOT EXISTS "color" text;

ALTER TABLE "where_people_positions"
    DROP CONSTRAINT IF EXISTS "where_people_positions_color_ck";

ALTER TABLE "where_people_positions"
    ADD CONSTRAINT "where_people_positions_color_ck"
    CHECK ("color" IS NULL OR "color" ~ '^#[0-9A-F]{6}$');

COMMENT ON COLUMN "where_people_positions"."color" IS
    'Colour of the position (#RRGGBB, empty = none).';
