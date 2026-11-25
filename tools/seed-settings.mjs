// Fills the settings lists of the admin menu (`img/Admin Menü`) with the values a
// theatre needs, so the address and article masks have a choice instead of an empty
// list. The script only uses the public API and is repeatable: an entry that already
// exists by its first value is left as it is.
//
// Usage: node tools/seed-settings.mjs [--api http://127.0.0.1:8090]
//                                     [--user admin2 --password start [--mandant 3] | --token <token>]

import { sessionHeader } from './session.mjs';

const apiIndex = process.argv.indexOf('--api');
const API = (
  (apiIndex > 0 ? process.argv[apiIndex + 1] : process.env.WHERE_API) ?? 'http://127.0.0.1:8090'
).replace(/\/+$/, '');

const session = await sessionHeader(API);

async function call(path, init = {}) {
  const response = await fetch(`${API}/api/v1${path}`, {
    ...init,
    headers: { 'content-type': 'application/json', ...session, ...(init.headers ?? {}) },
  });
  if (!response.ok) throw new Error(`${path} -> ${response.status} ${await response.text()}`);
  return response.status === 204 ? null : response.json();
}

/** `values` is keyed by the column of the list, everything travels as text. */
const LISTS = {
  // Anredeformen je Sprache: Höflichkeitsform (Type), Geschlecht (1 neutral, 2 weiblich,
  // 3 sächlich, 4 männlich), „per Du“ und der Text selbst.
  anreden: [
    { Type: '1', Geschlecht: '1', Anrede: 'Sehr geehrte Damen und Herren', Position: '1' },
    { Type: '1', Geschlecht: '2', Anrede: 'Sehr geehrte', Position: '2' },
    { Type: '1', Geschlecht: '3', Anrede: 'Sehr geehrtes', Position: '3' },
    { Type: '1', Geschlecht: '4', Anrede: 'Sehr geehrter', Position: '4' },
    { Type: '1', Geschlecht: '1', Anrede: 'Hallo', Position: '5' },
    { Type: '2', Geschlecht: '1', Persoenlich: 'true', Anrede: 'Hi', Position: '6' },
    { Type: '2', Geschlecht: '2', Persoenlich: 'true', Anrede: 'Liebe', Position: '7' },
    { Type: '2', Geschlecht: '4', Persoenlich: 'true', Anrede: 'Lieber', Position: '8' },
    { Type: '2', Geschlecht: '3', Persoenlich: 'true', Anrede: 'Liebes', Position: '9' },
    { Type: '3', Geschlecht: '4', Anrede: 'Herr', Position: '10' },
    { Type: '3', Geschlecht: '2', Anrede: 'Frau', Position: '11' },
    { Type: '3', Geschlecht: '3', Anrede: 'Fräulein', Position: '12' },
  ],

  // Anredeformen der Adressmaske (Kurzliste).
  'anreden-kurz': [{ Anrede: 'Frau' }, { Anrede: 'Herr' }, { Anrede: 'Divers' }, { Anrede: 'Firma' }],

  // Gruppen der Adresssuche; `Type` 1 = Kunde, 2 = Lieferant, 3 = Interessent.
  adressgruppen: [
    { Name: 'Theater', Type: '1' },
    { Name: 'Veranstalter', Type: '1' },
    { Name: 'Kulturzentrum', Type: '1' },
    { Name: 'Ausbildung', Type: '1' },
    { Name: 'Hersteller', Type: '2' },
    { Name: 'Händler', Type: '2' },
    { Name: 'Team', Type: '3' },
  ],

  qualifikationen: [
    { Qualifikation: 'Beleuchtung' },
    { Qualifikation: 'Ton' },
    { Qualifikation: 'Rigging' },
    { Qualifikation: 'Pyrotechnik' },
    { Qualifikation: 'Stapler' },
  ],

  'status-location': [{ Status: 'aktiv' }, { Status: 'in Planung' }, { Status: 'gesperrt' }],

  'kategorie-location': [
    { Kategorie: 'Theater' },
    { Kategorie: 'Halle' },
    { Kategorie: 'Open Air' },
    { Kategorie: 'Studio' },
  ],

  // Die Anschlusstypen aus `img/Admin Menü/Adressen/Stromanschlüsse Location ändern.PNG`.
  stromanschluesse: [
    { Anschlusstyp: 'Schuko 16A 1~ bei 230V [3680W]', Position: '1' },
    { Anschlusstyp: 'CEE 16A 3~ [ca. 11kW]', Position: '2' },
    { Anschlusstyp: 'CEE 32A 3~ [ca. 22kW]', Position: '3' },
    { Anschlusstyp: 'CEE 63A 3~ [ca. 43kW]', Position: '4' },
    { Anschlusstyp: 'CEE 125 3~ [ca. 86kW]', Position: '5' },
  ],

  'merkmale-location': [
    { Attribut: 'Traversenpunkte' },
    { Attribut: 'Zufahrt für 40-Tonner' },
    { Attribut: 'Barrierefrei' },
    { Attribut: 'Lagerraum vorhanden' },
  ],

  hersteller: [
    { Hersteller: 'Bühnenbau Meister & Söhne' },
    { Hersteller: 'Lichttechnik Nord GmbH' },
    { Hersteller: 'Rigging Systems International' },
    { Hersteller: 'Tontechnik Süd OHG' },
    { Hersteller: 'Verbrauchsmaterial Eventbedarf AG' },
  ],

  eigentuemer: [{ Eigentuemer: 'Eigenbestand' }, { Eigentuemer: 'Kunde' }, { Eigentuemer: 'Kooperation' }],

  artikelzustand: [
    { Zustand: 'neu' },
    { Zustand: 'gebraucht' },
    { Zustand: 'in Reparatur' },
    { Zustand: 'defekt' },
    { Zustand: 'ausgemustert' },
  ],

  // Provisionsklassen, die ein Artikel führen kann (`Artikel_Proviklasse`).
  provisionsklassen: [
    { Index: 'Standard 10 %', Parent_ID: '0', Child_ID: '0' },
    { Index: 'Eigenregie 0 %', Parent_ID: '0', Child_ID: '0' },
    { Index: 'Vermittlung 15 %', Parent_ID: '0', Child_ID: '0' },
    { Index: 'Sondervereinbarung', Parent_ID: '0', Child_ID: '0' },
  ],

  // Suchbegriffe der Artikelrecherche; `Pfad` hält die Position im Baum.
  'artikel-suchindex': [
    { Index: 'Licht', Pfad: 'Licht', Parent_ID: '0', Child_ID: '0' },
    { Index: 'Scheinwerfer', Pfad: 'Licht/Scheinwerfer', Parent_ID: '0', Child_ID: '0' },
    { Index: 'Moving Light', Pfad: 'Licht/Moving Light', Parent_ID: '0', Child_ID: '0' },
    { Index: 'Ton', Pfad: 'Ton', Parent_ID: '0', Child_ID: '0' },
    { Index: 'Mikrofon', Pfad: 'Ton/Mikrofon', Parent_ID: '0', Child_ID: '0' },
    { Index: 'Bühne', Pfad: 'Bühne', Parent_ID: '0', Child_ID: '0' },
    { Index: 'Podest', Pfad: 'Bühne/Podest', Parent_ID: '0', Child_ID: '0' },
  ],

  preisklassen: [
    { Name: 'Standard', CalcType: '1' },
    { Name: 'Stammkunde', CalcType: '1', MargeProzent: '-10' },
    { Name: 'Sozialpreis', CalcType: '1', MargeProzent: '-25' },
    { Name: 'Verkauf Liste', Verkaufsartikel: 'true' },
  ],

  sprachen: [
    { Sprache: 'Deutsch', ISO_639_1: 'de', ISO_639_2: 'deu', Muttersprache: 'true' },
    { Sprache: 'Englisch', ISO_639_1: 'en', ISO_639_2: 'eng' },
    { Sprache: 'Französisch', ISO_639_1: 'fr', ISO_639_2: 'fra' },
  ],

  waehrungen: [
    { Waehrung_Kurzform: 'EUR', Waehrung_Langform: 'Euro', Wechselkurs_o: '1', Waehrungs_ID: '1' },
    { Waehrung_Kurzform: 'CHF', Waehrung_Langform: 'Schweizer Franken', Wechselkurs_o: '1,04', Waehrungs_ID: '2' },
  ],

  versandarten: [
    { Versandart: 'Selbstabholung' },
    { Versandart: 'Spedition' },
    { Versandart: 'Paketversand' },
    { Versandart: 'Kurier' },
  ],

  // Die Listen des Statusfensters, in der Reihenfolge seiner Reiter.
  status: [
    { Status: 'Aufgabe auf "In Bearbeitung" setzen', Type: '1', Sort_ID: '1' },
    { Status: 'warte auf Kundendaten', Type: '1', Sort_ID: '2' },
    { Status: 'wird Auftrag', Type: '1', Sort_ID: '3' },
    { Status: 'eventuell Storno', Type: '1', Sort_ID: '4' },
    { Status: 'offen', Type: '2', Sort_ID: '1' },
    { Status: 'bezahlt', Type: '2', Sort_ID: '2' },
    { Status: 'gemahnt', Type: '2', Sort_ID: '3' },
    { Status: 'offen', Type: '3', Sort_ID: '1' },
    { Status: 'gepackt', Type: '3', Sort_ID: '2' },
    { Status: 'versendet', Type: '3', Sort_ID: '3' },
    { Status: 'angefragt', Type: '4', Sort_ID: '1' },
    { Status: 'bestellt', Type: '4', Sort_ID: '2' },
    { Status: 'geliefert', Type: '4', Sort_ID: '3' },
    { Status: 'offen', Type: '5', Sort_ID: '1' },
    { Status: 'geprüft', Type: '5', Sort_ID: '2' },
    { Status: 'gebucht', Type: '5', Sort_ID: '3' },
  ],

  banken: [
    { Name: 'Musterbank Musterstadt', BIC: 'MUSTDEFFXXX', BLZ: '50000001', Ort: 'Musterstadt', Land_ISO3: 'DEU' },
    { Name: 'Nordhafen Sparkasse', BIC: 'NOHADEHHXXX', BLZ: '20000002', Ort: 'Hamburg', Land_ISO3: 'DEU' },
  ],

  'fibu-konten': [
    { KontoNr: '4100', Bezeichnung: 'Mieterträge', Konto_Typ: '4', Aktiv: 'true' },
    { KontoNr: '8100', Bezeichnung: 'Erlöse Verkauf', Konto_Typ: '4', Aktiv: 'true' },
    { KontoNr: '1400', Bezeichnung: 'Forderungen aus Lieferungen', Konto_Typ: '1', Aktiv: 'true' },
    { KontoNr: '1576', Bezeichnung: 'Verbindlichkeiten aus Lieferungen', Konto_Typ: '3', Aktiv: 'true' },
    { KontoNr: '4930', Bezeichnung: 'Bürobedarf', Konto_Typ: '2', Aktiv: 'true' },
  ],

  zahlungskonditionen: [
    { Text_ANMV: 'sofort netto', Zahlungsziel: '0', SystemDefault: 'true' },
    { Text_ANMV: '14 Tage netto', Zahlungsziel: '14' },
    { Text_ANMV: '30 Tage netto, 2 % Skonto in 10 Tagen', Zahlungsziel: '30', Skonto1_Tage: '10', Skonto1_Rabatt: '2' },
  ],

  projekte: [
    { Name: 'Spielzeit 2026/27 Theater Nordstadt', Name_kurz: 'NT 26/27', Von_Date: '2026-09-01', Bis_Date: '2027-07-31' },
    { Name: 'Open Air Seeburg 2027', Name_kurz: 'OA 27', Von_Date: '2027-05-01', Bis_Date: '2027-09-30' },
  ],
};

const catalogue = await call('/settings');

// Die erste Spalte ist der Name eines Eintrags — darüber wird verglichen.
const nameOf = (area) => area.name_column ?? area.fields[0].column;

let created = 0;
let skipped = 0;
for (const area of catalogue) {
  const vorlagen = LISTS[area.key] ?? [];
  if (vorlagen.length === 0) continue;
  const vorhanden = await call(`/settings/${area.key}`);
  const column = nameOf(area);
  const bekannt = new Set(vorhanden.rows.map((row) => (row.values[column] ?? '').trim().toLowerCase()));
  for (const values of vorlagen) {
    const name = (values[column] ?? '').trim().toLowerCase();
    // Der Status hat denselben Text in mehreren Listen — dort zählt Text und Liste.
    const schluessel = area.key === 'status' ? `${name}|${values.Type ?? ''}` : name;
    const schonDa =
      area.key === 'status'
        ? vorhanden.rows.some(
            (row) =>
              (row.values[column] ?? '').trim().toLowerCase() === name &&
              (row.values.Type ?? '') === (values.Type ?? ''),
          )
        : bekannt.has(name);
    if (schonDa) {
      skipped++;
      continue;
    }
    await call(`/settings/${area.key}`, { method: 'POST', body: JSON.stringify({ values }) });
    bekannt.add(schluessel);
    created++;
  }
}

console.log(`Einstellungen: ${created} Einträge angelegt, ${skipped} bereits vorhanden.`);

// Lagerplätze hängen an einem Lager — sie entstehen erst, wenn ein Lager existiert.
const lager = await call('/settings/lager');
const plaetze = await call('/settings/lagerplaetze');
if (lager.rows.length > 0 && plaetze.rows.length === 0) {
  const lagerId = lager.rows[0].id;
  const vorlagen = [
    { Name: 'Regal A', Prefix: 'A', Position: '1' },
    { Name: 'Regal B', Prefix: 'B', Position: '2' },
    { Name: 'Bühnenfläche', Prefix: 'BÜ', Position: '3' },
    { Name: 'Foyer', Prefix: 'FO', Position: '4' },
  ];
  for (const vorlage of vorlagen) {
    await call('/settings/lagerplaetze', {
      method: 'POST',
      body: JSON.stringify({ values: { ...vorlage, Lager_Primkey: String(lagerId) } }),
    });
  }
  console.log(`Lagerplätze: ${vorlagen.length} Plätze für Lager ${lager.rows[0].values.Name} angelegt.`);
}

// Der Mandant der Demo: die Firma, die in den Masken überall steht.
const mandanten = await call('/tenants');
if (mandanten.length === 0) {
  await call('/tenants', {
    method: 'POST',
    body: JSON.stringify({
      name: 'Theater Ulm',
      short_name: 'TH',
      tenant_id: 0,
      currency: 'EUR',
      print_language: 'Deutsch [deu]',
      street: 'Herbert-von-Karajan-Platz 1',
      postal_code: '89073',
      city: 'Ulm',
      country_iso3: 'DEU',
    }),
  });
  console.log('Mandant: Theater Ulm angelegt.');
}

// Der Administrator der Demo: das Konto, das in der Benutzerverwaltung steht. Es
// kommt mit dem Startkennwort der Erstinstallation (`123456789`) und muss beim
// ersten Anmelden geändert werden — dieselbe Regel wie im Serverstart.
const benutzer = await call('/users?view=local-active');
if (benutzer.length === 0) {
  await call('/users', {
    method: 'POST',
    body: JSON.stringify({
      user_name: 'admin',
      display_name: 'Administrator',
      short_name: 'Adm',
      local_login: true,
      mobile_login: false,
      password: '123456789',
      change_next_login: true,
      never_expires: true,
      color: '#000000',
      mandants: [1],
    }),
  });
  console.log('Benutzer: Administrator [Adm] angelegt (Startkennwort 123456789, Wechsel beim ersten Login).');
}

// Berechtigungsgruppe der Administration: alle Rechte des Katalogs, damit die
// Einrichtung nach der Rechteprüfung der API nicht gesperrt ist. Ohne Gruppe hat ein
// Konto **keine** Rechte — auch das Anlegen einer Adresse oder eines Artikels hängt
// daran. Weitere Konten bekommen bewusst keine Gruppe: so lässt sich prüfen, dass
// Menüs und Masken ohne Recht gar nicht erst aufgehen.
const baum = await call('/permissions');
const alleRechte = baum
  .flatMap((zweig) => zweig.areas)
  .flatMap((area) => area.rights.map((recht) => recht.key));

async function gruppe(name, rechte) {
  const vorhanden = (await call('/user-groups')).find((eintrag) => eintrag.name === name);
  if (vorhanden) {
    const fehlend = rechte.filter((recht) => !vorhanden.max_rights.includes(recht));
    if (fehlend.length === 0) return vorhanden;
    return call(`/user-groups/${vorhanden.id}`, {
      method: 'PUT',
      body: JSON.stringify({
        name,
        max_rights: [...vorhanden.max_rights, ...fehlend],
        min_rights: vorhanden.min_rights,
        mandants: vorhanden.mandants,
        lager: vorhanden.lager,
      }),
    });
  }
  const angelegt = await call('/user-groups', {
    method: 'POST',
    body: JSON.stringify({
      name,
      max_rights: rechte,
      min_rights: [],
      mandants: [],
      lager: [],
    }),
  });
  console.log(`Berechtigungsgruppe „${name}“ mit ${rechte.length} Rechten angelegt.`);
  return angelegt;
}

const verwaltung = await gruppe('Administratoren', alleRechte);

// Nur der Administrator der Demo kommt in die Verwaltung; jedes andere Konto bleibt
// ohne Gruppe (und damit ohne Recht), bis es in der Benutzerverwaltung zugeordnet wird.
for (const konto of await call('/users?view=local-active')) {
  const verwaltet = (konto.name ?? '').startsWith('Administrator');
  const gruppen = (konto.groups ?? []).map((eintrag) => eintrag.id);
  if (verwaltet && !gruppen.includes(verwaltung.id)) {
    await call(`/users/${konto.id}/groups/${verwaltung.id}`, { method: 'PUT' });
    console.log(`Benutzer: „${konto.name}“ in „${verwaltung.name}“ aufgenommen.`);
  }
}
