<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import {
		api,
		errorText,
		type Employee,
		type GroupRef,
		type Language,
		type PermissionArea,
		type PermissionGroup,
		type Tenant,
		type User,
		type UserGroup,
		type UserGroupInput,
		type UserInput,
		type UserSummary,
		type UserView
	} from '../../../lib/api';
	import { askConfirm } from '../../../lib/confirm.svelte';
	import { escapeWindows } from '../../../lib/escape.svelte';
	import { modules } from '../../../lib/modules.svelte';
	import ColorPicker from '../../../lib/ColorPicker.svelte';
	import '../../../lib/search/search-window.css';

	/** The four lists behind `Aktuelle Ansicht`. */
	const VIEWS: { key: UserView; label: string }[] = [
		{ key: 'local-active', label: t('Aktive lokale Benutzer') },
		{ key: 'mobile-active', label: t('Aktive mobile Benutzer') },
		{ key: 'local-inactive', label: t('Inaktive lokale Benutzer') },
		{ key: 'mobile-inactive', label: t('Inaktive mobile Benutzer') }
	];

	/** The four tabs of the user dialog. */
	type Tab = 'login' | 'sso' | 'kommunikation' | 'zuordnung';
	const TABS: { key: Tab; label: string }[] = [
		{ key: 'login', label: t('Login') },
		{ key: 'sso', label: t('SSO') },
		{ key: 'kommunikation', label: t('Kommunikation') },
		{ key: 'zuordnung', label: t('Mandanten & Lager') }
	];

	/** The two right sets of the group dialog. */
	type Rechtesatz = 'max' | 'min';

	interface Form {
		id: number | null;
		user_name: string;
		display_name: string;
		short_name: string;
		local_login: boolean;
		mobile_login: boolean;
		/** New password; empty leaves the stored one in place. */
		password: string;
		/** Repetition of the new password — the verification of the mask. */
		password_repeat: string;
		has_password: boolean;
		change_next_login: boolean;
		cannot_change: boolean;
		never_expires: boolean;
		disabled: boolean;
		language: string;
		extension: string;
		color: string;
		sso_ad_active: boolean;
		sso_ad_user: string;
		sso_ad_domain: string;
		sso_mb_active: boolean;
		sso_mb_os_user: string;
		sso_mb_machine: string;
		sso_default_entries: boolean;
		sso_default_mandant: string;
		sso_default_lager: string;
		function: string;
		phone: string;
		mobile: string;
		fax: string;
		email: string;
		email_provider: number;
		smtp_server: string;
		smtp_port: string;
		email_signature: string;
		bcc: string;
		im1_type: string;
		im1: string;
		im2_type: string;
		im2: string;
		profile_mandant: string;
		address_linked: boolean;
		personal_linked: boolean;
		/** The employee behind the account (`where_employees."user_id"`), otherwise `null`. */
		employee: number | null;
		mandants: number[];
		lager: number[];
	}

	interface GroupForm {
		id: number | null;
		name: string;
		max: string[];
		min: string[];
		mandants: number[];
		lager: number[];
		departments: number[];
		dispo_departments: number[];
	}

	/** The departments a group can be restricted to — one per area. */
	type DepartmentKind = 'departments' | 'dispo_departments';

	let view = $state<UserView>('local-active');
	let user = $state<UserSummary[]>([]);
	/** All groups with their users — the right tree. */
	let allGroups = $state<UserGroup[]>([]);
	let choice = $state<UserSummary | null>(null);
	let group = $state<GroupRef | null>(null);
	let form = $state<Form | null>(null);
	let tab = $state<Tab>('login');
	let groupForm = $state<GroupForm | null>(null);
	let rechtesatz = $state<Rechtesatz>('max');
	let tree = $state<PermissionGroup[]>([]);
	let sheet = $state<string>('uebersicht');
	let isOpen = $state<Record<string, boolean>>({});
	/** Expanded/collapsed nodes of the two trees; a missing entry counts as open. */
	let isUserOpen = $state<Record<number, boolean>>({});
	let isGroupOpen = $state<Record<number, boolean>>({});
	/** What is being dragged right now, and what it hovers over. */
	let dragging = $state<{ kind: 'benutzer' | 'gruppe'; id: number } | null>(null);
	let above = $state<string | null>(null);
	let tenants = $state<Tenant[]>([]);
	let lager = $state<{ id: number; name: string | null; is_default: boolean }[]>([]);
	/** The departments a group can be restricted to in personnel. */
	let departments = $state<{ id: number; name: string }[]>([]);
	/** The open picker `+ Abteilung hinzufügen` — one per area. */
	let departmentChoice = $state<DepartmentKind | null>(null);
	let languages = $state<Language[]>([]);
	/** The employees of the personnel module — the `Mitarbeiterliste` selection. */
	let employees = $state<Employee[]>([]);
	let accountDialog = $state(false);
	let message = $state('');
	let load = $state(true);
	let error = $state<string | null>(null);

	onMount(loadAll);

	async function loadAll(onlyId?: number | null) {
		load = true;
		error = null;
		try {
			user = await api.users.list(view);
			allGroups = await api.users.groups();
			if (!tree.length) tree = await api.users.permissions();
			if (!tenants.length) tenants = await api.tenants.list();
			if (!lager.length) lager = await api.masterData.warehouses();
			if (!departments.length) departments = await departmentNames();
			if (!languages.length) languages = await api.masterData.languages();
			if (modulePersonal) employees = await employeeNames();
			const target = onlyId === undefined ? choice?.id : onlyId;
			if (target != null) await choose(target);
			else choice = null;
		} catch (e) {
			error = errorText(e);
		} finally {
			load = false;
		}
	}

	/**
	 * The departments of the personnel module for restricting a group. Without the
	 * module there are no departments — the list then stays empty.
	 */
	async function departmentNames(): Promise<{ id: number; name: string }[]> {
		try {
			const list = await api.people.lists.list('departments');
			return list.map((entry) => ({ id: entry.id, name: entry.name }));
		} catch {
			return [];
		}
	}

	/** Whether the personnel mask is available at all (module installed). */
	const modulePersonal = $derived(modules.isInstalled('people'));

	/**
	 * The employees for the `Mitarbeiterliste` selection. Without the module or without
	 * read right the list stays empty — the row then does not appear.
	 */
	async function employeeNames(): Promise<Employee[]> {
		try {
			return await api.people.list();
		} catch {
			return [];
		}
	}

	/** The active area of the rights tree. */
	const activeSheet = $derived<PermissionArea | null>(
		tree.flatMap((branch) => branch.areas).find((area) => area.key === sheet) ?? null
	);

	/** Visible right set of the group dialog. */
	const rightSetNow = $derived(
		groupForm ? (rechtesatz === 'max' ? groupForm.max : groupForm.min) : []
	);

	const gewaehlt = $derived(
		activeSheet ? activeSheet.rights.filter((right) => rightSetNow.includes(right.key)) : []
	);

	function fromUser(user: User): Form {
		return {
			id: user.id,
			user_name: user.user_name,
			display_name: user.display_name ?? '',
			short_name: user.short_name ?? '',
			local_login: user.local_login,
			mobile_login: user.mobile_login,
			password: '',
			password_repeat: '',
			has_password: user.has_password,
			change_next_login: user.change_next_login,
			cannot_change: user.cannot_change,
			never_expires: user.never_expires,
			disabled: user.disabled,
			language: user.language ?? '',
			extension: user.extension ?? '',
			color: user.color ?? '#000000',
			sso_ad_active: user.sso_ad_active,
			sso_ad_user: user.sso_ad_user ?? '',
			sso_ad_domain: user.sso_ad_domain ?? '',
			sso_mb_active: user.sso_mb_active,
			sso_mb_os_user: user.sso_mb_os_user ?? '',
			sso_mb_machine: user.sso_mb_machine ?? '',
			sso_default_entries: user.sso_default_entries,
			sso_default_mandant: user.sso_default_mandant == null ? '' : String(user.sso_default_mandant),
			sso_default_lager: user.sso_default_lager == null ? '' : String(user.sso_default_lager),
			function: user.function ?? '',
			phone: user.phone ?? '',
			mobile: user.mobile ?? '',
			fax: user.fax ?? '',
			email: user.email ?? '',
			email_provider: user.email_provider,
			smtp_server: user.smtp_server ?? '',
			smtp_port: user.smtp_port == null ? '' : String(user.smtp_port),
			email_signature: user.email_signature ?? '',
			bcc: user.bcc ?? '',
			im1_type: user.im1_type ?? '',
			im1: user.im1 ?? '',
			im2_type: user.im2_type ?? '',
			im2: user.im2 ?? '',
			profile_mandant: user.profile_mandant == null ? '' : String(user.profile_mandant),
			address_linked: user.address_linked,
			// The user is linked as soon as an employee points to it.
			personal_linked:
				employees.find((entry) => entry.user_id === user.id) !== undefined
					? true
					: user.personal_linked,
			employee: employees.find((entry) => entry.user_id === user.id)?.id ?? null,
			mandants: [...user.mandants],
			lager: [...user.lager]
		};
	}

	function emptyForm(): Form {
		return {
			id: null,
			user_name: '',
			display_name: '',
			short_name: '',
			local_login: view.startsWith('local'),
			mobile_login: view.startsWith('mobile'),
			password: '',
			password_repeat: '',
			has_password: false,
			change_next_login: true,
			cannot_change: false,
			never_expires: false,
			disabled: view.includes('inactive'),
			language: '',
			extension: '',
			color: '#000000',
			sso_ad_active: false,
			sso_ad_user: '',
			sso_ad_domain: '',
			sso_mb_active: false,
			sso_mb_os_user: '',
			sso_mb_machine: '',
			sso_default_entries: false,
			sso_default_mandant: '',
			sso_default_lager: '',
			function: '',
			phone: '',
			mobile: '',
			fax: '',
			email: '',
			email_provider: 0,
			smtp_server: '',
			smtp_port: '',
			email_signature: '',
			bcc: '',
			im1_type: '',
			im1: '',
			im2_type: '',
			im2: '',
			profile_mandant: '',
			address_linked: false,
			personal_linked: false,
			employee: null,
			mandants: [],
			lager: []
		};
	}

	/** The login name of an employee: first letter of the first name, dot, last name. */
	function anmeldenameVon(employee: Employee): string {
		const firstName = employee.first_name.trim();
		const lastName = employee.last_name.trim();
		const start = firstName === '' ? '' : `${firstName.charAt(0)}.`;
		return `${start}${lastName}`.toLowerCase();
	}

	/** The user short name from the initials — at most three characters. */
	function shortNameOf(employee: Employee): string {
		const firstName = employee.first_name.trim();
		const lastName = employee.last_name.trim();
		const text = `${firstName.charAt(0)}${lastName.charAt(0)}`.toUpperCase();
		return text === '' ? lastName.slice(0, 3).toUpperCase() : text;
	}

	/**
	 * Takes the data of the selected employee into the mask. Empty fields of the
	 * employee overwrite nothing — a change of the selection therefore leaves the input
	 * in place as far as the employee knows nothing else.
	 */
	function chooseEmployee(id: number | null) {
		form!.employee = id;
		if (id === null) return;
		const person = employees.find((entry) => entry.id === id);
		if (!person) return;
		form!.user_name = anmeldenameVon(person);
		form!.display_name = `${person.first_name} ${person.last_name}`.trim();
		form!.short_name = shortNameOf(person);
		form!.function = person.position ?? '';
		form!.phone = person.phone ?? '';
		form!.mobile = person.mobile ?? '';
		form!.email = person.email ?? '';
	}

	/** The name of an employee in the selection: `Nachname, Vorname`. */
	function employeeText(employee: Employee): string {
		const number = (employee.personnel_no ?? '').trim();
		const name = `${employee.last_name}, ${employee.first_name}`.trim();
		return number === '' ? name : `${name} · ${number}`;
	}

	async function choose(id: number) {
		error = null;
		try {
			const loaded = await api.users.get(id);
			choice =
				user.find((entry) => entry.id === id) ?? {
					id: loaded.id,
					name: loaded.display_name ?? loaded.user_name,
					short_name: loaded.short_name,
					groups: loaded.groups
				};
			group = null;
		} catch (e) {
			error = errorText(e);
		}
	}

	function changeView() {
		message = '';
		choice = null;
		group = null;
		loadAll(null);
	}

	function newUser() {
		form = emptyForm();
		tab = 'login';
		message = '';
	}

	async function editUser(id?: number) {
		const target = id ?? choice?.id;
		if (target == null) return;
		error = null;
		try {
			form = fromUser(await api.users.get(target));
			tab = 'login';
			message = '';
		} catch (e) {
			error = errorText(e);
		}
	}

	function count(text: string): number | null {
		const value = text.trim();
		if (value === '') return null;
		const count = Number(value);
		return Number.isNaN(count) ? null : count;
	}

	function input(): UserInput {
		const current = form!;
		return {
			user_name: current.user_name,
			display_name: current.display_name.trim() || null,
			short_name: current.short_name.trim() || null,
			local_login: current.local_login,
			mobile_login: current.mobile_login,
			password: current.password.trim() || null,
			change_next_login: current.change_next_login,
			cannot_change: current.cannot_change,
			never_expires: current.never_expires,
			disabled: current.disabled,
			language: current.language.trim() || null,
			color: current.color.trim() || null,
			sso_ad_active: current.sso_ad_active,
			sso_ad_user: current.sso_ad_user.trim() || null,
			sso_ad_domain: current.sso_ad_domain.trim() || null,
			sso_mb_active: current.sso_mb_active,
			sso_mb_os_user: current.sso_mb_os_user.trim() || null,
			sso_mb_machine: current.sso_mb_machine.trim() || null,
			sso_default_entries: current.sso_default_entries,
			sso_default_mandant: count(current.sso_default_mandant),
			sso_default_lager: count(current.sso_default_lager),
			function: current.function.trim() || null,
			phone: current.phone.trim() || null,
			mobile: current.mobile.trim() || null,
			fax: current.fax.trim() || null,
			email: current.email.trim() || null,
			email_provider: current.email_provider,
			smtp_server: current.smtp_server.trim() || null,
			smtp_port: count(current.smtp_port),
			email_signature: current.email_signature.trim() || null,
			bcc: current.bcc.trim() || null,
			im1_type: current.im1_type.trim() || null,
			im1: current.im1.trim() || null,
			im2_type: current.im2_type.trim() || null,
			im2: current.im2.trim() || null,
			profile_mandant: count(current.profile_mandant),
			extension: current.extension.trim() || null,
			mandants: [...current.mandants],
			lager: [...current.lager]
		};
	}

	/** The password verification of the mask: „Kennwort wiederholen". */
	const passwordDiffers = $derived.by(() => {
		if (!form) return { input: false, ab: false };
		const input = form.password !== '';
		return { input, ab: input && form.password !== form.password_repeat };
	});

	async function saveUser() {
		if (!form) return;
		error = null;
		message = '';
		if (passwordDiffers.ab) {
			tab = 'login';
			error = t('Die Kennwörter stimmen nicht überein.');
			return;
		}
		const selectedEmployee = form.employee;
		try {
			const user =
				form.id === null
					? await api.users.create(input())
					: await api.users.update(form.id, input());
			await linkEmployee(user.id, selectedEmployee);
			await loadAll(user.id);
			message = t('Gespeichert.');
		} catch (e) {
			error = errorText(e);
		}
	}

	/**
	 * Writes the employee behind the account: previously linked employees are
	 * released, the selected one gets the id of the account. Without the personnel
	 * module nothing happens.
	 */
	async function linkEmployee(userId: number, employeeId: number | null) {
		if (!modulePersonal) return;
		const previous = employees.filter(
			(entry) => entry.user_id === userId && entry.id !== employeeId
		);
		for (const entry of previous) {
			if (entry.id != null) await api.people.setUser(entry.id, null);
		}
		if (employeeId != null) await api.people.setUser(employeeId, userId);
		employees = await employeeNames();
	}

	async function deleteUser() {
		if (!choice) return;
		const ok = await askConfirm({
			title: t('Löschen'),
			message: t('Benutzer „{name}“ löschen?', { name: choice.name }),
			confirmLabel: t('Löschen'),
			cancelLabel: t('Behalten')
		});
		if (!ok) return;
		error = null;
		try {
			await linkEmployee(choice.id, null);
			await api.users.remove(choice.id);
			choice = null;
			group = null;
			await loadAll(null);
		} catch (e) {
			error = errorText(e);
		}
	}

	/** `+` of the right list: a new group that is assigned to the selected user. */
	function newGroup() {
		groupForm = {
			id: null,
			name: '',
			max: [],
			min: [],
			mandants: [],
			lager: [],
			departments: [],
			dispo_departments: []
		};
		rechtesatz = 'max';
		sheet = 'uebersicht';
		isOpen = {};
		departmentChoice = null;
	}

	/** Expanded/collapsed; without an entry „offen" applies. */
	function istOffen(stand: Record<number, boolean>, id: number): boolean {
		return stand[id] !== false;
	}

	function collapse(stand: Record<number, boolean>, id: number) {
		stand[id] = !istOffen(stand, id);
	}

	/**
	 * Drag and drop between the two trees (`Rechtevergabe.PNG`): a user
	 * onto a group or a group onto a user assigns the group.
	 */
	function startDrag(event: DragEvent, kind: 'benutzer' | 'gruppe', id: number) {
		dragging = { kind, id };
		if (event.dataTransfer) {
			event.dataTransfer.effectAllowed = 'link';
			event.dataTransfer.setData('text/plain', `${kind}:${id}`);
		}
	}

	function endDrag() {
		dragging = null;
		above = null;
	}

	function overTarget(event: DragEvent, kind: 'benutzer' | 'gruppe', id: number) {
		if (!dragging || dragging.kind === kind) return;
		event.preventDefault();
		if (event.dataTransfer) event.dataTransfer.dropEffect = 'link';
		above = `${kind}-${id}`;
	}

	async function assignGroup(event: DragEvent, kind: 'benutzer' | 'gruppe', id: number) {
		event.preventDefault();
		const source = dragging;
		dragging = null;
		above = null;
		if (!source || source.kind === kind) return;
		if (source.kind === 'benutzer') await assignTo(source.id, id);
		else await assignTo(id, source.id);
	}

	/** Assigns a group to a user. */
	async function assignTo(userId: number, groupId: number) {
		error = null;
		const name = allGroups.find((entry) => entry.id === groupId)?.name ?? t('Gruppe');
		const account = user.find((entry) => entry.id === userId)?.name ?? t('Benutzer');
		try {
			await api.users.addGroup(userId, groupId);
			isGroupOpen[groupId] = true;
			isUserOpen[userId] = true;
			await loadAll(choice?.id);
			message = t('„{group}“ ist jetzt bei „{account}“.', { group: name, account });
		} catch (e) {
			error = errorText(e);
		}
	}

	/** Takes a user out of a group again (`×` on the child row). */
	async function clearAssignment(userId: number, groupId: number) {
		const name = allGroups.find((entry) => entry.id === groupId)?.name ?? t('Gruppe');
		const account = allGroups
			.find((entry) => entry.id === groupId)
			?.member_users.find((entry) => entry.id === userId)?.name;
		const ok = await askConfirm({
			title: t('Zuordnung entfernen'),
			message: t('„{account}“ aus der Gruppe „{group}“ nehmen?', {
				account: account ?? t('Benutzer'),
				group: name
			}),
			confirmLabel: t('Entfernen'),
			cancelLabel: t('Behalten')
		});
		if (!ok) return;
		error = null;
		try {
			await api.users.removeGroup(userId, groupId);
			await loadAll(choice?.id);
			message = t('„{account}“ ist nicht mehr in „{group}“.', {
				account: account ?? t('Benutzer'),
				group: name
			});
		} catch (e) {
			error = errorText(e);
		}
	}

	async function openGroup(ref: GroupRef) {
		error = null;
		try {
			const loaded = await api.users.group(ref.id);
			group = ref;
			groupForm = {
				id: loaded.id,
				name: loaded.name,
				max: [...loaded.max_rights],
				min: [...loaded.min_rights],
				mandants: [...loaded.mandants],
				lager: [...loaded.lager],
				departments: [...loaded.departments],
				dispo_departments: [...loaded.dispo_departments]
			};
			rechtesatz = 'max';
			sheet = 'uebersicht';
			departmentChoice = null;
		} catch (e) {
			error = errorText(e);
		}
	}

	async function copyGroup() {
		if (!group) return;
		error = null;
		try {
			const copy = await api.users.groupCopy(group.id);
			if (choice) await api.users.addGroup(choice.id, copy.id);
			await loadAll(choice?.id);
			message = t('Kopie „{name}“ angelegt.', { name: copy.name });
		} catch (e) {
			error = errorText(e);
		}
	}

	async function removeGroup() {
		if (!group || !choice) return;
		const ok = await askConfirm({
			title: t('Zuordnung entfernen'),
			message: t('Berechtigungsgruppe „{group}“ beim Benutzer „{user}“ entfernen?', {
				group: group.name,
				user: choice.name
			}),
			confirmLabel: t('Entfernen'),
			cancelLabel: t('Behalten')
		});
		if (!ok) return;
		error = null;
		try {
			await api.users.removeGroup(choice.id, group.id);
			group = null;
			await loadAll(choice.id);
		} catch (e) {
			error = errorText(e);
		}
	}

	/** Delete the group itself — with its rights, departments and assignments. */
	async function deleteGroup() {
		if (!group) return;
		const ok = await askConfirm({
			title: t('Löschen'),
			message: t(
				'Berechtigungsgruppe „{group}“ mit ihren Rechten und Zuordnungen löschen?',
				{ group: group.name }
			),
			confirmLabel: t('Löschen'),
			cancelLabel: t('Behalten')
		});
		if (!ok) return;
		error = null;
		try {
			await api.users.groupRemove(group.id);
			group = null;
			await loadAll(choice?.id ?? null);
			message = t('Berechtigungsgruppe gelöscht.');
		} catch (e) {
			error = errorText(e);
		}
	}

	function groupInput(): UserGroupInput {
		const current = groupForm!;
		return {
			name: current.name,
			max_rights: [...current.max],
			min_rights: [...current.min],
			mandants: [...current.mandants],
			lager: [...current.lager],
			departments: [...current.departments],
			dispo_departments: [...current.dispo_departments]
		};
	}

	async function saveGroup() {
		if (!groupForm) return;
		error = null;
		try {
			const id = groupForm.id;
			const saved =
				id === null
					? await api.users.groupCreate(groupInput())
					: await api.users.groupUpdate(id, groupInput());
			if (id === null && choice) await api.users.addGroup(choice.id, saved.id);
			groupForm = null;
			await loadAll(choice?.id);
			message = t('Berechtigungsgruppe „{name}“ gespeichert.', { name: saved.name });
		} catch (e) {
			error = errorText(e);
		}
	}

	/** Toggle a right of the visible set. */
	function toggleRight(key: string) {
		if (!groupForm) return;
		const list = rechtesatz === 'max' ? groupForm.max : groupForm.min;
		const offset = list.indexOf(key);
		if (offset >= 0) list.splice(offset, 1);
		else list.push(key);
		groupForm[rechtesatz] = [...list];
	}

	/**
	 * All rights of the catalogue — the overview therefore also switches `Admin` on.
	 */
	const alleRechtsschluessel = $derived(
		tree.flatMap((branch) => branch.areas).flatMap((area) => area.rights.map((right) => right.key))
	);

	/**
	 * The `Max. Rechte` button of the mask: sets **all** rights of the catalogue — every
	 * category from `Adressen` to `Admin`.
	 */
	function setMaxRights() {
		if (!groupForm) {
			rechtesatz = 'max';
			return;
		}
		rechtesatz = 'max';
		groupForm.max = [...alleRechtsschluessel];
	}

	/**
	 * `Alles aktivieren` / `Alles deaktivieren`: in an area its rights, in the
	 * overview the whole catalogue including `Admin`.
	 */
	function allRights(an: boolean) {
		if (!groupForm) return;
		const keys = activeSheet
			? activeSheet.rights.map((right) => right.key)
			: alleRechtsschluessel;
		const others = rightSetNow.filter((key) => !keys.includes(key));
		groupForm[rechtesatz] = an ? [...others, ...keys] : others;
	}

	function toggleBranch(key: string) {
		isOpen[key] = !isOpen[key];
	}

	/** Select and deselect tenant or warehouse of the group in the group dialog. */
	function toggleScope(kind: 'mandants' | 'lager', id: number) {
		if (!groupForm) return;
		const list = groupForm[kind];
		const offset = list.indexOf(id);
		if (offset >= 0) list.splice(offset, 1);
		else list.push(id);
		groupForm[kind] = [...list];
	}

	/** The departments the group is restricted to in an area. */
	function selectedDepartments(kind: DepartmentKind) {
		if (!groupForm) return [];
		return departments.filter((entry) => groupForm![kind].includes(entry.id));
	}

	/** What the picker of the area still offers. */
	function freeDepartments(kind: DepartmentKind) {
		if (!groupForm) return [];
		return departments.filter((entry) => !groupForm![kind].includes(entry.id));
	}

	/** `alle` lifts the restriction of the area: no department in the group. */
	function departmentAll(kind: DepartmentKind) {
		if (!groupForm) return;
		groupForm[kind] = [];
		departmentChoice = null;
	}

	function addDepartment(kind: DepartmentKind, id: number) {
		if (!groupForm || groupForm[kind].includes(id)) return;
		groupForm[kind] = [...groupForm[kind], id];
		departmentChoice = null;
	}

	function removeDepartment(kind: DepartmentKind, id: number) {
		if (!groupForm) return;
		groupForm[kind] = groupForm[kind].filter((entry) => entry !== id);
	}

	// `Esc` closes the picker before the window below gets its turn.
	$effect(() => (departmentChoice ? escapeWindows(() => (departmentChoice = null)) : undefined));
</script>

<svelte:head><title>{t('Benutzerverwaltung — WHERE')}</title></svelte:head>

<h1>{t('Benutzerverwaltung')}</h1>

<div class="head">
	<span class="caption">{t('Aktuelle Ansicht')}</span>
	<select class="text" value={view} onchange={(e) => {
		view = e.currentTarget.value as UserView;
		changeView();
	}}>
		{#each VIEWS as entry (entry.key)}
			<option value={entry.key}>{entry.label}</option>
		{/each}
	</select>
</div>

<div class="window">
	<div class="column">
		<div class="bar">
			<button type="button" class="square" title={t('Benutzer hinzufügen')} onclick={newUser}>
				<svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
					<circle cx="6" cy="4.6" r="2.6" fill="currentColor" />
					<path d="M1.4 14c0-2.5 2-4.4 4.6-4.4s4.6 1.9 4.6 4.4z" fill="currentColor" />
					<path d="M12.6 3.2v4.2M10.5 5.3h4.2" stroke="currentColor" stroke-width="1.4" />
				</svg>
			</button>
			<button
				type="button"
				class="square"
				title={t('Benutzer entfernen')}
				disabled={!choice}
				onclick={deleteUser}
			>
				<svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
					<circle cx="6" cy="4.6" r="2.6" fill="currentColor" />
					<path d="M1.4 14c0-2.5 2-4.4 4.6-4.4s4.6 1.9 4.6 4.4z" fill="currentColor" />
					<path d="M10.5 5.3h4.2" stroke="currentColor" stroke-width="1.4" />
				</svg>
			</button>
		</div>
		<div class="list">
			<table>
				<thead>
					<tr><th>{t('Benutzer ({n})', { n: user.length })}</th></tr>
				</thead>
				<tbody>
					{#each user as entry (entry.id)}
						<tr
							class:marked={choice?.id === entry.id}
							class:over={above === `benutzer-${entry.id}`}
							draggable="true"
							onclick={() => choose(entry.id)}
							ondblclick={() => editUser(entry.id)}
							ondragstart={(e) => startDrag(e, 'benutzer', entry.id)}
							ondragend={endDrag}
							ondragover={(e) => overTarget(e, 'benutzer', entry.id)}
							ondragleave={() => (above = null)}
							ondrop={(e) => assignGroup(e, 'benutzer', entry.id)}
							title={t('Doppelklick öffnet den Benutzer; eine Gruppe hierher ziehen ordnet sie zu')}
						>
							<td>
								{#if entry.groups.length}
									<button
										type="button"
										class="branch-arrow"
										title={istOffen(isUserOpen, entry.id) ? t('zugeklappt') : t('aufgeklappt')}
										onclick={(e) => {
											e.stopPropagation();
											collapse(isUserOpen, entry.id);
										}}>{istOffen(isUserOpen, entry.id) ? '▾' : '▸'}</button
									>
								{:else}
									<span class="arrow-place" aria-hidden="true"></span>
								{/if}
								{entry.name}{entry.short_name ? ` [${entry.short_name}]` : ''}
							</td>
						</tr>
						{#if istOffen(isUserOpen, entry.id)}
							{#each entry.groups as zugeordnet (zugeordnet.id)}
								<tr
									class="child"
									class:marked={group?.id === zugeordnet.id && choice?.id === entry.id}
									class:over={above === `kind-${zugeordnet.id}-${entry.id}`}
									onclick={() => {
										group = zugeordnet;
										choose(entry.id);
									}}
									ondblclick={() => openGroup(zugeordnet)}
									ondragover={(e) => {
										if (dragging?.kind !== 'benutzer') return;
										e.preventDefault();
										above = `kind-${zugeordnet.id}-${entry.id}`;
									}}
									ondragleave={() => (above = null)}
									ondrop={(e) => assignGroup(e, 'gruppe', zugeordnet.id)}
									title={t('Doppelklick öffnet die Rechte; einen Benutzer hierher ziehen ordnet ihn zu')}
								>
									<td>{zugeordnet.name}</td>
								</tr>
							{/each}
						{/if}
					{:else}
						<tr>
							<td class="small">{load ? t('Lade …') : t('Kein Benutzer in dieser Ansicht.')}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</div>

	<div class="column">
		<div class="bar right">
			<button
				type="button"
				class="square"
				title={t('Berechtigungsgruppe hinzufügen')}
				disabled={!choice}
				onclick={newGroup}
			>
				<svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
					<circle cx="4.6" cy="4.6" r="2.1" fill="currentColor" />
					<circle cx="9.8" cy="4.6" r="2.1" fill="currentColor" />
					<path d="M0.6 13.4c0-2.1 1.8-3.8 4-3.8s4 1.7 4 3.8z" fill="currentColor" />
					<path d="M8.4 13.4c0-2.1 1.5-3.8 3.3-3.8 1.8 0 3.3 1.7 3.3 3.8z" fill="currentColor" />
					<path d="M12.8 1.4v3.4M11.1 3.1h3.4" stroke="currentColor" stroke-width="1.3" />
				</svg>
			</button>
			<button
				type="button"
				class="square"
				title={t('Berechtigungsgruppe kopieren')}
				disabled={!group}
				onclick={copyGroup}
			>
				<svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
					<rect x="2.4" y="1.6" width="7.2" height="9.6" fill="none" stroke="currentColor" />
					<rect x="6.4" y="4.8" width="7.2" height="9.6" fill="none" stroke="currentColor" />
				</svg>
			</button>
			<button
				type="button"
				class="square"
				title={t('Zuordnung des Benutzers aufheben')}
				disabled={!group}
				onclick={removeGroup}
			>
				<svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
					<circle cx="6" cy="4.6" r="2.6" fill="currentColor" />
					<path d="M1.4 14c0-2.5 2-4.4 4.6-4.4s4.6 1.9 4.6 4.4z" fill="currentColor" />
					<path d="M10.5 5.3h4.2" stroke="currentColor" stroke-width="1.4" />
				</svg>
			</button>
			<button
				type="button"
				class="square"
				title={t('Berechtigungsgruppe löschen')}
				disabled={!group}
				onclick={deleteGroup}
			>
				<svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
					<path d="M2.4 4.3h11.2" stroke="currentColor" stroke-width="1.3" />
					<path d="M5.6 4.3V2.5h4.8v1.8" fill="none" stroke="currentColor" />
					<path d="M3.9 4.3l.9 9.4h6.4l.9-9.4" fill="none" stroke="currentColor" />
					<path d="M6.6 6.6v5M9.4 6.6v5" stroke="currentColor" stroke-width="1.1" />
				</svg>
			</button>
		</div>
		<div class="list">
			<table>
				<thead>
					<tr><th>{t('Berechtigungsgruppen ({n})', { n: allGroups.length })}</th></tr>
				</thead>
				<tbody>
					{#each allGroups as entry (entry.id)}
						<tr
							class:marked={group?.id === entry.id}
							class:over={above === `gruppe-${entry.id}`}
							draggable="true"
							onclick={() => (group = { id: entry.id, name: entry.name })}
							ondblclick={() => openGroup({ id: entry.id, name: entry.name })}
							ondragstart={(e) => startDrag(e, 'gruppe', entry.id)}
							ondragend={endDrag}
							ondragover={(e) => overTarget(e, 'gruppe', entry.id)}
							ondragleave={() => (above = null)}
							ondrop={(e) => assignGroup(e, 'gruppe', entry.id)}
							title={t('Doppelklick öffnet die Rechte; einen Benutzer hierher ziehen ordnet ihn zu')}
						>
							<td>
								{#if entry.member_users.length}
									<button
										type="button"
										class="branch-arrow"
										title={istOffen(isGroupOpen, entry.id) ? t('zugeklappt') : t('aufgeklappt')}
										onclick={(e) => {
											e.stopPropagation();
											collapse(isGroupOpen, entry.id);
										}}>{istOffen(isGroupOpen, entry.id) ? '▾' : '▸'}</button
									>
								{:else}
									<span class="arrow-place" aria-hidden="true"></span>
								{/if}
								{entry.name}
							</td>
						</tr>
						{#if istOffen(isGroupOpen, entry.id)}
							{#each entry.member_users as mitglied (mitglied.id)}
								<tr
									class="child"
									class:marked={choice?.id === mitglied.id}
									draggable="true"
									onclick={() => choose(mitglied.id)}
									ondblclick={() => editUser(mitglied.id)}
									ondragstart={(e) => startDrag(e, 'benutzer', mitglied.id)}
									ondragend={endDrag}
									title={t('Doppelklick öffnet den Benutzer')}
								>
									<td>
										{mitglied.name}
										<button
											type="button"
											class="branch-arrow remove"
											title={t('Aus der Gruppe nehmen')}
											onclick={(e) => {
												e.stopPropagation();
												clearAssignment(mitglied.id, entry.id);
											}}>×</button
										>
									</td>
								</tr>
							{:else}
								<tr class="child empty"><td>{t('keine Benutzer')}</td></tr>
							{/each}
						{/if}
					{:else}
						<tr>
							<td class="small">
								{load ? t('Lade …') : t('Keine Gruppe — mit `+` eine anlegen.')}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</div>
</div>

<footer class="foot">
	<span class="spacer"></span>
	<a class="btn" href="/">{t('Schließen')}</a>
</footer>

{#if error}
	<p class="error">{error}</p>
{/if}
{#if message}
	<p class="message">{message}</p>
{/if}

{#if form}
	<div class="sw-backdrop" role="presentation">
		<div
			class="sw-window"
			role="dialog"
			aria-modal="true"
			aria-label={t('Benutzer')}
			style="width: min(34rem, 100%)"
		>
			<div class="sw-titlebar">
				<span class="sw-icon" aria-hidden="true"></span>
				<span class="sw-title"
					>{form.id === null
						? t('Benutzer hinzufügen')
						: t('Benutzer — {name}', { name: form.display_name || form.user_name })}</span
				>
				<button class="sw-close" type="button" title={t('Schließen')} onclick={() => (form = null)}
					>×</button
				>
			</div>
			<div class="tabs">
				{#each TABS as entry (entry.key)}
					<button type="button" class:active={tab === entry.key} onclick={() => (tab = entry.key)}
						>{entry.label}</button
					>
				{/each}
			</div>
			<div class="sw-content">
				{#if tab === 'login'}
					<div class="check-pair">
						<label class="check">
							<input
								type="checkbox"
								checked={form.local_login}
								onchange={(e) => (form!.local_login = e.currentTarget.checked)}
							/>
							{t('Login local')}
						</label>
						<label class="check">
							<input
								type="checkbox"
								checked={form.mobile_login}
								onchange={(e) => (form!.mobile_login = e.currentTarget.checked)}
							/>
							{t('Login mobile')}
						</label>
					</div>
					<div class="row">
						<span class="caption">{t('Anmeldename')}</span>
						<input
							class="text"
							maxlength="40"
							value={form.user_name}
							oninput={(e) => (form!.user_name = e.currentTarget.value)}
						/>
					</div>
					{#if modulePersonal && employees.length > 0}
						<div class="row">
							<span class="caption">{t('Mitarbeiterliste')}</span>
							<select
								class="text"
								onchange={(e) =>
									chooseEmployee(
										e.currentTarget.value === '' ? null : Number(e.currentTarget.value)
									)}
							>
								<option value="" selected={form.employee == null}>—</option>
								{#each employees as entry (entry.id)}
									<option value={entry.id} selected={form.employee === entry.id}
										>{employeeText(entry)}</option
									>
								{/each}
							</select>
						</div>
					{/if}
					<div class="row">
						<span class="caption">{t('Vollständiger Name')}</span>
						<input
							class="text"
							maxlength="80"
							value={form.display_name}
							oninput={(e) => (form!.display_name = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">{t('Kennwort')}</span>
						<input
							class="text"
							type="password"
							autocomplete="new-password"
							placeholder={form.has_password
								? t('gespeichert — leer lassen zum Behalten')
								: t('noch keines')}
							value={form.password}
							oninput={(e) => (form!.password = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">{t('Kennwort wiederholen')}</span>
						<input
							class="text"
							class:error-field={passwordDiffers.ab}
							type="password"
							autocomplete="new-password"
							disabled={!passwordDiffers.input}
							placeholder={passwordDiffers.input
								? t('zur Kontrolle noch einmal')
								: t('erst oben eingeben')}
							value={form.password_repeat}
							oninput={(e) => (form!.password_repeat = e.currentTarget.value)}
						/>
					</div>
					{#if passwordDiffers.ab}
						<div class="row">
							<span class="caption"></span>
							<span class="warning">{t('Die Kennwörter stimmen nicht überein.')}</span>
						</div>
					{/if}
					<div class="row">
						<span class="caption">{t('Benutzerkürzel')}</span>
						<input
							class="text short"
							maxlength="3"
							value={form.short_name}
							oninput={(e) => (form!.short_name = e.currentTarget.value)}
						/>
						<span class="unit">{t('max. 3 Zeichen')}</span>
					</div>
					<div class="row">
						<span class="caption">{t('Terminfarbe')}</span>
						<ColorPicker
							bind:value={form.color}
							label={t('Terminfarbe des Benutzers')}
						/>
					</div>
					<div class="block">
						<label class="check">
							<input
								type="checkbox"
								checked={form.change_next_login}
								onchange={(e) => (form!.change_next_login = e.currentTarget.checked)}
							/>
							{t('Kennwort ändern bei der nächsten Anmeldung')}
						</label>
						<label class="check">
							<input
								type="checkbox"
								checked={form.cannot_change}
								onchange={(e) => (form!.cannot_change = e.currentTarget.checked)}
							/>
							{t('Benutzer kann Kennwort nicht ändern')}
						</label>
						<label class="check">
							<input
								type="checkbox"
								checked={form.never_expires}
								onchange={(e) => (form!.never_expires = e.currentTarget.checked)}
							/>
							{t('Kennwort läuft nie ab')}
						</label>
						<label class="check">
							<input
								type="checkbox"
								checked={form.disabled}
								onchange={(e) => (form!.disabled = e.currentTarget.checked)}
							/>
							{t('Zugang deaktiviert')}
						</label>
						<label class="check">
							<input type="checkbox" checked={!form.address_linked} disabled />
							{t('Benutzer nicht mit Adresse verlinkt')}
						</label>
						<label class="check off">
							<input type="checkbox" checked={!form.personal_linked} disabled />
							{t('Benutzer nicht mit Personal verlinkt')}
						</label>
					</div>
					<div class="row">
						<span class="caption">{t('Sprache')}</span>
						<select
							class="text"
							value={form.language}
							onchange={(e) => (form!.language = e.currentTarget.value)}
						>
							<option value="">{t('nach Systemsprache')}</option>
							{#each languages as sprache (sprache.id)}
								<option value={sprache.language ?? ''}>{sprache.language ?? ''}</option>
							{/each}
						</select>
					</div>
					<div class="row">
						<label class="check off">
							<input type="checkbox" checked={false} disabled />
							{t('2FA aktiv')}
						</label>
					</div>
				{:else if tab === 'sso'}
					<div class="block">
						<label class="check title">
							<input
								type="checkbox"
								checked={form.sso_ad_active}
								onchange={(e) => (form!.sso_ad_active = e.currentTarget.checked)}
							/>
							{t('Active Directory - Single Sign On')}
						</label>
						<div class="row">
							<span class="caption">{t('AD-Benutzername')}</span>
							<input
								class="text"
								value={form.sso_ad_user}
								oninput={(e) => (form!.sso_ad_user = e.currentTarget.value)}
							/>
						</div>
						<div class="row">
							<span class="caption">{t('AD-Domäne')}</span>
							<input
								class="text"
								value={form.sso_ad_domain}
								oninput={(e) => (form!.sso_ad_domain = e.currentTarget.value)}
							/>
						</div>
					</div>
					<div class="block">
						<label class="check title">
							<input
								type="checkbox"
								checked={form.sso_mb_active}
								onchange={(e) => (form!.sso_mb_active = e.currentTarget.checked)}
							/>
							{t('Computer-Benutzerprofil-Bindung - Single Sign On')}
						</label>
						<div class="row">
							<span class="caption">{t('OS-Benutzername')}</span>
							<input
								class="text"
								value={form.sso_mb_os_user}
								oninput={(e) => (form!.sso_mb_os_user = e.currentTarget.value)}
							/>
							<span class="unit">+</span>
						</div>
						<div class="row">
							<span class="caption">{t('OS-Computername')}</span>
							<input
								class="text"
								value={form.sso_mb_machine}
								oninput={(e) => (form!.sso_mb_machine = e.currentTarget.value)}
							/>
							<span class="unit">+</span>
						</div>
					</div>
					<div class="block">
						<label class="check title">
							<input
								type="checkbox"
								checked={form.sso_default_entries}
								onchange={(e) => (form!.sso_default_entries = e.currentTarget.checked)}
							/>
							{t('Automatische Mandanten- und Lagerwahl')}
						</label>
						<div class="row">
							<span class="caption">{t('Mandant')}</span>
							<select
								class="text"
								value={form.sso_default_mandant}
								onchange={(e) => (form!.sso_default_mandant = e.currentTarget.value)}
							>
								<option value="">{t('— keiner —')}</option>
								{#each tenants as entry (entry.id)}
									<option value={String(entry.id)}>{entry.name}</option>
								{/each}
							</select>
						</div>
						<div class="row">
							<span class="caption">{t('Lager')}</span>
							<select
								class="text"
								value={form.sso_default_lager}
								onchange={(e) => (form!.sso_default_lager = e.currentTarget.value)}
							>
								<option value="">{t('— keines —')}</option>
								{#each lager as entry (entry.id)}
									<option value={String(entry.id)}>{entry.name ?? t('Lager {id}', { id: entry.id })}</option>
								{/each}
							</select>
						</div>
					</div>
				{:else if tab === 'kommunikation'}
					<h3>{t('Persönliche Kontaktdaten')}</h3>
					<div class="row">
						<span class="caption">{t('Funktion')}</span>
						<input
							class="text"
							value={form.function}
							oninput={(e) => (form!.function = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">{t('Telefon')}</span>
						<input
							class="text"
							value={form.phone}
							oninput={(e) => (form!.phone = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">{t('Mobiltelefon')}</span>
						<input
							class="text"
							value={form.mobile}
							oninput={(e) => (form!.mobile = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">{t('Fax')}</span>
						<input class="text" value={form.fax} oninput={(e) => (form!.fax = e.currentTarget.value)} />
					</div>
					<div class="row">
						<span class="caption">{t('eMail-Adresse')}</span>
						<input
							class="text"
							value={form.email}
							oninput={(e) => (form!.email = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">{t('eMail-Provider')}</span>
						<button
							type="button"
							class="square red"
							title={t('Auf Standard zurücksetzen')}
							onclick={() => (form!.email_provider = 0)}>×</button
						>
						<select
							class="text"
							value={String(form.email_provider)}
							onchange={(e) => (form!.email_provider = Number(e.currentTarget.value))}
						>
							<option value="0">{t('SMTP Mailserver')}</option>
							<option value="1">{t('OAuth 2.0 (Microsoft 365 / Google)')}</option>
						</select>
					</div>
					<div class="row">
						<span class="caption"></span>
						<button type="button" class="btn" onclick={() => (accountDialog = true)}>
							{t('eMail-Konto Einstellungen')}
						</button>
					</div>
					<div class="row">
						<span class="caption">{t('Instant-Messenger 1')}</span>
						<input
							class="text short"
							placeholder={t('Typ')}
							value={form.im1_type}
							oninput={(e) => (form!.im1_type = e.currentTarget.value)}
						/>
						<input
							class="text"
							value={form.im1}
							oninput={(e) => (form!.im1 = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">{t('Instant-Messenger 2')}</span>
						<input
							class="text short"
							placeholder={t('Typ')}
							value={form.im2_type}
							oninput={(e) => (form!.im2_type = e.currentTarget.value)}
						/>
						<input
							class="text"
							value={form.im2}
							oninput={(e) => (form!.im2 = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">{t('Standardprofil für Mandant')}</span>
						<select
							class="text"
							value={form.profile_mandant}
							onchange={(e) => (form!.profile_mandant = e.currentTarget.value)}
						>
							<option value="">{t('— keiner —')}</option>
							{#each tenants as entry (entry.id)}
								<option value={String(entry.id)}>{entry.name}</option>
							{/each}
						</select>
					</div>
					<h3>{t('Daten für Telefonidentifizierung (Profilunabhängig)')}</h3>
					<div class="row">
						<span class="caption">{t('Interne Durchwahlnummer')}</span>
						<input
							class="text short"
							maxlength="10"
							value={form.extension}
							oninput={(e) => (form!.extension = e.currentTarget.value)}
						/>
					</div>
				{:else}
					<div class="grid-title">{t('Mandant')}</div>
					<div class="grid">
						<div class="grid-head"><span>{t('Aktiv')}</span><span>{t('Mandant')}</span></div>
						{#each tenants as entry (entry.id)}
							<div class="grid-row">
								<input
									type="checkbox"
									checked={form.mandants.includes(entry.id)}
									onchange={() => {
										const offset = form!.mandants.indexOf(entry.id);
										if (offset >= 0) form!.mandants.splice(offset, 1);
										else form!.mandants.push(entry.id);
										form!.mandants = [...form!.mandants];
									}}
								/>
								<span>{entry.name}</span>
							</div>
						{:else}
							<p class="small">{t('Noch kein Mandant angelegt.')}</p>
						{/each}
					</div>
					<div class="grid-title">{t('Lager')}</div>
					<div class="grid">
						<div class="grid-head"><span>{t('Aktiv')}</span><span>{t('Lager')}</span></div>
						{#each lager as entry (entry.id)}
							<div class="grid-row">
								<input
									type="checkbox"
									checked={form.lager.includes(entry.id)}
									onchange={() => {
										const offset = form!.lager.indexOf(entry.id);
										if (offset >= 0) form!.lager.splice(offset, 1);
										else form!.lager.push(entry.id);
										form!.lager = [...form!.lager];
									}}
								/>
								<span>{entry.name ?? t('Lager {id}', { id: entry.id })}</span>
							</div>
						{:else}
							<p class="small">{t('Noch kein Lager angelegt.')}</p>
						{/each}
					</div>
				{/if}
			</div>
			<div class="sw-footer">
				<button type="button" class="sw-button" onclick={() => (form = null)}>{t('Abbrechen')}</button>
				<button
					type="button"
					class="sw-button primary"
					disabled={!form.user_name.trim() ||
						(!form.local_login && !form.mobile_login) ||
						passwordDiffers.ab}
					onclick={saveUser}>{t('Ok')}</button
				>
			</div>
		</div>
	</div>
{/if}

{#if accountDialog && form}
	<div class="sw-backdrop" role="presentation">
		<div
			class="sw-window"
			role="dialog"
			aria-modal="true"
			aria-label={t('eMail-Konto')}
			style="width: min(30rem, 100%)"
		>
			<div class="sw-titlebar">
				<span class="sw-icon" aria-hidden="true"></span>
				<span class="sw-title">{t('eMail-Konto Einstellungen')}</span>
				<button class="sw-close" type="button" title={t('Schließen')} onclick={() => (accountDialog = false)}
					>×</button
				>
			</div>
			<div class="sw-content">
				<div class="row">
					<span class="caption">{t('SMTP-Server')}</span>
					<input
						class="text"
						value={form.smtp_server}
						oninput={(e) => (form!.smtp_server = e.currentTarget.value)}
					/>
				</div>
				<div class="row">
					<span class="caption">{t('Port')}</span>
					<input
						class="numeric"
						type="number"
						min="1"
						max="65535"
						value={form.smtp_port}
						oninput={(e) => (form!.smtp_port = e.currentTarget.value)}
					/>
				</div>
				<div class="row">
					<span class="caption">{t('BCC-Adresse')}</span>
					<input class="text" value={form.bcc} oninput={(e) => (form!.bcc = e.currentTarget.value)} />
				</div>
				<label class="field">
					{t('Signatur')}
					<textarea
						class="text"
						rows="5"
						value={form.email_signature}
						oninput={(e) => (form!.email_signature = e.currentTarget.value)}
					></textarea>
				</label>
			</div>
			<div class="sw-footer">
				<button type="button" class="sw-button primary" onclick={() => (accountDialog = false)}>
					{t('Übernehmen')}
				</button>
			</div>
		</div>
	</div>
{/if}

{#if groupForm}
	<div class="sw-backdrop" role="presentation">
		<div
			class="sw-window"
			role="dialog"
			aria-modal="true"
			aria-label={t('Gruppenrechte')}
			style="width: min(56rem, 100%)"
		>
			<div class="sw-titlebar">
				<span class="sw-icon" aria-hidden="true"></span>
				<span class="sw-title">{t('Einstellungen: Gruppenrechte')}</span>
				<button
					class="sw-close"
					type="button"
					title={t('Schließen')}
					onclick={() => (groupForm = null)}>×</button
				>
			</div>
			<div class="sw-content">
				<div class="heading">{t('Übersicht der Gruppe')}</div>
				<div class="row">
					<span class="caption">{t('Gruppe')}</span>
					<input
						class="text"
						maxlength="40"
						value={groupForm.name}
						oninput={(e) => (groupForm!.name = e.currentTarget.value)}
					/>
				</div>
				<div class="scope">
					<div class="scope-list">
						<div class="scope-head">{t('Mandanten')}</div>
						{#each tenants as entry (entry.id)}
							<div class="scope-row">
								<input
									type="checkbox"
									checked={groupForm.mandants.includes(entry.id)}
									onchange={() => toggleScope('mandants', entry.id)}
								/>
								{entry.name}
							</div>
						{:else}
							<p class="small">{t('Kein Mandant.')}</p>
						{/each}
					</div>
					<div class="scope-list">
						<div class="scope-head">{t('Lager')}</div>
						{#each lager as entry (entry.id)}
							<div class="scope-row">
								<input
									type="checkbox"
									checked={groupForm.lager.includes(entry.id)}
									onchange={() => toggleScope('lager', entry.id)}
								/>
								{entry.name ?? t('Lager {id}', { id: entry.id })}
							</div>
						{:else}
							<p class="small">{t('Kein Lager.')}</p>
						{/each}
					</div>
					<div class="scope-list">
						<div class="scope-head">{t('Client')}</div>
						<p class="small">{t('WHERE führt noch keine Clients.')}</p>
					</div>
				</div>
				<div class="rights">
					<div class="rights-left">
						<div class="set">
							<button
								type="button"
								class:active={rechtesatz === 'max'}
								title={t('Setzt alle Rechte aller Kategorien — Adressen, Interessenten, Kunden, Artikel … bis Admin')}
								onclick={setMaxRights}>{t('Max. Rechte')}</button
							>
							<button
								type="button"
								class:active={rechtesatz === 'min'}
								title={t('Wechselt auf die Min. Rechte; die Auswahl bleibt, wie sie ist')}
								onclick={() => (rechtesatz = 'min')}>{t('Min. Rechte')}</button
							>
						</div>
						<div class="tree">
							<button
								type="button"
								class="node leaf"
								class:active={sheet === 'uebersicht'}
								onclick={() => (sheet = 'uebersicht')}>{t('Übersicht')}</button
							>
							{#each tree as branch (branch.key)}
								<button type="button" class="node branch" onclick={() => toggleBranch(branch.key)}>
									<span class="arrow">{isOpen[branch.key] ? '▾' : '▸'}</span>
									{t(branch.label)}
								</button>
								{#if isOpen[branch.key]}
									{#each branch.areas as area (area.key)}
										<button
											type="button"
											class="node leaf indented"
											class:active={sheet === area.key}
											onclick={() => (sheet = area.key)}>{t(area.label)}</button
										>
									{/each}
								{/if}
							{/each}
						</div>
					</div>
					<div class="rights-right">
						<div class="rights-head">
							<span class="rights-title"
								>{sheet === 'uebersicht' ? t('Übersicht') : (activeSheet?.label ?? '')}</span
							>
							<button
								type="button"
								class="btn"
								title={activeSheet
									? t('Alle Rechte von {area} setzen', { area: activeSheet.label })
									: t('Alle Rechte des Katalogs setzen — einschließlich Admin')}
								onclick={() => allRights(true)}>{t('Alles aktivieren')}</button
							>
							<button
								type="button"
								class="btn"
								title={activeSheet
									? t('Alle Rechte von {area} entfernen', { area: activeSheet.label })
									: t('Alle Rechte des Katalogs entfernen')}
								onclick={() => allRights(false)}>{t('Alles deaktivieren')}</button
							>
						</div>
						{#if activeSheet}
							<div class="rights-list">
								{#each activeSheet.rights as right (right.key)}
									<label class="check">
										<input
											type="checkbox"
											checked={rightSetNow.includes(right.key)}
											onchange={() => toggleRight(right.key)}
										/>
										{t(right.label)}
									</label>
								{/each}
							</div>
							<p class="small">
								{t('{count} von {total} Rechten in {set} gesetzt.', {
									count: gewaehlt.length,
									total: activeSheet.rights.length,
									set: rechtesatz === 'max' ? t('Max. Rechte') : t('Min. Rechte')
								})}
							</p>
						{:else}
							<div class="rights-list">
								<table class="branches">
									<thead>
										<tr><th>{t('Zweig')}</th><th class="center">{t('Bereiche')}</th><th class="center">{t('Rechte')}</th></tr>
									</thead>
									<tbody>
										{#each tree as branch (branch.key)}
											<tr>
												<td>{t(branch.label)}</td>
												<td class="center">{branch.areas.length}</td>
												<td class="center">
													{branch.areas.reduce((summe, area) => summe + area.rights.length, 0)}
												</td>
											</tr>
										{/each}
									</tbody>
								</table>
								<p class="small">
									{t('{count} von {total} Rechten in {set} gesetzt.', {
										count: rightSetNow.length,
										total: alleRechtsschluessel.length,
										set: rechtesatz === 'max' ? t('Max. Rechte') : t('Min. Rechte')
									})}
								</p>
								<p class="small">
									{t('`Max. Rechte` setzt alle Rechte aller Kategorien; links einen Zweig öffnen, um einzelne Bereiche zu setzen.')}
								</p>
							</div>
						{/if}
						{#if sheet === 'personal'}
							<div class="departments">
								<div class="scope-head">{t('Abteilungen')}</div>
								<div class="department-rows">
									{#if selectedDepartments('departments').length === 0}
										<span class="department-row">{t('alle')}</span>
									{:else}
										{#each selectedDepartments('departments') as entry (entry.id)}
											<span class="department-row">
												{entry.name}
												<button
													type="button"
													class="drop"
													title={t('Abteilung entfernen')}
													onclick={() => removeDepartment('departments', entry.id)}>×</button
												>
											</span>
										{/each}
									{/if}
								</div>
								<div class="department-picker">
									<button
										type="button"
										class="btn"
										onclick={() =>
											(departmentChoice = departmentChoice === 'departments' ? null : 'departments')}
										>{t('+ Abteilung hinzufügen')}</button
									>
									{#if departmentChoice === 'departments'}
										<div class="choice" role="listbox" aria-label={t('Abteilungen')}>
											<button
												type="button"
												role="option"
												aria-selected="false"
												onclick={() => departmentAll('departments')}>{t('alle')}</button
											>
											{#each freeDepartments('departments') as entry (entry.id)}
												<button
													type="button"
													role="option"
													aria-selected="false"
													onclick={() => addDepartment('departments', entry.id)}
													>{entry.name}</button
												>
											{:else}
												<span class="empty-choice">{t('Keine weitere Abteilung')}</span>
											{/each}
										</div>
									{/if}
								</div>
							</div>
						{/if}
						{#if sheet === 'personaldispo'}
							<div class="departments">
								<div class="scope-head">{t('Abteilungen')}</div>
								<div class="department-rows">
									{#if selectedDepartments('dispo_departments').length === 0}
										<span class="department-row">{t('alle')}</span>
									{:else}
										{#each selectedDepartments('dispo_departments') as entry (entry.id)}
											<span class="department-row">
												{entry.name}
												<button
													type="button"
													class="drop"
													title={t('Abteilung entfernen')}
													onclick={() => removeDepartment('dispo_departments', entry.id)}>×</button
												>
											</span>
										{/each}
									{/if}
								</div>
								<div class="department-picker">
									<button
										type="button"
										class="btn"
										onclick={() =>
											(departmentChoice = departmentChoice === 'dispo_departments'
												? null
												: 'dispo_departments')}
										>{t('+ Abteilung hinzufügen')}</button
									>
									{#if departmentChoice === 'dispo_departments'}
										<div class="choice" role="listbox" aria-label={t('Abteilungen')}>
											<button
												type="button"
												role="option"
												aria-selected="false"
												onclick={() => departmentAll('dispo_departments')}>{t('alle')}</button
											>
											{#each freeDepartments('dispo_departments') as entry (entry.id)}
												<button
													type="button"
													role="option"
													aria-selected="false"
													onclick={() => addDepartment('dispo_departments', entry.id)}
													>{entry.name}</button
												>
											{:else}
												<span class="empty-choice">{t('Keine weitere Abteilung')}</span>
											{/each}
										</div>
									{/if}
								</div>
							</div>
						{/if}
					</div>
				</div>
			</div>
			<div class="sw-footer">
				<button type="button" class="sw-button" onclick={() => (groupForm = null)}>{t('Abbrechen')}</button>
				<button
					type="button"
					class="sw-button primary"
					disabled={!groupForm.name.trim()}
					onclick={saveGroup}>{t('Ok')}</button
				>
			</div>
		</div>
	</div>
{/if}

<style>
	.error {
		margin: 0 0 1rem;
		padding: 0.5rem 0.7rem;
		border: 1px solid var(--bad);
		border-radius: 4px;
		color: var(--bad);
		background: color-mix(in srgb, var(--bad) 8%, var(--surface));
	}
	.message {
		margin: 0.5rem 0 0;
		font-size: 0.8rem;
		color: var(--muted);
	}
	.head {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.5rem;
		max-width: 44rem;
	}
	.window {
		display: grid;
		grid-template-columns: minmax(16rem, 22rem) minmax(18rem, 24rem);
		gap: 0.9rem;
		align-items: start;
	}
	.column {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}
	.bar {
		display: flex;
		gap: 0.25rem;
	}
	.bar.right {
		justify-content: flex-end;
	}
	.square {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 1.9rem;
		height: 1.7rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.8rem;
		line-height: 1;
		cursor: pointer;
	}
	.square:disabled {
		color: var(--muted-2);
		cursor: default;
	}
	.square.red {
		color: var(--bad);
	}
	.list {
		min-height: 20rem;
		max-height: 24rem;
		overflow: auto;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
	}
	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.82rem;
	}
	thead th {
		position: sticky;
		top: 0;
		padding: 0.35rem 0.45rem;
		text-align: left;
		font-weight: 600;
		color: var(--accent);
		background: var(--surface-2);
		border-bottom: 1px solid var(--border);
	}
	tbody td {
		padding: 0.25rem 0.45rem;
		border-bottom: 1px solid var(--surface-3);
	}
	tbody tr {
		cursor: pointer;
	}
	tbody tr:nth-child(even) {
		background: var(--surface-2);
	}
	tbody tr:hover {
		background: var(--highlight);
	}
	tbody tr.marked {
		background: var(--highlight);
		outline: 1px solid var(--accent);
		outline-offset: -1px;
	}
	tbody tr.over {
		background: color-mix(in srgb, var(--good) 22%, var(--surface));
		outline: 1px dashed var(--good);
		outline-offset: -1px;
	}
	tbody tr.child td {
		padding-left: 1.6rem;
		color: var(--muted);
	}
	tbody tr.child.empty td {
		color: var(--muted-2);
		font-style: italic;
	}
	.branch-arrow {
		width: 1.1rem;
		margin-right: 0.1rem;
		padding: 0;
		border: none;
		background: none;
		color: var(--muted-2);
		font: inherit;
		font-size: 0.8rem;
		line-height: 1;
		cursor: pointer;
	}
	.branch-arrow.remove {
		width: 0.9rem;
		margin-left: 0.4rem;
		opacity: 0.5;
	}
	.branch-arrow.remove:hover {
		opacity: 1;
		color: var(--bad);
	}
	.arrow-place {
		display: inline-block;
		width: 1.1rem;
	}
	.warning {
		font-size: 0.78rem;
		color: var(--bad);
	}
	.error-field,
	.text.error-field {
		border-color: var(--bad);
	}
	.foot {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		max-width: 48rem;
		margin-top: 0.9rem;
		padding-top: 0.6rem;
		border-top: 1px solid var(--border);
	}
	.spacer {
		flex: 1;
	}
	.btn {
		padding: 0.3rem 0.7rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.82rem;
		cursor: pointer;
		text-decoration: none;
	}
	.btn:disabled {
		color: var(--muted-2);
		cursor: default;
	}
	.small {
		font-size: 0.78rem;
		color: var(--muted-2);
	}
	.tabs {
		display: flex;
		gap: 0.2rem;
		padding: 0 0.75rem;
		border-bottom: 1px solid var(--border);
		background: var(--surface-2);
	}
	.tabs button {
		padding: 0.3rem 0.7rem;
		border: 1px solid transparent;
		border-bottom: none;
		border-radius: 3px 3px 0 0;
		background: none;
		color: var(--muted);
		font: inherit;
		font-size: 0.82rem;
		cursor: pointer;
	}
	.tabs button.active {
		background: var(--surface);
		border-color: var(--border);
		color: var(--accent);
		font-weight: 600;
	}
	.row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 1.9rem;
	}
	.caption {
		width: 9.5rem;
		flex-shrink: 0;
		font-size: 0.8rem;
		color: var(--muted);
	}
	.text {
		flex: 1;
		min-width: 0;
		padding: 0.25rem 0.4rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
	}
	.text.short {
		flex: 0 0 5.5rem;
	}
	.numeric {
		width: 6rem;
		padding: 0.2rem 0.35rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
		text-align: right;
	}
	.unit {
		font-size: 0.76rem;
		color: var(--muted-2);
	}
	.block {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		margin: 0.5rem 0;
	}
	.check {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.82rem;
	}
	.check.off {
		color: var(--muted-2);
	}
	.check.title {
		font-weight: 600;
		color: var(--accent);
	}
	.check-pair {
		display: flex;
		gap: 1.5rem;
		margin-bottom: 0.4rem;
	}
	.sw-content h3 {
		margin: 0.7rem 0 0.3rem;
		font-size: 0.85rem;
		color: var(--accent);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		margin-bottom: 0.4rem;
		font-size: 0.8rem;
		color: var(--muted);
	}
	textarea.text {
		resize: vertical;
	}
	.grid-title {
		margin: 0.5rem 0 0.25rem;
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--accent);
	}
	.grid {
		border: 1px solid var(--border);
		border-radius: 4px;
		max-height: 12rem;
		overflow: auto;
	}
	.grid-head {
		display: grid;
		grid-template-columns: 4rem 1fr;
		gap: 0.4rem;
		padding: 0.25rem 0.45rem;
		background: var(--surface-2);
		border-bottom: 1px solid var(--border);
		font-size: 0.78rem;
		font-weight: 600;
		color: var(--muted);
	}
	.grid-row {
		display: grid;
		grid-template-columns: 4rem 1fr;
		gap: 0.4rem;
		align-items: center;
		padding: 0.2rem 0.45rem;
		font-size: 0.82rem;
	}
	.heading {
		margin: 0 0 0.5rem;
		padding: 0.2rem 0.4rem;
		background: var(--surface-3);
		font-size: 0.78rem;
		font-weight: 600;
		color: var(--muted);
		text-align: center;
	}
	.scope {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 0.4rem;
		margin: 0.4rem 0 0.6rem;
	}
	.scope-list {
		min-height: 6rem;
		max-height: 9rem;
		overflow: auto;
		border: 1px solid var(--border);
		border-radius: 3px;
	}
	.scope-head {
		position: sticky;
		top: 0;
		padding: 0.2rem 0.4rem;
		background: var(--surface-2);
		border-bottom: 1px solid var(--border);
		font-size: 0.78rem;
		color: var(--muted);
		text-align: center;
	}
	.scope-row {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.15rem 0.4rem;
		font-size: 0.8rem;
	}
	/* The departments of the group: they apply to personnel and personnel disposition. */
	.departments {
		display: grid;
		gap: 0.35rem;
		margin-top: 0.6rem;
		padding-top: 0.5rem;
		border-top: 1px solid var(--border);
	}
	.department-rows {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
	}
	.department-row {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.1rem 0.4rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		font-size: 0.8rem;
	}
	.department-row .drop {
		border: 0;
		background: transparent;
		color: var(--muted);
		font: inherit;
		line-height: 1;
		cursor: pointer;
	}
	.department-row .drop:hover {
		color: var(--bad);
	}
	.department-picker {
		position: relative;
		justify-self: start;
	}
	.choice {
		position: absolute;
		z-index: 5;
		right: 0;
		bottom: calc(100% + 0.2rem);
		min-width: 12rem;
		max-height: 12rem;
		overflow: auto;
		display: grid;
		padding: 0.2rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface);
		box-shadow: 0 6px 18px rgb(0 0 0 / 25%);
	}
	.choice button {
		padding: 0.25rem 0.4rem;
		border: 0;
		border-radius: 2px;
		background: transparent;
		color: var(--text);
		font: inherit;
		font-size: 0.82rem;
		text-align: left;
		cursor: pointer;
	}
	.choice button:hover {
		background: var(--surface-2);
	}
	.empty-choice {
		padding: 0.25rem 0.4rem;
		font-size: 0.8rem;
		color: var(--muted-2);
	}
	.rights {
		display: grid;
		grid-template-columns: 15rem minmax(0, 1fr);
		gap: 0.5rem;
		min-height: 22rem;
	}
	.rights-left {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}
	.set {
		display: flex;
		gap: 0.2rem;
	}
	.set button {
		flex: 1;
		padding: 0.25rem 0.4rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--muted);
		font: inherit;
		font-size: 0.78rem;
		cursor: pointer;
	}
	.set button.active {
		background: var(--surface);
		color: var(--accent);
		font-weight: 600;
	}
	.tree {
		display: flex;
		flex-direction: column;
		max-height: 20rem;
		overflow: auto;
		border: 1px solid var(--border);
		border-radius: 3px;
		padding: 0.2rem;
	}
	.node {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.15rem 0.3rem;
		border: none;
		border-radius: 2px;
		background: none;
		color: var(--text);
		font: inherit;
		font-size: 0.8rem;
		text-align: left;
		cursor: pointer;
	}
	.node.branch {
		font-weight: 600;
	}
	.node.indented {
		padding-left: 1.4rem;
	}
	.node.active {
		background: var(--highlight);
		color: var(--accent);
	}
	.arrow {
		width: 0.7rem;
		color: var(--muted-2);
	}
	.rights-right {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface);
	}
	.rights-head {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.3rem 0.4rem;
		border-bottom: 1px solid var(--border);
		background: var(--surface-2);
	}
	.rights-title {
		flex: 1;
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--accent);
	}
	.rights-list {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		padding: 0.4rem;
		overflow: auto;
		max-height: 18rem;
	}
	.branches {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.8rem;
	}
	.branches th {
		text-align: left;
		padding: 0.2rem 0.35rem;
		border-bottom: 1px solid var(--border);
		color: var(--muted);
		font-weight: 600;
	}
	.branches td {
		padding: 0.15rem 0.35rem;
		border-bottom: 1px solid var(--surface-3);
	}
	.branches .center {
		text-align: center;
		width: 5rem;
	}
</style>
