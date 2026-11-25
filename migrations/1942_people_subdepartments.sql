-- Sub-departments: the second level under a department.
--
-- `Beleuchtung` is a department; `Podium` and `Großes Haus` are the houses it works
-- in — its sub-departments. The employee mask offers them under the department, and
-- a location names the sub-department that works there. Like the department, the
-- parent is a name, so a rename of either side is carried along by the API.

CREATE TABLE IF NOT EXISTS "where_people_subdepartments" (
    "id"         bigserial PRIMARY KEY,
    "name"       text NOT NULL,
    "department" text,
    "sort_order" integer NOT NULL DEFAULT 0,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "changed_at" timestamptz NOT NULL DEFAULT now()
);

-- Each name exactly once, regardless of upper and lower case.
CREATE UNIQUE INDEX IF NOT EXISTS "where_people_subdepartments_name_idx"
    ON "where_people_subdepartments" (lower("name"));

CREATE INDEX IF NOT EXISTS "where_people_subdepartments_department_idx"
    ON "where_people_subdepartments" (lower("department"));

COMMENT ON TABLE "where_people_subdepartments" IS
    'Sub-departments under a department of the employee master of WHERE - are my people?';
COMMENT ON COLUMN "where_people_subdepartments"."department" IS
    'Name of the department it belongs to (where_people_departments."name").';

ALTER TABLE "where_employees"
    ADD COLUMN IF NOT EXISTS "subdepartment" text;

COMMENT ON COLUMN "where_employees"."subdepartment" IS
    'Sub-department the employee belongs to (name of where_people_subdepartments).';
