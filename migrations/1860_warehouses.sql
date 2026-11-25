-- Lager und Lagerplätze (`img/Admin Menü/Lager und Lagerplätze`).
--
-- `"Storage_Lager"` trägt Name, Vorgabe-Kennzeichen und die beiden Lagerortungs-Arten.
-- Die sieben Schalter der Maske fehlten in der importierten Tabelle; sie stehen hier als
-- `where_*`-Spalten, wie schon bei den Einsatzbereichen der Geräte.
--
-- `"Mandanten_ID"` gab es bereits — sie ordnet ein Lager seinem Mandanten zu. Erst mit
-- gewähltem Mandanten lässt die Maske ein Lager zuweisen.

ALTER TABLE "Storage_Lager" ADD COLUMN IF NOT EXISTS "where_zugriff" boolean NOT NULL DEFAULT false;
ALTER TABLE "Storage_Lager" ADD COLUMN IF NOT EXISTS "where_prioritaet" boolean NOT NULL DEFAULT false;
ALTER TABLE "Storage_Lager" ADD COLUMN IF NOT EXISTS "where_such_std" boolean NOT NULL DEFAULT false;
ALTER TABLE "Storage_Lager" ADD COLUMN IF NOT EXISTS "where_entnahme" boolean NOT NULL DEFAULT false;
ALTER TABLE "Storage_Lager" ADD COLUMN IF NOT EXISTS "where_anmelden" boolean NOT NULL DEFAULT false;
ALTER TABLE "Storage_Lager" ADD COLUMN IF NOT EXISTS "where_fremd" boolean NOT NULL DEFAULT false;
ALTER TABLE "Storage_Lager" ADD COLUMN IF NOT EXISTS "where_konzern" boolean NOT NULL DEFAULT false;

COMMENT ON COLUMN "Storage_Lager"."where_zugriff" IS 'Lager ist für die Auswahl freigegeben';
COMMENT ON COLUMN "Storage_Lager"."where_prioritaet" IS 'Lager hat Vorrang bei der Reserve';
COMMENT ON COLUMN "Storage_Lager"."where_such_std" IS 'Standardsuchlager der Verfügbarkeit';
COMMENT ON COLUMN "Storage_Lager"."where_entnahme" IS 'Entnahme aus diesem Lager erlaubt';
COMMENT ON COLUMN "Storage_Lager"."where_anmelden" IS 'Anmeldung am Lager verlangt';
COMMENT ON COLUMN "Storage_Lager"."where_fremd" IS 'Fremdlager (Bestand eines Dritten)';
COMMENT ON COLUMN "Storage_Lager"."where_konzern" IS 'Konzerninternes Lager oder ohne Disposition';

CREATE INDEX IF NOT EXISTS "Storage_Lager_where_flags_idx" ON "Storage_Lager" ("where_zugriff");
