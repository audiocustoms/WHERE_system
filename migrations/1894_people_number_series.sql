-- Personnel numbers as a number series.
--
-- The personnel number used to be free text. It now runs through
-- `where_number_series` like article and customer numbers and starts at 1001;
-- the mask assigns it automatically on creation when the field stays empty.

INSERT INTO "where_number_series" ("key", "name", "start_value", "format", "digits")
VALUES ('employee', 'Personalnummer', 1001, '{number}', 0)
ON CONFLICT ("key") DO NOTHING;

-- Existing installations: raise the counter to the highest personnel number
-- already issued so no number is offered twice. Only when it lies above the
-- start value — legacy data such as „0001" stays out of consideration.
WITH "hoechste" AS (
    SELECT MAX((NULLIF(regexp_replace("personnel_no", '\D', '', 'g'), ''))::bigint) AS "wert"
      FROM "where_employees"
     WHERE "personnel_no" ~ '\d'
)
UPDATE "where_number_series" AS "s"
   SET "current_value" = "hoechste"."wert",
       "changed_at" = now()
  FROM "hoechste"
 WHERE "s"."key" = 'employee'
   AND "hoechste"."wert" >= "s"."start_value"
   AND "s"."current_value" < "hoechste"."wert";
