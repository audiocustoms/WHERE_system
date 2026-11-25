-- Indexes for the bank list (`img/Admin Menü/Bank - Swift BIC Liste`).
--
-- The list is a lookup table with some 43 000 rows once it is synchronised with the
-- online source; the window searches it while the user types. The legacy table itself
-- stays as it is — the columns and their meaning do not change.

CREATE INDEX IF NOT EXISTS "where_banken_bic_idx"
    ON "Storage_Banken" ("BIC");

CREATE INDEX IF NOT EXISTS "where_banken_name_idx"
    ON "Storage_Banken" (lower("Name"));

CREATE INDEX IF NOT EXISTS "where_banken_ort_idx"
    ON "Storage_Banken" (lower("Ort"));
