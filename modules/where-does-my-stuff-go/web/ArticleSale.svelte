<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { api, type ArticlePricing, type ScaleCategory, type ScaleStep } from '$core/api';

	interface Props {
		pricing: ArticlePricing;
	}

	let { pricing = $bindable() }: Props = $props();

	let scales = $state<ScaleCategory[]>([]);
	let steps = $state<ScaleStep[]>([]);
	let stepsOf = $state<number | null>(null);

	$effect(() => {
		api.masterData
			.scales(true)
			.then((rows) => (scales = rows))
			.catch(() => (scales = []));
	});

	const scaleId = $derived(pricing.quantity_scale);

	$effect(() => {
		const id = scaleId;
		if (id === null) {
			steps = [];
			stepsOf = null;
			return;
		}
		if (id === stepsOf) return;
		api.masterData
			.scaleSteps(id)
			.then((rows) => {
				steps = rows;
				stepsOf = id;
			})
			.catch(() => {
				steps = [];
				stepsOf = id;
			});
	});

	function setScale(value: string) {
		pricing.quantity_scale = value ? Number(value) : null;
		steps = [];
		stepsOf = null;
	}
</script>

<div class="columns">
	<section class="left">
		<div class="prices">
			<label class="field">
				<span>{t('VKPreis netto')}</span>
				<input bind:value={pricing.sale_price_net} inputmode="decimal" placeholder="0,00" />
			</label>
			<label class="field">
				<span>{t('VKPreis brutto')}</span>
				<input bind:value={pricing.sale_price_gross} inputmode="decimal" placeholder="0,00" />
			</label>
			<label class="field">
				<span>{t('UVP brutto')}</span>
				<input bind:value={pricing.sale_retail_price} inputmode="decimal" placeholder="0,00" />
			</label>
			<label class="field">
				<span>{t('EKPreis netto')}</span>
				<input bind:value={pricing.sale_cost_price_net} inputmode="decimal" placeholder="0,00" />
			</label>
			<label class="field">
				<span>{t('MwSt (%)')}</span>
				<input type="number" min="0" max="100" bind:value={pricing.sale_vat_rate} placeholder="19" />
			</label>
			<label class="field">
				<span>{t('EK → VK Aufschlag (%)')}</span>
				<input bind:value={pricing.sale_markup_percent} inputmode="decimal" placeholder="0,00" />
			</label>
		</div>

		<div class="scale">
			<label class="field">
				<span>{t('Staffel')}</span>
				<select value={scaleId ?? ''} onchange={(e) => setScale(e.currentTarget.value)}>
					<option value="">{t('— keine Staffel —')}</option>
					{#each scales as s (s.id)}
						<option value={s.id}>{s.name ?? t('Staffel {n}', { n: s.id })}</option>
					{/each}
				</select>
			</label>
			<table>
				<thead>
					<tr><th>{t('ab Menge')}</th><th>{t('Staffelrabatt')}</th></tr>
				</thead>
				<tbody>
					{#each steps as s, i (i)}
						<tr><td>{s.from_value}</td><td>{s.discount}</td></tr>
					{:else}
						<tr><td colspan="2" class="empty">{t('Keine Stufen hinterlegt.')}</td></tr>
					{/each}
				</tbody>
			</table>
			<p class="note">{t('Staffeln werden zentral gepflegt, nicht je Artikel.')}</p>
		</div>
	</section>

	<section class="right">
		<fieldset>
			<legend>{t('Verkaufseigenschaften')}</legend>
			<label class="check">
				<input type="checkbox" bind:checked={pricing.sale_no_discount} />
				{t('Nicht rabattierbar')}
			</label>
			<label class="check">
				<input type="checkbox" bind:checked={pricing.sell_full_packages} />
				{t('Nur Vielfaches der Kolligröße verkaufen')}
			</label>
		</fieldset>

		<fieldset>
			<legend>{t('Kolliverkauf')}</legend>
			<label class="field row">
				<span>{t('Kolli-Einheit')}</span>
				<input bind:value={pricing.package_unit} />
			</label>
			<label class="field row">
				<span>{t('Kolligröße')}</span>
				<input bind:value={pricing.package_size} inputmode="decimal" placeholder="0" />
			</label>
			<label class="field row">
				<span>{t('Meldebestand')}</span>
				<input bind:value={pricing.reorder_level} inputmode="decimal" placeholder="0" />
			</label>
		</fieldset>

		<fieldset>
			<legend>{t('FiBu-Kontierung')}</legend>
			<label class="field row">
				<span>{t('Ertragskonto')}</span>
				<input bind:value={pricing.sale_revenue_account} />
			</label>
			<label class="field row">
				<span>{t('Kostenstelle')}</span>
				<input bind:value={pricing.sale_cost_center} />
			</label>
		</fieldset>

		<fieldset>
			<legend>{t('Weitere Informationen')}</legend>
			<p class="note">
				{t('Festes Material, Alternativen und Preisklassen folgen mit der Verleih-Domäne.')}
			</p>
		</fieldset>
	</section>
</div>

<style>
	.columns {
		display: grid;
		grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
		gap: 1.25rem;
		padding: 1rem;
	}
	@media (max-width: 900px) {
		.columns {
			grid-template-columns: 1fr;
		}
	}
	.prices {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.5rem 0.75rem;
		max-width: 32rem;
	}
	.scale {
		margin-top: 0.75rem;
		max-width: 32rem;
	}
	.scale table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.8rem;
	}
	.scale th,
	.scale td {
		border: 1px solid var(--border);
		padding: 0.2rem 0.4rem;
		text-align: left;
	}
	.scale th {
		background: var(--surface-2);
		font-weight: 600;
	}
	.scale td.empty {
		color: var(--muted);
		text-align: center;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		font-size: 0.78rem;
		margin-bottom: 0.5rem;
	}
	.field span {
		color: var(--muted);
	}
	.field.row {
		display: grid;
		grid-template-columns: 11rem minmax(0, 1fr);
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.35rem;
	}
	input,
	select {
		padding: 0.3rem 0.45rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		font-family: inherit;
		font-size: 0.85rem;
		width: 100%;
		box-sizing: border-box;
	}
	fieldset {
		border: 1px solid var(--surface-3);
		border-radius: 4px;
		padding: 0.5rem 0.6rem 0.7rem;
		margin: 0 0 0.75rem;
	}
	legend {
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--accent);
		padding: 0 0.3rem;
	}
	.check {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.8rem;
		margin: 0.2rem 0;
	}
	.check input {
		width: auto;
	}
	.note {
		font-size: 0.78rem;
		color: var(--muted);
	}
</style>
