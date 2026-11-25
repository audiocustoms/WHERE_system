-- Number series: central allocation of article, customer, order and document numbers.
--
-- The legacy application spreads this task over twelve tables:
--   * "Nummernkreislauf"  — one row with 17 counters (KundenID, RechnungID, ...)
--   * "ArtikelNr", "KundenNr", "LieferantenNr", "DebitorenNr", "KreditorenNr", "SN_IdentNr"
--   * "AuftragsNr", "ARechnungsNr", "ERechnungsNr", "GSSTNr", "Best_VorgangsNr"
--     — each with "Format" + currYear/currMonth/currDay for date parts
--
-- That is unusable for maintenance: 17 columns in one row, a legacy format string
-- whose syntax we cannot reconstruct without source code, and no audit.
-- This migration therefore creates *one* table with one row per series.
-- The origin of every row is recorded in "legacy_source" so the import stays
-- unambiguous; the legacy tables are not modified.

CREATE TABLE IF NOT EXISTS "where_number_series" (
    "key"      text PRIMARY KEY,
    "name"     text NOT NULL,
    "prefix"         varchar(16) NOT NULL DEFAULT '',
    -- Placeholders: {number} {year} {month} {day}; {number} is padded to "digits".
    "format"          varchar(64) NOT NULL DEFAULT '{number}',
    "digits"         smallint NOT NULL DEFAULT 0,
    "start_value"       bigint NOT NULL DEFAULT 1,
    "step"    bigint NOT NULL DEFAULT 1,
    -- Last number issued in the current period; 0 = none yet.
    "current_value"  bigint NOT NULL DEFAULT 0,
    "yearly_reset" boolean NOT NULL DEFAULT false,
    "year"            integer,
    "active"           boolean NOT NULL DEFAULT true,
    "legacy_source"  text,
    "changed_at"    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT "where_number_series_digits_ck" CHECK ("digits" BETWEEN 0 AND 12),
    CONSTRAINT "where_number_series_step_ck" CHECK ("step" > 0),
    CONSTRAINT "where_number_series_start_ck" CHECK ("start_value" >= 0),
    CONSTRAINT "where_number_series_value_ck" CHECK ("current_value" >= 0)
);

-- ---------------------------------------------------------------- Initial rows
-- Take over the counters of the source data so an existing installation does not
-- issue the same numbers again. Without a source the series starts at 1.
INSERT INTO "where_number_series" ("key", "name", "legacy_source")
VALUES
    ('article',       'Artikelnummer',              '"ArtikelNr"."ArtikelNr"'),
    ('customer',      'Kundennummer',               '"KundenNr"."KundenNr"'),
    ('supplier',      'Lieferantennummer',          '"LieferantenNr"."LieferantenNr"'),
    ('debtor',        'Debitorennummer',            '"DebitorenNr"."DebitorenNr"'),
    ('creditor',      'Kreditorennummer',           '"KreditorenNr"."KreditorenNr"'),
    ('order',         'Auftragsnummer',             '"AuftragsNr"."AuftragsNr"'),
    ('sales_order',   'Verkaufsauftragsnummer',     '"Nummernkreislauf"."VAuftragID"'),
    ('purchase_order','Bestellnummer',              '"Nummernkreislauf"."BestellungID"'),
    ('invoice',       'Rechnungsnummer',            '"ARechnungsNr"."RechnungsNr"'),
    ('e_invoice',     'E-Rechnungsnummer',          '"ERechnungsNr"."RechnungsNr"'),
    ('credit_note',   'Gutschriftnummer',           '"GSSTNr"."RechnungsNr"'),
    ('journal',       'Journalnummer',              '"Nummernkreislauf"."JournalID"'),
    ('journal_technical', 'Journal-Technik-Nummer', '"Nummernkreislauf"."JournalTechID"'),
    ('rental_contract', 'Mietvertragsnummer',       '"Nummernkreislauf"."MietVertragID"'),
    ('device',        'Gerätenummer',              '"Nummernkreislauf"."GeräteID"'),
    ('serial_number', 'Seriennummer',               '"SN_IdentNr"."Counter"'),
    ('article_bundle', 'Artikel-Bundle-Nummer',     '"Nummernkreislauf"."ArtikelBundleID"'),
    ('qm_ticket',     'QM-Ticket-Nummer',           '"Nummernkreislauf"."QMTicketID"'),
    ('schedule',      'Schedule-Nummer',            '"Nummernkreislauf"."ScheduleID"'),
    ('schedule_technical', 'Schedule-Technik-Nummer', '"Nummernkreislauf"."ScheduleTechID"'),
    ('tenant',        'Mandantennummer',            '"Nummernkreislauf"."Mandanten_ID"')
ON CONFLICT ("key") DO NOTHING;

-- Article numbers used to start at 1001 (COALESCE(MAX(...), 1000) + 1). The series
-- adopts the highest issued value so the next number follows seamlessly.
UPDATE "where_number_series"
   SET "current_value" = COALESCE((SELECT MAX("ArtikelNr") FROM "Artikel"), 1000)
 WHERE "key" = 'article'
   AND "current_value" = 0;
