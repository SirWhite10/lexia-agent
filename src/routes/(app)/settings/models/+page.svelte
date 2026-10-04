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

	let { data, form }: { data: PageData; form: ActionData | null } = $props();

	const tiers = [
		{ name: 'small', description: 'fast classification and gate decisions' },
		{ name: 'general', description: 'everyday conversation and tool use' },
		{ name: 'complex', description: 'coding, planning, multi-step work' },
		{ name: 'deep', description: 'long reasoning and hard analysis' }
	];


	// Assignments confirmed by the server this session. A row updates from the
	// action's own result, so choosing a model never re-runs the load (and with
	// it the catalogue fetch).
	let assignments = $state<Record<string, string>>({});
	// A model chosen in the picker but not yet confirmed. It fills the hidden
	// field the next submit posts and is dropped once the server echoes the row.
	let picks = $state<Record<string, string>>({});

	let openUseCase = $state<string | null>(null);
	let query = $state('');
	let highlight = $state(0);
	let filterInput = $state<HTMLInputElement | null>(null);
	let optionList = $state<HTMLElement | null>(null);
	let rowForms = $state<Record<string, HTMLFormElement | null>>({});

	const rows = $derived(data.useCases.map((row) => ({ ...row, modelId: assignments[row.useCase] ?? row.modelId })));
	const modelNames = $derived(new Map(data.catalogue.map((model) => [model.id, model.name])));

	const matches = $derived.by(() => {
		const needle = query.trim().toLowerCase();
		if (!needle) return data.catalogue;
		return data.catalogue.filter(
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
		picks[useCase] = modelId;
		const rowForm = rowForms[useCase];
		closePicker();
		// The hidden field renders from `picks`, so the DOM must catch up before
		// the form is submitted or the previous id would be posted.
		await tick();
		rowForm?.requestSubmit();
	}

	// `use:enhance` hands this function the submission; the callback it returns
	// receives the action result, which is where a picked model is folded back
	// into its row.
	const assignSubmit: SubmitFunction = ({ formData }) => async ({ result, update }) => {
		if (result.type === 'success' && result.data && 'assignment' in result.data) {
			assignments[result.data.assignment.useCase] = result.data.assignment.modelId;
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
	<meta name="description" content="Map use-cases to OpenRouter models and test them from the Lexosa server." />
</svelte:head>

<section class="mx-auto w-full max-w-2xl flex-1 space-y-4 px-5 py-6" aria-label="Models">
	<p class="mb-5 text-sm text-muted-foreground">
		Choose which OpenRouter model runs each kind of work. Credentials stay on the host and never reach the browser.
	</p>

	<Card.Root>
		<Card.Header>
			<Card.Title>Model routes</Card.Title>
			<Card.Description>
				Each row names a use-case and the model that runs it. <strong>Always</strong> is the fallback used when
				nothing more specific matches, so it cannot be removed.
			</Card.Description>
		</Card.Header>
		<Card.Content>
			<p class="mb-4 rounded-md border border-border bg-muted/50 p-3 text-xs text-muted-foreground">
				These assignments route work on this Lexosa host. EVE is a separate process that compiles its own model
				from its authored agent source at boot, so it keeps running that model until the router starts consuming
				the assignments made here.
			</p>

			{#if !data.openrouterConfigured}
				<Alert.Root class="mb-4">
					<Alert.Title>API key not configured</Alert.Title>
					<Alert.Description>
						Add your key in <a class="underline underline-offset-4" href="/settings/providers">Settings → Providers</a>, or set
						<code>OPENROUTER_API_KEY</code> in the host environment. The catalogue cannot be listed without one.
					</Alert.Description>
				</Alert.Root>
			{:else if data.catalogueError}
				<Alert.Root class="mb-4">
					<Alert.Title>Model catalogue unavailable</Alert.Title>
					<Alert.Description>{data.catalogueError}</Alert.Description>
				</Alert.Root>
			{/if}

			<ul class="divide-y divide-border">
				{#each rows as row (row.useCase)}
					<li class="flex flex-col gap-2 py-3 sm:flex-row sm:items-start sm:justify-between sm:gap-4">
						<div class="min-w-0">
							<p class="flex flex-wrap items-center gap-2 text-sm font-medium">
								{row.label}
								{#if row.useCase === 'always'}
									<Badge variant="secondary">Fallback</Badge>
								{:else if !row.builtin}
									<Badge variant="outline">Custom</Badge>
								{/if}
							</p>
							<p class="font-mono text-xs text-muted-foreground">{row.useCase}</p>
						</div>

						<div class="flex items-start gap-2 sm:w-80">
							<form
								bind:this={rowForms[row.useCase]}
								method="POST"
								action="?/assign"
								use:enhance={assignSubmit}
								class="relative min-w-0 flex-1"
							>
								<input type="hidden" name="useCase" value={row.useCase} />
								<input type="hidden" name="modelId" value={picks[row.useCase] ?? row.modelId} />

								<!-- Without a credential there is no catalogue to filter, so the
								     trigger stays closed and the alert above explains why. -->
								<button
									type="button"
									class="flex h-8 w-full items-center justify-between gap-2 rounded-lg border border-input bg-transparent px-2.5 text-left text-sm outline-none transition-colors hover:bg-muted focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50"
									aria-expanded={openUseCase === row.useCase}
									disabled={!data.openrouterConfigured || Boolean(data.catalogueError)}
									onclick={() => togglePicker(row.useCase)}
								>
									<span class="truncate" class:text-muted-foreground={!row.modelId}>
										{row.modelId ? (modelNames.get(row.modelId) ?? row.modelId) : 'Not chosen'}
									</span>
									<ChevronDownIcon class="size-4 shrink-0 opacity-60" />
								</button>

								{#if openUseCase === row.useCase}
									<div class="absolute left-0 right-0 z-20 mt-1 rounded-lg border border-border bg-popover p-2 text-popover-foreground shadow-md">
										<Input
											bind:ref={filterInput}
											bind:value={query}
											oninput={() => (highlight = 0)}
											onkeydown={onFilterKeydown}
											aria-label={`Filter models for ${row.label}`}
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
													No model matches “{query.trim()}”.
												</li>
											{/each}
										</ul>
									</div>
								{/if}
							</form>

							{#if row.modelId}
								<!-- An assigned model is not a one-way door: the same row clears
								     back to "Not chosen" by submitting an empty id. -->
								<Button
									type="button"
									variant="ghost"
									size="icon"
									aria-label={`Clear the model for ${row.label}`}
									onclick={async () => {
										picks[row.useCase] = '';
										// The hidden field renders from `picks`, so the DOM has to catch up
										// before submitting or the previous id would be posted back.
										await tick();
										rowForms[row.useCase]?.requestSubmit();
									}}
								>
									<XIcon />
								</Button>
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

			<form method="POST" action="?/createUseCase" use:enhance class="mt-4">
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
								autocomplete="off"
								aria-invalid={form && 'createError' in form ? 'true' : undefined}
							/>
							<Button type="submit" size="sm" class="h-8"><PlusIcon />Add</Button>
						</div>
						<Field.FieldDescription>
							The use-case id is derived from the name, so “Meeting summaries” is stored as
							<code>meeting-summaries</code>.
						</Field.FieldDescription>
						{#if form && 'createError' in form}
							<Field.FieldError>{form.createError}</Field.FieldError>
						{/if}
					</Field.Field>
				</Field.FieldGroup>
			</form>

			{#if form && 'removeError' in form}
				<p class="mt-3 text-sm text-destructive" role="alert">{form.removeError}</p>
			{/if}
			{#if form && 'assignError' in form}
				<p class="mt-3 text-sm text-destructive" role="alert">{form.assignError}</p>
			{/if}
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>OpenRouter model test</Card.Title>
			<Card.Description>
				Send one short test request and inspect the model reply and token usage. Test requests may incur provider charges.
			</Card.Description>
		</Card.Header>
		<Card.Content>
			{#if !data.openrouterConfigured}
				<Alert.Root class="mb-4">
					<Alert.Title>API key not configured</Alert.Title>
					<Alert.Description>
						Add your key in <a class="underline underline-offset-4" href="/settings/providers">Settings → Providers</a>, or set
						<code>OPENROUTER_API_KEY</code> in the host environment. The key is never sent to the browser.
					</Alert.Description>
				</Alert.Root>
			{/if}

			<form method="POST" action="?/test" use:enhance>
				<Field.FieldGroup>
					<Field.Field data-invalid={form?.error ? true : undefined}>
						<Field.FieldLabel for="openrouter-model-id">Model ID</Field.FieldLabel>
						<Input
							id="openrouter-model-id"
							name="modelId"
							placeholder="provider/model"
							value={form?.modelId ?? data.defaultModelId}
							maxlength={160}
							autocomplete="off"
							aria-invalid={form?.error ? 'true' : undefined}
							required
						/>
						<Field.FieldDescription>
							Defaults to the <strong>Always</strong> model. Use an exact model slug from the <a class="underline underline-offset-4" href="https://openrouter.ai/models" target="_blank" rel="noreferrer">OpenRouter model catalog</a>.
						</Field.FieldDescription>
						{#if form?.error}
							<Field.FieldError>{form.error}</Field.FieldError>
						{/if}
					</Field.Field>
					<div class="flex flex-wrap items-center gap-3">
						<Button type="submit" disabled={!data.openrouterConfigured}>Test model</Button>
						<span class="text-xs text-muted-foreground">Fixed prompt · maximum 48 output tokens · not saved</span>
					</div>
				</Field.FieldGroup>
			</form>

			{#if form && 'success' in form && form.success}
				<div class="mt-5 rounded-md border p-4" aria-live="polite">
					<p class="mb-2 text-sm font-medium">Reply from <code>{form.result.model}</code></p>
					<p class="whitespace-pre-wrap text-sm">{form.result.reply}</p>
					<p class="mt-3 text-xs text-muted-foreground">
						Tokens: {form.result.usage.promptTokens ?? '—'} input · {form.result.usage.completionTokens ?? '—'} output · {form.result.usage.totalTokens ?? '—'} total
					</p>
				</div>
			{/if}
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>Logical model tiers</Card.Title>
			<Card.Description>Planned routing roles; configuring OpenRouter here does not yet connect these tiers to EVE.</Card.Description>
		</Card.Header>
		<Card.Content>
			<dl class="space-y-4">
				{#each tiers as tier (tier.name)}
					<div class="flex flex-col gap-1">
						<dt class="font-mono text-xs">{tier.name}</dt>
						<dd class="text-sm text-muted-foreground">{tier.description}</dd>
					</div>
				{/each}
			</dl>
		</Card.Content>
	</Card.Root>
</section>