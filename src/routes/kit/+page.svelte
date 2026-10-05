<script lang="ts">
	import '#lib/components/kit/kit.css';
	import ArtifactsGallery from './artifacts-gallery.svelte';
	import ComponentsShowcase from './components-showcase.svelte';
	import KitNav, { type KitSection } from './kit-nav.svelte';
	import ChatSurface from '#lib/components/kit/chat/chat-surface.svelte';
	import Sheet from '#lib/components/kit/sheet.svelte';
	import Tooltip from '#lib/components/kit/tooltip.svelte';
	import Button from '#lib/components/kit/button.svelte';

	/**
	 * The kit test page. It answers one question — how does the GPUI Kit interface hold up
	 * in a browser — so it is built as three real layouts rather than one responsive
	 * approximation: a phone column with a sheet for navigation, a tablet rail beside the
	 * conversation, and a desktop sidebar with the conversation in a bounded column.
	 */
	let section = $state<KitSection>('chat');
	let navOpen = $state(false);
	let mode = $state<'dark' | 'light'>('dark');

	/**
	 * Width presets only mean something on a wide window: on the device itself the layout
	 * is already the size you are asking about. They exist so one browser window can be
	 * compared against the three target sizes without resizing the window by hand.
	 */
	let preset = $state<'auto' | 'phone' | 'tablet' | 'desktop'>('auto');

	$effect(() => {
		const stored = localStorage.getItem('kit-mode');
		if (stored === 'light' || stored === 'dark') mode = stored;
	});

	$effect(() => {
		localStorage.setItem('kit-mode', mode);
	});
	const PRESETS: Array<{ id: typeof preset; label: string; width: string }> = [
		{ id: 'auto', label: 'Auto', width: '100%' },
		{ id: 'phone', label: 'Phone', width: '390px' },
		{ id: 'tablet', label: 'Tablet', width: '834px' },
		{ id: 'desktop', label: 'Desktop', width: '100%' }
	];

	const TITLES: Record<KitSection, string> = {
		chat: 'Chat',
		components: 'Components',
		artifacts: 'Artifacts'
	};
</script>

<svelte:head>
	<title>GPUI Kit in the browser</title>
	<meta name="description" content="GPUI Kit component layer and default theme, ported to Svelte for browser testing." />
</svelte:head>

<div
	class="kit mx-auto flex h-[100dvh] max-w-full flex-col overflow-hidden bg-(--kit-background) text-[color:var(--kit-foreground)]"
	style:max-width={PRESETS.find((entry) => entry.id === preset)?.width}
	data-mode={mode}
	data-preset={preset}
>
	<header class="flex shrink-0 items-center gap-(--kit-space-sm) border-b border-(--kit-border) bg-(--kit-card) px-(--kit-space-sm) py-(--kit-space-sm) md:px-(--kit-space-md)">
		<Button variant="ghost" size="small" iconOnly class="md:hidden" onclick={() => (navOpen = true)} aria-label="Open sections">
			<svg viewBox="0 0 16 16" class="size-4" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round">
				<path d="M2.5 4h11M2.5 8h11M2.5 12h11" />
			</svg>
		</Button>

		<div class="flex min-w-0 flex-col">
			<div class="flex min-w-0 items-center gap-(--kit-space-sm)">
				<h1 class="truncate text-[length:var(--kit-text-sm)] leading-[length:var(--kit-leading-sm)] font-semibold">GPUI Kit</h1>
				<!-- WF-DEC-003: the browser build is alpha until WF-INV-002 reports a real
				     web renderer. The marker sits on the surface, not only in a ticket, so a
				     reviewer on a handheld knows what they are looking at. -->
				<Tooltip label="The web target is alpha: the theme and the component layer are real, the browser renderer is still unproven here. See WF-DEC-003.">
					<span class="shrink-0 rounded-(--kit-radius-full) bg-[color-mix(in_oklab,var(--kit-warning)_22%,transparent)] px-(--kit-space-sm) py-px text-[length:var(--kit-text-xs)] leading-[length:var(--kit-leading-xs)] font-semibold text-[color:var(--kit-warning)]">
						Web alpha
					</span>
				</Tooltip>
			</div>
			<p class="truncate text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) text-[color:var(--kit-muted-foreground)]">
				{TITLES[section]} · browser preview
			</p>
		</div>
		<div class="ml-auto flex items-center gap-(--kit-space-sm)">
			<div class="hidden items-center gap-(--kit-space-xxs) rounded-(--kit-radius-md) border border-(--kit-border) p-(--kit-space-xxs) md:flex" role="group" aria-label="Preview width">
				{#each PRESETS as entry (entry.id)}
					<button
						type="button"
						data-kit="preset"
						aria-pressed={preset === entry.id}
						onclick={() => (preset = entry.id)}
						class="rounded-(--kit-radius-sm) px-(--kit-space-sm) py-[3px] text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) font-medium transition-colors duration-(--kit-duration-fast)"
						class:bg-(--kit-accent)={preset === entry.id}
						class:text-[color:var(--kit-foreground)]={preset === entry.id}
						class:text-[color:var(--kit-muted-foreground)]={preset !== entry.id}
					>
						{entry.label}
					</button>
				{/each}
			</div>

			<Tooltip label={mode === 'dark' ? 'Switch to the light theme' : 'Switch to the dark theme'}>
				<Button variant="ghost" size="small" iconOnly onclick={() => (mode = mode === 'dark' ? 'light' : 'dark')} aria-label="Switch theme">
					{#if mode === 'dark'}
						<svg viewBox="0 0 16 16" class="size-4" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round">
							<circle cx="8" cy="8" r="3.2" />
							<path d="M8 1.5v1.6M8 12.9v1.6M1.5 8h1.6M12.9 8h1.6M3.4 3.4l1.1 1.1M11.5 11.5l1.1 1.1M12.6 3.4l-1.1 1.1M4.5 11.5l-1.1 1.1" />
						</svg>
					{:else}
						<svg viewBox="0 0 16 16" class="size-4" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linejoin="round">
							<path d="M13.5 9.5A5.8 5.8 0 0 1 6.5 2.5a5.8 5.8 0 1 0 7 7z" />
						</svg>
					{/if}
				</Button>
			</Tooltip>

			<a
				href="/"
				class="hidden h-[24px] items-center rounded-(--kit-radius-md) border border-(--kit-border) px-(--kit-space-sm) text-[length:var(--kit-text-xs)] font-medium text-[color:var(--kit-muted-foreground)] transition-colors duration-(--kit-duration-fast) hover:bg-(--kit-muted) hover:text-[color:var(--kit-foreground)] sm:inline-flex"
			>
				Back to Lexia
			</a>
		</div>
	</header>

	<div class="flex min-h-0 flex-1">
		<KitNav
			class="hidden w-[200px] shrink-0 border-r border-(--kit-sidebar-border) md:flex xl:w-[248px]"
			{section}
			onSelect={(next) => (section = next)}
		>
			{#snippet footer()}
				<p>
					Ported from <span class="font-mono">gpui-component 0.7.0</span>. Colours, radii, spacing and
					typography are transcribed from the crate, not invented.
				</p>
			{/snippet}
		</KitNav>

		<main
			class="flex min-h-0 min-w-0 flex-1 flex-col"
			class:border-x={preset !== 'auto'}
			class:border-(--kit-border)={preset !== 'auto'}
		>
			{#if section === 'chat'}
				<ChatSurface />
			{:else if section === 'components'}
				<ComponentsShowcase />
			{:else}
				<ArtifactsGallery />
			{/if}
		</main>
	</div>

	<Sheet bind:open={navOpen} title="Sections" side="left" width="280px">
		<KitNav
			{section}
			onSelect={(next) => {
				section = next;
				navOpen = false;
			}}
		/>
	</Sheet>
</div>