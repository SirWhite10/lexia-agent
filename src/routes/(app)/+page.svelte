<script lang="ts">
	import { enhance } from '$app/forms';
	import { invalidateAll } from '$app/navigation';
	import BotIcon from '@lucide/svelte/icons/bot';
	import CircleStopIcon from '@lucide/svelte/icons/circle-stop';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';

	let { data, form } = $props();

	// The massive chat: one continuous conversation for this installation. Run
	// progress arrives over SSE; the database is authoritative, so a refresh
	// replays from the last event id rather than losing anything.
	type RunView = {
		runId: string;
		status: string;
		actions: Array<{ id: string; outcome: string; status: string; progress: string | null; error: string | null }>;
	};
	let activeRuns = $state<Record<string, RunView>>({});
	let draft = $state('');
	let lastEventId = 0;

	async function streamRun(runId: string) {
		const source = new EventSource(`/api/runs/${runId}/events`);
		source.addEventListener('snapshot', (event) => {
			const snapshot = JSON.parse((event as MessageEvent).data);
			activeRuns[runId] = { runId, status: snapshot.run.status, actions: snapshot.actions };
		});
		for (const type of ['action.queued', 'action.running', 'action.progress', 'action.succeeded', 'action.failed', 'action.cancelled']) {
			source.addEventListener(type, (event) => {
				const messageEvent = event as MessageEvent;
				lastEventId = Number(messageEvent.lastEventId || lastEventId);
				applyActionEvent(runId, type, JSON.parse(messageEvent.data));
			});
		}
		source.addEventListener('run.terminal', () => {
			source.close();
			// The run is durable; pull the authoritative message log back in.
			invalidateAll();
		});
	}

	function applyActionEvent(runId: string, type: string, payload: Record<string, unknown>) {
		const view = activeRuns[runId] ?? { runId, status: 'active', actions: [] };
		const actionId = String(payload.actionId ?? '');
		const existing = view.actions.find((action) => action.id === actionId);
		const status =
			type === 'action.queued'
				? 'queued'
				: type === 'action.running'
					? 'running'
					: type === 'action.succeeded'
						? 'succeeded'
						: type === 'action.failed'
							? 'failed'
							: 'cancelled';
		if (existing) {
			existing.status = status;
			if (typeof payload.progress === 'string') existing.progress = payload.progress;
		} else {
			view.actions.push({ id: actionId, outcome: 'Working…', status, progress: null, error: null });
		}
		activeRuns[runId] = view;
	}
</script>

<svelte:head>
	<title>Lexia</title>
	<meta name="description" content="Lexia and its standing sub-agents" />
</svelte:head>

<section class="mx-auto flex w-full max-w-2xl flex-1 flex-col space-y-4 px-5 py-6" aria-label="Chat">
	<div class="mb-2 flex flex-wrap items-center justify-between gap-3">
		<p class="min-w-0 flex-1 text-sm text-muted-foreground">
			One conversation with Lexia. It remembers, and it can spin up sub-agents.
		</p>
		<Button href="/agents" variant="outline" size="sm">Sub-agents</Button>
	</div>

	{#if !data.eveUp}
		<p class="rounded-lg border border-border bg-muted px-4 py-3 text-sm text-muted-foreground">
			EVE is not answering, so nothing can reply yet. The host starts it automatically —
			give it a moment, and check the server log for the <code class="font-mono">[eve]</code>
			output if it stays down.
		</p>
	{/if}

	{#if form?.error}
		<p class="text-destructive text-sm" role="alert">{form.error}</p>
	{/if}

	<!-- The chat log. This is the single massive conversation; it grows for the
	     life of the install and is never split into separate chats. -->
	<div class="flex-1 space-y-3" aria-label="Conversation">
		{#if !data.bootstrapped}
			<!-- Setup stays reachable until it is done, not only on an empty chat:
			     anyone who sent a turn before finishing setup still needs the door. -->
			<div class="rounded-lg border border-border bg-card p-5">
				<p class="mb-1 text-xs font-bold tracking-[0.16em] text-secondary">GET STARTED</p>
				<p class="mb-3 text-sm text-muted-foreground">
					Name your agent, say what it is for, and give it a starting personality. Its
					memory builds itself from here.
				</p>
				<Button href="/bootstrap" size="sm">Set up your agent</Button>
			</div>
		{/if}

		{#if data.messages.length === 0}
			{#if data.bootstrapped}
				<div class="rounded-lg border border-border bg-card p-5">
					<p class="mb-1 text-xs font-bold tracking-[0.16em] text-secondary">LEXIA</p>
					<p class="text-sm text-muted-foreground">
						Nothing here yet. Ask for something and it becomes the first run.
					</p>
				</div>
			{/if}
		{:else}
			{#each data.messages as message (message.id)}
				<div class="flex items-start gap-2">
					{#if message.authorKind === 'user'}
						<div class="ml-auto max-w-[80%] rounded-lg bg-primary px-3 py-2 text-sm text-primary-foreground">
							{message.body}
						</div>
					{:else}
						<BotIcon class="mt-1 size-4 shrink-0 text-muted-foreground" />
						<div class="max-w-[80%] rounded-lg border border-border bg-card px-3 py-2 text-sm">
							{message.body}
						</div>
					{/if}
				</div>
			{/each}
		{/if}

		<!-- Delegation entries: one parent request with action-level progress. The
		     full transcript stays in the run detail, not the chat. -->
		{#each Object.values(activeRuns) as view (view.runId)}
			<div class="rounded-lg border border-border bg-card p-3" data-run-status={view.status}>
				<div class="mb-2 flex items-center justify-between gap-2">
					<p class="font-mono text-xs text-muted-foreground">run · {view.status}</p>
					{#if view.status === 'active' || view.status === 'waiting'}
						<form method="POST" action="?/cancel" use:enhance>
							<input type="hidden" name="runId" value={view.runId} />
							<Button type="submit" variant="ghost" size="icon-sm" aria-label="Cancel run">
								<CircleStopIcon />
							</Button>
						</form>
					{/if}
				</div>
				<ul class="space-y-1">
					{#each view.actions as action (action.id)}
						<li class="flex items-center justify-between gap-2 text-sm">
							<span class="truncate">{action.outcome}</span>
							<span class="font-mono text-xs text-muted-foreground">
								{action.progress ?? action.status}
							</span>
						</li>
					{/each}
				</ul>
			</div>
		{/each}
	</div>

	<form
		method="POST"
		action="?/send"
		class="flex items-center gap-2"
		use:enhance={() => {
			return async ({ result, update }) => {
				const runId = (result.type === 'success' ? result.data?.runId : undefined) as string | undefined;
				draft = '';
				await update();
				if (runId) streamRun(runId);
			};
		}}
	>
		<Input name="body" bind:value={draft} placeholder="Tell Lexia to do something…" aria-label="Message" />
		<Button type="submit" disabled={draft.trim().length === 0}>Send</Button>
	</form>
</section>
