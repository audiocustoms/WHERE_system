-- Article pricing: tabs "Miete" and "Verkauf" of the form (img/miete.PNG, img/verkauf.PNG)
--
-- The prices already live in "Art_MandantDaten" (one row per article and client), the
-- flags in "Artikel", the scales in "StaffelKategorie" and "Staffelrabatt". This
-- migration only adds the lookup index used when reading a price row and the two
-- standard scales the form offers in its "Staffel" dropdown.

CREATE INDEX IF NOT EXISTS "Art_MandantDaten_ArtikelMandant_idx"
    ON "Art_MandantDaten" ("Artikel_Primkey", "Mandanten_ID");

CREATE INDEX IF NOT EXISTS "Staffelrabatt_Kategorie_Miettage_idx"
    ON "Staffelrabatt" ("Kategorie_Primkey", "Miettage");

-- ---------------------------------------------------------------- standard scales
-- "Standard Faktorenstaffel": factor per day of use, as the rental tab shows it.
INSERT INTO "StaffelKategorie" ("Primary_Key", "Name", "Verkauf", "Standard", "Mandanten_ID")
SELECT 1, 'Standard Faktorenstaffel', false, true, 0
WHERE NOT EXISTS (SELECT 1 FROM "StaffelKategorie" WHERE "Primary_Key" = 1);

INSERT INTO "Staffelrabatt" ("Miettage", "Faktor", "Kategorie_Primkey", "Mandanten_ID")
SELECT v.days, v.factor, 1, 0
  FROM (VALUES (1, 1.0), (2, 1.5), (3, 2.0), (4, 2.5), (5, 3.0), (6, 3.5), (7, 4.0))
       AS v(days, factor)
 WHERE NOT EXISTS (
    SELECT 1 FROM "Staffelrabatt" WHERE "Kategorie_Primkey" = 1 AND "Miettage" = v.days
 );

-- "Standard Rabattstaffel": discount per quantity, as the sale tab shows it.
INSERT INTO "StaffelKategorie" ("Primary_Key", "Name", "Verkauf", "Standard", "Mandanten_ID")
SELECT 2, 'Standard Rabattstaffel', true, true, 0
WHERE NOT EXISTS (SELECT 1 FROM "StaffelKategorie" WHERE "Primary_Key" = 2);

INSERT INTO "Staffelrabatt" ("Miettage", "Rabatt", "Kategorie_Primkey", "Mandanten_ID")
SELECT v.qty, v.discount, 2, 0
  FROM (VALUES (10, 5.0), (25, 10.0), (50, 15.0), (100, 20.0)) AS v(qty, discount)
 WHERE NOT EXISTS (
    SELECT 1 FROM "Staffelrabatt" WHERE "Kategorie_Primkey" = 2 AND "Miettage" = v.qty
 );

-- Explicit keys do not consume sequence values.
SELECT setval(
    pg_get_serial_sequence('"StaffelKategorie"', 'Primary_Key'),
    (SELECT GREATEST(COALESCE(MAX("Primary_Key"), 1), 1) FROM "StaffelKategorie")
);
