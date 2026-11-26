<script lang="ts">
	import { t } from '$core/i18n.svelte';

	/**
	 * Password change at the first sign-in (`ChangeNextLogin`).
	 *
	 * The initial installation ships the administrator with a start password; until
	 * he has set his own, the application does not let him work. The dialog therefore
	 * cannot be closed — only changed or signed out.
	 */
	import { onMount } from 'svelte';
	import { auth } from './auth.svelte';

	let newValue = $state('');
	let repetition = $state('');
	let error = $state<string | null>(null);
	let sent = $state(false);
	let firstField = $state<HTMLInputElement | null>(null);

	onMount(() => firstField?.focus());

	const tooShort = $derived(newValue.length > 0 && newValue.length < 8);
	const doesNotMatch = $derived(repetition.length > 0 && newValue !== repetition);
	const ready = $derived(newValue.length >= 8 && newValue === repetition && !auth.load);

	async function save(event: SubmitEvent) {
		event.preventDefault();
		error = null;
		if (newValue.length < 8) {
			error = t('Das neue Kennwort muss mindestens 8 Zeichen haben.');
			return;
		}
		if (newValue !== repetition) {
			error = t('Die Kennwörter stimmen nicht überein.');
			return;
		}
		sent = true;
		const ok = await auth.changePassword(newValue, repetition);
		if (!ok) error = auth.error ?? t('Das Kennwort konnte nicht geändert werden.');
		sent = false;
	}

	async function signOut() {
		await auth.logout();
	}
</script>

<div class="mask" role="dialog" aria-modal="true" aria-labelledby="kennwort-titel">
	<form class="card" onsubmit={save}>
		<header>
			<h1 id="kennwort-titel">{t('Kennwort ändern')}</h1>
			<p class="subtitle">
				{t('Das Konto')} <strong>{auth.label}</strong> {t('arbeitet noch mit dem Kennwort der Erstinstallation. Bitte jetzt ein eigenes Kennwort setzen — ohne diese Änderung geht es nicht weiter.')}
			</p>
		</header>

		<div class="content">
			<label class="field">
				{t('Neues Kennwort')}
				<input
					class="text"
					type="password"
					autocomplete="new-password"
					bind:this={firstField}
					bind:value={newValue}
					oninput={() => (error = null)}
				/>
			</label>
			{#if tooShort}
				<p class="hint">{t('Mindestens 8 Zeichen.')}</p>
			{/if}

			<label class="field">
				{t('Kennwort wiederholen')}
				<input
					class="text"
					type="password"
					autocomplete="new-password"
					bind:value={repetition}
					disabled={newValue.length === 0}
					oninput={() => (error = null)}
				/>
			</label>
			{#if doesNotMatch}
				<p class="hint">{t('Die Kennwörter stimmen nicht überein.')}</p>
			{/if}

			{#if error}
				<p class="error">{error}</p>
			{/if}

			<div class="buttons">
				<button type="submit" class="save" disabled={!ready}>
					{sent ? t('Speichere …') : t('Kennwort ändern')}
				</button>
				<button type="button" class="sign-out" onclick={signOut} disabled={auth.load}>
					{t('Abmelden')}
				</button>
			</div>
		</div>
	</form>
</div>

<style>
	.mask {
		position: fixed;
		inset: 0;
		z-index: 60;
		display: grid;
		place-items: center;
		padding: 1.5rem;
		box-sizing: border-box;
		/* The application stays visible, but is locked and dimmed. */
		background: rgb(15 23 42 / 42%);
		backdrop-filter: blur(2px);
	}
	.card {
		width: min(26rem, 100%);
		border: 1px solid var(--border);
		border-radius: 8px;
		background: var(--surface);
		box-shadow: 0 18px 50px rgb(0 0 0 / 28%);
		overflow: hidden;
	}
	header {
		padding: 1rem 1.2rem;
		border-bottom: 1px solid var(--border);
		background: var(--surface-2);
	}
	h1 {
		margin: 0 0 0.35rem;
		font-size: 1.1rem;
	}
	.subtitle {
		margin: 0;
		font-size: 0.8rem;
		line-height: 1.5;
		color: var(--muted);
	}
	.content {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		padding: 1rem 1.2rem 1.1rem;
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
	.text:disabled {
		opacity: 0.55;
	}
	.hint {
		margin: -0.35rem 0 0;
		font-size: 0.76rem;
		color: var(--bad);
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
	.buttons {
		display: flex;
		gap: 0.5rem;
		margin-top: 0.3rem;
	}
	.save {
		flex: 1;
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
	.save:disabled {
		opacity: 0.55;
		cursor: default;
	}
	.sign-out {
		padding: 0.45rem 0.8rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.85rem;
		cursor: pointer;
	}
</style>
