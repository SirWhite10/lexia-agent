<script lang="ts">
	import { enhance } from '$app/forms';
	import ArchiveIcon from '@lucide/svelte/icons/archive';
	import BotIcon from '@lucide/svelte/icons/bot';
	import CheckIcon from '@lucide/svelte/icons/check';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import XIcon from '@lucide/svelte/icons/x';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Card from '#lib/components/ui/card/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';

	let { data, form } = $props();

	const sub = $derived(data.subAgent);
	const versions = $derived(data.promptVersions);
	const currentVersion = $derived(versions.find((entry) => entry.version === sub.promptVersion));

	// Inline role-card and prompt editors; one surface at a time.
	let editingRole = $state(false);
	let editingPrompt = $state(false);
</script>

<svelte:head>
	<title>{sub.name} — Lexia</title>
	<meta name="description" content="{sub.name} — standing sub-agent of Lexia" />
</svelte:head>

<section class="mx-auto w-full max-w-2xl flex-1 space-y-4 px-5 py-6" aria-label="Sub-agent">
	<div class="mb-5 flex flex-wrap items-center justify-between gap-3">
		<div class="flex min-w-0 items-center gap-2">
			<BotIcon class="shrink-0 text-muted-foreground" />
			<h1 class="truncate text-xl font-bold">{sub.name}</h1>
			<span class="rounded-full bg-muted px-2 py-0.5 font-mono text-xs text-muted-foreground">
				{sub.status}
			</span>
		</div>
		<form method="POST" action="?/archive" use:enhance>
			<input type="hidden" name="id" value={sub.id} />
			<Button type="submit" variant="ghost" size="sm" aria-label="Archive {sub.name}">
				<ArchiveIcon /> Archive
			</Button>
		</form>
	</div>

	{#if form?.error}
		<p class="text-destructive text-sm" role="alert">{form.error}</p>
	{/if}

	<!-- The role card is the whole identity of a standing sub-agent: what it is
	     for, what it may read, what it may do. -->
	<Card.Root>
		<Card.Header>
			<div class="flex items-center justify-between gap-2">
				<Card.Title>Role card</Card.Title>
				{#if !editingRole}
					<Button
						type="button"
						variant="ghost"
						size="icon-sm"
						aria-label="Edit role card"
						onclick={() => (editingRole = true)}
					>
						<PencilIcon />
					</Button>
				{/if}
			</div>
		</Card.Header>
		<Card.Content>
			{#if editingRole}
				<form
					method="POST"
					action="?/role"
					class="space-y-3"
					use:enhance={() => {
						return async ({ result, update }) => {
							await update();
							if (result.type !== 'failure') editingRole = false;
						};
					}}
				>
					<input type="hidden" name="id" value={sub.id} />
					<Field.Group>
						<Field.Field>
							<Field.Label for="focus">Focus</Field.Label>
							<Input id="focus" name="focus" value={sub.focus} placeholder="What this sub-agent is for" />
						</Field.Field>
						<Field.Field>
							<Field.Label for="memoryScope">Memory scope</Field.Label>
							<Input
								id="memoryScope"
								name="memoryScope"
								value={sub.memoryScope}
								placeholder="comma,separated,tags"
							/>
						</Field.Field>
						<Field.Field>
							<Field.Label for="capabilities">Capabilities</Field.Label>
							<Input
								id="capabilities"
								name="capabilities"
								value={sub.capabilities}
								placeholder="comma,separated,capabilities"
							/>
						</Field.Field>
					</Field.Group>
					<div class="flex items-center gap-1">
						<Button type="submit" variant="ghost" size="icon-sm" aria-label="Save role card" class="text-primary">
							<CheckIcon />
						</Button>
						<Button
							type="button"
							variant="ghost"
							size="icon-sm"
							aria-label="Cancel role card edit"
							onclick={() => (editingRole = false)}
						>
							<XIcon />
						</Button>
					</div>
				</form>
			{:else}
				<dl class="space-y-4 text-sm">
					<div>
						<dt class="mb-1 text-xs font-bold tracking-[0.16em] text-secondary">FOCUS</dt>
						<dd>{sub.focus || 'No focus set.'}</dd>
					</div>
					<div>
						<dt class="mb-1 text-xs font-bold tracking-[0.16em] text-secondary">MEMORY SCOPE</dt>
						<dd class="font-mono text-xs">{sub.memoryScope || 'Unscoped.'}</dd>
					</div>
					<div>
						<dt class="mb-1 text-xs font-bold tracking-[0.16em] text-secondary">CAPABILITIES</dt>
						<dd class="font-mono text-xs">{sub.capabilities || 'None allowed.'}</dd>
					</div>
				</dl>
			{/if}
		</Card.Content>
	</Card.Root>

	<!-- Prompt versions never overwrite: each revision is a new version so
	     "what did it run with?" stays answerable. -->
	<Card.Root>
		<Card.Header>
			<div class="flex items-center justify-between gap-2">
				<Card.Title>
					System prompt{sub.promptVersion ? ` (version ${sub.promptVersion})` : ''}
				</Card.Title>
				{#if !editingPrompt}
					<Button
						type="button"
						variant="ghost"
						size="icon-sm"
						aria-label="Revise system prompt"
						onclick={() => (editingPrompt = true)}
					>
						<PencilIcon />
					</Button>
				{/if}
			</div>
		</Card.Header>
		<Card.Content class="space-y-4">
			{#if editingPrompt}
				<form
					method="POST"
					action="?/prompt"
					class="space-y-3"
					use:enhance={() => {
						return async ({ result, update }) => {
							await update();
							if (result.type !== 'failure') editingPrompt = false;
						};
					}}
				>
					<input type="hidden" name="id" value={sub.id} />
					<Field.Field>
						<Field.Label for="body">New system prompt</Field.Label>
						<Textarea
							id="body"
							name="body"
							rows={8}
							required
							value={currentVersion?.body ?? ''}
						></Textarea>
						<Field.Description>
							Saving creates version {sub.promptVersion ? sub.promptVersion + 1 : 1}; earlier versions
							are kept.
						</Field.Description>
					</Field.Field>
					<div class="flex items-center gap-1">
						<Button type="submit" variant="ghost" size="icon-sm" aria-label="Save prompt version" class="text-primary">
							<CheckIcon />
						</Button>
						<Button
							type="button"
							variant="ghost"
							size="icon-sm"
							aria-label="Cancel prompt edit"
							onclick={() => (editingPrompt = false)}
						>
							<XIcon />
						</Button>
					</div>
				</form>
			{:else if currentVersion}
				<pre
					class="overflow-x-auto rounded-md bg-muted p-3 font-mono text-xs whitespace-pre-wrap">{currentVersion.body}</pre
				>
			{:else}
				<p class="text-muted-foreground text-sm">No prompt recorded.</p>
			{/if}

			{#if versions.length > 1}
				<details class="text-sm">
					<summary class="cursor-pointer text-muted-foreground hover:text-foreground">
						Prompt history ({versions.length} versions)
					</summary>
					<ul class="mt-2 space-y-2">
						{#each versions as entry (entry.version)}
							<li>
								<p class="mb-1 font-mono text-xs text-muted-foreground">
									version {entry.version} · {new Date(entry.createdAt).toLocaleString()}
									{#if entry.version === sub.promptVersion}(current){/if}
								</p>
								<pre
									class="overflow-x-auto rounded-md bg-muted p-3 font-mono text-xs whitespace-pre-wrap">{entry.body}</pre
								>
							</li>
						{/each}
					</ul>
				</details>
			{/if}
		</Card.Content>
	</Card.Root>

	{#if sub.status === 'standing'}
		<form method="POST" action="?/retire" use:enhance>
			<input type="hidden" name="id" value={sub.id} />
			<Button type="submit" variant="outline" size="sm">Retire sub-agent</Button>
		</form>
	{/if}
</section>
