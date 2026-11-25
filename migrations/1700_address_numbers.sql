-- Address numbers start after the imported data.
--
-- The number series "customer", "supplier", "debtor" and "creditor" are created
-- empty (migration `1100_number_series.sql`), because the legacy installation keeps
-- its counters in separate tables that an import does not fill. Without this
-- alignment the first new address would get number 1 next to the imported 1001…
-- The article series bootstraps in the same way.

UPDATE "where_number_series"
   SET "current_value" = COALESCE((SELECT MAX("KundenNr") FROM "Adresse"), 0),
       "changed_at" = now()
 WHERE "key" = 'customer'
   AND "current_value" = 0;

UPDATE "where_number_series"
   SET "current_value" = COALESCE((SELECT MAX("lief_LieferantenNr") FROM "Adresse"), 0),
       "changed_at" = now()
 WHERE "key" = 'supplier'
   AND "current_value" = 0;

-- Debtor and creditor numbers are text columns; only purely numeric values count.
UPDATE "where_number_series"
   SET "current_value" = COALESCE(
           (SELECT MAX("knd_Debitor"::bigint)
              FROM "Adresse"
             WHERE "knd_Debitor" ~ '^[0-9]{1,18}$'),
           0),
       "changed_at" = now()
 WHERE "key" = 'debtor'
   AND "current_value" = 0;

UPDATE "where_number_series"
   SET "current_value" = COALESCE(
           (SELECT MAX("lief_Kreditor"::bigint)
              FROM "Adresse"
             WHERE "lief_Kreditor" ~ '^[0-9]{1,18}$'),
           0),
       "changed_at" = now()
 WHERE "key" = 'creditor'
   AND "current_value" = 0;
