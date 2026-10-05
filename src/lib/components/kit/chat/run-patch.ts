/**
 * Run patch application, kept out of the component so the harness's behaviour can be
 * read — and exercised — without rendering a chat.
 *
 * Every function here is pure and returns a new `RunView`. The chat surface replays
 * scripted patches against live run state, so an in-place mutation would leak between
 * the script and the rendered card and make a run look finished that is still running.
 */

import type { RunAction, RunView } from './model.js';

export type RunPatch =
	| { type: 'add'; action: RunAction }
	| { type: 'status'; id: string; status: RunAction['status']; progress?: string | null }
	| { type: 'finish'; outcome: 'succeeded' | 'failed' | 'cancelled' };

/** `add` appends; a repeated id replaces the existing action rather than duplicating it. */
function withAction(run: RunView, action: RunAction): RunView {
	const exists = run.actions.some((candidate) => candidate.id === action.id);
	return {
		...run,
		actions: exists ? run.actions.map((candidate) => (candidate.id === action.id ? action : candidate)) : [...run.actions, action]
	};
}

/** An unknown action id is ignored rather than creating a phantom row the user cannot explain. */
function withStatus(run: RunView, id: string, status: RunAction['status'], progress: string | null | undefined): RunView {
	if (!run.actions.some((candidate) => candidate.id === id)) return run;
	return {
		...run,
		actions: run.actions.map((action) =>
			action.id === id ? { ...action, status, progress: progress === undefined ? action.progress : progress } : action
		)
	};
}

export function applyRunPatch(run: RunView, patch: RunPatch): RunView {
	if (patch.type === 'finish') return { ...run, status: patch.outcome };
	if (patch.type === 'add') return withAction(run, patch.action);
	return withStatus(run, patch.id, patch.status, patch.progress);
}