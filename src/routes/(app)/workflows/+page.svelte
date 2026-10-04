<script lang="ts">
	import WorkflowIcon from '@lucide/svelte/icons/workflow';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import * as Item from '#lib/components/ui/item/index.js';

	let { data } = $props();
</script>

<svelte:head>
	<title>Workflows — Lexosa</title>
	<meta name="description" content="Repeatable Lexosa workflows" />
</svelte:head>

<section class="mx-auto w-full max-w-2xl flex-1 px-5 py-6" aria-label="Workflows">
	<p class="mb-5 text-sm text-muted-foreground">
		Documented, repeatable operations shipped with the agent source in this repository.
	</p>

	{#if data.workflows.length === 0}
		<Empty.Root class="rounded-lg border border-border bg-card">
			<Empty.Header>
				<Empty.Media variant="icon"><WorkflowIcon /></Empty.Media>
				<Empty.Title>No workflows installed</Empty.Title>
				<Empty.Description>
					Installed workflows appear here as named folders under <code class="font-mono text-xs">workflow/</code>.
				</Empty.Description>
			</Empty.Header>
		</Empty.Root>
	{:else}
		<Item.Group class="gap-0 rounded-lg border border-border bg-card">
			{#each data.workflows as workflow, i (workflow.name)}
				<Item.Root>
					<Item.Media><WorkflowIcon /></Item.Media>
					<Item.Content>
						<Item.Title>{workflow.name}</Item.Title>
						<Item.Description class="font-mono text-xs">workflow/{workflow.name}</Item.Description>
					</Item.Content>
				</Item.Root>
				{#if i < data.workflows.length - 1}
					<Item.Separator class="my-0" />
				{/if}
			{/each}
		</Item.Group>
	{/if}
</section>
