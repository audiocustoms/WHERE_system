/**
 * The rights the masks of the application ask for.
 *
 * The keys are those of the rights tree in the user administration
 * (`crates/where-domain/src/permissions.rs`) — the same ones the API checks per
 * route in `crates/where-api/src/authz.rs`. Only those a mask has to hide or
 * lock something for are listed here.
 */
export const RIGHT = {
	/** `administrative Einstellungen` → `Zugriff erlauben`: the `Admin` menu. */
	admin: 'administrative-einstellungen.zugriff',

	mayReadAddresses: 'adressen.adresse-lesen',
	mayCreateAddress: 'adressen.adresse-anlegen',
	mayEditAddress: 'adressen.adresse-editieren',
	mayDeleteAddress: 'adressen.adresse-loeschen',

	mayReadArticles: 'artikel.artikel-lesen',
	mayCreateArticle: 'artikel.artikel-anlegen',
	mayEditArticle: 'artikel.artikel-editieren',
	mayDeleteArticle: 'artikel.artikel-loeschen',

	mayReadAppointments: 'teamagenda.termine-lesen',
	mayCreateAppointments: 'teamagenda.termine-anlegen',
	mayEditAppointments: 'teamagenda.termine-editieren',

	mayReadPersonal: 'personal.personal-lesen',
	mayCreatePersonal: 'personal.personal-anlegen',
	mayEditPersonal: 'personal.personal-editieren',
	mayDeletePersonal: 'personal.personal-loeschen',
	maySeePersonal: 'personal.persoenliche-daten-sehen',

	mayReadDispo: 'personaldispo.personaldispo-lesen',
	mayCreateDispo: 'personaldispo.personaldispo-anlegen',
	mayEditDispo: 'personaldispo.personaldispo-editieren'
} as const;

export type Right = (typeof RIGHT)[keyof typeof RIGHT];
