-- The stock history keeps the *name* of the place it booked, not only its id.
--
-- A warehouse can be renamed and the journal follows (it resolves the name at read
-- time). A place can be removed once nothing stands there any more — a line that only
-- carries the id would then lose its meaning, and "Ort -> " is not a booking anybody can
-- read later. The label of that moment stays, like the bin and the condition do.

ALTER TABLE "where_stock_movement" ADD COLUMN IF NOT EXISTS "place_from_label" text;
ALTER TABLE "where_stock_movement" ADD COLUMN IF NOT EXISTS "place_to_label" text;

COMMENT ON COLUMN "where_stock_movement"."place_to_label" IS
    'Name of the place at the time of the booking (kept when the place is removed)';
COMMENT ON COLUMN "where_stock_movement"."place_from_label" IS
    'Name of the place at the time of the booking (kept when the place is removed)';

-- Lines written by the first version of the feature carry the id only; the name is
-- still there while the place exists.
UPDATE "where_stock_movement" m
   SET "place_to_label" = p."name"
  FROM "where_place" p
 WHERE p."id" = m."place_to"
   AND m."place_to_label" IS NULL;

UPDATE "where_stock_movement" m
   SET "place_from_label" = p."name"
  FROM "where_place" p
 WHERE p."id" = m."place_from"
   AND m."place_from_label" IS NULL;
