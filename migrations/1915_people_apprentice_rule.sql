-- Apprentices in the employee master and in the duty plan.
--
-- `where_employees.is_apprentice` marks an apprentice; the mask offers it above
-- the `Status` switch and it is off by default.
--
-- `where_people_planning.apprentice_needs_company` carries the German rule that
-- an apprentice may never be alone in a shift. It is on by default and can be
-- switched off outside Germany.

ALTER TABLE "where_employees"
    ADD COLUMN IF NOT EXISTS "is_apprentice" boolean NOT NULL DEFAULT false;

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "apprentice_needs_company" boolean NOT NULL DEFAULT true;

COMMENT ON COLUMN "where_employees"."is_apprentice" IS
    'The employee is an apprentice.';

COMMENT ON COLUMN "where_people_planning"."apprentice_needs_company" IS
    'An apprentice may never be alone in a shift (Germany).';
