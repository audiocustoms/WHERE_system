<script lang="ts">
	import { onMount } from 'svelte';
	import animatedLogo from '../../../../logo/WHERE-animated.svg';
	import { errorText, type LoginTenant, type LoginUser } from './api';
	import { auth } from './auth.svelte';
	import { modules } from './modules.svelte';
	import { isDark, toggleTheme } from './theme.svelte';

	/**
	 * The last login: tenant and user are preselected on the next start of the
	 * application. `localStorage` and not `sessionStorage`, because the memory shall
	 * survive a closing of the window.
	 */
	const LAST_KEY = 'where.last-login';

	type LastLogin = { mandant: number; user_name: string };

	function lastLogin(): LastLogin | null {
		try {
			const roh = localStorage.getItem(LAST_KEY);
			if (!roh) return null;
			const value = JSON.parse(roh) as Partial<LastLogin>;
			if (typeof value.mandant !== 'number' || typeof value.user_name !== 'string') return null;
			return { mandant: value.mandant, user_name: value.user_name };
		} catch {
			return null;
		}
	}

	function remember(mandantId: number, userName: string) {
		try {
			localStorage.setItem(LAST_KEY, JSON.stringify({ mandant: mandantId, user_name: userName }));
		} catch {
			// Without localStorage the next start begins without a preselection.
		}
	}

	/** The tenants for the selection left next to `Benutzer`. */
	let tenants = $state<LoginTenant[]>([]);
	/** The chosen tenant — one is always chosen. */
	let mandant = $state<LoginTenant | null>(null);
	/** The accounts of the selection list — those of the chosen tenant. */
	let accounts = $state<LoginUser[]>([]);
	let choice = $state<LoginUser | null>(null);
	let username = $state('');
	let password = $state('');
	let error = $state<string | null>(null);
	let loading = $state(true);
	let passwordField = $state<HTMLInputElement | null>(null);

	onMount(async () => {
		try {
			tenants = await auth.tenants();
			if (tenants.length === 0) return;
			// Last signed-in tenant, otherwise the first of the list.
			const last = lastLogin();
			const start =
				tenants.find((entry) => entry.id === last?.mandant) ?? tenants[0];
			await mandantWaehlen(start, last?.user_name);
		} catch (e) {
			error = errorText(e);
		} finally {
			loading = false;
		}
	});

	/** Loads the accounts of the tenant; `vorwahl` marks the one signed in last. */
	async function loadAccounts(gewaehlt: LoginTenant, prefix?: string) {
		accounts = await auth.accounts(gewaehlt.id);
		password = '';
		error = null;
		const remembered = prefix
			? accounts.find((account) => account.user_name.toLowerCase() === prefix.toLowerCase())
			: undefined;
		// With exactly one account it is already chosen.
		if (remembered) choose(remembered);
		else if (accounts.length === 1) choose(accounts[0]);
		else {
			choice = null;
			username = '';
		}
	}

	/** Tenant switch: the user list follows the selection. */
	async function mandantWaehlen(gewaehlt: LoginTenant, prefix?: string) {
		mandant = gewaehlt;
		error = null;
		try {
			await loadAccounts(gewaehlt, prefix);
		} catch (e) {
			error = errorText(e);
		}
	}

	function choose(account: LoginUser) {
		choice = account;
		username = account.user_name;
		password = '';
		error = null;
		passwordField?.focus();
	}

	async function signIn(event: SubmitEvent) {
		event.preventDefault();
		error = null;
		if (!mandant) {
			error = 'Bitte zuerst einen Mandanten wählen.';
			return;
		}
		if (!username.trim()) {
			error = 'Bitte den Benutzernamen eingeben.';
			return;
		}
		if (!password) {
			error = 'Bitte das Kennwort eingeben.';
			return;
		}
		const ok = await auth.login(username, password, mandant.id);
		if (ok) remember(mandant.id, username.trim());
		else error = auth.error ?? 'Die Anmeldung ist fehlgeschlagen.';
	}
</script>

<div class="mask">
	<form class="card" onsubmit={signIn}>
		<header>
			<span class="mark">
				<img src={animatedLogo} alt="" class="logo" />
			</span>
			<div>
				<h1>Anmeldung</h1>
				<p class="subtitle">{modules.productName}</p>
			</div>
			<button
				type="button"
				class="theme-switch"
				role="switch"
				aria-checked={isDark()}
				title={isDark() ? 'Helles Design einschalten' : 'Dunkles Design einschalten'}
				onclick={toggleTheme}
			>
				<span class="track" class:on={isDark()}><span class="knob"></span></span>
				<span class="design-text">Design</span>
			</button>
		</header>

		<div class="content">
			<section class="tenant-list">
				<h2>Mandant</h2>
				{#if loading}
					<p class="small">Lade …</p>
				{:else}
					<ul>
						{#each tenants as entry (entry.id)}
							<li>
								<button
									type="button"
									class:active={mandant?.id === entry.id}
									title={`Benutzer von ${entry.name} anzeigen`}
									onclick={() => mandantWaehlen(entry)}
								>
									<span class="name">{entry.name}</span>
									{#if entry.short_name}<span class="short-name">[{entry.short_name}]</span>{/if}
								</button>
							</li>
						{:else}
							<li><p class="small">Kein Mandant angelegt.</p></li>
						{/each}
					</ul>
				{/if}
			</section>

			<section class="list">
				<h2>Benutzer</h2>
				{#if loading}
					<p class="small">Lade …</p>
				{:else}
					<ul>
						{#each accounts as account (account.id)}
							<li>
								<button
									type="button"
									class:active={choice?.id === account.id}
									disabled={!account.has_password}
									title={account.has_password
										? `Als ${account.user_name} anmelden`
										: 'Für dieses Konto ist kein Kennwort gesetzt'}
									onclick={() => choose(account)}
								>
									<span class="name">{account.display_name ?? account.user_name}</span>
									{#if account.short_name}<span class="short-name">[{account.short_name}]</span>{/if}
								</button>
							</li>
						{:else}
							<li>
								<p class="small">
									{mandant
										? `Kein Benutzer mit lokalem Login bei ${mandant.name}.`
										: 'Kein Benutzer mit lokalem Login angelegt.'}
								</p>
							</li>
						{/each}
					</ul>
				{/if}
			</section>

			<section class="form">
				<h2>Anmeldung</h2>
				<label class="field">
					Benutzername
					<!-- svelte-ignore a11y_autofocus -->
					<input
						class="text"
						autocomplete="username"
						autofocus={!choice}
						bind:value={username}
						oninput={() => (choice = null)}
					/>
				</label>
				<label class="field">
					Kennwort
					<input
						class="text"
						type="password"
						autocomplete="current-password"
						bind:this={passwordField}
						bind:value={password}
					/>
				</label>
				{#if error}
					<p class="error">{error}</p>
				{/if}
				<button type="submit" class="sign-in" disabled={auth.load}>
					{auth.load ? 'Melde an …' : 'Anmelden'}
				</button>
				{#if choice && !choice.has_password}
					<p class="small">
						Für „{choice.display_name ?? choice.user_name}“ ist noch kein Kennwort gesetzt —
						es wird in der Benutzerverwaltung vergeben.
					</p>
				{/if}
			</section>
		</div>

		<footer>
			<span class="small">
				Beim Schließen der Anwendung wird die Sitzung beendet; ein neuer Start beginnt hier.
			</span>
		</footer>
	</form>
</div>

<style>
	.mask {
		position: fixed;
		inset: 0;
		display: grid;
		/* `safe center`: if the card does not fit the height (phone, small window),
		   it is not cut off at the top; the mask scrolls instead. */
		place-items: safe center;
		overflow: auto;
		padding: 1.5rem;
		box-sizing: border-box;
		background:
			radial-gradient(circle at 20% 10%, color-mix(in srgb, var(--accent) 12%, transparent), transparent 55%),
			var(--bg);
	}
	.card {
		width: min(60rem, 100%);
		border: 1px solid var(--border);
		border-radius: 8px;
		background: var(--surface);
		box-shadow: 0 18px 50px rgb(0 0 0 / 18%);
		overflow: hidden;
	}
	header {
		display: flex;
		align-items: center;
		gap: 1.1rem;
		padding: 1.5rem 1.5rem 1.3rem;
		border-bottom: 1px solid var(--border);
		background:
			radial-gradient(
				circle at 12% 25%,
				color-mix(in srgb, var(--accent) 14%, transparent),
				transparent 62%
			),
			var(--surface-2);
	}
	/* Frame around the animated logo: it brings its own white background
	   and is only cropped, rounded and slightly set off here. */
	.mark {
		display: grid;
		place-items: center;
		flex: none;
		width: 7rem;
		height: 7rem;
		border-radius: 1.15rem;
		overflow: hidden;
		box-shadow:
			0 0 0 1px color-mix(in srgb, var(--accent) 26%, var(--border)),
			0 8px 22px rgb(15 23 42 / 20%);
	}
	.logo {
		width: 7rem;
		height: 7rem;
		display: block;
	}
	/* The logo is drawn black on white — in the dark design it stands
	   inverted, thus white on black. */
	:global([data-theme='dark']) .logo {
		filter: invert(1) drop-shadow(0 0 14px rgb(255 255 255 / 22%));
	}
	h1 {
		margin: 0;
		font-size: 1.7rem;
		letter-spacing: -0.01em;
	}
	.subtitle {
		margin: 0.25rem 0 0;
		font-size: 0.95rem;
		color: var(--muted);
	}
	/* Design switch as in the header of the application. */
	.theme-switch {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		margin-left: auto;
		align-self: flex-start;
		padding: 0 0.15rem;
		border: 0;
		background: transparent;
		font: inherit;
		font-size: 0.78rem;
		color: var(--muted);
		cursor: pointer;
	}
	.track {
		position: relative;
		display: block;
		width: 1.85rem;
		height: 1rem;
		border-radius: 1rem;
		background: var(--border);
		box-shadow: inset 0 1px 2px rgb(15 23 42 / 22%);
		transition: background 0.15s ease;
	}
	.track.on {
		background: var(--accent);
	}
	.knob {
		position: absolute;
		top: 0.13rem;
		left: 0.13rem;
		width: 0.74rem;
		height: 0.74rem;
		border-radius: 50%;
		background: #fff;
		box-shadow: 0 1px 2px rgb(0 0 0 / 35%);
		transition: transform 0.15s ease;
	}
	.track.on .knob {
		transform: translateX(0.85rem);
	}
	.subtitle {
		margin: 0.1rem 0 0;
		font-size: 0.8rem;
		color: var(--muted);
	}
	.content {
		display: grid;
		grid-template-columns:
			minmax(10.5rem, 0.8fr) minmax(11rem, 0.95fr)
			minmax(16rem, 1.15fr);
		gap: 0;
	}
	.list,
	.tenant-list {
		padding: 0.9rem 1rem;
		border-right: 1px solid var(--border);
	}
	h2 {
		margin: 0 0 0.5rem;
		font-size: 0.85rem;
		color: var(--accent);
	}
	/* Both lists have the same height; more entries than space scroll inside the list
	   (own scrollbar), so the mask does not wander. */
	ul {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		margin: 0;
		padding: 0 0.15rem 0 0;
		list-style: none;
		height: 12rem;
		overflow-y: auto;
		scrollbar-width: thin;
		scrollbar-color: var(--border) transparent;
	}
	ul button {
		display: flex;
		align-items: baseline;
		gap: 0.45rem;
		width: 100%;
		padding: 0.35rem 0.45rem;
		border: 1px solid transparent;
		border-radius: 4px;
		background: none;
		color: var(--text);
		font: inherit;
		font-size: 0.85rem;
		text-align: left;
		cursor: pointer;
	}
	ul button:hover:not(:disabled) {
		background: var(--highlight);
	}
	ul button.active {
		border-color: var(--accent);
		background: var(--highlight);
	}
	ul button:disabled {
		color: var(--muted-2);
		cursor: default;
	}
	.short-name {
		color: var(--muted-2);
		font-size: 0.8rem;
	}
	.form {
		display: flex;
		flex-direction: column;
		gap: 0.55rem;
		padding: 0.9rem 1.1rem 1rem;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.8rem;
		color: var(--muted);
	}
	.text {
		padding: 0.4rem 0.5rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.9rem;
	}
	.error {
		margin: 0;
		padding: 0.35rem 0.5rem;
		border: 1px solid var(--bad);
		border-radius: 4px;
		color: var(--bad);
		background: color-mix(in srgb, var(--bad) 8%, var(--surface));
		font-size: 0.8rem;
	}
	.sign-in {
		padding: 0.45rem 0.8rem;
		border: 1px solid var(--accent);
		border-radius: 4px;
		background: var(--accent);
		color: #fff;
		font: inherit;
		font-size: 0.88rem;
		font-weight: 600;
		cursor: pointer;
	}
	.sign-in:disabled {
		opacity: 0.7;
		cursor: default;
	}
	footer {
		padding: 0.6rem 1.2rem;
		border-top: 1px solid var(--border);
		background: var(--surface-2);
	}
	.small {
		font-size: 0.75rem;
		color: var(--muted-2);
	}
	@media (max-width: 40rem) {
		header {
			flex-wrap: wrap;
			gap: 0.8rem;
			padding: 1.1rem 1.2rem 1rem;
		}
		.mark {
			width: 5rem;
			height: 5rem;
			border-radius: 0.85rem;
		}
		.logo {
			width: 5rem;
			height: 5rem;
		}
		h1 {
			font-size: 1.35rem;
		}
		.subtitle {
			font-size: 0.85rem;
		}
		.content {
			grid-template-columns: minmax(0, 1fr);
		}
		.list,
		.tenant-list {
			border-right: none;
			border-bottom: 1px solid var(--border);
		}
		ul {
			height: 8rem;
		}
	}
</style>
