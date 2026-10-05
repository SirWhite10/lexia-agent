/**
 * Chat model for the kit test page.
 *
 * These shapes mirror the real chat (`src/routes/(app)/+page.server.ts` and its
 * SSE endpoint at `src/routes/api/runs/[id]/events`) rather than inventing a
 * convenient one. That is deliberate: this page is a UI test page, and a model
 * that drifts from production makes every observation here worthless. When the
 * server model changes, this file changes with it.
 */

import type { Artifact } from '../artifacts/types.js';

export type ChatAuthor = 'user' | 'lexia' | 'system';

export type ActionStatus = 'queued' | 'running' | 'succeeded' | 'failed' | 'cancelled';

export type RunStatus = 'active' | 'waiting' | 'succeeded' | 'failed' | 'cancelled';

export type RunAction = {
	id: string;
	outcome: string;
	/** Capability that will run it, e.g. `chat.reply`, `subagent.spawn`. Mirrors the server field. */
	capability: string;
	status: ActionStatus;
	/** Free-text progress from the run stream, e.g. "3/7 rows". */
	progress: string | null;
	error: string | null;
	/** Named sub-agent when the action delegated, so the fan-out is visible. */
	agent?: string;
	durationMs?: number;
	/** Actions a sub-agent performed, shown one level down. */
	children?: RunAction[];
};

export type RunView = {
	runId: string;
	title: string;
	status: RunStatus;
	startedAt: number;
	actions: RunAction[];
};

export type ChatAttachment = {
	name: string;
	kind: 'image' | 'file' | 'code';
	size: string;
	/** Set once the upload finishes; the attachment card greys out until then. */
	state?: 'uploading' | 'ready' | 'failed';
};

export type ChatMessage = {
	id: string;
	authorKind: ChatAuthor;
	body: string;
	createdAt: number;
	attachments?: ChatAttachment[];
	/** Artifacts render inline directly beneath the message that produced them. */
	artifacts?: Artifact[];
	/** Streaming replies set this while tokens arrive; the caret renders until it clears. */
	streaming?: boolean;
	/** The run that produced this message, so a message can be traced back to its tool calls. */
	runId?: string;
	/** Tools, skills, memory and agents added to the turn. Files are attachments, not context. */
	context?: Array<{ token: string; kind: string; label: string; detail?: string }>;
};

/** The states the real chat distinguishes, plus what the harness can force. */
export type ChatHarnessState = {
	/** Mirrors `eveUp` from the page load: EVE is not answering. */
	eveUp: boolean;
	/** Mirrors `bootstrapped`: the setup card stays reachable until setup is done. */
	bootstrapped: boolean;
	/** Mirrors `form.error` from a failed turn. */
	error: string | null;
	/** Clears the conversation log to its empty state. */
	emptyConversation: boolean;
};