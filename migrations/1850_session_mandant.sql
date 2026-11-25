-- Der Mandant, in dem eine Sitzung angemeldet wurde (`img/Admin Menü/Login Logout`).
--
-- Die Anmeldemaske bietet links die Mandanten an und zeigt nur deren Benutzer; der
-- gewählte Mandant reist mit dem Token, damit die Kopfzeile nach einem Neuladen weiß,
-- welche Installation offen ist. Jeder Mandant hat seine eigene Datenbank — die Spalte
-- hält den Bezug zu `"Mandanten"."Primary_Key"` dieses Datenbestands.

ALTER TABLE "where_sessions" ADD COLUMN IF NOT EXISTS "mandant" integer;

COMMENT ON COLUMN "where_sessions"."mandant" IS
    'Mandant der Anmeldung ("Mandanten"."Primary_Key"); NULL = ohne Mandantenwahl.';

CREATE INDEX IF NOT EXISTS "where_sessions_mandant_idx" ON "where_sessions" ("mandant");
