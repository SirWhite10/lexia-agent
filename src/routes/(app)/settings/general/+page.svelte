<script lang="ts">
	import { mode, toggleMode } from 'mode-watcher';
	import * as Card from '#lib/components/ui/card/index.js';
	import { Label } from '#lib/components/ui/label/index.js';
	import * as RadioGroup from '#lib/components/ui/radio-group/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import { SIDEBAR_SECTION_STARTS, sidebarPreferences } from '#lib/sidebar-preferences.svelte.js';

	// The radio group hands back a plain string; only a known option is applied,
	// so a stale value in storage can never leave the shell in an unknown state.
	const applySectionStart = (value: string) => {
		const option = SIDEBAR_SECTION_STARTS.find((candidate) => candidate.value === value);
		if (option) sidebarPreferences.setSectionStart(option.value);
	};
</script>

<svelte:head>
	<title>General — Lexosa</title>
	<meta name="description" content="Workspace preferences for this device." />
</svelte:head>

<section class="mx-auto w-full max-w-2xl flex-1 space-y-4 px-5 py-6" aria-label="General settings">
	<p class="mb-5 text-sm text-muted-foreground">Workspace preferences for this device.</p>

	<Card.Root>
		<Card.Header>
			<Card.Title>Appearance</Card.Title>
			<Card.Description>Lexosa is dark-first; light mode is an explicit alternative.</Card.Description>
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
			<Card.Title>Sidebar</Card.Title>
			<Card.Description>Which sidebar sections open when the app loads.</Card.Description>
		</Card.Header>
		<Card.Content>
			<RadioGroup.Root
				value={sidebarPreferences.sectionStart}
				onValueChange={applySectionStart}
				aria-label="Sidebar sections on load"
			>
				{#each SIDEBAR_SECTION_STARTS as option (option.value)}
					<div class="flex items-start gap-3 py-1.5">
						<RadioGroup.Item
							value={option.value}
							id={`sidebar-section-start-${option.value}`}
							class="mt-0.5"
						/>
						<div class="grid gap-0.5">
							<Label for={`sidebar-section-start-${option.value}`} class="font-semibold">
								{option.label}
							</Label>
							<p class="text-sm text-muted-foreground">{option.description}</p>
						</div>
					</div>
				{/each}
			</RadioGroup.Root>
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>Lexosa</Card.Title>
			<Card.Description>Server-side agent harness — Jev routes, EVE executes.</Card.Description>
		</Card.Header>
	</Card.Root>
</section>
