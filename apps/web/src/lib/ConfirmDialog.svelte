<script lang="ts">
	import { answerConfirm, confirmRequest } from './confirm.svelte';
	import { escapeWindows } from './escape.svelte';
	import './search/search-window.css';

	const request = $derived(confirmRequest());

	/** Focus is on „Weiter bearbeiten", so Return does not abort anything. */
	let cancelButton = $state<HTMLButtonElement | null>(null);
	$effect(() => {
		if (request) cancelButton?.focus();
	});

	/**
	 * `Esc` answers an open confirmation with „nein" — the same effect as the button
	 * carrying `request.cancelLabel`. It is registered only while the confirmation is
	 * open, so it does not displace the window underneath.
	 */
	$effect(() => {
		if (!request) return;
		return escapeWindows(() => answerConfirm(false));
	});
</script>

{#if request}
	<div class="sw-backdrop confirmation" role="presentation">
		<div
			class="sw-window"
			role="alertdialog"
			aria-modal="true"
			aria-label={request.title}
			style="width: min(26rem, 100%)"
		>
			<div class="sw-titlebar">
				<span class="sw-icon" aria-hidden="true"></span>
				<span class="sw-title">{request.title}</span>
			</div>

			<div class="sw-content">
				<p class="text">{request.message}</p>
			</div>

			<div class="sw-footer">
				<button
					class="sw-button"
					type="button"
					bind:this={cancelButton}
					onclick={() => answerConfirm(false)}
				>
					{request.cancelLabel}
				</button>
				<button class="sw-button sw-primary" type="button" onclick={() => answerConfirm(true)}>
					{request.confirmLabel}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	/* Above every window, also above a second one (e.g. the number dialog). */
	.confirmation {
		z-index: 200;
	}
	.text {
		margin: 0;
		font-size: 0.85rem;
		line-height: 1.4;
	}
</style>
