<script lang="ts" module>
	import { cn } from '#lib/utils.js';
	import type { ChatHarnessState, ChatMessage, RunView } from './model.js';
	import type { ComposerSend } from './composer.svelte';
	import { applyRunPatch } from './run-patch.js';
	import { textWithoutContext } from './composer-model.js';
	import Alert from '../alert.svelte';
	import Button from '../button.svelte';
	import MessageScroller from '../message-scroller.svelte';
	import Composer from './composer.svelte';
	import MessageItem from './message-item.svelte';
	import RunCard from './run-card.svelte';

	export type ChatSurfaceProps = {
		/**
		 * The harness state the test page drives. It exists because every state the real
		 * chat can reach must be reachable here too: an unreachable state is a state that
		 * was never actually tested.
		 */
		harness?: ChatHarnessState;
		class?: string | undefined;
	};

	const DEFAULT_STATE: ChatHarnessState = { eveUp: true, bootstrapped: true, error: null, emptyConversation: false };
</script>

<script lang="ts">
	import { seededArtifacts, scriptedRun, seededConversation } from '../demo/transcript.js';
	import Spinner from '../spinner.svelte';

	let { harness = $bindable({ ...DEFAULT_STATE }), class: className }: ChatSurfaceProps = $props();

	let draft = $state('');

	let messages = $state<ChatMessage[]>(seededConversation(Date.now()));
	let runs = $state<RunView[]>([]);
	let hydrated = $state(false);

	/** Wall-clock timers for the scripted run. Held so cancel and teardown can clear them. */
	let timers: Array<ReturnType<typeof setTimeout>> = [];

	$effect(() => {
		hydrated = true;
		return () => {
			for (const timer of timers) clearTimeout(timer);
			timers = [];
		};
	});

	const visibleMessages = $derived(harness.emptyConversation ? [] : messages);
	const blockedReason = $derived(harness.eveUp ? null : 'EVE is not running, so nothing can answer yet.');
	const liveRun = $derived(runs.find((run) => run.status === 'active' || run.status === 'waiting') ?? null);

	function stopTimers() {
		for (const timer of timers) clearTimeout(timer);
		timers = [];
	}

	/**
	 * The composer reports the text it was given plus whatever context it held. Files
	 * become the message's attachments, so the sent turn shows exactly what was added;
	 * the rest is recorded on the message as context for the run detail.
	 */
	function send(payload: ComposerSend) {
		if (blockedReason) return;
		// The message reads as a sentence; the tokens it carried travel as attachments
		// and context chips, not as punctuation in the body.
		const body = textWithoutContext(payload.text, payload.catalog) || payload.text.trim();
		const attachments = payload.items
			.filter((item) => item.kind === 'file')
			.map((item) => ({ name: item.label, kind: 'file' as const, size: item.detail ?? '', state: item.state ?? 'ready' }));
		messages = [
			...messages,
			{
				id: `m${Date.now()}`,
				authorKind: 'user',
				body,
				createdAt: Date.now(),
				attachments: attachments.length ? attachments : undefined,
				context: payload.items.filter((item) => item.kind !== 'file')
			}
		];
		startRun(body);
	}

	/**
	 * Replays `scriptedRun` against a real `RunView`, in the same order the server would
	 * emit the equivalent events. The point is that the progress card, the fan-out and the
	 * artifact arrival are exercised by the same code path a live run would drive.
	 */
	function startRun(title: string) {
		stopTimers();
		const runId = `run_${Math.random().toString(36).slice(2, 6)}`;
		const run: RunView = { runId, title, status: 'active', startedAt: Date.now(), actions: [] };
		runs = [...runs, run];

		for (const step of scriptedRun) {
			timers.push(
				setTimeout(() => {
					runs = runs.map((candidate) =>
						candidate.runId === runId ? applyRunPatch(candidate, step.action) : candidate
					);

					if (step.action.type === 'finish' && step.action.outcome === 'succeeded') {
						messages = [
							...messages,
							{
								id: `m${Date.now()}`,
								authorKind: 'lexia',
								body: 'Done. Two sub-agents ran under it, and the artifacts they produced are below.',
								createdAt: Date.now(),
								runId,
								artifacts: [seededArtifacts.subagentTimeline, seededArtifacts.actionTable, seededArtifacts.tokenWidget]
							}
						];
					}
				}, step.at)
			);
		}
	}

	function cancel(runId: string) {
		stopTimers();
		runs = runs.map((run) => (run.runId === runId ? { ...run, status: 'cancelled' } : run));
	}

</script>

<section data-kit="chat-surface" aria-label="Chat" class={cn('flex min-h-0 flex-1 flex-col', className)}>
	<!-- The harness bar is the point of this page: every state the production chat can
	     reach is one control away, so it can be judged on the device you hold. -->
	<div class="flex flex-wrap items-center gap-(--kit-space-sm) border-b border-(--kit-border) bg-(--kit-card) px-(--kit-space-md) py-(--kit-space-sm) md:px-(--kit-space-xl)">
		<Button variant="primary" size="small" onclick={() => startRun('Simulated turn')} disabled={Boolean(liveRun)}>
			{#if liveRun}<Spinner size="sm" />{:else}
				<svg viewBox="0 0 16 16" class="size-3.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M8 4.5v7M5 9l3 3 3-3" /></svg>
			{/if}
			Simulate a run
		</Button>

		<label class="flex items-center gap-(--kit-space-xs) text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">
			<input
				type="checkbox"
				class="size-[14px] accent-(--kit-primary)"
				checked={!harness.eveUp}
				onchange={(event) => (harness = { ...harness, eveUp: !event.currentTarget.checked })}
			/>
			EVE offline
		</label>
		<label class="flex items-center gap-(--kit-space-xs) text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">
			<input
				type="checkbox"
				class="size-[14px] accent-(--kit-primary)"
				checked={!harness.bootstrapped}
				onchange={(event) => (harness = { ...harness, bootstrapped: !event.currentTarget.checked })}
			/>
			Show setup card
		</label>
		<label class="flex items-center gap-(--kit-space-xs) text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">
			<input
				type="checkbox"
				class="size-[14px] accent-(--kit-primary)"
				checked={harness.error !== null}
				onchange={(event) => (harness = { ...harness, error: event.currentTarget.checked ? 'EVE returned no reply.' : null })}
			/>
			Show error
		</label>
		<label class="flex items-center gap-(--kit-space-xs) text-[length:var(--kit-text-xs)] text-[color:var(--kit-muted-foreground)]">
			<input
				type="checkbox"
				class="size-[14px] accent-(--kit-primary)"
				checked={harness.emptyConversation}
				onchange={(event) => (harness = { ...harness, emptyConversation: event.currentTarget.checked })}
			/>
			Empty conversation
		</label>
	</div>

	<MessageScroller>
		{#if !harness.eveUp}
			<Alert variant="warning" title="EVE is not running">
				Nothing can answer until it is. Start it with <code class="font-mono">cd my-agent &amp;&amp; bunx eve dev</code> and send again.
			</Alert>
		{/if}

		{#if harness.error}
			<Alert variant="danger" title="That turn failed">{harness.error}</Alert>
		{/if}

		{#if !harness.bootstrapped}
			<!-- Setup stays reachable until it is done, not only on an empty chat: anyone who
			     sent a turn before finishing setup still needs the door. -->
			<div class="w-full rounded-(--kit-radius-lg) border border-(--kit-border) bg-(--kit-card) p-(--kit-space-lg)">
				<p class="mb-(--kit-space-xs) text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) font-bold tracking-[0.16em] text-[color:var(--kit-muted-foreground)]">GET STARTED</p>
				<p class="mb-(--kit-space-md) text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm) text-[color:var(--kit-muted-foreground)]">
					Name your agent, say what it is for, and give it a starting personality. Its memory builds itself from here.
				</p>
				<a
					href="/bootstrap"
					class="inline-flex h-[32px] items-center rounded-(--kit-radius-lg) border border-(--kit-primary) bg-(--kit-primary) px-[10px] text-[length:var(--kit-text-md)] font-medium text-[color:var(--kit-primary-foreground)]"
				>
					Set up your agent
				</a>
			</div>
		{/if}

		{#if visibleMessages.length === 0 && runs.length === 0}
			<div class="w-full rounded-(--kit-radius-lg) border border-(--kit-border) bg-(--kit-card) p-(--kit-space-lg)">
				<p class="mb-(--kit-space-xs) text-[length:var(--kit-text-xs)] leading-(--kit-leading-xs) font-bold tracking-[0.16em] text-[color:var(--kit-muted-foreground)]">LEXIA</p>
				<p class="text-[length:var(--kit-text-sm)] leading-(--kit-leading-sm) text-[color:var(--kit-muted-foreground)]">
					Nothing here yet. Ask for something and it becomes the first run.
				</p>
			</div>
		{:else}
			{#each visibleMessages as message (message.id)}
				<MessageItem {message} {hydrated} />
			{/each}
		{/if}

		{#each runs as run (run.runId)}
			<RunCard {run} oncancel={cancel} />
		{/each}
	</MessageScroller>

	<Composer
		{blockedReason}
		running={Boolean(liveRun)}
		onsend={send}
		onstop={() => liveRun && cancel(liveRun.runId)}
	>
		{#snippet hint()}
			<span>One conversation. It remembers.</span>
			<a href="/agents" class="underline underline-offset-2 hover:text-[color:var(--kit-foreground)]">Sub-agents</a>
			<span aria-hidden="true">·</span>
			<span><kbd class="rounded-(--kit-radius-sm) bg-(--kit-secondary) px-1">Enter</kbd> sends</span>
		{/snippet}
	</Composer>
</section>