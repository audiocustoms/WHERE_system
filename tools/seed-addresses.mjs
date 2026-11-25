// Fills a running WHERE instance with addresses of theatre and event technology.
//
// Usage: node seed-addresses.mjs [--api http://127.0.0.1:8090]
//                                [--user admin2 --password start [--mandant 3] | --token <token>]
//
// The script only uses the public API — no direct database access. It is
// repeatable: an address that already exists by name is updated to the data
// below, so an older seed run is brought to the current shape (postal code,
// active flag) instead of being skipped.
//
// The records are shaped so the address search form (img/suche/Adresssuche.PNG)
// has something to find: several address groups, customers and suppliers,
// phone numbers, e-mail addresses and debtor numbers.

import { sessionHeader } from './session.mjs';

const args = { api: process.env.WHERE_API ?? 'http://127.0.0.1:8090' };
for (let i = 2; i < process.argv.length; i++) {
	if (process.argv[i] === '--api') args.api = process.argv[++i].replace(/\/+$/, '');
}
args.api = args.api.replace(/\/+$/, '');

const session = await sessionHeader(args.api);

async function api(path, init) {
	const response = await fetch(`${args.api}${path}`, {
		...init,
		headers: { 'content-type': 'application/json', ...session, ...(init?.headers ?? {}) }
	});
	if (!response.ok) {
		const problem = await response.text();
		throw new Error(`${init?.method ?? 'GET'} ${path} → ${response.status}: ${problem}`);
	}
	return response.status === 204 ? null : response.json();
}

// ---------------------------------------------------------------------------
// Source data
// ---------------------------------------------------------------------------

/** `group` is the free-text "Adressgruppe" of the search form. */
const ADDRESSES = [
	// ---- Kunden: Häuser und Veranstalter
	{
		customer_no: 1001,
		title: 'Stadttheater',
		name1: 'Stadttheater Musterstadt',
		name2: 'Technische Abteilung',
		salutation: 'Frau',
		street: 'Theaterplatz 1',
		postal_code: '12345',
		city: 'Musterstadt',
		country: 'Deutschland',
		phone: '0123 456-0',
		email: 'technik@stadttheater-musterstadt.example',
		debtor: '10001',
		group: 'Theater',
		is_customer: true,
		is_supplier: false
	},
	{
		customer_no: 1002,
		title: 'Opernhaus',
		name1: 'Opernhaus Musterstadt',
		name2: 'Bühnenbetrieb',
		street: 'Opernring 12',
		postal_code: '12345',
		city: 'Musterstadt',
		country: 'Deutschland',
		phone: '0123 456-200',
		email: 'buehne@opernhaus-musterstadt.example',
		debtor: '10002',
		group: 'Theater',
		is_customer: true,
		is_supplier: false
	},
	{
		customer_no: 1003,
		title: 'Schauspielhaus',
		name1: 'Schauspielhaus Nordstadt',
		street: 'Kammerspielweg 7',
		postal_code: '54321',
		city: 'Nordstadt',
		country: 'Deutschland',
		phone: '0543 210-88',
		email: 'verwaltung@schauspielhaus-nordstadt.example',
		debtor: '10003',
		group: 'Theater',
		is_customer: true,
		is_supplier: false
	},
	{
		customer_no: 1004,
		title: 'Kulturzentrum',
		name1: 'Kulturzentrum Alte Fabrik',
		name2: 'Veranstaltungstechnik',
		street: 'Industriestraße 45',
		postal_code: '98765',
		city: 'Südhafen',
		country: 'Deutschland',
		phone: '0987 654-12',
		email: 'technik@alte-fabrik-suedhafen.example',
		debtor: '10004',
		group: 'Kulturzentrum',
		is_customer: true,
		is_supplier: false
	},
	{
		customer_no: 1005,
		title: 'Festivalagentur',
		name1: 'Open Air Festival GmbH',
		street: 'Am Festplatz 3',
		postal_code: '23456',
		city: 'Seeburg',
		country: 'Deutschland',
		phone: '0234 567-99',
		email: 'produktion@openair-seeburg.example',
		debtor: '10005',
		group: 'Veranstalter',
		is_customer: true,
		is_supplier: false
	},
	{
		customer_no: 1006,
		title: 'Tourneebetrieb',
		name1: 'Tourneetheater Wanderbühne',
		street: 'Gasthausgasse 9',
		postal_code: '34567',
		city: 'Bergstadt',
		country: 'Deutschland',
		phone: '0345 678-41',
		email: 'leitung@wanderbuehne.example',
		debtor: '10006',
		group: 'Theater',
		is_customer: true,
		is_supplier: false
	},
	{
		customer_no: 1007,
		title: 'Musicalhaus',
		name1: 'Musicalhaus HafenCity',
		name2: 'Bühnentechnik',
		street: 'Hafenpromenade 88',
		postal_code: '20095',
		city: 'HafenCity',
		country: 'Deutschland',
		phone: '040 123-456',
		email: 'buehnentechnik@musicalhaus-hafencity.example',
		debtor: '10007',
		group: 'Theater',
		is_customer: true,
		is_supplier: false
	},
	{
		customer_no: 1008,
		title: 'Hochschule',
		name1: 'Hochschule für Darstellende Kunst',
		street: 'Campusallee 2',
		postal_code: '76543',
		city: 'Kunstberg',
		country: 'Deutschland',
		phone: '0765 432-10',
		email: 'requisite@hfk-kunstberg.example',
		debtor: '10008',
		group: 'Ausbildung',
		is_customer: true,
		is_supplier: false
	},

	// ---- Lieferanten: Hersteller und Händler
	{
		customer_no: 2001,
		supplier_no: 2001,
		creditor: '70001',
		name1: 'Lichttechnik Nord GmbH',
		name2: 'Vertrieb',
		street: 'Gewerbering 14',
		postal_code: '21079',
		city: 'Nordhafen',
		country: 'Deutschland',
		phone: '040 987-654',
		email: 'vertrieb@lichttechnik-nord.example',
		debtor: '20001',
		group: 'Hersteller',
		is_customer: false,
		is_supplier: true
	},
	{
		customer_no: 2002,
		supplier_no: 2002,
		creditor: '70002',
		name1: 'Bühnenbau Meister & Söhne',
		street: 'Zimmererstraße 6',
		postal_code: '65432',
		city: 'Holzheim',
		country: 'Deutschland',
		phone: '0654 321-77',
		email: 'info@buehnenbau-meister.example',
		debtor: '20002',
		group: 'Hersteller',
		is_customer: false,
		is_supplier: true
	},
	{
		customer_no: 2003,
		supplier_no: 2003,
		creditor: '70003',
		name1: 'Tontechnik Süd OHG',
		name2: 'Beschallungstechnik',
		street: 'Schallweg 21',
		postal_code: '80331',
		city: 'München',
		country: 'Deutschland',
		phone: '089 112-233',
		email: 'bestellung@tontechnik-sued.example',
		debtor: '20003',
		group: 'Hersteller',
		is_customer: false,
		is_supplier: true
	},
	{
		customer_no: 2004,
		supplier_no: 2004,
		creditor: '70004',
		name1: 'Verbrauchsmaterial Eventbedarf AG',
		street: 'Lagerstraße 100',
		postal_code: '44135',
		city: 'Dortmund',
		country: 'Deutschland',
		phone: '0231 445-566',
		email: 'verkauf@eventbedarf.example',
		debtor: '20004',
		group: 'Händler',
		is_customer: false,
		is_supplier: true
	},
	{
		customer_no: 2005,
		supplier_no: 2005,
		creditor: '70005',
		name1: 'Rigging Systems International',
		name2: 'Niederlassung Deutschland',
		street: 'Kranweg 5',
		postal_code: '60311',
		city: 'Frankfurt',
		country: 'Deutschland',
		phone: '069 778-899',
		email: 'sales@rigging-systems.example',
		debtor: '20005',
		group: 'Hersteller',
		is_customer: false,
		is_supplier: true
	},

	// ---- Personen: Geburtstage für den Kalender
	{
		customer_no: 9001,
		name1: 'Lena Hartmann',
		salutation: 'Frau',
		street: 'Gartenweg 4',
		postal_code: '12347',
		city: 'Musterstadt',
		country: 'Deutschland',
		phone: '0170 1112233',
		email: 'lena.hartmann@example.org',
		group: 'Team',
		is_customer: false,
		is_supplier: false,
		is_interested: true,
		is_employee: true,
		is_person: true,
		birthday: '1988-11-21'
	},
	{
		customer_no: 9002,
		name1: 'Jonas Weber',
		salutation: 'Herr',
		street: 'Am Hafen 9',
		postal_code: '21079',
		city: 'Hamburg',
		country: 'Deutschland',
		phone: '0170 4455667',
		email: 'jonas.weber@example.org',
		group: 'Team',
		is_customer: false,
		is_supplier: false,
		is_interested: true,
		is_employee: true,
		is_person: true,
		// Ein Schalttag: im Kalender fällt er in normalen Jahren auf den 28.02.
		birthday: '1980-02-29'
	}
];

// ---------------------------------------------------------------------------
// Seed
// ---------------------------------------------------------------------------

const existing = await api('/api/v1/addresses?limit=500&active=all');
const known = new Map(
	existing.items.map((a) => [(a.name1 ?? '').trim().toLowerCase(), a.id])
);

let created = 0;
let updated = 0;

for (const address of ADDRESSES) {
	// `active` fehlte in älteren Läufen; die Spalte hat in der Altschema-Definition
	// den Vorgabewert `false`, deshalb wird sie hier ausdrücklich gesetzt.
	const body = {
		title: null,
		name3: null,
		salutation: null,
		vat_id: null,
		auftragssperre: false,
		mahnsperre: false,
		active: true,
		...address
	};
	const id = known.get(address.name1.trim().toLowerCase());
	if (id) {
		const stored = await api(`/api/v1/addresses/${id}`, {
			method: 'PUT',
			body: JSON.stringify(body)
		});
		updated++;
		console.log(`  updated: ${stored.customer_no ?? '—'}  ${stored.name1}  (${stored.group ?? '—'})`);
		continue;
	}
	const stored = await api('/api/v1/addresses', { method: 'POST', body: JSON.stringify(body) });
	created++;
	console.log(`  created: ${stored.customer_no ?? '—'}  ${stored.name1}  (${stored.group ?? '—'})`);
}

console.log(`\ndone: ${created} addresses created, ${updated} updated.`);
