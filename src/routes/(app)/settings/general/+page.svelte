<script lang="ts">
	import { mode, toggleMode } from 'mode-watcher';
	import * as Card from '#lib/components/ui/card/index.js';
	import { Label } from '#lib/components/ui/label/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
</script>

<svelte:head>
	<title>General — Lexia</title>
	<meta name="description" content="Workspace preferences for this device." />
</svelte:head>

<section class="mx-auto w-full max-w-2xl flex-1 space-y-4 px-5 py-6" aria-label="General settings">
	<p class="mb-5 text-sm text-muted-foreground">Workspace preferences for this device.</p>

	<Card.Root>
		<Card.Header>
			<Card.Title>Appearance</Card.Title>
			<Card.Description>Lexia is dark-first; light mode is an explicit alternative.</Card.Description>
		</Card.Header>
		<Card.Content>
			<div class="flex items-center justify-between gap-4">
				<Label for="dark-mode" class="font-semibold">Dark mode</Label>
				<Switch
					id="dark-mode"
					checked={mode.current === 'dark'}
					onCheckedChange={() => toggleMode()}
				/>
			</div>
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>Lexia</Card.Title>
			<Card.Description>Server-side agent harness — Jev routes, EVE executes.</Card.Description>
		</Card.Header>
	</Card.Root>
</section>
