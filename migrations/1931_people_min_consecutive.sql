-- Kleinstes Arbeitsblock, bevor ein freier Tag kommen darf.
--
-- `max_consecutive_days` begrenzt, wie viele Tage in Folge gearbeitet werden
-- dürfen; `min_consecutive_days` ist die Untergrenze: ein Mitarbeiter arbeitet
-- mindestens so viele Tage am Stück, bevor er einen freien Tag bekommt. Geplant
-- wird das als Vorrang — der Bedarf entscheidet, ob es aufgeht.
ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "min_consecutive_days" smallint NOT NULL DEFAULT 1;

ALTER TABLE "where_people_planning"
    DROP CONSTRAINT IF EXISTS "where_people_planning_min_consecutive_ck";

ALTER TABLE "where_people_planning"
    ADD CONSTRAINT "where_people_planning_min_consecutive_ck"
    CHECK ("min_consecutive_days" BETWEEN 1 AND 31);

ALTER TABLE "where_people_planning"
    DROP CONSTRAINT IF EXISTS "where_people_planning_consecutive_order_ck";

ALTER TABLE "where_people_planning"
    ADD CONSTRAINT "where_people_planning_consecutive_order_ck"
    CHECK ("min_consecutive_days" <= "max_consecutive_days");

COMMENT ON COLUMN "where_people_planning"."min_consecutive_days" IS
    'Working days in a row before a day off may follow (1 = no minimum).';
