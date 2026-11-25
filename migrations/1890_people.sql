-- Employees of `WHERE - are my people?`.
--
-- Kept in the shared database like every other table, so the payroll data joins
-- the address and order data without a second connection. The rows are only
-- reachable while the `people` module is registered (the API guard and the UI
-- both check the module registry).

CREATE TABLE IF NOT EXISTS "where_employees" (
    "id"            bigserial PRIMARY KEY,
    "personnel_no"  text,
    "first_name"    text NOT NULL,
    "last_name"     text NOT NULL,
    "email"         text,
    "phone"         text,
    "position"      text,
    "department"    text,
    "active"        boolean NOT NULL DEFAULT true,
    "created_at"    timestamptz NOT NULL DEFAULT now(),
    "changed_at"    timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS "where_employees_personnel_no_idx"
    ON "where_employees" ("personnel_no")
    WHERE "personnel_no" IS NOT NULL;

COMMENT ON TABLE "where_employees" IS
    'Employee master data of WHERE - are my people?';
