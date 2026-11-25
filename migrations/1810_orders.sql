-- Order settings (`img/Admin Menü/Aufträge`).
--
-- WHERE has no order engine yet, but the "Auftragseinstellungen" window has three
-- lists that a later module will read: the default time plan of a new order, the
-- nine headings of the printed order and the macro list. They get tables of their
-- own; the switches and numbers of the `Regeln` and `Zeiten` tabs live in
-- `"where_settings"` under the prefix `order.` and need no migration.

CREATE TABLE IF NOT EXISTS "where_order_timeline" (
    "key"      text PRIMARY KEY,
    "label"    text NOT NULL,
    "position" integer NOT NULL,
    "shown"    boolean NOT NULL DEFAULT true
);

COMMENT ON TABLE "where_order_timeline" IS
    'Default time plan of a new order; `shown` is the "Anzeigen" column of the window.';

CREATE TABLE IF NOT EXISTS "where_order_headings" (
    "number" integer PRIMARY KEY,
    "shown"  boolean NOT NULL DEFAULT true,
    "label"  text
);

COMMENT ON TABLE "where_order_headings" IS
    'The nine headings of the order form; `label` is the text the user gives them.';

CREATE TABLE IF NOT EXISTS "where_order_macros" (
    "id"         bigserial PRIMARY KEY,
    "label"      text NOT NULL,
    "for_sale"   boolean NOT NULL DEFAULT false,
    "active"     boolean NOT NULL DEFAULT true,
    "created_by" text,
    "body"       text,
    "changed_at" timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE "where_order_macros" IS
    'Macro list of the order mask; `body` holds the steps a macro will run later.';

INSERT INTO "where_order_timeline" ("key", "label", "position", "shown") VALUES
    ('dispo', 'Dispo', 1, true),
    ('vorbereitung', 'Vorbereitung', 2, true),
    ('packen', 'Packen', 3, true),
    ('anliefern', 'Anliefern', 4, true),
    ('aufbau', 'Aufbau', 5, true),
    ('probe', 'Probe', 6, true),
    ('event', 'Event', 7, true),
    ('abbau', 'Abbau', 8, true),
    ('rueckpacken', 'Rückpacken', 9, true),
    ('rueckliefern', 'Rückliefern', 10, true)
ON CONFLICT ("key") DO NOTHING;

INSERT INTO "where_order_headings" ("number", "shown", "label")
SELECT nummer, true, NULL
FROM generate_series(1, 9) AS nummer
ON CONFLICT ("number") DO NOTHING;
