<script lang="ts">
	import Accordion from '#lib/components/kit/accordion.svelte';
	import Alert from '#lib/components/kit/alert.svelte';
	import Avatar from '#lib/components/kit/avatar.svelte';
	import Badge from '#lib/components/kit/badge.svelte';
	import Button from '#lib/components/kit/button.svelte';
	import Card from '#lib/components/kit/card.svelte';
	import Checkbox from '#lib/components/kit/checkbox.svelte';
	import DataTable from '#lib/components/kit/data-table.svelte';
	import DescriptionList from '#lib/components/kit/description-list.svelte';
	import Dialog from '#lib/components/kit/dialog.svelte';
	import Empty from '#lib/components/kit/empty.svelte';
	import Input from '#lib/components/kit/input.svelte';
	import Kbd from '#lib/components/kit/kbd.svelte';
	import Marker from '#lib/components/kit/marker.svelte';
	import Progress from '#lib/components/kit/progress.svelte';
	import Separator from '#lib/components/kit/separator.svelte';
	import Sheet from '#lib/components/kit/sheet.svelte';
	import Shimmer from '#lib/components/kit/shimmer.svelte';
	import Skeleton from '#lib/components/kit/skeleton.svelte';
	import Spinner from '#lib/components/kit/spinner.svelte';
	import StatusBar from '#lib/components/kit/status-bar.svelte';
	import Switch from '#lib/components/kit/switch.svelte';
	import Tabs from '#lib/components/kit/tabs.svelte';
	import Textarea from '#lib/components/kit/textarea.svelte';
	import Tooltip from '#lib/components/kit/tooltip.svelte';
	import KitGroup from './kit-group.svelte';

	let dialogOpen = $state(false);
	let sheetOpen = $state(false);
	let tab = $state('preview');
	let notifications = $state(true);
	let terms = $state(false);
	let indeterminate = $state(false);
	let codeSearch = $state('');
	let invalid = $state(true);
	let openPanel = $state<string | null>('run');

	const releases = [
		{ version: '2026.9.3', channel: 'stable', actions: '1,284', duration: '38 ms', status: 'Promoted' },
		{ version: '2026.9.2', channel: 'stable', actions: '1,201', duration: '41 ms', status: 'Rolled back' },
		{ version: '2026.9.1-rc3', channel: 'rc', actions: '1,198', duration: '44 ms', status: 'Staged' },
		{ version: '2026.8.9', channel: 'stable', actions: '1,150', duration: '40 ms', status: 'Retained' }
	];
</script>

<section aria-label="Components" class="flex min-h-0 flex-1 flex-col">
	<header class="border-b border-(--kit-border) px-(--kit-space-md) py-(--kit-space-md) md:px-(--kit-space-xl)">
		<h2 class="text-[length:var(--kit-text-lg)] leading-(--kit-leading-lg) font-semibold">Components</h2>
		<p class="mt-(--kit-space-xs) max-w-[70ch] text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm) text-[color:var(--kit-muted-foreground)]">
			The GPUI Kit component set, live. Sizes, variants and states are the values
			transcribed from the crate, so this page shows the real geometry rather than an
			approximation of it.
		</p>
	</header>

	<div class="kit-scroll min-h-0 flex-1 overflow-y-auto px-(--kit-space-md) py-(--kit-space-lg) md:px-(--kit-space-xl)">
		<div class="flex flex-col gap-(--kit-space-lg) xl:grid xl:grid-cols-2 xl:items-start">
			<KitGroup title="Buttons" note="Heights and padding from button.rs: 20, 24 and 32px. Status variants dim on hover because the theme has no separate hover token for them.">
				<div class="flex flex-wrap items-center gap-(--kit-space-sm)">
					<Button variant="primary">Save the run</Button>
					<Button variant="secondary">Inspect actions</Button>
					<Button variant="default">Copy</Button>
					<Button variant="ghost">Dismiss</Button>
					<Button variant="danger">Cancel run</Button>
					<Button variant="success">Promote</Button>
					<Button variant="warning">Staged</Button>
					<Button variant="link">Read the docs</Button>
				</div>
				<Separator label="Sizes" />
				<div class="flex flex-wrap items-center gap-(--kit-space-sm)">
					<Button variant="secondary" size="xsmall">Extra small</Button>
					<Button variant="secondary" size="small">Small</Button>
					<Button variant="secondary" size="medium">Medium</Button>
					<Button variant="secondary" size="large">Large</Button>
					<Button variant="secondary" size="medium" outline>Outline</Button>
				</div>
				<Separator label="States" />
				<div class="flex flex-wrap items-center gap-(--kit-space-sm)">
					<Button variant="primary" loading>Sending</Button>
					<Button variant="secondary" disabled>Disabled</Button>
					<Tooltip label="Keyboard and pointer reachable, never hover-only">
						<Button variant="ghost" iconOnly aria-label="Refresh the run list">
							<svg viewBox="0 0 16 16" class="size-4" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round">
								<path d="M13.5 8a5.5 5.5 0 1 1-1.7-4M13.5 2.5V6H10" />
							</svg>
						</Button>
					</Tooltip>
					<Kbd keys={['mod', 'k']} />
				</div>
			</KitGroup>

			<KitGroup title="Inputs" note="Three heights, a validation state that is not colour-only, and controls that stay operable with a keyboard.">
				<div class="flex flex-col gap-(--kit-space-md)">
					<div class="flex flex-wrap items-end gap-(--kit-space-md)">
						<label class="flex flex-col gap-(--kit-space-xs)">
							<span class="text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">Small</span>
							<Input size="sm" placeholder="Filter runs" aria-label="Filter runs, small" />
						</label>
						<label class="flex flex-col gap-(--kit-space-xs)">
							<span class="text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">Medium</span>
							<Input placeholder="Filter runs" aria-label="Filter runs" />
						</label>
						<label class="flex flex-col gap-(--kit-space-xs)">
							<span class="text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">Large</span>
							<Input size="lg" placeholder="Filter runs" aria-label="Filter runs, large" />
						</label>
					</div>

					<label class="flex flex-col gap-(--kit-space-xs)">
						<span class="text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">Invalid, with the reason in text</span>
						<Input bind:value={codeSearch} invalid={invalid} error="Enter a run id, for example run_7fq2." aria-label="Run id" />
					</label>

					<label class="flex flex-col gap-(--kit-space-xs)">
						<span class="text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">Notes for the turn</span>
						<Textarea rows={3} placeholder="What should Lexia do with this run?" aria-label="Notes for the turn" />
					</label>

					<div class="flex flex-wrap items-center gap-(--kit-space-xl)">
						<Switch bind:checked={notifications}>
							{#snippet label()}Stream run events{/snippet}
						</Switch>
						<Checkbox bind:checked={terms}>
							{#snippet label()}Attach the full transcript{/snippet}
						</Checkbox>
						<Checkbox bind:checked={indeterminate} indeterminate>
							{#snippet label()}Some sub-agents failed{/snippet}
						</Checkbox>
					</div>
				</div>
			</KitGroup>

			<KitGroup title="Status and feedback" note="Loading, streaming and progress states. A spinner is never the only signal: each row is labelled too.">
				<div class="flex flex-wrap items-center gap-(--kit-space-lg)">
					<div class="flex items-center gap-(--kit-space-sm)"><Spinner size="sm" /><span class="text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">small</span></div>
					<div class="flex items-center gap-(--kit-space-sm)"><Spinner size="md" /><span class="text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">medium</span></div>
					<div class="flex items-center gap-(--kit-space-sm)"><Spinner size="xl" /><span class="text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">extra large</span></div>
				</div>
				<Separator />
				<div class="flex flex-col gap-(--kit-space-md)">
					<Progress value={64} ariaLabel="Artifact rendering progress">
						{#snippet label()}Rendering artifacts{/snippet}
					</Progress>
					<Progress value={null} tone="danger" ariaLabel="Waiting on EVE">
						{#snippet label()}Waiting on EVE{/snippet}
					</Progress>
				</div>
				<Separator label="Streaming" />
				<Shimmer text="Spinning up the scout sub-agent, reading the crate theme tokens" />
				<Separator label="Loading placeholder" />
				<div class="flex flex-col gap-(--kit-space-sm)"><Skeleton lines={3} /></div>
			</KitGroup>

			<KitGroup title="Labels and identity" note="Badges, status markers, avatars and the empty state.">
				<div class="flex flex-wrap items-center gap-(--kit-space-sm)">
					<Badge>Default</Badge>
					<Badge variant="secondary">Secondary</Badge>
					<Badge variant="outline">Outline</Badge>
					<Badge variant="success">Succeeded</Badge>
					<Badge variant="warning">Staged</Badge>
					<Badge variant="danger">Failed</Badge>
					<Badge variant="info">Streaming</Badge>
				</div>
				<Separator />
				<div class="flex flex-wrap items-center gap-(--kit-space-lg)">
					<div class="flex items-center gap-(--kit-space-xs)"><Marker variant="success" /><span class="text-[length:var(--kit-text-xs)]">Healthy</span></div>
					<div class="flex items-center gap-(--kit-space-xs)"><Marker variant="warning" /><span class="text-[length:var(--kit-text-xs)]">Degraded</span></div>
					<div class="flex items-center gap-(--kit-space-xs)"><Marker variant="danger" /><span class="text-[length:var(--kit-text-xs)]">Offline</span></div>
				</div>
				<Separator />
				<div class="flex flex-wrap items-center gap-(--kit-space-md)">
					<Avatar name="Lexia" size="sm" />
					<Avatar name="Lexia" size="md" />
					<Avatar name="Scout" size="lg" />
					<Avatar name="Reviewer" size="xl" />
				</div>
			</KitGroup>

			<KitGroup title="Overlays" note="Dialog and Sheet both use the platform dialog element, so focus trapping and Escape dismissal are the browser's, not a re-implementation.">
				<div class="flex flex-wrap items-center gap-(--kit-space-sm)">
					<Button variant="secondary" onclick={() => (dialogOpen = true)}>Open the confirm dialog</Button>
					<Button variant="secondary" onclick={() => (sheetOpen = true)}>Open the side sheet</Button>
				</div>
				<Separator />
				<Tabs
					items={[
						{ id: 'preview', label: 'Preview', badge: '16' },
						{ id: 'code', label: 'Code' },
						{ id: 'notes', label: 'Notes', disabled: true }
					]}
					bind:value={tab}
				/>
				<Accordion
					bind:value={openPanel}
					items={[
						{ id: 'run', title: 'What a run is' },
						{ id: 'actions', title: 'What an action is' },
						{ id: 'events', title: 'What the stream carries' }
					]}
				/>
			</KitGroup>

			<KitGroup title="Data display" note="Sortable table, cards, key-value records and the status bar.">
				<DataTable
					caption="EVE releases and their run history"
					footnote="4 releases · newest first"
					columns={[
						{ key: 'version', label: 'Release' },
						{ key: 'channel', label: 'Channel' },
						{ key: 'actions', label: 'Actions', numeric: true },
						{ key: 'duration', label: 'Duration', numeric: true },
						{ key: 'status', label: 'Status' }
					]}
					rows={releases}
				/>
				<Card title="Promotion gate" description="A release is promoted only after it builds, health-checks and is accepted.">
					<p class="text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm) text-[color:var(--kit-muted-foreground)]">
						The card shell keeps a hairline between its header and body instead of nesting a
						second card, which is what makes a dense screen readable.
					</p>
					{#snippet footer()}
						<Button variant="secondary" size="small">Read the release notes</Button>
					{/snippet}
				</Card>
				<DescriptionList
					items={[
						{ label: 'Release', value: '2026.9.3', hint: 'Promoted from app-data/releases' },
						{ label: 'Transport', value: 'http://127.0.0.1:8787' },
						{ label: 'Model route', value: 'openrouter/stealth/space-bunny-alpha' }
					]}
				/>
				<StatusBar
					items={[
						{ label: 'EVE', value: 'running', tone: 'success' },
						{ label: 'Runs', value: '12 active' },
						{ label: 'Memory', value: '1,284 notes' },
						{ label: 'Latency', value: '1,204 ms', tone: 'warning' }
					]}
				/>
			</KitGroup>

			<KitGroup title="Empty and error" note="The states a screen has to answer for, not an afterthought.">
				<Alert variant="warning" title="EVE is not running" description="Nothing can answer until it is started." />
				<Alert variant="danger" title="That run no longer exists" description="It may have been cancelled and cleaned up." />
				<Alert variant="info" title="Replay from the last event id" description="A refresh replays the stream instead of losing the turn." />
				<Empty title="No runs yet" description="Ask for something and it becomes the first run.">
					{#snippet icon()}
						<svg viewBox="0 0 24 24" class="size-6" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.25">
							<rect x="3" y="5" width="18" height="14" rx="3" />
							<path d="M8 10h8M8 14h5" stroke-linecap="round" />
						</svg>
					{/snippet}
					<Button variant="primary" size="small">Start a run</Button>
				</Empty>
			</KitGroup>
		</div>
	</div>
</section>

<Dialog bind:open={dialogOpen} title="Cancel run_7fq2?" description="Two sub-agents are still working. Cancelling keeps everything already written.">
	<p class="text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm) text-[color:var(--kit-muted-foreground)]">
		The run record stays in the database either way; cancelling only stops the work in
		flight.
	</p>
	{#snippet footer()}
		<Button variant="ghost" onclick={() => (dialogOpen = false)}>Keep working</Button>
		<Button variant="danger" onclick={() => (dialogOpen = false)}>Cancel the run</Button>
	{/snippet}
</Dialog>

<Sheet bind:open={sheetOpen} title="Run actions" side="right" description="Everything the run did, in order.">
	<ol class="flex flex-col gap-(--kit-space-md) text-[length:var(--kit-text-sm)]">
		<li>Claimed the reply action with a 120s lease.</li>
		<li>Streamed the reply off EVE.</li>
		<li>Recorded the turn as a note.</li>
	</ol>
</Sheet>