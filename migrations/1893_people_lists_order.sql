-- Order of the two personnel lists.
--
-- Departments and positions are not kept alphabetically but in the order that is
-- set with the arrows in the mask. The name stays the second key so rows that
-- came in later do not stand at random.

ALTER TABLE "where_people_departments"
    ADD COLUMN IF NOT EXISTS "sort_order" integer NOT NULL DEFAULT 0;
ALTER TABLE "where_people_positions"
    ADD COLUMN IF NOT EXISTS "sort_order" integer NOT NULL DEFAULT 0;

-- Existing installations: number alphabetically once.
WITH "nummeriert" AS (
    SELECT "id", row_number() OVER (ORDER BY lower("name")) - 1 AS "rang"
    FROM "where_people_departments"
)
UPDATE "where_people_departments" AS "d"
SET "sort_order" = "nummeriert"."rang"
FROM "nummeriert"
WHERE "d"."id" = "nummeriert"."id";

WITH "nummeriert" AS (
    SELECT "id", row_number() OVER (ORDER BY lower("name")) - 1 AS "rang"
    FROM "where_people_positions"
)
UPDATE "where_people_positions" AS "p"
SET "sort_order" = "nummeriert"."rang"
FROM "nummeriert"
WHERE "p"."id" = "nummeriert"."id";

CREATE INDEX IF NOT EXISTS "where_people_departments_sort_idx"
    ON "where_people_departments" ("sort_order");
CREATE INDEX IF NOT EXISTS "where_people_positions_sort_idx"
    ON "where_people_positions" ("sort_order");

COMMENT ON COLUMN "where_people_departments"."sort_order" IS
    'Order in the mask; smaller means further up.';
COMMENT ON COLUMN "where_people_positions"."sort_order" IS
    'Order in the mask; smaller means further up.';
