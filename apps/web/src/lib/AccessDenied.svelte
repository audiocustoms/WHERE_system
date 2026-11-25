<script lang="ts">
	/**
	 * Panel for a mask whose right is missing: it does not open at all, it names the
	 * missing right and offers the way back.
	 */
	import { goto } from '$app/navigation';

	interface Props {
		/** What the mask would be, e.g. `Artikelliste`. */
		pageName: string;
		/** The key of the missing right, e.g. `artikel.artikel-lesen`. */
		right: string;
		/** The right in words, e.g. `Artikel lesen`. */
		rightLabel: string;
	}

	let { pageName, right, rightLabel }: Props = $props();
</script>

<section class="denied">
	<h1>Kein Zugriff</h1>
	<p>
		Für die Maske <strong>{pageName}</strong> fehlt das Recht
		<strong>{rightLabel}</strong> (<code>{right}</code>). Es wird in der Benutzerverwaltung
		über eine Berechtigungsgruppe vergeben.
	</p>
	<button type="button" onclick={() => goto('/')}>Zur Zentrale</button>
</section>

<style>
	.denied {
		max-width: 34rem;
		margin: 3rem auto;
		padding: 1.4rem 1.6rem;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--surface);
	}
	h1 {
		margin: 0 0 0.6rem;
		font-size: 1.1rem;
	}
	p {
		margin: 0 0 1rem;
		font-size: 0.88rem;
		color: var(--muted);
		line-height: 1.5;
	}
	code {
		font-size: 0.8rem;
		color: var(--muted-2);
	}
	button {
		padding: 0.35rem 0.75rem;
		border: 1px solid var(--accent);
		border-radius: 4px;
		background: var(--accent);
		color: #fff;
		font: inherit;
		font-size: 0.85rem;
		cursor: pointer;
	}
</style>
