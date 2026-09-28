<script lang="ts">
	import { enhance } from '$app/forms';
	import ArchiveIcon from '@lucide/svelte/icons/archive';
	import BotIcon from '@lucide/svelte/icons/bot';
	import CheckIcon from '@lucide/svelte/icons/check';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import XIcon from '@lucide/svelte/icons/x';
	import type { SubAgent } from '#lib/server/agents.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import * as Item from '#lib/components/ui/item/index.js';
	import { Input } from '#lib/components/ui/input/index.js';

	let { data, form } = $props();

	// Inline rename: one row at a time swaps its name line for the edit form.
	let editingId = $state<string | null>(null);
</script>

<svelte:head>
	<title>Sub-agents — Lexia</title>
	<meta name="description" content="Standing sub-agents of Lexia" />
</svelte:head>

{#snippet renameForm(sub: SubAgent)}
	<!-- use:enhance submits via fetch and applies the result in place. Keep the
	     editor open on failure so the reported error can be corrected in place. -->
	<form
		method="POST"
		action="?/rename"
		class="flex items-center gap-1"
		use:enhance={() => {
			return async ({ result, update }) => {
				await update();
				if (result.type !== 'failure') editingId = null;
			};
		}}
	>
		<input type="hidden" name="id" value={sub.id} />
		<Input
			name="name"
			value={sub.name}
			aria-label="Name"
			class="h-7 w-36 text-sm"
			autofocus
			onkeydown={(e) => {
				if (e.key === 'Escape') editingId = null;
			}}
		/>
		<Button
			type="submit"
			variant="ghost"
			size="icon-sm"
			aria-label="Save name"
			class="text-primary"
		>
			<CheckIcon />
		</Button>
		{#if form?.error && form.id === sub.id}
			<span class="text-destructive text-xs">{form.error}</span>
		{/if}
	</form>
{/snippet}

{#snippet editButton(sub: SubAgent)}
	<Button
		type="button"
		variant="ghost"
		size="icon-sm"
		aria-label="Edit name {sub.name}"
		onclick={() => (editingId = sub.id)}
	>
		<PencilIcon />
	</Button>
{/snippet}

{#snippet actions(sub: SubAgent)}
	{#if editingId === sub.id}
		<Button
			type="button"
			variant="ghost"
			size="icon-sm"
			aria-label="Cancel renaming {sub.name}"
			onclick={() => (editingId = null)}
		>
			<XIcon />
		</Button>
	{:else}
		<form method="POST" action="?/archive" use:enhance>
			<input type="hidden" name="id" value={sub.id} />
			<Button
				type="submit"
				variant="ghost"
				size="icon-sm"
				aria-label="Archive {sub.name}"
				title="Archive"
			>
				<ArchiveIcon />
			</Button>
		</form>
	{/if}
{/snippet}

<section class="mx-auto w-full max-w-2xl flex-1 space-y-4 px-5 py-6" aria-label="Sub-agents">
	<p class="mb-5 text-sm text-muted-foreground">
		Standing sub-agents Lexia delegates work to.
	</p>

	{#if data.subAgents.length === 0}
		<Empty.Root class="rounded-lg border border-border bg-card">
			<Empty.Header>
				<Empty.Media variant="icon"><BotIcon /></Empty.Media>
				<Empty.Title>No sub-agents yet</Empty.Title>
				<Empty.Description>
					Lexia creates a sub-agent when a request needs parallel work.
				</Empty.Description>
			</Empty.Header>
		</Empty.Root>
	{:else}
		<Item.Group class="gap-0 rounded-lg border border-border bg-card">
			{#each data.subAgents as sub, i (sub.id)}
				<Item.Root>
					<Item.Media><BotIcon /></Item.Media>
					<Item.Content>
						{#if editingId === sub.id}
							{@render renameForm(sub)}
						{:else}
							<Item.Title>
								<a href={'/agents/' + sub.id}>{sub.name}</a>
								{@render editButton(sub)}
							</Item.Title>
						{/if}
						<Item.Description class="font-mono text-xs">
							{sub.status} · created {new Date(sub.createdAt).toLocaleString()}
						</Item.Description>
					</Item.Content>
					{@render actions(sub)}
				</Item.Root>
				{#if i < data.subAgents.length - 1}
					<Item.Separator class="my-0" />
				{/if}
			{/each}
		</Item.Group>
	{/if}

	<!-- Agent-task history (WF-IMP-006): one-shot delegations dispatched under a
	     run, with their outcome. Standing sub-agents above are the persistent
	     half; these are the jobs they were given. -->
	<div class="rounded-lg border border-border bg-card">
		<p class="border-b border-border px-4 py-3 text-sm font-medium">Agent tasks</p>
		{#if data.agentTasks.length === 0}
			<p class="px-4 py-3 text-sm text-muted-foreground">
				No delegated tasks yet. Tasks appear here when Lexia hands work to a sub-agent.
			</p>
		{:else}
			<ul class="divide-y divide-border">
				{#each data.agentTasks as task (task.actionId)}
					<li class="flex items-center justify-between gap-3 px-4 py-2.5">
						<div class="min-w-0">
							<p class="truncate text-sm">{task.outcome}</p>
							<p class="truncate font-mono text-xs text-muted-foreground">
								{task.capability} · {new Date(task.createdAt).toLocaleString()}
							</p>
						</div>
						<span class="shrink-0 font-mono text-xs text-muted-foreground">{task.status}</span>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</section>
