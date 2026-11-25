-- A row of the disposition whose name is free text instead of a location.
--
-- The board knows two kinds of dispo rows: the houses of the master data and rows a
-- planner types himself ("Technik", "Bühne links"). A typed row carries everything a
-- house carries — events per day, the staffing each flag needs and whether a foreman
-- is planned — so it *is* a location with `is_text = true`; only its name is not
-- maintained in the master data. The employee mask and the master-data list leave
-- those rows out, the board, the print and the demand use them like any other house.

ALTER TABLE "where_people_locations"
    ADD COLUMN IF NOT EXISTS "is_text" boolean NOT NULL DEFAULT false;

COMMENT ON COLUMN "where_people_locations"."is_text" IS
    'The row was typed on the board instead of maintained as a house; its name is free text.';

-- Two typed rows may share a name, a house may not: the uniqueness moves to the houses.
DROP INDEX IF EXISTS "where_people_locations_name_idx";

CREATE UNIQUE INDEX IF NOT EXISTS "where_people_locations_name_idx"
    ON "where_people_locations" (lower(btrim("name")))
    WHERE "is_text" = false;

-- The free-text rows of the early board become typed rows of the disposition; their
-- texts stay as events without a time.
DO $$
DECLARE
    zeile record;
    neu bigint;
    bezeichnung text;
    nummer integer;
BEGIN
    IF to_regclass('public.where_people_duty_note_rows') IS NULL THEN
        RETURN;
    END IF;
    FOR zeile IN
        SELECT "id", "label" FROM "where_people_duty_note_rows" ORDER BY "sort_order", "id"
    LOOP
        bezeichnung := COALESCE(NULLIF(btrim(zeile."label"), ''), 'Hinweis');
        nummer := 1;
        WHILE EXISTS (
            SELECT 1 FROM "where_people_locations" l
             WHERE lower(btrim(l."name")) = lower(bezeichnung) AND l."is_text" = true
        ) LOOP
            nummer := nummer + 1;
            bezeichnung := COALESCE(NULLIF(btrim(zeile."label"), ''), 'Hinweis') || ' ' || nummer;
        END LOOP;
        INSERT INTO "where_people_locations" ("name", "is_text", "sort_order")
        VALUES (
            bezeichnung,
            true,
            (SELECT coalesce(max(l."sort_order"), 0) + 1 FROM "where_people_locations" l)
        )
        RETURNING "id" INTO neu;
        IF to_regclass('public.where_people_duty_note_cells') IS NOT NULL THEN
            INSERT INTO "where_people_dispo_entries" ("location_id", "day", "title")
            SELECT neu, c."day", c."note"
              FROM "where_people_duty_note_cells" c
             WHERE c."row_id" = zeile."id";
        END IF;
    END LOOP;
END $$;

DROP TABLE IF EXISTS "where_people_duty_note_flags";
DROP TABLE IF EXISTS "where_people_duty_note_cells";
DROP TABLE IF EXISTS "where_people_duty_note_rows";
