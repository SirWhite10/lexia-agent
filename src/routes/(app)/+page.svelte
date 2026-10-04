<script lang="ts">
	import { deserialize, enhance } from '$app/forms';
	import { invalidateAll } from '$app/navigation';
	import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';
	import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
	import BotIcon from '@lucide/svelte/icons/bot';
	import FileAudioIcon from '@lucide/svelte/icons/file-audio';
	import FileIcon from '@lucide/svelte/icons/file';
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import MicIcon from '@lucide/svelte/icons/mic';
	import PaperclipIcon from '@lucide/svelte/icons/paperclip';
	import SquareIcon from '@lucide/svelte/icons/square';
	import XIcon from '@lucide/svelte/icons/x';
	import { scale } from 'svelte/transition';
	import TurnActivity from '#lib/components/features/chat/turn-activity.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import type { Attachment } from '#lib/server/attachments.js';

	let { data, form } = $props();

	// The massive chat: one continuous conversation for this installation. Run
	// progress arrives over SSE; the database is authoritative, so a refresh
	// replays from the last event id rather than losing anything.
	type RunAction = {
		id: string;
		capability: string;
		outcome: string;
		status: string;
		progress: string | null;
		error: string | null;
	};
	type RunView = {
		runId: string;
		messageId: string | null;
		status: string;
		createdAt: string;
		actions: RunAction[];
	};
	/** One ordered stream of everything that happened: the messages, and the work
	 * the agent did in between them. A run is created before its reply, so it
	 * lands above that reply — the reasoning sits where it belongs. */
	type TimelineEntry =
		| { kind: 'message'; at: string; message: (typeof data.messages)[number] }
		| { kind: 'run'; at: string; run: RunView };
	let activeRuns = $state<Record<string, RunView>>({});
	let draft = $state('');
	let lastEventId = 0;

	/** Five lines is the ceiling before the composer scrolls: a wall of text in
	 * the input hides the conversation the user is answering. */
	const MAX_COMPOSER_LINES = 5;

	let composer = $state<HTMLTextAreaElement | null>(null);
	let fileInput = $state<HTMLInputElement | null>(null);
	let pendingAttachments = $state<Attachment[]>([]);
	let uploadError = $state<string | null>(null);
	let isRecording = $state(false);
	// Deliberately not reactive: a MediaRecorder is a live browser handle, not
	// render state, and proxying it would be meaningless work on every tick.
	let recorder: MediaRecorder | null = null;

	/** Live stream state wins while a run is in flight; the loaded state is what
	 * survives a refresh. */
	let runViews = $derived.by<RunView[]>(() => {
		const streamed = new Map(Object.values(activeRuns).map((view) => [view.runId, view]));
		return data.runs.map(
			(run: { id: string; messageId: string | null; status: string; createdAt: string; actions: RunAction[] }) => {
				const live = streamed.get(run.id);
				return {
					runId: run.id,
					messageId: run.messageId,
					status: live?.status ?? run.status,
					createdAt: run.createdAt,
					actions: live?.actions ?? run.actions
				};
			}
		);
	});

	/** Work worth showing at all. A plain chat turn needs no entry: the reply is
	 * the record that it worked. */
	const ROUTINE_CAPABILITIES = new Set(['chat.reply', 'note.write']);

	function isWorthShowing(run: RunView): boolean {
		if (['active', 'waiting', 'planning'].includes(run.status)) return true;
		if (run.status === 'failed' || run.status === 'partially_complete') return true;
		return run.actions.some((action) => !ROUTINE_CAPABILITIES.has(action.capability));
	}

	function runTitle(run: RunView): string {
		const delegation = run.actions.find((action) => action.capability === 'subagent.delegate');
		if (delegation) return delegation.outcome;
		if (['active', 'waiting', 'planning'].includes(run.status)) return 'Working on it';
		return 'The request did not finish';
	}

	let timeline = $derived.by<TimelineEntry[]>(() => {
		const entries: TimelineEntry[] = [
			...data.messages.map((message) => ({ kind: 'message' as const, at: message.createdAt, message })),
			...runViews.filter(isWorthShowing).map((run) => ({ kind: 'run' as const, at: run.createdAt, run }))
		];
		return entries.sort((left, right) => left.at.localeCompare(right.at));
	});

	function subAgentName(authorId: string | null): string {
		if (!authorId) return 'A sub-agent';
		return data.subAgents?.find((agent) => agent.id === authorId)?.name ?? 'A sub-agent';
	}

	let activeRunId = $derived(
		Object.values(activeRuns).find((view) => view.status === 'active' || view.status === 'waiting')?.runId ?? null
	);
	let canSend = $derived(draft.trim().length > 0 || pendingAttachments.length > 0 || isRecording);

	async function streamRun(runId: string) {
		const source = new EventSource(`/api/runs/${runId}/events`);
		source.addEventListener('snapshot', (event) => {
			const snapshot = JSON.parse((event as MessageEvent).data);
			// The message the run belongs to comes from the load; the stream only
			// carries status and action progress.
			activeRuns[runId] = { runId, messageId: null, status: snapshot.run.status, createdAt: '', actions: snapshot.actions };
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
			// The run is durable: drop the streamed copy so the reloaded state is
			// authoritative, rather than a stale card outliving its own data.
			delete activeRuns[runId];
			invalidateAll();
		});
	}

	function applyActionEvent(runId: string, type: string, payload: Record<string, unknown>) {
		const view = activeRuns[runId] ?? { runId, messageId: null, status: 'active', actions: [] };
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
			view.actions.push({
				id: actionId,
				capability: String(payload.capability ?? ''),
				outcome: String(payload.outcome ?? 'Working…'),
				status,
				progress: null,
				error: null
			});
		}
		activeRuns[runId] = view;
	}

	function resizeComposer(): void {
		if (!composer) return;
		composer.style.height = 'auto';
		const lineHeight = Number.parseFloat(getComputedStyle(composer).lineHeight) || 20;
		composer.style.height = `${Math.min(composer.scrollHeight, lineHeight * MAX_COMPOSER_LINES)}px`;
	}

	function onComposerKeydown(event: KeyboardEvent): void {
		// Enter sends, Shift+Enter breaks the line, and an IME composing a
		// candidate must never be read as a send.
		if (event.key !== 'Enter' || event.shiftKey || event.isComposing) return;
		event.preventDefault();
		// One turn at a time. Without this, a second Enter while the first is still
		// waiting on EVE posts another message, which is how four keypresses became
		// four turns.
		if (canSend && !activeRunId && !sending) (event.currentTarget as HTMLTextAreaElement).form?.requestSubmit();
	}

	/** Uploads picked files and recorded audio through the same action, using the
	 * request shape `use:enhance` uses, so both paths share one server contract. */
	async function upload(files: File[]): Promise<void> {
		if (files.length === 0) return;
		uploadError = null;

		const payload = new FormData();
		for (const file of files) payload.append('files', file);

		try {
			const response = await fetch('?/attach', {
				method: 'POST',
				body: payload,
				headers: { accept: 'application/json', 'x-sveltekit-action': 'true' }
			});
			const result = deserialize(await response.text());
			if (result.type === 'success') {
				const saved = (result.data as { attachments?: Attachment[] } | undefined)?.attachments ?? [];
				pendingAttachments = [...pendingAttachments, ...saved];
			} else if (result.type === 'failure') {
				uploadError = (result.data as { error?: string } | undefined)?.error ?? 'That upload did not go through.';
			} else {
				uploadError = 'That upload did not go through.';
			}
		} catch {
			uploadError = 'That upload did not go through.';
		}
	}
	function onFilesChosen(event: Event): void {
		const input = event.currentTarget as HTMLInputElement;
		void upload([...(input.files ?? [])]);
		// Reset so re-picking the same file still fires a change event.
		input.value = '';
	}

	async function toggleRecording(): Promise<void> {
		if (isRecording) {
			recorder?.stop();
			return;
		}

		try {
			const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
			const chunks: Blob[] = [];
			const mediaRecorder = new MediaRecorder(stream);

			mediaRecorder.ondataavailable = (event) => {
				if (event.data.size > 0) chunks.push(event.data);
			};
			mediaRecorder.onstop = () => {
				for (const track of stream.getTracks()) track.stop();
				recorder = null;
				isRecording = false;
				const stamp = new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-');
				void upload([
					new File(chunks, `voice-note-${stamp}.webm`, { type: mediaRecorder.mimeType || 'audio/webm' })
				]);
			};

			mediaRecorder.start();
			recorder = mediaRecorder;
			isRecording = true;
		} catch {
			// A refused or missing microphone is expected in a browser that has
			// never been asked; say so instead of failing silently.
			uploadError = 'The microphone is not available. Allow microphone access and try again.';
		}
	}

	function formatBytes(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
		return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
	}

	/** A few pixels of slack, or ordinary rubber-banding at the end of the log
	 * would read as "the user scrolled away" and stop following new messages. */
	const BOTTOM_SLACK_PX = 64;

	let sending = $state(false);
	let pinnedToBottom = $state(true);

	function isAtBottom(): boolean {
		return window.scrollY + window.innerHeight >= document.documentElement.scrollHeight - BOTTOM_SLACK_PX;
	}

	function onWindowScroll(): void {
		pinnedToBottom = isAtBottom();
	}

	function jumpToLatest(): void {
		window.scrollTo({ top: document.documentElement.scrollHeight, behavior: 'smooth' });
	}

	$effect(() => {
		// Follow the conversation as it grows, but never yank the view away from
		// someone who has scrolled up to read something.
		void data.messages.length;
		void runViews.length;
		if (!pinnedToBottom) return;
		window.scrollTo({ top: document.documentElement.scrollHeight });
	});

	$effect(() => {
		// Depend on the draft so the composer grows with what is typed, and run
		// once after hydration to fit text restored by the browser.
		void draft;
		resizeComposer();
	});
</script>

<svelte:head>
	<title>Lexosa</title>
	<meta name="description" content="Lexosa and its standing sub-agents" />
</svelte:head>


<svelte:window onscroll={onWindowScroll} onresize={onWindowScroll} />
<section class="mx-auto flex w-full max-w-2xl flex-1 flex-col px-5 pb-2 pt-6" aria-label="Chat">
	<div class="mb-2 flex flex-wrap items-center justify-between gap-3">
		<p class="min-w-0 flex-1 text-sm text-muted-foreground">
			One conversation with Lexosa. It remembers, and it can spin up sub-agents.
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
					<p class="mb-1 text-xs font-bold tracking-[0.16em] text-secondary">LEXOSA</p>
					<p class="text-sm text-muted-foreground">
						Nothing here yet. Ask for something and it becomes the first run.
					</p>
				</div>
			{/if}
		{:else}
		<!-- Messages and the work between them, in the order they happened. -->
		{#each timeline as entry (entry.kind === 'message' ? entry.message.id : entry.run.runId)}
			{#if entry.kind === 'message'}
				{@const message = entry.message}
				{#if message.authorKind === 'user'}
					<div class="ml-auto flex max-w-[80%] flex-col items-end gap-1">
						{#if (data.attachments?.[message.id] ?? []).length > 0}
							<div class="flex flex-wrap justify-end gap-1">
								{#each data.attachments[message.id] as attachment (attachment.id)}
									<span
										class="inline-flex items-center gap-1 rounded-md border border-border bg-card px-2 py-1 text-xs text-muted-foreground"
									>
										{#if attachment.kind === 'audio'}
											<FileAudioIcon class="size-3" />
										{:else}
											<FileIcon class="size-3" />
										{/if}
										{attachment.name}
									</span>
								{/each}
							</div>
						{/if}
						<div class="rounded-lg bg-primary px-3 py-2 text-sm text-primary-foreground">
							{message.body}
						</div>
					</div>
				{:else if message.authorKind === 'sub_agent'}
					<!-- A sub-agent's finding is work the agent did, not a message to
					     read past: one line, with the detail a click away. -->
					<TurnActivity
						title={`${subAgentName(message.authorId)} reported back`}
						status="sub-agent"
						details={[{ label: 'Finding', value: message.body }]}
					/>
				{:else}
					<div class="flex items-start gap-2">
						<BotIcon class="mt-1 size-4 shrink-0 text-muted-foreground" />
						<div class="max-w-[80%] rounded-lg border border-border bg-card px-3 py-2 text-sm">
							{message.body}
						</div>
					</div>
				{/if}
			{:else}
				<TurnActivity
					title={runTitle(entry.run)}
					status={entry.run.status}
					details={entry.run.actions.map((action) => ({
						label: action.outcome,
						value: action.error ?? action.progress ?? action.status,
						failed: Boolean(action.error)
					}))}
				/>
			{/if}
		{/each}
		{/if}

	</div>

	<form
		method="POST"
		action="?/send"
		autocomplete="off"
		class="composer sticky bottom-0 z-10 mt-2"
		use:enhance={() => {
			return async ({ result, update }) => {
				// Marked before the first await: the round trip waits on EVE, and an
				// Enter pressed during it must not post a second turn.
				sending = true;
				try {
					const sentRunId =
						result.type === 'success' ? ((result.data as { runId?: string } | undefined)?.runId ?? null) : null;
					// Only a real send consumes the composer; cancelling a run must not
					// throw away what the user was about to say next.
					if (sentRunId) {
						draft = '';
						pendingAttachments = [];
						uploadError = null;
					}
					await update();
					if (sentRunId) streamRun(sentRunId);
				} finally {
					sending = false;
				}
			};
		}}
	>
		{#if !pinnedToBottom}
			<div class="mb-2 flex justify-center">
				<Button type="button" variant="secondary" size="sm" class="rounded-full" onclick={jumpToLatest}>
					<ArrowDownIcon class="size-3.5" />
					Jump to latest
				</Button>
			</div>
		{/if}
		<input
			type="hidden"
			name="attachmentIds"
			value={JSON.stringify(pendingAttachments.map((attachment) => attachment.id))}
		/>

		{#if pendingAttachments.length > 0 || isRecording}
			<div class="flex flex-wrap gap-1.5">
				{#each pendingAttachments as attachment (attachment.id)}
					<span class="composer-chip">
						{#if attachment.kind === 'audio'}
							<FileAudioIcon class="size-3" />
						{:else}
							<FileIcon class="size-3" />
						{/if}
						<span class="max-w-40 truncate">{attachment.name}</span>
						<span class="text-muted-foreground/70">{formatBytes(attachment.size)}</span>
						<button
							type="button"
							class="composer-chip-remove"
							aria-label={`Remove ${attachment.name}`}
							onclick={() => (pendingAttachments = pendingAttachments.filter((item) => item.id !== attachment.id))}
						>
							<XIcon class="size-3" />
						</button>
					</span>
				{/each}
				{#if isRecording}
					<span class="composer-chip">
						<span class="recording-dot" aria-hidden="true"></span>
						Recording…
					</span>
				{/if}
			</div>
		{/if}

		{#if uploadError}
			<p class="text-xs text-destructive" role="alert">{uploadError}</p>
		{/if}

		<div class="flex items-end gap-1.5">
			<Button
				type="button"
				variant="ghost"
				size="icon"
				aria-label="Attach files"
				title="Attach files"
				onclick={() => fileInput?.click()}
			>
				<PaperclipIcon class="size-4" />
			</Button>
			<!-- Kept outside the send form: a file input inside it would upload the
			     same bytes a second time when the message is sent. -->
			<input
				bind:this={fileInput}
				type="file"
				multiple
				class="sr-only"
				tabindex="-1"
				aria-hidden="true"
				onchange={onFilesChosen}
			/>

			<Textarea
				bind:ref={composer}
				bind:value={draft}
				name="body"
				rows={1}
				autocomplete="off"
				placeholder="Tell Lexosa to do something…"
				aria-label="Message"
				class="composer-field"
				oninput={resizeComposer}
				onkeydown={onComposerKeydown}
			/>

			<Button
				type="button"
				variant="ghost"
				size="icon"
				data-recording={isRecording ? '' : undefined}
				aria-label={isRecording ? 'Stop recording' : 'Record a voice note'}
				title={isRecording ? 'Stop recording' : 'Record a voice note'}
				onclick={toggleRecording}
			>
				<MicIcon class="size-4" />
			</Button>

			{#key activeRunId}
				<span class="send-swap" transition:scale={{ duration: 140, start: 0.6 }}>
					<Button
						type="submit"
						size="icon"
						class="send-button"
						formaction={activeRunId ? '?/cancel' : undefined}
						disabled={!activeRunId && !canSend}
						data-ready={!activeRunId && canSend ? '' : undefined}
						aria-label={activeRunId ? 'Stop responding' : 'Send message'}
						title={activeRunId ? 'Stop responding' : 'Send message'}
					>
					{#if activeRunId}
						<input type="hidden" name="runId" value={activeRunId} />
						<span class="relative grid size-4 place-items-center" aria-hidden="true">
							<LoaderCircleIcon class="absolute size-4 animate-spin opacity-60" />
							<SquareIcon class="size-1.5 fill-current" />
						</span>
					{:else}
						<ArrowUpIcon class="size-4" />
					{/if}
					</Button>
				</span>
			{/key}
		</div>
	</form>
</section>

<style>
	:global(:root) {
		/* Ready-to-send green, kept out of the pink/violet palette in DESIGN.md
		   because it has one job: saying the send button is live. */
		--composer-ready: oklch(0.72 0.17 152);
		--composer-ready-foreground: oklch(0.16 0.03 152);
	}

	.composer {
		display: grid;
		gap: 0.5rem;
		border: 1px solid var(--border);
		border-radius: 0.75rem;
		padding: 0.5rem;
		background: var(--card);
		/* The composer sticks to the bottom of the viewport, so it needs to read as
		   a layer above the conversation rather than as another message card. */
		box-shadow: 0 -0.75rem 1.25rem -0.75rem oklch(0.04 0 0 / 35%);
	}

	/* Class selectors handed to a component's `class` prop sit outside this
	   component's scope, so they are addressed globally rather than dropped as
	   dead. The base textarea grows itself with `field-sizing: content`; sizing is
	   done here so the five-line ceiling stays explicit. */
	:global(.composer-field) {
		min-height: 1.75rem;
		max-height: 7.5rem;
		field-sizing: fixed;
		resize: none;
		border: 0;
		padding: 0.35rem 0.25rem;
		background: transparent;
		box-shadow: none;
		overflow-y: auto;
	}

	.composer-chip {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		border: 1px solid var(--border);
		border-radius: 9999px;
		padding: 0.15rem 0.5rem;
		font-size: 0.75rem;
		color: var(--muted-foreground);
	}

	.composer-chip-remove {
		display: inline-grid;
		place-items: center;
		border: 0;
		padding: 0;
		background: transparent;
		color: inherit;
		cursor: pointer;
	}

	.composer-chip-remove:hover {
		color: var(--foreground);
	}

	.recording-dot {
		width: 0.5rem;
		height: 0.5rem;
		border-radius: 9999px;
		background: var(--destructive);
		animation: composer-pulse 1.2s ease-in-out infinite;
	}

	:global(button[data-recording]) {
		color: var(--destructive);
	}

	:global(.send-button) {
		transition:
			background-color 200ms ease,
			color 200ms ease,
			transform 160ms ease;
	}

	:global(.send-button[data-ready]) {
		background-color: var(--composer-ready);
		color: var(--composer-ready-foreground);
	}

	:global(.send-button[data-ready]:hover) {
		filter: brightness(1.06);
	}

	@keyframes composer-pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.2;
		}
	}
</style>