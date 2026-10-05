<script lang="ts">
	import { enhance } from '$app/forms';
	import type { ActionData, PageData } from './$types';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Card from '#lib/components/ui/card/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';

	let { data, form }: { data: PageData; form: ActionData | null } = $props();

	const SOURCE_LABEL = {
		environment: 'Environment variable',
		stored: 'Stored key',
		none: 'Not configured'
	} as const;
</script>

<svelte:head>
	<title>Providers — Lexosa</title>
	<meta name="description" content="Credentials for the model providers Lexosa can call." />
</svelte:head>

<section class="mx-auto w-full max-w-2xl flex-1 space-y-4 px-5 py-6" aria-label="Providers">
	<p class="mb-5 text-sm text-muted-foreground">
		One credential per provider. Each key stays on this host and is never sent to the browser.
	</p>

	{#if form && 'error' in form && form.error}
		<p class="text-sm text-destructive" role="alert">{form.error}</p>
	{/if}

	{#each data.providers as provider (provider.id)}
		{@const saved = form && 'provider' in form && form.provider === provider.id}
		<Card.Root>
			<Card.Header>
				<div class="flex items-center justify-between gap-4">
					<Card.Title>{provider.label}</Card.Title>
					<Badge variant={provider.keySource === 'none' ? 'outline' : 'secondary'}>
						{SOURCE_LABEL[provider.keySource]}
					</Badge>
				</div>
				<Card.Description>{provider.summary}</Card.Description>
			</Card.Header>
			<Card.Content>
				<div class="mb-4 flex flex-wrap gap-1.5">
					{#each provider.modalities as modality (modality)}
						<span class="rounded-full border border-border px-2 py-0.5 text-xs text-muted-foreground">
							{modality}
						</span>
					{/each}
				</div>

				<form
					method="POST"
					action="?/saveKey"
					use:enhance={() => {
						return async ({ update }) => {
							await update({ reset: false });
						};
					}}
				>
					<Field.FieldGroup>
						<input type="hidden" name="provider" value={provider.id} />
						<Field.Field>
							<Field.FieldLabel for={`key-${provider.id}`}>API key</Field.FieldLabel>
							<Input
								id={`key-${provider.id}`}
								name="apiKey"
								type="password"
								placeholder="Paste the key to store it on this host"
								autocomplete="off"
								spellcheck="false"
							/>
							<Field.FieldDescription>
								Stored at <code>app-data/state/{provider.id}.key</code>, readable only by you.
								<code>{provider.envVar}</code> in the host environment wins over it. Submit the field
								empty to remove the stored key.
								{#if provider.baseUrlEnv}
									Reached at <code>{provider.baseUrl}</code>. Point it elsewhere with
									<code>{provider.baseUrlEnv}</code> in the host environment and restart; no
									key is needed unless that server wants one.
								{/if}
								{#if !provider.hasCatalogue}
									{provider.label} publishes no model list we have verified, so its models are
									typed by hand in Settings → Models.
								{/if}
							</Field.FieldDescription>
						</Field.Field>
						<div class="flex flex-wrap items-center gap-3">
							<Button type="submit" size="sm">Save key</Button>
							{#if saved && 'keySource' in form}
								<span class="text-xs text-muted-foreground" role="status">
									Saved. Now using the {SOURCE_LABEL[form.keySource ?? 'none']}.
									{#if 'restarted' in form && form.restarted}The agent restarted with it.{/if}
								</span>
							{/if}
						</div>
					</Field.FieldGroup>
				</form>
			</Card.Content>
		</Card.Root>
	{/each}

	<p class="rounded-md border border-border bg-muted/50 p-3 text-xs text-muted-foreground">
		These credentials configure which provider and model Lexosa would use. Nothing calls the
		speech, image or video APIs yet, and a model chosen here does not change what EVE runs
		until the router consumes it.
	</p>
</section>