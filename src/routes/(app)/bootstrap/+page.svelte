<script lang="ts">
	import { enhance } from '$app/forms';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Card from '#lib/components/ui/card/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Label } from '#lib/components/ui/label/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';

	let { data, form } = $props();

	// Working style lines start empty and the form adds one row per click, so
	// nobody has to guess the format the personality file expects.
	let traits = $state(['', '']);
</script>

<svelte:head>
	<title>Set up your agent — Lexia</title>
	<meta name="description" content="Name your agent and give it a starting personality." />
</svelte:head>

<section class="mx-auto w-full max-w-2xl flex-1 space-y-4 px-5 py-6" aria-label="Set up your agent">
	<div class="space-y-1">
		<h1 class="text-xl font-bold">Set up your agent</h1>
		<p class="text-sm text-muted-foreground">
			Two files get written: the system prompt your agent runs on, and the personality block it
			grows over time. Memory builds itself as you work — nothing to set up there.
		</p>
	</div>

	{#if form?.error}
		<p class="text-destructive text-sm" role="alert">{form.error}</p>
	{/if}

	<form method="POST" action="?/create" class="space-y-4" use:enhance>
		<Card.Root>
			<Card.Header>
				<Card.Title>Identity</Card.Title>
				<Card.Description>What this agent is called and what it is for.</Card.Description>
			</Card.Header>
			<Card.Content class="space-y-3">
				<Field.Field>
					<Label for="name">Name</Label>
					<Input id="name" name="name" placeholder="Lexia" required />
				</Field.Field>
				<Field.Field>
					<Label for="purpose">Purpose</Label>
					<Textarea
						id="purpose"
						name="purpose"
						rows={3}
						placeholder="Keeps the lights, the calendar, and the coding queue straight without being asked twice."
						required
					></Textarea>
				</Field.Field>
			</Card.Content>
		</Card.Root>

		<Card.Root>
			<Card.Header>
				<Card.Title>Voice</Card.Title>
				<Card.Description>How it should come across.</Card.Description>
			</Card.Header>
			<Card.Content>
				<Field.Field>
					<Label for="tone">Tone</Label>
					<Textarea
						id="tone"
						name="tone"
						rows={2}
						placeholder="Plain and direct. Says what it did, not what it intends to do."
					></Textarea>
				</Field.Field>
			</Card.Content>
		</Card.Root>

		<Card.Root>
			<Card.Header>
				<Card.Title>Working style</Card.Title>
				<Card.Description>
					Short behaviour lines seeded as the first personality traits. It adds more as it
					learns you.
				</Card.Description>
			</Card.Header>
			<Card.Content class="space-y-2">
				{#each traits as trait, i (i)}
					<div class="flex items-center gap-2">
						<Input
							name="traits"
							value={trait}
							placeholder="checks before doing anything irreversible"
							aria-label="Trait {i + 1}"
							oninput={(event) => (traits[i] = event.currentTarget.value)}
						/>
						<Button
							type="button"
							variant="ghost"
							size="icon-sm"
							aria-label="Remove trait {i + 1}"
							disabled={traits.length === 1}
							onclick={() => traits.splice(i, 1)}
						>
							✕
						</Button>
					</div>
				{/each}
				<Button
					type="button"
					variant="outline"
					size="xs"
					onclick={() => (traits = [...traits, ''])}
				>
					+ Add a trait
				</Button>
			</Card.Content>
		</Card.Root>

		<div class="flex items-center gap-3">
			<Button type="submit">Create agent</Button>
			<p class="text-muted-foreground text-xs">
				Writes <span class="font-mono">agent/instructions.md</span> and your personality file.
			</p>
		</div>
	</form>
</section>
