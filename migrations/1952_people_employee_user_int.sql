-- The account of the user administration is an `integer` (`User_Prefs."Primary_Key"`),
-- and `1947_people_employee_user.sql` creates the column that way. Databases that ran an
-- earlier version of that file carry `"user_id"` as `bigint`, and the migrations were
-- already marked as applied — so the column never changed there.
--
-- The masks read the value as `i32` (`EmployeeRow`), and a `bigint` column makes that read
-- fail ("Option<i32> is not compatible with INT8"), which took the whole employee list
-- down. This migration brings every such database onto the type of the model. Running it
-- on an `integer` column is a no-op, so it is safe for every installation.

ALTER TABLE "where_employees"
    ALTER COLUMN "user_id" TYPE integer USING "user_id"::integer;
