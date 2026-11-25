<script lang="ts">
	interface Props {
		/** Year of the choice; `null` = all years. */
		year?: number | null;
		/** Quarter 1–4; `null` = all quarters. */
		quarter?: number | null;
		/** Month 1–12; `null` = all months. */
		month?: number | null;
		/** Label of the date basis; without it the row is left out. */
		bases?: { value: string; text: string }[];
		base?: string;
	}

	let {
		year = $bindable(null),
		quarter = $bindable(null),
		month = $bindable(null),
		bases,
		base = $bindable('')
	}: Props = $props();

	const TODAY = new Date();
	const YEAR = TODAY.getFullYear();
	const YEARS = [YEAR, YEAR - 1, YEAR - 2, YEAR - 3];
	const QUARTERS = [1, 2, 3, 4];
	const MONTHS = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

	year ??= YEAR;
	month ??= TODAY.getMonth() + 1;

	function chooseQuarter(value: number | null) {
		quarter = quarter === value ? null : value;
	}

	function chooseMonth(value: number | null) {
		month = month === value ? null : value;
	}
</script>

{#if bases}
	<div class="sw-row">
		<span class="sw-label">Grundlage</span>
		<select bind:value={base} aria-label="Grundlage">
			{#each bases as option (option.value)}
				<option value={option.value}>{option.text}</option>
			{/each}
		</select>
	</div>
{/if}

<div class="sw-row">
	<span class="sw-label">Jahr</span>
	<select class="narrow" bind:value={year} aria-label="Jahr">
		{#each YEARS as j (j)}
			<option value={j}>{j}</option>
		{/each}
	</select>
</div>

<div class="sw-row">
	<span class="sw-label">Quartal</span>
	<div class="sw-grid">
		{#each QUARTERS as q (q)}
			<button
				class="sw-choice"
				class:active={quarter === q}
				type="button"
				onclick={() => chooseQuarter(q)}
			>
				{'I'.repeat(q)}
			</button>
		{/each}
		<button class="sw-choice wide" class:active={quarter === null} type="button" onclick={() => (quarter = null)}>
			Alle
		</button>
	</div>
</div>

<div class="sw-row">
	<span class="sw-label">Monat</span>
	<div class="sw-grid">
		{#each MONTHS as m (m)}
			<button class="sw-choice" class:active={month === m} type="button" onclick={() => chooseMonth(m)}>
				{m}
			</button>
		{/each}
	</div>
</div>
