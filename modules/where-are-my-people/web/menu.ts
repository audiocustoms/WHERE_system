import type { ModuleMenuContribution } from '$core/menuTypes';

/**
 * Menu ribbon of `WHERE - are my people?` — human resources.
 */
const contribution: ModuleMenuContribution = {
	append: [
		{
			menu: 'Stammdaten',
			after: 'Artikelpreisanpassung',
			items: [
				{ kind: 'separator' },
				{
					kind: 'item',
					label: 'Personal',
					href: '/people',
					module: 'people'
				},
				{
					kind: 'item',
					label: 'Abteilungen und Positionen',
					href: '/people/lists',
					need: [
						'personal.personal-lesen',
						'personaldispo.personaldispo-lesen',
						'administrative-einstellungen.zugriff'
					],
					module: 'people'
				},
				{
					kind: 'item',
					label: 'Dienstplan',
					href: '/people/planning',
					need: ['personaldispo.personaldispo-lesen', 'administrative-einstellungen.zugriff'],
					module: 'people'
				}
			]
		},
		{
			// The settings of the product, in the group `Module` of the admin menu.
			menu: 'Admin',
			after: 'Modulverwaltung',
			items: [
				{
					kind: 'item',
					label: 'WHERE - are my people?',
					href: '/admin/modules/people',
					module: 'people'
				}
			]
		}
	],
	tools: [
		{
			label: 'Dienstplan',
			icon: 'calendar',
			tone: 'blue',
			href: '/people/schedule',
			module: 'people'
		},
		{ label: 'Personal', icon: 'people', tone: 'teal', href: '/people', module: 'people' }
	]
};

export default contribution;
