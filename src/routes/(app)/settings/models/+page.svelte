<script lang="ts">
	import { enhance } from '$app/forms';
	import type { ActionData, PageData } from './$types';
	import * as Alert from '#lib/components/ui/alert/index.js';
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
</script>

<svelte:head>
	<title>Models — Lexia</title>
	<meta name="description" content="Test OpenRouter models and review Lexia's logical model tiers." />
</svelte:head>

<section class="mx-auto w-full max-w-2xl flex-1 space-y-4 px-5 py-6" aria-label="Models">
	<p class="mb-5 text-sm text-muted-foreground">
		Test an OpenRouter model from the Lexia server. The logical tiers below are not yet wired to EVE's runtime.
	</p>
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
						Set <code>OPENROUTER_API_KEY</code> in the Lexia host environment. The key is never sent to the browser.
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
							Use an exact model slug from the <a class="underline underline-offset-4" href="https://openrouter.ai/models" target="_blank" rel="noreferrer">OpenRouter model catalog</a>.
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
