-- Personal data of `WHERE - are my people?`.
--
-- `where_employees` carried only the fields of the first draft so far. The
-- input mask now writes person, address, employment and qualifications; the
-- qualifications live in their own table so the list stays sortable and can be
-- evaluated.

ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "salutation" text;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "title" text;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "birth_date" date;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "birth_place" text;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "nationality" text;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "marital_status" text;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "street" text;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "postal_code" text;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "city" text;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "country" text;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "mobile" text;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "entry_date" date;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "exit_date" date;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "employment_type" text;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "weekly_hours" numeric(5,2);
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "cost_center" text;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "manager" text;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "social_security_no" text;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "tax_id" text;
ALTER TABLE "where_employees" ADD COLUMN IF NOT EXISTS "notes" text;

CREATE INDEX IF NOT EXISTS "where_employees_entry_date_idx"
    ON "where_employees" ("entry_date");

CREATE TABLE IF NOT EXISTS "where_employee_qualifications" (
    "id"            bigserial PRIMARY KEY,
    "employee_id"   bigint NOT NULL REFERENCES "where_employees" ("id") ON DELETE CASCADE,
    "qualification" text NOT NULL,
    "position"      integer NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS "where_employee_qualifications_unique_idx"
    ON "where_employee_qualifications" ("employee_id", "qualification");

COMMENT ON TABLE "where_employee_qualifications" IS
    'Qualifications of one employee, in the order of the mask.';
