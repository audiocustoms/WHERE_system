import type { ModuleMenuContribution } from '$core/menuTypes';

/**
 * Menu ribbon of `WHERE - does my stuff go?`.
 *
 * The entries only appear while the module is registered in the
 * installation (`modules.isInstalled('go')`).
 */
const contribution: ModuleMenuContribution = {
	ownMenus: [
		{
			label: 'Auftrag',
			items: [
				{ kind: 'item', label: 'Mietangebot anlegen' },
				{ kind: 'item', label: 'Verkaufsangebot anlegen' },
				{ kind: 'separator' },
				{ kind: 'item', label: 'Auftragsliste' },
				{ kind: 'separator' },
				{ kind: 'item', label: 'Artikelverfügbarkeit' },
				{ kind: 'item', label: 'Dispokonfliktliste' }
			]
		}
	],
	append: [
		{
			menu: 'Lager',
			before: 'Werkstatt Logbuch',
			items: [
				{ kind: 'separator' },
				{ kind: 'item', label: 'Packschein anlegen' },
				{ kind: 'item', label: 'Packscheinliste' },
				{ kind: 'separator' },
				{ kind: 'item', label: 'Packschein vorpacken' },
				{ kind: 'item', label: 'Packschein ausbuchen' },
				{ kind: 'item', label: 'Packschein einbuchen' },
				{ kind: 'item', label: 'Ohne Packschein einbuchen' },
				{ kind: 'item', label: 'Artikelbewegungsprotokoll' }
			]
		},
		{
			menu: 'Admin',
			after: 'Artikel',
			items: [
				{
					kind: 'item',
					label: 'Aufträge',
					href: '/admin/orders',
					module: 'go'
				},
				{
					kind: 'item',
					label: 'Preisklassen',
					href: '/admin/settings#preisklassen',
					module: 'go'
				},
				{ kind: 'item', label: 'Staffel - Mengenrabatt', module: 'go' },
				{ kind: 'item', label: 'Staffel - Zeitfaktoren', module: 'go' },
				{
					kind: 'item',
					label: 'Versandarten',
					href: '/admin/settings#versandarten',
					module: 'go'
				}
			]
		}
	],
	tools: [
		{ label: 'Aufträge', icon: 'document', tone: 'orange', module: 'go' },
		{ label: 'Packscheine', icon: 'packing', tone: 'red', module: 'go' },
		{ label: 'Dispokonflikte', icon: 'conflict', tone: 'slate', module: 'go' }
	]
};

export default contribution;
