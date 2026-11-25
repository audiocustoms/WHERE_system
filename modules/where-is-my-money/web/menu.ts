import type { ModuleMenuContribution } from '$core/menuTypes';

/**
 * Menu ribbon of `WHERE - is my money?` — accounting and bookkeeping.
 */
const contribution: ModuleMenuContribution = {
	ownMenus: [
		{
			label: 'Faktura',
			items: [
				{ kind: 'item', label: 'Ausgangsrechnung anlegen' },
				{ kind: 'item', label: 'Rechnungsausgangsbuch' },
				{ kind: 'item', label: 'Zahlungseingangsliste' },
				{ kind: 'separator' },
				{ kind: 'item', label: 'Mahn- Opoliste' },
				{ kind: 'separator' },
				{ kind: 'item', label: 'Storno-Rechnungskorrektur anlegen' },
				{ kind: 'separator' },
				{ kind: 'item', label: 'Eingangsrechnung erfassen' },
				{ kind: 'item', label: 'Rechnungseingangsbuch' },
				{ kind: 'item', label: 'Zahlungsausgangsliste' },
				{ kind: 'item', label: 'E-Rechnung Vorschau' },
				{ kind: 'separator' },
				{ kind: 'item', label: 'Kasseneintrag anlegen' },
				{ kind: 'item', label: 'Kassenbuch' },
				{ kind: 'item', label: 'Kassenabschluss durchführen' },
				{ kind: 'separator' },
				{ kind: 'item', label: 'Import Bank-Buchungsliste' }
			]
		}
	],
	append: [
		{
			menu: 'Admin',
			after: 'Artikel',
			items: [
				{
					kind: 'item',
					label: 'Bank - SWIFT/BIC Liste',
					href: '/admin/banks',
					module: 'money'
				},
				{
					kind: 'item',
					label: 'Bankverbindungen',
					href: '/admin/bank-accounts',
					module: 'money'
				},
				{ kind: 'item', label: 'DATEV', module: 'money' },
				{ kind: 'item', label: 'Electronic Banking', module: 'money' },
				{ kind: 'item', label: 'Faktura', module: 'money' },
				{
					kind: 'item',
					label: 'FiBu-Konten/Kostenstellen',
					href: '/admin/settings#fibu-konten',
					module: 'money'
				},
				{ kind: 'item', label: 'Kassenbücher', module: 'money' },
				{ kind: 'item', label: 'Mahnwesen', module: 'money' },
				{
					kind: 'item',
					label: 'Zahlungskonditionen',
					href: '/admin/settings#zahlungskonditionen',
					module: 'money'
				}
			]
		}
	],
	tools: [
		{ label: 'ARE-Buch', icon: 'book', tone: 'blue', module: 'money' },
		{ label: 'ARE +', icon: 'book-add', tone: 'blue', module: 'money' }
	]
};

export default contribution;
