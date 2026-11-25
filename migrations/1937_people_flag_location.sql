-- The staffing of a flag belongs to a location.
--
-- A `Vst` in the Großes Haus needs another crew than a `Vst` in the Podium. The
-- rows that stand today were meant for every house, so they are copied to each
-- location once; from then on every house keeps its own numbers.

ALTER TABLE "where_people_flag_staffing"
    ADD COLUMN IF NOT EXISTS "location_id" bigint;

-- The old index allows one row per position and flag; the copy below needs the
-- same position in every house, so it goes first.
DROP INDEX IF EXISTS "where_people_flag_staffing_position_idx";

INSERT INTO "where_people_flag_staffing"
    ("flag_id", "location_id", "position", "needed", "sort_order")
SELECT s."flag_id", l."id", s."position", s."needed", s."sort_order"
  FROM "where_people_flag_staffing" s
 CROSS JOIN "where_people_locations" l
 WHERE s."location_id" IS NULL;

DELETE FROM "where_people_flag_staffing" WHERE "location_id" IS NULL;

ALTER TABLE "where_people_flag_staffing"
    ALTER COLUMN "location_id" SET NOT NULL;

ALTER TABLE "where_people_flag_staffing"
    ADD CONSTRAINT "where_people_flag_staffing_location_fk"
    FOREIGN KEY ("location_id") REFERENCES "where_people_locations" ("id") ON DELETE CASCADE;

CREATE UNIQUE INDEX IF NOT EXISTS "where_people_flag_staffing_position_idx"
    ON "where_people_flag_staffing" ("flag_id", "location_id", lower(btrim("position")));

CREATE INDEX IF NOT EXISTS "where_people_flag_staffing_location_idx"
    ON "where_people_flag_staffing" ("location_id");

COMMENT ON TABLE "where_people_flag_staffing" IS
    'Staffing of a flag per location: which positions a house needs and how many, `*` for any position.';
COMMENT ON COLUMN "where_people_flag_staffing"."location_id" IS
    'House the staffing belongs to; a location without rows for a flag asks for nobody.';
