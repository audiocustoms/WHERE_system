-- An employee can be the person behind an account of the user administration: the user
-- mask picks the employee, then the row names the account (`User_Prefs."Primary_Key"`,
-- an `integer`). The reference stays loose — the accounts live in the installation
-- database, the employees in the database of the tenant — so it is no foreign key on
-- purpose. NULL = the employee has no account.
ALTER TABLE "where_employees"
    ADD COLUMN IF NOT EXISTS "user_id" integer;

-- One account belongs to one employee.
CREATE UNIQUE INDEX IF NOT EXISTS "where_employees_user_idx"
    ON "where_employees" ("user_id")
    WHERE "user_id" IS NOT NULL;

COMMENT ON COLUMN "where_employees"."user_id" IS
    'Account of the user administration that belongs to this employee (loose reference)';
