-- What a shift is for and who may work it.
--
-- The disposition of PeM names no shift, only a start time. To turn an event
-- into demand the planning has to know which shift an event belongs to and
-- which shifts are open to everybody:
--
--   band            frueh | spaet | zwischen
--                   `frueh` and `spaet` are the two halves of the day; the
--                   boundary is the start of the earliest `spaet` shift (13:30
--                   in the theatre). `zwischen` marks the Zwischenschichten:
--                   an emergency means for the days where neither half fits,
--                   never a target of the automatic derivation.
--   apprentice_only  a shift that only apprentices may work. The apprentice
--                   shifts of the theatre (F-AZ, S-AZ, S-WE-AZ) are marked
--                   here, and `src/duty_repo.rs` refuses anybody else.

ALTER TABLE "where_people_shifts"
    ADD COLUMN IF NOT EXISTS "band" text NOT NULL DEFAULT 'frueh';

ALTER TABLE "where_people_shifts"
    ADD COLUMN IF NOT EXISTS "apprentice_only" boolean NOT NULL DEFAULT false;

ALTER TABLE "where_people_shifts"
    DROP CONSTRAINT IF EXISTS "where_people_shifts_band_ck";
ALTER TABLE "where_people_shifts"
    ADD CONSTRAINT "where_people_shifts_band_ck"
    CHECK ("band" IN ('frueh', 'spaet', 'zwischen'));

-- The shifts the theatre already has, by their short name.
UPDATE "where_people_shifts" SET "band" = 'frueh' WHERE "short_name" = 'F';
UPDATE "where_people_shifts" SET "band" = 'frueh', "apprentice_only" = true
    WHERE "short_name" = 'F-AZ';
UPDATE "where_people_shifts" SET "band" = 'spaet' WHERE "short_name" = 'S';
UPDATE "where_people_shifts" SET "band" = 'spaet' WHERE "short_name" = 'S-WE';
UPDATE "where_people_shifts" SET "band" = 'spaet', "apprentice_only" = true
    WHERE "short_name" = 'S-AZ';
UPDATE "where_people_shifts" SET "band" = 'spaet', "apprentice_only" = true
    WHERE "short_name" = 'S-WE-AZ';
UPDATE "where_people_shifts" SET "band" = 'zwischen' WHERE "short_name" IN ('ZS1', 'ZS2');

COMMENT ON COLUMN "where_people_shifts"."band" IS
    'Half of the day the shift belongs to: frueh, spaet or zwischen.';
COMMENT ON COLUMN "where_people_shifts"."apprentice_only" IS
    'Only apprentices may be planned into this shift.';
