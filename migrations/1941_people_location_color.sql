-- Colour of a location in the duty plan.
--
-- Like a department, a house can carry a colour: the name cell of its disposition
-- row takes it, so `Großes Haus` and `Podium` are told apart at a glance. The colour
-- is maintained with the location (`Stammdaten → Abteilungen und Positionen`);
-- empty means no tint.

ALTER TABLE "where_people_locations"
    ADD COLUMN IF NOT EXISTS "color" text;

ALTER TABLE "where_people_locations"
    DROP CONSTRAINT IF EXISTS "where_people_locations_color_ck";

ALTER TABLE "where_people_locations"
    ADD CONSTRAINT "where_people_locations_color_ck"
    CHECK ("color" IS NULL OR "color" ~ '^#[0-9A-F]{6}$');

COMMENT ON COLUMN "where_people_locations"."color" IS
    'Colour of the location in the disposition row of the duty plan (#RRGGBB, empty = no tint).';
