-- Master data of the employee mask: departments and positions.
--
-- Both lists belong to `WHERE - are my people?` and live in the shared database
-- like the employee master. They are maintained under
-- `Stammdaten → Personal → Abteilungen und Positionen` and suggested in the mask
-- of the employee.

CREATE TABLE IF NOT EXISTS "where_people_departments" (
    "id"         bigserial PRIMARY KEY,
    "name"       text NOT NULL,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "changed_at" timestamptz NOT NULL DEFAULT now()
);

-- Each name exactly once, regardless of upper and lower case.
CREATE UNIQUE INDEX IF NOT EXISTS "where_people_departments_name_idx"
    ON "where_people_departments" (lower("name"));

CREATE TABLE IF NOT EXISTS "where_people_positions" (
    "id"         bigserial PRIMARY KEY,
    "name"       text NOT NULL,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "changed_at" timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS "where_people_positions_name_idx"
    ON "where_people_positions" (lower("name"));

COMMENT ON TABLE "where_people_departments" IS
    'Departments of the employee master of WHERE - are my people?';
COMMENT ON TABLE "where_people_positions" IS
    'Positions of the employee master of WHERE - are my people?';
