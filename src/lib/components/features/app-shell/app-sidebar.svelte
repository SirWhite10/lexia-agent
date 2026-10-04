<script lang="ts">
	import { page } from '$app/state';
	import BotIcon from '@lucide/svelte/icons/bot';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
	import SlidersHorizontalIcon from '@lucide/svelte/icons/sliders-horizontal';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import type { SubAgent } from '#lib/server/agents.js';
	import { sidebarPreferences } from '#lib/sidebar-preferences.svelte.js';
	import { isNavActive, settingsItems, workflowsItem } from './nav-items.js';

	let { subAgents }: { subAgents: SubAgent[] } = $props();

	// Section state comes from the preference store so Settings → General can
	// change it and so the sections a user opens survive a reload.
	let subAgentsOpen = $derived(sidebarPreferences.open['sub-agents']);

	// The settings section is the rail's footer, so it stays lit whenever any of
	// its pages is open rather than only when General is the active one.
	let settingsActive = $derived(settingsItems.some((item) => isNavActive(item, page.url.pathname)));
	let settingsOpen = $derived(sidebarPreferences.open.settings);
</script>

{#snippet subAgentRows(list: SubAgent[])}
	{#each list as sub (sub.id)}
		<Sidebar.MenuItem>
			<Sidebar.MenuButton
				isActive={page.url.pathname === `/agents/${sub.id}`}
				tooltipContent={sub.name}
				class="h-9 group-data-[collapsible=icon]:justify-center group-data-[collapsible=icon]:gap-0"
			>
				{#snippet child({ props })}
					<a href={`/agents/${sub.id}`} {...props}>
						<BotIcon />
						<span class="group-data-[collapsible=icon]:hidden">{sub.name}</span>
					</a>
				{/snippet}
			</Sidebar.MenuButton>
		</Sidebar.MenuItem>
	{/each}
{/snippet}

<!-- Desktop workspace navigation. On mobile the same destinations live in the
     bottom tab bar (app-bottom-nav.svelte); this rail is the desktop form. -->
<Sidebar.Root collapsible="icon">
	<Sidebar.Header>
		<Sidebar.Menu class="gap-1.5 group-data-[collapsible=icon]:gap-1.5">
			<Sidebar.MenuItem>
				<Sidebar.MenuButton
					size="lg"
					tooltipContent="Lexosa"
					class="group-data-[collapsible=icon]:justify-center group-data-[collapsible=icon]:gap-0"
				>
					{#snippet child({ props })}
						<a href="/" {...props}>
							<span
								class="grid size-6 shrink-0 place-items-center rounded-[min(var(--radius-md),10px)] bg-primary text-xs text-primary-foreground"
								aria-hidden="true">✦</span
							>
							<span class="font-bold group-data-[collapsible=icon]:hidden">Lexosa</span>
						</a>
					{/snippet}
				</Sidebar.MenuButton>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Header>

	<Sidebar.Content>
		<!-- Sub-agents section: Lexosa's standing sub-agents plus the manage and
		     expand controls. The list mirrors the sub-agent area on /agents. -->
		<Sidebar.Group>
			<div
				class="flex items-center gap-0.5 px-2 group-data-[collapsible=icon]:-mt-8 group-data-[collapsible=icon]:opacity-0"
			>
				<button
					type="button"
					class="flex-1 rounded-md px-1 py-1 text-left text-xs font-semibold tracking-wide text-muted-foreground hover:text-foreground"
					aria-expanded={subAgentsOpen}
					onclick={() => sidebarPreferences.toggle('sub-agents')}
				>
					Sub-agents
				</button>
				<Button
					href="/agents"
					variant="ghost"
					size="icon-sm"
					aria-label="Manage sub-agents"
					title="Manage sub-agents"
				>
					<SlidersHorizontalIcon />
				</Button>
				<Button
					variant="ghost"
					size="icon-sm"
					aria-label={subAgentsOpen ? 'Collapse sub-agents section' : 'Expand sub-agents section'}
					onclick={() => sidebarPreferences.toggle('sub-agents')}
				>
					{#if subAgentsOpen}
						<ChevronUpIcon />
					{:else}
						<ChevronDownIcon />
					{/if}
				</Button>
			</div>
			<Sidebar.GroupContent class={subAgentsOpen ? undefined : 'hidden'}>
				<Sidebar.Menu class="gap-1.5 group-data-[collapsible=icon]:gap-1.5">
					{@render subAgentRows(subAgents)}
					{#if subAgents.length === 0}
						<Sidebar.MenuItem>
							<span
								class="px-2 py-1 text-xs text-muted-foreground group-data-[collapsible=icon]:hidden"
							>
								No sub-agents yet
							</span>
						</Sidebar.MenuItem>
					{/if}
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>

		<Sidebar.Group>
			<Sidebar.GroupContent>
				<Sidebar.Menu class="gap-1.5 group-data-[collapsible=icon]:gap-1.5">
					<Sidebar.MenuItem>
						<Sidebar.MenuButton
							isActive={isNavActive(workflowsItem, page.url.pathname)}
							tooltipContent={workflowsItem.title}
							class="h-10 group-data-[collapsible=icon]:justify-center group-data-[collapsible=icon]:gap-0"
						>
							{#snippet child({ props })}
								<a href={workflowsItem.href} {...props}>
									<workflowsItem.icon />
									<span class="group-data-[collapsible=icon]:hidden">{workflowsItem.title}</span>
								</a>
							{/snippet}
						</Sidebar.MenuButton>
					</Sidebar.MenuItem>
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>

	</Sidebar.Content>

	<!-- Settings lives in the footer: it is where the rail ends, not another
	     destination competing with the work. -->
	<Sidebar.Footer>
		<Sidebar.Group>
			<div
				class="flex items-center gap-0.5 px-2 group-data-[collapsible=icon]:-mt-8 group-data-[collapsible=icon]:opacity-0"
			>
				<button
					type="button"
					class="flex-1 rounded-md px-1 py-1 text-left text-xs font-semibold tracking-wide transition-colors hover:text-foreground {settingsActive
						? 'bg-sidebar-accent text-sidebar-accent-foreground'
						: 'text-muted-foreground'}"
					aria-expanded={settingsOpen}
					aria-current={settingsActive ? 'page' : undefined}
					onclick={() => sidebarPreferences.toggle('settings')}
				>
					Settings
				</button>
				<Button
					variant="ghost"
					size="icon-sm"
					aria-label={settingsOpen ? 'Collapse settings section' : 'Expand settings section'}
					onclick={() => sidebarPreferences.toggle('settings')}
				>
					{#if settingsOpen}
						<ChevronUpIcon />
					{:else}
						<ChevronDownIcon />
					{/if}
				</Button>
			</div>
			<Sidebar.GroupContent class={settingsOpen ? undefined : 'hidden'}>
				<Sidebar.Menu class="gap-1 group-data-[collapsible=icon]:gap-1">
					<Sidebar.MenuItem>
						<Sidebar.MenuSub>
							{#each settingsItems as item (item.href)}
								<Sidebar.MenuSubItem>
									<Sidebar.MenuSubButton
										href={item.href}
										isActive={isNavActive(item, page.url.pathname)}
									>
										{item.title}
									</Sidebar.MenuSubButton>
								</Sidebar.MenuSubItem>
							{/each}
						</Sidebar.MenuSub>
					</Sidebar.MenuItem>
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>
	</Sidebar.Footer>
</Sidebar.Root>
