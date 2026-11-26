<script lang="ts">
	import { statusMessage } from './statusMessage.svelte';

	const message = $derived(statusMessage());
</script>

{#if message}
	<div class="toast" role="status" aria-live="polite">{message}</div>
{/if}

<style>
	.toast {
		position: fixed;
		left: 50%;
		bottom: 2.2rem;
		transform: translateX(-50%);
		z-index: 300;
		max-width: min(34rem, 90vw);
		padding: 0.5rem 0.9rem;
		border: 1px solid var(--border);
		border-radius: 5px;
		background: var(--surface);
		color: var(--text);
		font-size: 0.84rem;
		box-shadow: 0 6px 20px rgb(0 0 0 / 22%);
	}
</style>
