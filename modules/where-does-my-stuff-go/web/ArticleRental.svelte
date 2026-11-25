<script lang="ts">
	import { api, type ArticlePricing, type ScaleCategory, type ScaleStep } from '$core/api';

	interface Props {
		pricing: ArticlePricing;
	}

	let { pricing = $bindable() }: Props = $props();

	type Variant = 'base' | 'roll_days' | 'off_days' | 'weekend';
	/** Keys of the pricing fields that hold a money value. */
	type PriceKey = {
		[K in keyof ArticlePricing]: ArticlePricing[K] extends string | null ? K : never;
	}[keyof ArticlePricing];

	let variant = $state<Variant>('base');

	let scales = $state<ScaleCategory[]>([]);
	let steps = $state<ScaleStep[]>([]);
	let stepsOf = $state<number | null>(null);

	const variants: { key: Variant; label: string; flag: keyof ArticlePricing | null }[] = [
		{ key: 'base', label: 'Einsatztage', flag: null },
		{ key: 'roll_days', label: 'Rolltage', flag: 'rental_roll_days_active' },
		{ key: 'off_days', label: '"off Days"', flag: 'rental_off_days_active' },
		{ key: 'weekend', label: 'Wochenende', flag: 'rental_weekend_active' }
	];

	/** Price row per usage-day kind — the weekend price has neither base price nor scale. */
	const priceRows: Record<
		Variant,
		{ base: PriceKey | null; price: PriceKey; ek: PriceKey; scale: keyof ArticlePricing | null }
	> = {
		base: {
			base: 'rental_base_price',
			price: 'rental_price_per_day',
			ek: 'rental_cost_price',
			scale: 'factor_scale'
		},
		roll_days: {
			base: 'rental_base_price_roll_days',
			price: 'rental_price_per_day_roll_days',
			ek: 'rental_cost_price_roll_days',
			scale: 'factor_scale_roll_days'
		},
		off_days: {
			base: 'rental_base_price_off_days',
			price: 'rental_price_per_day_off_days',
			ek: 'rental_cost_price_off_days',
			scale: 'factor_scale_off_days'
		},
		weekend: {
			base: null,
			price: 'rental_price_weekend',
			ek: 'rental_cost_price_weekend',
			scale: null
		}
	};

	const current = $derived(priceRows[variant]);
	const money = pricing as Record<PriceKey, string | null>;

	function moneyValue(key: PriceKey | null): string {
		return key ? (money[key] ?? '') : '';
	}

	function setMoney(key: PriceKey, event: Event) {
		const text = (event.currentTarget as HTMLInputElement).value.trim();
		money[key] = text === '' ? null : text;
	}

	const activeFlag = $derived(variants.find((v) => v.key === variant)?.flag ?? null);
	const active = $derived(
		activeFlag === null ? true : Boolean((pricing as unknown as Record<string, unknown>)[activeFlag])
	);

	function setActive(next: boolean) {
		const flag = activeFlag;
		if (flag) Object.assign(pricing, { [flag]: next });
	}

	const scaleId = $derived(
		current.scale ? (pricing[current.scale] as number | null) : null
	);

	function setScale(value: string) {
		const id = value ? Number(value) : null;
		if (current.scale) Object.assign(pricing, { [current.scale]: id });
		steps = [];
		stepsOf = null;
	}

	$effect(() => {
		api.masterData
			.scales(false)
			.then((rows) => (scales = rows))
			.catch(() => (scales = []));
	});

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
</script>

<div class="columns">
	<section class="left">
		<nav class="tabs small" aria-label="Einsatztage">
			{#each variants as v (v.key)}
				<button
					type="button"
					class="tab"
					class:active={variant === v.key}
					onclick={() => (variant = v.key)}
				>
					{v.label}
				</button>
			{/each}
		</nav>

		<div class="head">
			<label class="check">
				<input
					type="checkbox"
					checked={activeFlag === null ? true : active}
					disabled={activeFlag === null}
					onchange={(e) => setActive(e.currentTarget.checked)}
				/>
				Berechnung active
			</label>
			{#if activeFlag === null}
				<span class="note">Der Grundpreis gilt immer.</span>
			{/if}
		</div>

		<div class="prices">
			<div class="field">
				<span>Basispreis</span>
				<input
					value={moneyValue(current.base)}
					disabled={current.base === null}
					inputmode="decimal"
					placeholder={current.base === null ? '—' : '0,00'}
					oninput={(e) => current.base && setMoney(current.base, e)}
				/>
			</div>
			<div class="field">
				<span>{variant === 'weekend' ? 'Pauschalpreis' : 'Mietpreis / Tag'}</span>
				<input
					value={moneyValue(current.price)}
					inputmode="decimal"
					placeholder="0,00"
					oninput={(e) => setMoney(current.price, e)}
				/>
			</div>
			<div class="field">
				<span>Kalk. EKPreis / T</span>
				<input
					value={moneyValue(current.ek)}
					inputmode="decimal"
					placeholder="0,00"
					oninput={(e) => setMoney(current.ek, e)}
				/>
			</div>
			<label class="field">
				<span>MwSt (%)</span>
				<input type="number" min="0" max="100" bind:value={pricing.rental_vat_rate} placeholder="19" />
			</label>
		</div>

		<div class="pair">
			<label class="field">
				<span>Kaution</span>
				<input bind:value={pricing.deposit} inputmode="decimal" placeholder="0,00" />
			</label>
			<label class="field">
				<span>Gebrauchtpreis</span>
				<input bind:value={pricing.used_sale_price} inputmode="decimal" placeholder="0,00" />
			</label>
		</div>

		{#if current.scale}
			<div class="scale">
				<label class="field">
					<span>Staffel</span>
					<select value={scaleId ?? ''} onchange={(e) => setScale(e.currentTarget.value)}>
						<option value="">— keine Staffel —</option>
						{#each scales as s (s.id)}
							<option value={s.id}>{s.name ?? `Staffel ${s.id}`}</option>
						{/each}
					</select>
				</label>
				<table>
					<thead>
						<tr><th>Tage</th><th>Berechnungsfaktor</th></tr>
					</thead>
					<tbody>
						{#each steps as s, i (i)}
							<tr><td>{s.from_value}</td><td>{s.factor}</td></tr>
						{:else}
							<tr><td colspan="2" class="empty">Keine Stufen hinterlegt.</td></tr>
						{/each}
					</tbody>
				</table>
				<p class="note">Staffeln werden zentral gepflegt, nicht je Artikel.</p>
			</div>
		{:else}
			<p class="note">Für den Wochenendpreis gilt keine Staffel.</p>
		{/if}
	</section>

	<section class="right">
		<fieldset>
			<legend>Zusätzliche Eigenschaften</legend>
			<label class="check">
				<input type="checkbox" bind:checked={pricing.packaging_release} />
				Freigabe für Verpackungseinheiten
			</label>
			<label class="check">
				<input type="checkbox" bind:checked={pricing.rental_no_discount} />
				Nicht rabattierbar
			</label>
			<label class="check">
				<input type="checkbox" bind:checked={pricing.rental_not_insurable} />
				Nicht versicherbar
			</label>
			<label class="check">
				<input type="checkbox" bind:checked={pricing.material_only} />
				Nur als Material einsetzbar
			</label>
			<label class="check">
				<input type="checkbox" bind:checked={pricing.unique_article} />
				Unikatartikel (Verweigerung bei überbuchter Anlage)
			</label>
		</fieldset>

		<fieldset>
			<legend>CheckOut/In-Eigenschaften</legend>
			<label class="check">
				<input type="checkbox" bind:checked={pricing.checkout_requires_serial} />
				CheckOut/In nur mit SN (Ident-Nr.)
			</label>
			<label class="check">
				<input type="checkbox" bind:checked={pricing.fixed_installation} />
				Artikel ist ein nicht zerlegbares Bundle
			</label>
			<label class="field row">
				<span>Annähender Lagerplatz</span>
				<input bind:value={pricing.approximate_location} />
			</label>
		</fieldset>

		<fieldset>
			<legend>Weitere Stationen bei CheckIn</legend>
			<label class="check">
				<input type="checkbox" bind:checked={pricing.inspection_on_checkin} />
				Sichtprüfung
			</label>
			<label class="check">
				<input type="checkbox" bind:checked={pricing.function_check_on_checkin} />
				Funktionsprüfung
			</label>
			<label class="check">
				<input type="checkbox" bind:checked={pricing.consumption_recording} />
				Verbrauchserfassung
			</label>
		</fieldset>

		<fieldset>
			<legend>FiBu-Kontierung</legend>
			<label class="field row">
				<span>Ertragskonto</span>
				<input bind:value={pricing.rental_revenue_account} />
			</label>
			<label class="field row">
				<span>Kostenstelle</span>
				<input bind:value={pricing.rental_cost_center} />
			</label>
		</fieldset>

		<fieldset>
			<legend>Weitere Informationen</legend>
			<label class="field row">
				<span>Kapitalbindung netto</span>
				<input bind:value={pricing.capital_binding} inputmode="decimal" placeholder="0,00" />
			</label>
			<p class="note">
				Festes Material, Alternativen und Preisklassen folgen mit der Verleih-Domäne.
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
	.tabs {
		display: flex;
		gap: 0.25rem;
		padding: 0 0 0.25rem;
		border-bottom: 1px solid var(--surface-3);
		margin-bottom: 0.6rem;
		flex-wrap: wrap;
	}
	.tab {
		padding: 0.3rem 0.7rem;
		font-size: 0.82rem;
		border: 1px solid transparent;
		border-bottom: none;
		border-radius: 4px 4px 0 0;
		background: transparent;
		color: var(--muted);
		cursor: pointer;
		font-family: inherit;
	}
	.tab.active {
		background: var(--surface);
		border-color: var(--border);
		color: var(--text);
		font-weight: 600;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		margin-bottom: 0.5rem;
	}
	.prices,
	.pair {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.5rem 0.75rem;
		max-width: 30rem;
	}
	.scale {
		margin-top: 0.5rem;
		max-width: 30rem;
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
