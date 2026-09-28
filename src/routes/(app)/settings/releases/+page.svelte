<script lang="ts">
	import type { PageData } from './$types';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import * as Item from '#lib/components/ui/item/index.js';

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>Releases — Lexia</title>
	<meta name="description" content="Immutable EVE releases staged on this installation." />
</svelte:head>

<section class="mx-auto w-full max-w-2xl flex-1 space-y-4 px-5 py-6" aria-label="Releases">
	<p class="mb-5 text-sm text-muted-foreground">
		Immutable EVE releases staged on this installation.
	</p>
	<p class="font-mono text-xs text-muted-foreground">
		download → stage → inspect → validate → eve build → health check → accept → promote → restart EVE
		→ retain rollback
	</p>

	{#if data.releases.length === 0}
		<Empty.Root class="rounded-lg border border-border bg-card">
			<Empty.Header>
				<Empty.Title>No releases installed</Empty.Title>
				<Empty.Description>Staged and active releases appear here.</Empty.Description>
			</Empty.Header>
		</Empty.Root>
	{:else}
		<Item.Group class="gap-0 rounded-lg border border-border bg-card">
			{#each data.releases as release, i (release.name)}
				<Item.Root>
					<Item.Content>
						<Item.Title>{release.name}</Item.Title>
						<Item.Description class="font-mono text-xs"
							>app-data/releases/{release.name}</Item.Description
						>
					</Item.Content>
				</Item.Root>
				{#if i < data.releases.length - 1}
					<Item.Separator class="my-0" />
				{/if}
			{/each}
		</Item.Group>
	{/if}
</section>
