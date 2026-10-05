<script lang="ts">
	import { enhance, type SubmitFunction } from '$app/forms';
	import type { ActionData, PageData } from './$types';
	import { tick } from 'svelte';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import TrashIcon from '@lucide/svelte/icons/trash-2';
	import XIcon from '@lucide/svelte/icons/x';
	import * as Alert from '#lib/components/ui/alert/index.js';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Card from '#lib/components/ui/card/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as NativeSelect from '#lib/components/ui/native-select/index.js';

	let { data, form }: { data: PageData; form: ActionData | null } = $props();

	type ProviderId = string;
	type Pick = { modelId: string; providerId: ProviderId };

	// Assignments confirmed by the server this session. A row updates from the
	// action's own result, so choosing a model never re-runs the load (and with
	// it every catalogue fetch).
	let assignments = $state<Record<string, Pick>>({});
	// A model chosen in the picker but not yet confirmed. It fills the hidden
	// fields the next submit posts and is dropped once the server echoes the row.
	let picks = $state<Record<string, Pick>>({});
	// Which provider each row is currently shopping at. Defaults to the row's own
	// assignment so a saved provider stays selected.
	let rowProvider = $state<Record<string, ProviderId>>({});

	let openUseCase = $state<string | null>(null);
	let query = $state('');
	let highlight = $state(0);
	let filterInput = $state<HTMLInputElement | null>(null);
	let optionList = $state<HTMLElement | null>(null);
	let rowForms = $state<Record<string, HTMLFormElement | null>>({});

	const rows = $derived(
		data.useCases.map((row) => {
			const cards = data.providers[row.modality] ?? [];
			const saved = assignments[row.useCase];
			const picked = picks[row.useCase];
			const modelId = picked?.modelId ?? saved?.modelId ?? row.modelId;
			// An unassigned row still needs a provider to shop at, and the first one
			// on offer is the only honest default: an empty select would read as a
			// provider that does not exist.
			const assigned = picked?.providerId ?? saved?.providerId ?? row.providerId;
			const provider = rowProvider[row.useCase] ?? (assigned || cards[0]?.id || '');
			return { ...row, modelId, providerId: provider, provider };
		})
	);

	const grouped = $derived(
		data.modalityOrder
			.map((modality) => ({ modality, label: data.modalityLabels[modality], rows: rows.filter((row) => row.modality === modality) }))
			.filter((group) => group.rows.length > 0)
	);

	/** Names for one provider's catalogue. Kept per provider so a row switching
	 * provider cannot show a model from the other one. */
	const modelNames = (providerId: ProviderId) =>
		new Map((data.catalogues[providerId] ?? []).map((model) => [model.id, model.name]));

	const matches = $derived.by(() => {
		const providerId = openUseCase ? (rowProvider[openUseCase] ?? '') : '';
		const catalogue = data.catalogues[providerId] ?? [];
		const needle = query.trim().toLowerCase();
		if (!needle) return catalogue;
		return catalogue.filter(
			(model) => model.name.toLowerCase().includes(needle) || model.id.toLowerCase().includes(needle)
		);
	});

	// Keep the keyboard-highlighted option inside the scroll box as it moves.
	$effect(() => {
		highlight;
		optionList?.querySelector<HTMLElement>('[data-highlighted="true"]')?.scrollIntoView({ block: 'nearest' });
	});

	$effect(() => {
		if (openUseCase) filterInput?.focus();
	});

	function closePicker(): void {
		openUseCase = null;
		query = '';
		highlight = 0;
	}

	function togglePicker(useCase: string): void {
		if (openUseCase === useCase) {
			closePicker();
			return;
		}
		openUseCase = useCase;
		query = '';
		highlight = 0;
	}

	function onFilterKeydown(event: KeyboardEvent): void {
		if (event.key === 'ArrowDown') {
			event.preventDefault();
			highlight = Math.min(highlight + 1, matches.length - 1);
			return;
		}
		if (event.key === 'ArrowUp') {
			event.preventDefault();
			highlight = Math.max(highlight - 1, 0);
			return;
		}
		if (event.key === 'Enter') {
			// The highlight is only a proposal: with nothing highlighted, Enter
			// must not post an empty model id.
			const chosen = matches[highlight];
			if (!chosen) return;
			event.preventDefault();
			void choose(chosen.id);
			return;
		}
		if (event.key === 'Escape') {
			event.preventDefault();
			closePicker();
		}
	}

	async function choose(modelId: string): Promise<void> {
		const useCase = openUseCase;
		if (!useCase) return;
		picks[useCase] = { modelId, providerId: rowProvider[useCase] ?? '' };
		const rowForm = rowForms[useCase];
		closePicker();
		// The hidden fields render from `picks`, so the DOM must catch up before
		// the form is submitted or the previous id would be posted.
		await tick();
		rowForm?.requestSubmit();
	}

	/** Providers on offer for a row: everything that serves its modality, whether
	 * or not a key is configured yet — the operator may be about to add one. */
	function providersFor(row: (typeof rows)[number]) {
		return data.providers[row.modality] ?? [];
	}

	const assignSubmit: SubmitFunction = ({ formData }) => async ({ result, update }) => {
		if (result.type === 'success' && result.data && 'assignment' in result.data) {
			assignments[result.data.assignment.useCase] = {
				modelId: result.data.assignment.modelId,
				providerId: result.data.assignment.providerId
			};
			delete picks[String(formData.get('useCase') ?? '')];
		}
		// `reset: false` keeps the picker's hidden fields intact and
		// `refreshAll: false` keeps the row update free of a catalogue refetch;
		// the result is still applied so a refusal reaches the page as `form`.
		await update({ reset: false, refreshAll: false });
	};
</script>

<svelte:head>
	<title>Models — Lexosa</title>
	<meta name="description" content="Map use-cases to provider models and test them from the Lexosa server." />
</svelte:head>

<section class="mx-auto flex w-full max-w-2xl flex-1 flex-col gap-4 px-5 py-6" aria-label="Models">
	<p class="text-sm text-muted-foreground">
		Choose which provider model runs each kind of work. Credentials stay on the host and never
		reach the browser.
	</p>

	<Card.Root>
		<Card.Header>
			<Card.Title>Model routes</Card.Title>
			<Card.Description>
				Each row names a use-case, the provider that serves it, and the model to run.
				<strong>Always</strong> is the fallback used when nothing more specific matches, so it
				cannot be removed.
			</Card.Description>
		</Card.Header>
		<Card.Content>
			<p class="mb-4 rounded-md border border-border bg-muted/50 p-3 text-xs text-muted-foreground">
				These assignments route work on this Lexosa host. EVE is a separate process that compiles
				its own model from its authored agent source at boot, so it keeps running that model
				until the router starts consuming the assignments made here.
			</p>

			{#if form && 'assignError' in form && form.assignError}
				<Alert.Root class="mb-4">
					<Alert.Title>That model was not saved</Alert.Title>
					<Alert.Description>{form.assignError}</Alert.Description>
				</Alert.Root>
			{/if}

			{#each grouped as group (group.modality)}
				<h3 class="mb-2 mt-5 text-xs font-bold tracking-[0.16em] text-secondary">{group.label.toUpperCase()}</h3>
				<ul class="divide-y divide-border">
					{#each group.rows as row (row.useCase)}
						{@const cards = providersFor(row)}
						{@const chosen = cards.find((card) => card.id === row.provider) ?? cards[0]}
						<li class="flex flex-col gap-2 py-3 sm:flex-row sm:items-start sm:justify-between sm:gap-4">
							<div class="min-w-0">
								<p class="text-sm font-semibold">{row.label}</p>
								<p class="font-mono text-xs text-muted-foreground">{row.useCase}</p>
								{#if row.modality === 'text' && row.useCase === 'always'}
									<Badge variant="secondary" class="mt-1">Fallback</Badge>
								{/if}
							</div>

							<div class="flex items-start gap-2 sm:w-96">
								<form
									bind:this={rowForms[row.useCase]}
									method="POST"
									action="?/assign"
									use:enhance={assignSubmit}
									class="relative min-w-0 flex-1"
								>
									<input type="hidden" name="useCase" value={row.useCase} />
									<input type="hidden" name="modelId" value={picks[row.useCase]?.modelId ?? row.modelId} />
									<input type="hidden" name="providerId" value={picks[row.useCase]?.providerId ?? row.provider} />

									<div class="mb-2 flex items-center gap-2">
										<NativeSelect.Root
											class="h-8 min-w-0 flex-1 text-xs"
											value={row.provider}
											aria-label={`Provider for ${row.label}`}
											onchange={(event) => {
												rowProvider[row.useCase] = event.currentTarget.value;
												closePicker();
											}}
										>
											{#each cards as card (card.id)}
												<NativeSelect.Option value={card.id}>
													{card.label}{card.configured ? '' : ' · no key'}
												</NativeSelect.Option>
											{/each}
										</NativeSelect.Root>
										{#if row.modelId && !(chosen?.hasCatalogue && chosen.coversModality)}
											<span class="max-w-32 truncate font-mono text-xs text-muted-foreground" title={row.modelId}>
												{row.modelId}
											</span>
										{/if}
									</div>

									{#if chosen?.hasCatalogue && chosen.coversModality}
										<button
											type="button"
											class="flex h-8 w-full items-center justify-between gap-2 rounded-lg border border-input bg-transparent px-2.5 text-left text-sm outline-none transition-colors hover:bg-muted focus-visible:border-ring focus-visible:ring-ring/50"
											aria-expanded={openUseCase === row.useCase}
											onclick={() => togglePicker(row.useCase)}
										>
											<span class="truncate" class:text-muted-foreground={!row.modelId}>
												{row.modelId ? (modelNames(chosen.id).get(row.modelId) ?? row.modelId) : 'Not chosen'}
											</span>
											<ChevronDownIcon class="size-4 shrink-0 opacity-60" />
										</button>

										{#if !chosen.configured}
											<p class="mt-1 text-xs text-muted-foreground">
												{chosen.label} has no key on this host. Add one in Settings → Providers.
											</p>
										{:else if data.catalogueErrors[chosen.id]}
											<p class="mt-1 text-xs text-destructive">{data.catalogueErrors[chosen.id]}</p>
										{/if}

										{#if openUseCase === row.useCase}
											<div class="absolute right-0 left-0 z-20 mt-1 rounded-lg border border-border bg-popover p-2 text-popover-foreground shadow-md">
												<Input
													bind:ref={filterInput}
													bind:value={query}
													oninput={() => (highlight = 0)}
													onkeydown={onFilterKeydown}
													aria-label={`Filter ${chosen.label} models for ${row.label}`}
													placeholder="Search models"
													autocomplete="off"
												/>

												<ul bind:this={optionList} class="mt-2 max-h-56 overflow-y-auto">
													{#each matches as model, index (model.id)}
														<li data-highlighted={index === highlight ? 'true' : undefined}>
															<button
																type="button"
																class="flex w-full flex-col items-start gap-0.5 rounded-md px-2 py-1.5 text-left text-sm aria-selected:bg-muted hover:bg-muted"
																onclick={() => void choose(model.id)}
															>
																<span class="truncate">{model.name}</span>
																<span class="truncate font-mono text-xs text-muted-foreground">{model.id}</span>
															</button>
														</li>
													{:else}
														<li class="px-2 py-3 text-sm text-muted-foreground">
															No {chosen.label} model matches “{query.trim()}”.
														</li>
													{/each}
												</ul>
											</div>
										{/if}
									{:else}
										<!-- No verified list for this provider, so the model id is typed. -->
										<Field.Field>
											<Field.FieldLabel for={`model-${row.useCase}`}>Model id</Field.FieldLabel>
											<Input
												id={`model-${row.useCase}`}
												value={row.modelId}
												oninput={(event) => {
													picks[row.useCase] = {
														modelId: event.currentTarget.value,
														providerId: row.provider
													};
												}}
												placeholder={row.modelId || 'Model id from this provider'}
												autocomplete="off"
												spellcheck="false"
											/>
											<Field.FieldDescription>
												{chosen?.label}
												{chosen?.hasCatalogue
													? 'lists models for text only, so this row takes the id typed here.'
													: 'publishes no model list we have verified, so its model id is typed here.'}
											</Field.FieldDescription>
										</Field.Field>
										<Button type="submit" size="sm" class="mt-2">
											Save model
										</Button>
									{/if}
								</form>

								{#if row.modelId}
									<form method="POST" action="?/assign" use:enhance={assignSubmit}>
										<input type="hidden" name="useCase" value={row.useCase} />
										<input type="hidden" name="modelId" value="" />
										<input type="hidden" name="providerId" value={row.provider} />
										<Button type="submit" variant="ghost" size="icon" aria-label={`Clear the model for ${row.label}`}>
											<XIcon />
										</Button>
									</form>
								{/if}

								{#if !row.builtin}
									<form method="POST" action="?/removeUseCase" use:enhance>
										<input type="hidden" name="useCase" value={row.useCase} />
										<Button type="submit" variant="ghost" size="icon" aria-label={`Remove ${row.label}`}>
											<TrashIcon />
										</Button>
									</form>
								{/if}
							</div>
						</li>
					{/each}
				</ul>
			{/each}

			<form method="POST" action="?/createUseCase" use:enhance class="mt-6">
				<Field.FieldGroup>
					<Field.Field data-invalid={form && 'createError' in form ? true : undefined}>
						<Field.FieldLabel for="new-use-case">New use-case</Field.FieldLabel>
						<div class="flex items-start gap-2">
							<Input
								id="new-use-case"
								name="label"
								value={form && 'label' in form ? (form.label ?? '') : ''}
								placeholder="Meeting summaries"
								maxlength={60}
								required
							/>
							<NativeSelect.Root name="modality" value="text" aria-label="Kind of work" class="w-36">
								{#each data.modalityOrder as modality (modality)}
									<NativeSelect.Option value={modality}>{data.modalityLabels[modality]}</NativeSelect.Option>
								{/each}
							</NativeSelect.Root>
							<Button type="submit" size="sm">
								<PlusIcon class="size-3.5" />
								Add
							</Button>
						</div>
						{#if form && 'createError' in form && form.createError}
							<Field.FieldError>{form.createError}</Field.FieldError>
						{/if}
					</Field.Field>
				</Field.FieldGroup>
			</form>
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>OpenRouter model test</Card.Title>
			<Card.Description>
				Send one short test request and inspect the model reply and token usage. Test requests may
				incur provider charges.
			</Card.Description>
		</Card.Header>
		<Card.Content>
			<form method="POST" action="?/test" use:enhance>
				<Field.FieldGroup>
					<Field.Field data-invalid={form && 'error' in form ? true : undefined}>
						<Field.FieldLabel for="openrouter-model-id">Model ID</Field.FieldLabel>
						<Input
							id="openrouter-model-id"
							name="modelId"
							placeholder="provider/model"
							value={form && 'modelId' in form ? (form.modelId ?? data.defaultModelId) : data.defaultModelId}
							maxlength={160}
							autocomplete="off"
							aria-invalid={form && 'error' in form ? 'true' : undefined}
							required
						/>
						<Field.FieldDescription>
							Use an exact model slug from the <a class="underline underline-offset-4" href="https://openrouter.ai/models" target="_blank" rel="noreferrer">OpenRouter model catalog</a>.
							Needs an OpenRouter key: add one in Settings → Providers.
						</Field.FieldDescription>
						{#if form && 'error' in form && form.error}
							<Field.FieldError>{form.error}</Field.FieldError>
						{/if}
					</Field.Field>
					<div class="flex flex-wrap items-center gap-3">
						<Button type="submit" disabled={!data.catalogues.openrouter}>Test model</Button>
						<span class="text-xs text-muted-foreground">Fixed prompt · maximum 48 output tokens · not saved</span>
					</div>
				</Field.FieldGroup>
			</form>

			{#if form && 'success' in form && form.success}
				<div class="mt-5 rounded-md border p-4" aria-live="polite">
					<p class="text-sm font-semibold">{form.result.model} replied</p>
					<p class="mt-1 text-sm">{form.result.reply}</p>
					<p class="mt-2 font-mono text-xs text-muted-foreground">
						{form.result.usage.promptTokens ?? 0} in · {form.result.usage.completionTokens ?? 0} out ·{' '}
						{form.result.usage.totalTokens ?? 0} total
					</p>
				</div>
			{/if}
		</Card.Content>
	</Card.Root>
</section>