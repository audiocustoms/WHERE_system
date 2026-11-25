-- The band of a shift is stored in English.
--
-- `frueh`, `spaet`, `zwischen` and `frei` were the values of the first version. The
-- code names them `early`, `late`, `intermediate` and `free`, so the column follows: a
-- sheet in German still says `früh` or `spät`, the import translates it.

ALTER TABLE "where_people_shifts"
    DROP CONSTRAINT IF EXISTS "where_people_shifts_band_ck";

ALTER TABLE "where_people_shifts"
    ALTER COLUMN "band" SET DEFAULT 'early';

UPDATE "where_people_shifts"
   SET "band" = CASE lower(btrim("band"))
                    WHEN 'frueh' THEN 'early'
                    WHEN 'spaet' THEN 'late'
                    WHEN 'zwischen' THEN 'intermediate'
                    WHEN 'frei' THEN 'free'
                    ELSE lower(btrim("band"))
                END;

ALTER TABLE "where_people_shifts"
    ADD CONSTRAINT "where_people_shifts_band_ck"
    CHECK ("band" IN ('early', 'late', 'intermediate', 'free'));

COMMENT ON COLUMN "where_people_shifts"."band" IS
    'Half of the day the shift belongs to: early, late, intermediate — or free for a whole free day.';
