<script lang="ts">
	import { enhance } from '$app/forms';
	import type { ActionData, PageData } from './$types';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Card from '#lib/components/ui/card/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';

	let { data, form }: { data: PageData; form: ActionData | null } = $props();

	const sourceLabel = $derived(
		data.openRouterKeySource === 'environment'
			? 'Environment variable'
			: data.openRouterKeySource === 'stored'
				? 'Stored key'
				: 'Not configured'
	);
</script>

<svelte:head>
	<title>Providers — Lexosa</title>
	<meta name="description" content="Model provider credentials for Lexosa." />
</svelte:head>

<section class="mx-auto w-full max-w-2xl flex-1 space-y-4 px-5 py-6" aria-label="Providers">
	<p class="mb-5 text-sm text-muted-foreground">
		Model provider credentials. They stay server-side and never reach the client.
	</p>

	<Card.Root>
		<Card.Header>
			<div class="flex items-center justify-between gap-4">
				<Card.Title>OpenRouter</Card.Title>
				<Badge variant={data.openrouterConfigured ? 'secondary' : 'outline'}>
					{data.openrouterConfigured ? 'Configured' : 'Not configured'}
				</Badge>
			</div>
			<Card.Description>
				The provider Lexosa talks to for completions and embeddings. In use: {sourceLabel}.
			</Card.Description>
		</Card.Header>
		<Card.Content>
			<form method="POST" action="?/saveKey" use:enhance>
				<Field.FieldGroup>
					<Field.Field>
						<Field.FieldLabel for="openrouter-key">API key</Field.FieldLabel>
						<Input
							id="openrouter-key"
							name="apiKey"
							type="password"
							placeholder={data.openRouterKeySource === 'stored' ? 'Stored — type to replace' : 'sk-or-v1-…'}
							autocomplete="off"
						/>
						<Field.FieldDescription>
							Saved to <code>app-data/state/openrouter.key</code>, readable only by you. Saving restarts the
							agent so new routes pick it up. Submitting the field empty removes the stored key.
						</Field.FieldDescription>
					</Field.Field>

					{#if data.openRouterKeySource === 'environment'}
						<Field.FieldDescription>
							<code>OPENROUTER_API_KEY</code> is set in the host environment and takes precedence, so a key
							saved here stays inactive until that variable is removed.
						</Field.FieldDescription>
					{/if}

					<div class="flex flex-wrap items-center gap-3">
						<Button type="submit" size="sm">Save key</Button>
						<span class="text-xs text-muted-foreground">Never leaves the host · not shown again after saving</span>
					</div>
				</Field.FieldGroup>
			</form>

			{#if form && 'saved' in form && form.saved}
				<p class="mt-4 text-sm text-muted-foreground" role="status" aria-live="polite">
					{#if form.source === 'stored'}
						Key saved. The agent restarted with it, so model routes can use it now.
					{:else if form.source === 'environment'}
						Key stored, but the host environment still wins — remove <code>OPENROUTER_API_KEY</code> to switch.
					{:else}
						Stored key removed. Restart <code>bun run dev</code> if the host environment does not supply one.
					{/if}
				</p>
			{/if}
		</Card.Content>
	</Card.Root>
</section>